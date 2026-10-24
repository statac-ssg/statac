//! Everything under a site's `assets/`, published under names made from
//! its content, and everything under `static/`, published as it is.
//!
//! Sass is compiled, and a stylesheet's addresses for other files in
//! `assets/` are written as the names those files are published under, so
//! each stylesheet is fingerprinted after the files it names. Scripts are
//! published as written: modules import each other by their own names, and
//! the import map a page carries sends those names to the published files.

mod address;
mod css;
mod fingerprint;
mod js;
mod proportion;
mod sass;

use std::borrow::Cow;
use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::highlight;
use crate::html::escaped;
use crate::platform::{self, Alike, Unwritable};
use crate::problem::{Hint, Kind, Named, Problem, Reach, Reason};
use crate::publish::Staging;
use crate::reads::{Key, Reads};
use crate::site::{Arena, AssetId};
use crate::source::{Place, Sources, Span};
use crate::suggest;

pub const ASSETS: &str = "assets";
pub const STATIC: &str = "static";

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AssetKind {
    Stylesheet,
    Script {
        module: bool,
        /// Whether it calls `import()`, which goes through the import map.
        loads_modules: bool,
    },
    Other,
}

#[derive(Debug)]
pub struct Asset {
    /// The name a template asks for it by: its path under `assets/`, a Sass
    /// file named as the stylesheet it becomes.
    pub name: String,
    /// Its path in the published site, fingerprint and all.
    pub published: String,
    /// The file it was made from, under the site's folder.
    pub source: String,
    pub kind: AssetKind,
    bytes: Vec<u8>,
}

impl Asset {
    /// The bytes as published.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug)]
pub struct StaticFile {
    /// Its path under `static/`, which is its path in the published site.
    pub name: String,
    at: String,
}

impl StaticFile {
    /// Where it really is: its path past every link under
    /// [`Assets::real`], to be read by [`platform::open_file`], which
    /// follows no link.
    pub fn at(&self) -> &str {
        &self.at
    }
}

/// A script published with the site, and the file it came from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Travelled {
    pub address: String,
    pub from: String,
}

/// What one page has been handed so far.
#[derive(Default, Debug)]
pub struct PageScripts {
    mapped: bool,
}

pub struct Assets {
    /// The site's own path, such as `/blog`, or empty at the top of its domain.
    base: String,
    assets: Arena<AssetId, Asset>,
    names: BTreeMap<String, AssetId>,
    /// Stylesheets Sass stopped on, which already have their problem.
    unwritten: BTreeSet<String>,
    statics: Vec<StaticFile>,
    static_names: HashSet<String>,
    /// The site's real path, which each file in `static/` really is under.
    real: PathBuf,
    /// The warning for a rule that narrows pictures and keeps their height,
    /// true only of a site whose pages show a picture with both.
    narrowed: Option<Problem>,
}

/// Reads, compiles and fingerprints everything under `site`'s `assets/`,
/// and finds everything under its `static/`. `base` is the path the site
/// is published under, empty at the top of its domain. `code_style` names
/// the colours of the `code.css` Statac writes beside them, unless
/// `assets/` has a `code.css` of its own.
pub fn gather(
    site: &Path,
    base: &str,
    code_style: Option<&str>,
    sources: &mut Sources,
) -> (Assets, Vec<Problem>) {
    let base = match base.trim_matches('/') {
        "" => String::new(),
        trimmed => format!("/{trimmed}"),
    };
    let mut problems = Vec::new();
    let real = fs::canonicalize(site).unwrap_or_else(|_| site.to_path_buf());
    let walked = walk(site, &real, ASSETS, false, &mut problems);
    // Sass is read as its author wrote it, the parts of a stylesheet too.
    let written: Vec<(String, String)> = walked
        .iter()
        .filter(|(rel, _)| split_extension(rel).1.eq_ignore_ascii_case("scss"))
        .cloned()
        .collect();
    let wanted: Vec<Wanted> = walked
        .into_iter()
        .filter_map(|(rel, at)| Wanted::new(&site.join(ASSETS), rel, at))
        .collect();
    let unsaved = sources.unsaved();
    let warnings = std::sync::Mutex::new(Vec::new());
    let read: Vec<Result<Vec<u8>, Failure>> = wanted
        .par_iter()
        .map(|wanted| match wanted.sass {
            true => {
                let compiled = sass::compile(&wanted.path, &site.join(ASSETS), unsaved);
                warnings
                    .lock()
                    .expect("no compile panics holding it")
                    .extend(compiled.warnings);
                match compiled.css {
                    Ok(css) => Ok(css.into_bytes()),
                    Err(error) => Err(Failure::Sass(error, compiled.refused)),
                }
            }
            false => unsaved.read_bytes(&real, &wanted.at).map_err(Failure::Io),
        })
        .collect();

    // A partial two stylesheets use warns once for each, and is told once.
    let mut warnings = warnings
        .into_inner()
        .expect("no compile panicked holding it");
    warnings.sort_by(|(a, _), (b, _)| {
        (a.file.name(), a.begin.line, a.begin.column).cmp(&(
            b.file.name(),
            b.begin.line,
            b.begin.column,
        ))
    });
    warnings.dedup_by(|(a, said_a), (b, said_b)| {
        said_a == said_b && a.file.name() == b.file.name() && a.begin == b.begin
    });
    for (loc, message) in &warnings {
        problems.push(sass::warned(loc, message, site, sources));
    }

    let narrowed = narrowed(&wanted, &read, &real, &written, sources);

    let mut files = Vec::new();
    let mut claimed: HashMap<String, String> = HashMap::new();
    let mut unwritten = BTreeSet::new();
    for (wanted, read) in wanted.into_iter().zip(read) {
        if let Some(first) = claimed.get(&wanted.name) {
            problems.push(Problem::nowhere(Kind::SameUrl {
                first: Box::new(Named::File(first.clone())),
                second: Box::new(Named::File(wanted.source)),
                alike: Alike::Exactly,
                address: format!("{base}/{}", wanted.name),
            }));
            continue;
        }
        claimed.insert(wanted.name.clone(), wanted.source.clone());
        match read {
            Ok(bytes) => {
                let (kind, workers) = classify(&wanted.name, &bytes);
                files.push(File {
                    name: wanted.name,
                    source: wanted.source,
                    bytes,
                    kind,
                    workers,
                });
            }
            Err(Failure::Io(error)) => problems.push(Problem::nowhere(Kind::CantRead {
                path: wanted.source.into(),
                reason: Reason::of(&error),
            })),
            Err(Failure::Sass(error, refused)) => {
                problems.push(sass::stopped(*error, refused, &wanted.path, site, sources));
                unwritten.insert(wanted.name);
            }
        }
    }
    if let Some(css) = code_style
        .filter(|_| !claimed.contains_key(highlight::STYLESHEET))
        .and_then(highlight::stylesheet)
    {
        files.push(File {
            name: highlight::STYLESHEET.to_owned(),
            source: crate::config::FILE_NAME.to_owned(),
            bytes: css.into_bytes(),
            kind: AssetKind::Stylesheet,
            workers: Vec::new(),
        });
    }

    let published = fingerprinted(&mut files, &base);
    let mut assets = Arena::default();
    let mut names = BTreeMap::new();
    let mut workers = Vec::new();
    for (file, published) in files.into_iter().zip(published) {
        let id = assets.push(Asset {
            name: file.name.clone(),
            published,
            source: file.source,
            kind: file.kind,
            bytes: file.bytes,
        });
        names.insert(file.name, id);
        workers.push((id, file.workers));
    }
    for (id, named) in workers {
        problems.extend(workers_named(
            assets.get(id),
            &named,
            &names,
            &base,
            sources,
        ));
    }

    let statics: Vec<StaticFile> = walk(site, &real, STATIC, true, &mut problems)
        .into_iter()
        .map(|(name, at)| StaticFile { name, at })
        .collect();
    let outputs = claims(&assets, &statics);
    for (rel, source) in &outputs {
        if let Some(why) = platform::unwritable(rel) {
            problems.push(Problem::nowhere(Kind::UnwritableName {
                path: source.clone(),
                why,
            }));
        }
    }
    let outputs: Vec<(String, Named)> = outputs
        .into_iter()
        .map(|(rel, source)| (rel, Named::File(source)))
        .collect();
    problems.extend(same_address(&outputs, &base));

    let static_names = statics.iter().map(|file| file.name.clone()).collect();
    let gathered = Assets {
        base,
        assets,
        names,
        unwritten,
        statics,
        static_names,
        real,
        narrowed,
    };
    (gathered, problems)
}

/// The warning for the first rule in the site's stylesheets that gives
/// `img` a width, where no rule says what a picture's height is. `read` is
/// what became of each of `wanted`, and `written` the Sass files as their
/// author wrote them, as [`walk`] found them under the site's real path
/// `real`.
fn narrowed(
    wanted: &[Wanted],
    read: &[Result<Vec<u8>, Failure>],
    real: &Path,
    written: &[(String, String)],
    sources: &mut Sources,
) -> Option<Problem> {
    let mut compiled_narrows = false;
    // Each stylesheet a rule can be shown in, with whether it is Sass.
    let mut shown: Vec<(String, Cow<str>, bool)> = Vec::new();
    for (wanted, read) in wanted.iter().zip(read) {
        let is_css = split_extension(&wanted.name).1.eq_ignore_ascii_case("css");
        let Some(css) = read.as_ref().ok().filter(|_| is_css) else {
            continue;
        };
        let Ok(css) = std::str::from_utf8(css) else {
            continue;
        };
        match wanted.sass {
            // What a mixin or a function adds to a rule is only in the CSS
            // Sass makes.
            true => {
                let said = proportion::said(css);
                if said.heights {
                    return None;
                }
                compiled_narrows |= said.narrows.is_some();
            }
            false => shown.push((wanted.source.clone(), Cow::Borrowed(css), false)),
        }
    }
    for (rel, at) in written {
        if let Ok(text) = sources.unsaved().read(real, at) {
            shown.push((format!("{ASSETS}/{rel}"), Cow::Owned(text), true));
        }
    }
    shown.sort();
    let mut first = None;
    for (at, (_, text, sass)) in shown.iter().enumerate() {
        let said = proportion::said(text);
        if said.heights {
            return None;
        }
        // A rule Sass never writes out narrows nothing.
        if first.is_none()
            && (!sass || compiled_narrows)
            && let Some(span) = said.narrows
        {
            first = Some((at, span));
        }
    }
    let (at, span) = first?;
    let (path, text, _) = shown.swap_remove(at);
    let property = text[span.clone()].to_owned();
    let source = sources.add(path, text.into_owned());
    Some(Problem::at(
        Kind::ImagesOutOfProportion { property },
        Place {
            source,
            span: Span::new(span.start, span.end),
        },
    ))
}

impl Assets {
    /// The address of a file in `assets/`, for a tag written by hand:
    /// `asset("font.woff2")` is `/font.1a2b3c4d.woff2`. An address
    /// on another site is given back as it is.
    pub fn asset(&self, name: &str, reads: &mut Reads) -> Result<String, Kind> {
        if elsewhere(name) {
            return Ok(name.to_owned());
        }
        Ok(match self.lookup(name, reads)? {
            Found::Asset(asset) => self.address(asset),
            Found::Unwritten(address) => address,
        })
    }

    /// A stylesheet's tag: `<link rel="stylesheet" href="...">`.
    pub fn css(&self, name: &str, reads: &mut Reads) -> Result<String, Kind> {
        let href = self.asset(name, reads)?;
        Ok(format!(
            "<link rel=\"stylesheet\" href=\"{}\">",
            escaped(&href)
        ))
    }

    /// A script's tag: `<script type="module" src="..."></script>` for a
    /// module, `<script src="..." defer></script>` otherwise. The first
    /// script on a page that loads modules brings the import map with it,
    /// ahead of its own tag.
    pub fn js(
        &self,
        name: &str,
        page: &mut PageScripts,
        reads: &mut Reads,
    ) -> Result<String, Kind> {
        let (src, kind) = match elsewhere(name) {
            true => (name.to_owned(), AssetKind::Other),
            false => match self.lookup(name, reads)? {
                Found::Asset(asset) => (self.address(asset), asset.kind),
                Found::Unwritten(address) => (address, AssetKind::Other),
            },
        };
        let (module, loads_modules) = match kind {
            AssetKind::Script {
                module,
                loads_modules,
            } => (module, loads_modules),
            _ => (false, false),
        };
        let mut out = String::new();
        if (module || loads_modules) && !page.mapped {
            page.mapped = true;
            out.push_str(&self.import_map(reads));
            out.push('\n');
        }
        let src = escaped(&src);
        match module {
            true => write!(out, "<script type=\"module\" src=\"{src}\"></script>"),
            false => write!(out, "<script src=\"{src}\" defer></script>"),
        }
        .expect("writing to a string can't fail");
        Ok(out)
    }

    /// The page's import map: one entry for every script in `assets/`,
    /// from the address it is written by to the address it is published at.
    pub fn import_map(&self, reads: &mut Reads) -> String {
        let mut out = String::from("<script type=\"importmap\">\n{\n  \"imports\": {");
        let mut first = true;
        for (name, &id) in &self.names {
            let asset = self.assets.get(id);
            if !matches!(asset.kind, AssetKind::Script { .. }) {
                continue;
            }
            reads.read(Key::Asset(id));
            out.push_str(if first { "\n" } else { ",\n" });
            first = false;
            let written = format!("{}/{name}", self.base);
            write!(
                out,
                "    {}: {}",
                crate::json::string(&written),
                crate::json::string(&self.address(asset))
            )
            .expect("writing to a string can't fail");
        }
        out.push_str("\n  }\n}\n</script>");
        out
    }

    /// Every script the site publishes, from `assets/` and `static/`, and
    /// where each came from.
    pub fn travelled(&self) -> Vec<Travelled> {
        let from_assets = self
            .names
            .values()
            .map(|&id| self.assets.get(id))
            .filter(|asset| matches!(asset.kind, AssetKind::Script { .. }))
            .map(|asset| Travelled {
                address: self.address(asset),
                from: asset.source.clone(),
            });
        let from_static = self
            .statics
            .iter()
            .filter(|file| is_script(&file.name))
            .map(|file| Travelled {
                address: format!("{}/{}", self.base, file.name),
                from: format!("{STATIC}/{}", file.name),
            });
        from_assets.chain(from_static).collect()
    }

    /// Writes every asset and copies everything in `static/`, all at the
    /// top of the site.
    pub fn publish(&self, staging: &Staging) -> Vec<Problem> {
        let assets: Vec<&Asset> = self.assets.iter().map(|(_, asset)| asset).collect();
        let written = assets
            .par_iter()
            .filter_map(|asset| staging.write(&asset.published, &asset.bytes).err());
        let copied = self.statics.par_iter().filter_map(|file| {
            let shown = format!("{STATIC}/{}", file.name);
            let source = platform::open_file(&self.real, &file.at);
            staging.copy(source, &shown, &file.name).err()
        });
        written.chain(copied).collect()
    }

    /// Every path this publishes in the site, with the file it comes
    /// from, for checking against what else the build publishes.
    pub fn claims(&self) -> Vec<(String, Named)> {
        claims(&self.assets, &self.statics)
            .into_iter()
            .map(|(rel, source)| (rel, Named::File(source)))
            .collect()
    }

    /// The stylesheets Sass stopped on, each by the path it would have
    /// been published at, which a page may still link.
    pub fn unwritten(&self) -> impl Iterator<Item = &str> {
        self.unwritten.iter().map(String::as_str)
    }

    pub fn get(&self, name: &str) -> Option<&Asset> {
        self.names.get(name).map(|&id| self.assets.get(id))
    }

    /// The asset a template asks for as `name`, as a read of it is recorded.
    pub fn id(&self, name: &str) -> Option<AssetId> {
        self.names.get(name).copied()
    }

    /// The name of every asset, as a template asks for it.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.names.keys().map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (AssetId, &Asset)> {
        self.assets.iter()
    }

    pub fn statics(&self) -> &[StaticFile] {
        &self.statics
    }

    /// The site's real path, which each file in `static/` really is under.
    pub fn real(&self) -> &Path {
        &self.real
    }

    /// The warning for a stylesheet rule that narrows pictures and keeps
    /// their height, to be told where a built page shows a picture given
    /// its width and height.
    pub fn narrowed(&self) -> Option<&Problem> {
        self.narrowed.as_ref()
    }

    fn address(&self, asset: &Asset) -> String {
        format!("{}/{}", self.base, asset.published)
    }

    fn lookup(&self, name: &str, reads: &mut Reads) -> Result<Found<'_>, Kind> {
        let wanted = name.trim_start_matches('/');
        if let Some(&id) = self.names.get(wanted) {
            reads.read(Key::Asset(id));
            return Ok(Found::Asset(self.assets.get(id)));
        }
        // Sass has already said why this stylesheet isn't there, and the
        // build stops for it, so the page needn't say so again.
        if self.unwritten.contains(wanted) {
            return Ok(Found::Unwritten(format!("{}/{wanted}", self.base)));
        }
        let hint = if self.static_names.contains(wanted) {
            Some(Hint::InStatic(format!("{}/{wanted}", self.base)))
        } else if let Some(under) = wanted.strip_prefix("assets/")
            && self.names.contains_key(under)
        {
            Some(Hint::Closest(under.to_owned()))
        } else {
            suggest::nearest(wanted, self.names.keys().map(String::as_str))
                .map(|name| Hint::Closest(name.to_owned()))
        };
        Err(Kind::MissingFile {
            path: format!("{ASSETS}/{wanted}"),
            hint,
        })
    }
}

enum Found<'a> {
    Asset(&'a Asset),
    /// A stylesheet Sass stopped on, at the address it would have had.
    Unwritten(String),
}

/// A file in `assets/` to publish.
struct Wanted {
    name: String,
    source: String,
    /// Its path in the site's folder, links and all, as Sass reads it.
    path: PathBuf,
    /// Where it really is, as [`walk`] gives it.
    at: String,
    sass: bool,
}

impl Wanted {
    /// The file at `rel` in the folder `assets`, really at `at`. `None`
    /// for a Sass partial, which is only ever part of another stylesheet.
    fn new(assets: &Path, rel: String, at: String) -> Option<Wanted> {
        let (stem, extension) = split_extension(&rel);
        let sass = matches!(extension.to_ascii_lowercase().as_str(), "scss" | "sass");
        let file_name = rel.rsplit('/').next().unwrap_or(&rel);
        if sass && file_name.starts_with('_') {
            return None;
        }
        Some(Wanted {
            name: match sass {
                true => format!("{stem}.css"),
                false => rel.clone(),
            },
            source: format!("{ASSETS}/{rel}"),
            path: platform::join(assets, &rel),
            at,
            sass,
        })
    }
}

enum Failure {
    Io(io::Error),
    /// With whether Sass reached outside `assets/`.
    Sass(Box<grass::Error>, bool),
}

struct File {
    name: String,
    source: String,
    bytes: Vec<u8>,
    kind: AssetKind,
    workers: Vec<Range<usize>>,
}

fn classify(name: &str, bytes: &[u8]) -> (AssetKind, Vec<Range<usize>>) {
    match split_extension(name).1.to_ascii_lowercase().as_str() {
        "css" => (AssetKind::Stylesheet, Vec::new()),
        "js" | "mjs" => {
            let script = std::str::from_utf8(bytes).map(js::read).unwrap_or_default();
            let kind = AssetKind::Script {
                module: script.module,
                loads_modules: script.loads_modules,
            };
            (kind, script.workers)
        }
        _ => (AssetKind::Other, Vec::new()),
    }
}

/// Each file's published name. Stylesheets have their addresses for other
/// files rewritten first, so their fingerprints cover the names they hold;
/// stylesheets that name each other in a ring share one fingerprint.
fn fingerprinted(files: &mut [File], base: &str) -> Vec<String> {
    let index: HashMap<&str, usize> = files
        .iter()
        .enumerate()
        .map(|(i, file)| (file.name.as_str(), i))
        .collect();
    let references: Vec<Vec<(Range<usize>, usize)>> = files
        .iter()
        .map(|file| match file.kind {
            AssetKind::Stylesheet => css::references(&file.bytes)
                .into_iter()
                .filter_map(|range| {
                    let written = std::str::from_utf8(&file.bytes[range.clone()]).ok()?;
                    let named = address::settled(&file.name, written, base)?;
                    index.get(named.as_str()).map(|&to| (range, to))
                })
                .collect(),
            _ => Vec::new(),
        })
        .collect();
    let edges: Vec<Vec<usize>> = references
        .iter()
        .map(|found| found.iter().map(|&(_, to)| to).collect())
        .collect();

    let mut published: Vec<Option<String>> = vec![None; files.len()];
    let rewrite = |i: usize, published: &[Option<String>]| -> Option<Vec<u8>> {
        if references[i].is_empty() {
            return None;
        }
        let bytes = &files[i].bytes;
        let replacements: Vec<(Range<usize>, String)> = references[i]
            .iter()
            .filter_map(|(range, to)| {
                let to = published[*to].as_ref()?;
                let written = std::str::from_utf8(&bytes[range.clone()]).ok()?;
                Some((range.clone(), address::respelled(written, to)))
            })
            .collect();
        Some(css::rewritten(bytes, &replacements))
    };
    let mut rewritten: Vec<Option<Vec<u8>>> = vec![None; files.len()];
    for ring in fingerprint::rings(&edges) {
        if let [i] = ring[..]
            && !edges[i].contains(&i)
        {
            let bytes = rewrite(i, &published);
            let mark = fingerprint::mark(bytes.as_deref().unwrap_or(&files[i].bytes));
            published[i] = Some(fingerprint::marked(&files[i].name, &mark));
            rewritten[i] = bytes;
            continue;
        }
        // The names from outside the ring are written; the ring's own are
        // still as the author wrote them.
        let mut whole = Vec::new();
        for &i in &ring {
            whole.extend_from_slice(files[i].name.as_bytes());
            whole.push(0);
            let bytes = rewrite(i, &published);
            whole.extend_from_slice(bytes.as_deref().unwrap_or(&files[i].bytes));
            whole.push(0);
        }
        let mark = fingerprint::mark(&whole);
        for &i in &ring {
            published[i] = Some(fingerprint::marked(&files[i].name, &mark));
        }
        for &i in &ring {
            rewritten[i] = rewrite(i, &published);
        }
    }
    for (file, bytes) in files.iter_mut().zip(rewritten) {
        if let Some(bytes) = bytes {
            file.bytes = bytes;
        }
    }
    published
        .into_iter()
        .map(|name| name.expect("every file is in a ring"))
        .collect()
}

/// A warning for each worker a script names by a file in `assets/`.
fn workers_named(
    script: &Asset,
    named: &[Range<usize>],
    names: &BTreeMap<String, AssetId>,
    base: &str,
    sources: &mut Sources,
) -> Vec<Problem> {
    let Ok(text) = std::str::from_utf8(&script.bytes) else {
        return Vec::new();
    };
    let mut source = None;
    let mut problems = Vec::new();
    for range in named {
        let written = &text[range.clone()];
        let Some(target) = address::settled(&script.name, written, base) else {
            continue;
        };
        if !names.contains_key(&target) {
            continue;
        }
        let source = *source.get_or_insert_with(|| sources.add(&script.source, text.to_owned()));
        problems.push(Problem::at(
            Kind::WorkerOutsideImportMap {
                written: written.to_owned(),
            },
            Place {
                source,
                span: Span::new(range.start, range.end),
            },
        ));
    }
    problems
}

fn claims(assets: &Arena<AssetId, Asset>, statics: &[StaticFile]) -> Vec<(String, String)> {
    let from_assets = assets
        .iter()
        .map(|(_, asset)| (asset.published.clone(), asset.source.clone()));
    let from_static = statics
        .iter()
        .map(|file| (file.name.clone(), format!("{STATIC}/{}", file.name)));
    from_assets.chain(from_static).collect()
}

/// Two outputs a disk would hold as one: the same file, or a file and a
/// folder, or one folder spelled two ways. `outputs` pairs each output's
/// path in the published site with the file it comes from, so pages can
/// be checked against assets and static files by adding them to
/// [`Assets::claims`].
pub fn same_address(outputs: &[(String, Named)], base: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut told = HashSet::new();
    let mut tell = |first: &Named, second: &Named, alike: Alike, rel: &str| {
        // Two photographs' copies meet again in the other order.
        if !told.contains(&(second.clone(), first.clone()))
            && told.insert((first.clone(), second.clone()))
        {
            let at = second.given_at().or(first.given_at());
            let kind = Kind::SameUrl {
                first: Box::new(first.clone()),
                second: Box::new(second.clone()),
                alike,
                address: format!("{base}/{rel}"),
            };
            problems.push(Problem::placed(kind, at));
        }
    };
    let mut files: BTreeMap<String, usize> = BTreeMap::new();
    let mut folders: BTreeMap<String, (String, Named)> = BTreeMap::new();
    // The folders whose every folder above is held under this spelling.
    // Another file in one finds each as it is and tells nothing.
    let mut settled: HashSet<&str> = HashSet::new();
    for (i, (rel, source)) in outputs.iter().enumerate() {
        match files.entry(platform::folded(rel)) {
            Entry::Occupied(first) => {
                let (first_rel, first_source) = &outputs[*first.get()];
                tell(first_source, source, platform::alike(first_rel, rel), rel);
            }
            Entry::Vacant(free) => {
                free.insert(i);
            }
        }
        let above = rel.rsplit_once('/').map_or("", |(above, _)| above);
        if settled.contains(above) {
            continue;
        }
        let mut respelt = false;
        let pieces: Vec<&str> = rel.split('/').collect();
        let source_pieces: Vec<&str> = match source {
            Named::File(path) => path.split('/').collect(),
            _ => Vec::new(),
        };
        for depth in 1..pieces.len() {
            let spelled = pieces[..depth].join("/");
            let kept = source_pieces.len().saturating_sub(pieces.len() - depth);
            // A folder is named by its source folder only where the source's
            // folders mirror the output's, as `static/` and `assets/` do; a
            // page's file doesn't.
            let folders_of = |p: &[&str]| p.iter().map(|f| platform::folded(f)).collect::<Vec<_>>();
            let mirrors = kept > 0
                && folders_of(&source_pieces[kept - 1..source_pieces.len() - 1])
                    == folders_of(&pieces[depth - 1..pieces.len() - 1]);
            let from = match mirrors {
                true => Named::File(source_pieces[..kept].join("/")),
                false => source.clone(),
            };
            match folders.get(&platform::folded(&spelled)) {
                Some((first_spelled, first_from)) if *first_spelled != spelled => {
                    let alike = platform::alike(first_spelled, &spelled);
                    tell(first_from, &from, alike, &spelled);
                    respelt = true;
                }
                Some(_) => {}
                None => {
                    folders.insert(platform::folded(&spelled), (spelled, from));
                }
            }
        }
        if !respelt {
            settled.insert(above);
        }
    }
    for (key, &i) in &files {
        if let Some((spelled, from)) = folders.get(key) {
            let (rel, source) = &outputs[i];
            tell(from, source, platform::alike(spelled, rel), rel);
        }
    }
    problems
}

/// Every file under the site's folder `shown`, by its path under it with
/// forward slashes, sorted, each with where it really is: its path past
/// every link under the site's real path `real`. Hidden names are passed
/// over unless `hidden` asks for them. The folder itself must be inside
/// the site, and a link inside it is followed only where it leads
/// somewhere inside the folder. Read where it really is, as
/// [`read_walked`] reads it, a file is reached through no link, so a link
/// put in its place or a folder's on the way after the walk is refused.
pub(crate) fn walk(
    site: &Path,
    real: &Path,
    shown: &str,
    hidden: bool,
    problems: &mut Vec<Problem>,
) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut folders = vec![(String::new(), site.join(shown))];
    // A link back up the tree would otherwise be walked for ever.
    let mut seen = HashSet::new();
    let shown_as = |rel: &str| match rel.is_empty() {
        true => shown.to_owned(),
        false => format!("{shown}/{rel}"),
    };
    let cant_read = |rel: &str, error: &io::Error| {
        Problem::nowhere(Kind::CantRead {
            path: PathBuf::from(shown_as(rel)),
            reason: Reason::of(error),
        })
    };
    // Every link must stay within the folder where it sits in the site: a
    // link in `static/` to the site's settings would publish them. The
    // bound is the site's real path and the folder's name, never where the
    // folder leads, so a folder that is itself a link out is refused too.
    let within = real.join(shown);
    let outside = |rel: &str, target: &Path| {
        Problem::nowhere(Kind::OutsideSite {
            reach: Reach::Link {
                path: shown_as(rel),
                within: (!rel.is_empty() || target.starts_with(real)).then(|| format!("{shown}/")),
            },
        })
    };
    while let Some((rel, folder)) = folders.pop() {
        let real_folder = match fs::canonicalize(&folder) {
            Ok(target) if !target.starts_with(&within) => {
                problems.push(outside(&rel, &target));
                continue;
            }
            Ok(target) if !seen.insert(target.clone()) => continue,
            Ok(target) => target,
            Err(error) if rel.is_empty() && error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => {
                problems.push(cant_read(&rel, &error));
                continue;
            }
        };
        let entries = match fs::read_dir(&folder).and_then(|entries| entries.collect()) {
            Ok(entries) => entries,
            Err(error) => {
                problems.push(cant_read(&rel, &error));
                continue;
            }
        };
        let mut entries: Vec<fs::DirEntry> = entries;
        entries.sort_by_key(fs::DirEntry::file_name);
        // Asked of many at once: a folder of ten thousand posts is asked
        // in a third of the time.
        let found_as: Vec<io::Result<fs::Metadata>> = entries
            .par_iter()
            .with_min_len(64)
            .map(|entry| fs::metadata(entry.path()))
            .collect();
        for (entry, found_as) in entries.into_iter().zip(found_as) {
            let file_name = entry.file_name();
            let joined = |name: &str| match rel.is_empty() {
                true => name.to_owned(),
                false => format!("{rel}/{name}"),
            };
            let Some(name) = file_name.to_str() else {
                problems.push(Problem::nowhere(Kind::UnwritableName {
                    path: shown_as(&joined(&file_name.to_string_lossy())),
                    why: Unwritable::NotText,
                }));
                continue;
            };
            if !hidden && name.starts_with('.') {
                continue;
            }
            let child = joined(name);
            let path = entry.path();
            let link = entry.file_type().is_ok_and(|kind| kind.is_symlink());
            let target = match link {
                true => match fs::canonicalize(&path) {
                    Ok(target) if target.starts_with(&within) => target,
                    Ok(target) => {
                        problems.push(outside(&child, &target));
                        continue;
                    }
                    Err(error) => {
                        problems.push(cant_read(&child, &error));
                        continue;
                    }
                },
                false => real_folder.join(name),
            };
            match found_as {
                Ok(metadata) if metadata.is_dir() => folders.push((child, path)),
                Ok(_) => match platform::slashed(target.strip_prefix(real).unwrap_or(&target)) {
                    Ok(at) => found.push((child, at)),
                    // Only through a link: the walk refuses such a name
                    // where it meets one.
                    Err(lossy) => problems.push(Problem::nowhere(Kind::UnwritableName {
                        path: lossy,
                        why: Unwritable::NotText,
                    })),
                },
                Err(error) => problems.push(cant_read(&child, &error)),
            }
        }
    }
    found.sort();
    found
}

/// The bytes of the file [`walk`] found at `at`, under the site's real
/// path `real`.
pub(crate) fn read_walked(real: &Path, at: &str) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    io::Read::read_to_end(&mut platform::open_file(real, at)?, &mut bytes)?;
    Ok(bytes)
}

/// `name` without and with its file's extension: `css/site.scss` is
/// `css/site` and `scss`.
fn split_extension(name: &str) -> (&str, &str) {
    let file_start = name.rfind('/').map_or(0, |slash| slash + 1);
    match name[file_start..].rfind('.').filter(|&dot| dot > 0) {
        Some(dot) => (&name[..file_start + dot], &name[file_start + dot + 1..]),
        None => (name, ""),
    }
}

pub fn is_script(name: &str) -> bool {
    matches!(
        split_extension(name).1.to_ascii_lowercase().as_str(),
        "js" | "mjs" | "cjs"
    )
}

/// An address on another site, which a template may hand through as it is.
fn elsewhere(name: &str) -> bool {
    name.starts_with("//") || name.contains("://")
}

#[cfg(test)]
mod tests;
