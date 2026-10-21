//! The JavaScript the built site carries, and the file each piece is told
//! of: the script published, the address loaded, the file that wrote it
//! into a page, or what is embedded that may run its own.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use rayon::prelude::*;

use super::scripts::{self, Finding, Found};
use super::{Built, Output, site_origin};
use crate::assets;
use crate::problem::Carried;
use crate::reads::Key;
use crate::site::PageId;
use crate::source::SourceId;
use crate::templates::Renderer;

/// One kind of JavaScript the built pages carry, and on how many pages.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Javascript {
    pub carried: Carried,
    /// Whether a browser runs it; where not, it is somewhere a browser may
    /// or may not run it, such as a comment, and counted all the same.
    pub sure: bool,
    pub pages: usize,
}

/// How much of a file is looked at to tell text from binary.
const SNIFFED: usize = 8 * 1024;

/// The endings of files a host sends as a picture, a recording, a font or
/// a PDF. A browser reads a file sent so as that and never as a document,
/// whatever it holds, so it runs no script from one. An SVG is a document.
const NEVER_DOCUMENTS: &[&str] = &[
    "jpg", "jpeg", "jfif", "png", "gif", "webp", "avif", "heic", "heif", "jxl", "bmp", "tif",
    "tiff", "ico", "mp3", "m4a", "aac", "ogg", "oga", "opus", "wav", "flac", "mp4", "m4v", "webm",
    "mov", "ogv", "woff", "woff2", "ttf", "otf", "eot", "pdf",
];

/// Whether the file `name` is one a browser never reads as a document.
fn never_a_document(name: &str) -> bool {
    let extension = Path::new(name).extension().and_then(|e| e.to_str());
    extension.is_some_and(|e| NEVER_DOCUMENTS.contains(&e.to_ascii_lowercase().as_str()))
}

/// A published file's text, where it is text: no NUL in its opening
/// stretch, as binary formats have and text doesn't. A browser may read any
/// text as a document, whatever the file is called, unless its name is one
/// [`never_a_document`] knows. A file that can't be read here can't be
/// published either, which publishing says.
fn text_of(bytes: &[u8]) -> Option<String> {
    if bytes[..bytes.len().min(SNIFFED)].contains(&0) {
        return None;
    }
    Some(String::from_utf8_lossy(bytes).into_owned())
}

/// [`text_of`] what `file`, open to read, holds, read no further than
/// its opening stretch where that is no text.
fn text_in(mut file: fs::File) -> Option<String> {
    let mut head = Vec::with_capacity(SNIFFED);
    std::io::Read::read_to_end(
        &mut std::io::Read::take(&mut file, SNIFFED as u64),
        &mut head,
    )
    .ok()?;
    if head.contains(&0) {
        return None;
    }
    std::io::Read::read_to_end(&mut file, &mut head).ok()?;
    text_of(&head)
}

/// Every file published as it is from `assets/`, `static/` or `content/`:
/// its address, the file it comes from, and what it is.
fn published_as_is(renderer: &Renderer) -> Vec<(String, String, AsIs<'_>)> {
    let assets = renderer.assets();
    let base = renderer.settings().base();
    let from_assets = assets.iter().map(|(_, asset)| {
        (
            format!("{base}/{}", asset.published),
            asset.source.clone(),
            AsIs::Bytes(asset.bytes()),
        )
    });
    let from_static = assets.statics().iter().map(|file| {
        (
            format!("{base}/{}", file.name),
            format!("{}/{}", assets::STATIC, file.name),
            AsIs::Static(file),
        )
    });
    let from_content = renderer.site().files.iter().map(|file| {
        (
            format!("{base}{}", file.address.as_str()),
            format!("content/{}", file.path),
            AsIs::Beside(&file.path),
        )
    });
    from_assets.chain(from_static).chain(from_content).collect()
}

/// A file published as it is.
enum AsIs<'a> {
    /// A file in `static/`.
    Static(&'a assets::StaticFile),
    /// A file beside the writing, by its path under `content/`.
    Beside(&'a str),
    /// What is published, already read.
    Bytes(&'a [u8]),
}

impl AsIs<'_> {
    /// The file opened where it really is and reached through no link, or
    /// none where it is already read.
    fn open(&self, renderer: &Renderer) -> Option<std::io::Result<fs::File>> {
        let assets = renderer.assets();
        match self {
            AsIs::Static(file) => Some(crate::platform::open_file(assets.real(), file.at())),
            AsIs::Beside(path) => Some(renderer.site().open_content(path)),
            AsIs::Bytes(_) => None,
        }
    }
}

/// What was found in the files published as they are, kept from one
/// build of a site to the next by whoever builds it again and again: a
/// file with the length and the time it had is not read a second time.
#[derive(Default)]
pub(crate) struct Remembered {
    /// Each by the file it comes from, from the site's folder.
    scripts: Mutex<HashMap<String, Read>>,
}

/// A file's length and when it was last changed, and what it held then.
type Read = (u64, std::time::SystemTime, Vec<Finding>);

impl Remembered {
    /// What the file `from` names, opened as `opened`, writes that runs
    /// JavaScript or may.
    fn found_in(&self, from: &str, opened: std::io::Result<fs::File>) -> Vec<Finding> {
        let stamp = (opened.as_ref().ok())
            .and_then(|file| file.metadata().ok())
            .and_then(|found| Some((found.len(), found.modified().ok()?)));
        if let Some((length, changed)) = stamp
            && let Some((was, then, findings)) =
                self.scripts.lock().expect("never poisoned").get(from)
            && (*was, *then) == (length, changed)
        {
            return findings.clone();
        }
        let text = opened.ok().and_then(text_in);
        let findings = text.map_or_else(Vec::new, |text| scripts::in_html(&text));
        let mut scripts = self.scripts.lock().expect("never poisoned");
        match stamp {
            Some((length, changed)) => {
                scripts.insert(from.to_owned(), (length, changed, findings.clone()));
            }
            None => {
                scripts.remove(from);
            }
        }
        findings
    }
}

/// Who a page's JavaScript is blamed on.
enum Blame<'a> {
    /// A page built from templates, blamed on those that write it.
    Built(&'a Built),
    /// A file published as it is, blamed on itself.
    File(String),
}

/// What each file's own text writes that runs JavaScript or may, read once.
struct Texts<'a> {
    renderer: &'a Renderer,
    /// The page each file of writing is.
    pages: HashMap<SourceId, PageId>,
    found: HashMap<SourceId, Vec<Finding>>,
    pieces: HashMap<SourceId, Vec<scripts::Piece>>,
}

impl<'a> Texts<'a> {
    fn new(renderer: &'a Renderer) -> Self {
        let pages = renderer.site().pages.iter();
        Texts {
            renderer,
            pages: pages
                .filter_map(|(id, page)| Some((page.source?, id)))
                .collect(),
            found: HashMap::new(),
            pieces: HashMap::new(),
        }
    }

    /// What a page's file puts on a page, where that isn't its text as it
    /// stands: writing in Markdown shows as code, or escapes, what its
    /// text read as HTML would take for markup. Asked only of a file whose
    /// text holds something to take so.
    fn html(&self, source: SourceId) -> Option<String> {
        let page = *self.pages.get(&source)?;
        self.renderer.written_by_its_file(page)
    }

    fn of(&mut self, source: SourceId) -> &[Finding] {
        if !self.found.contains_key(&source) {
            let mut found = scripts::in_html(self.renderer.sources().text(source));
            if !found.is_empty()
                && let Some(html) = self.html(source)
            {
                found = scripts::in_html(&html);
            }
            self.found.insert(source, found);
        }
        &self.found[&source]
    }

    /// Each piece of JavaScript the file writes in where a browser runs it.
    fn pieces(&mut self, source: SourceId) -> &[scripts::Piece] {
        if !self.pieces.contains_key(&source) {
            let pieces = if self.of(source).is_empty() {
                Vec::new()
            } else {
                match self.html(source) {
                    Some(html) => scripts::written(&html),
                    None => scripts::written(self.renderer.sources().text(source)),
                }
            };
            self.pieces.insert(source, pieces);
        }
        &self.pieces[&source]
    }

    /// Whether the file writes JavaScript in: where a browser runs it for
    /// `sure`, anywhere one may otherwise.
    fn writes(&mut self, source: SourceId, sure: bool) -> bool {
        self.of(source)
            .iter()
            .any(|finding| finding.found == Found::Written && (finding.sure || !sure))
    }

    /// Whether the file writes JavaScript in only where a browser may not
    /// run it.
    fn may_write(&mut self, source: SourceId) -> bool {
        self.of(source)
            .iter()
            .any(|finding| finding.found == Found::Written && !finding.sure)
    }
}

/// The files a built page read that could have written its JavaScript.
struct Reader {
    /// Every template and snippet it was built from, every page whose
    /// writing it shows, and its own file.
    read: Vec<SourceId>,
    /// Those of them that are snippets called by name.
    called: Vec<SourceId>,
    /// The page's own file, where it has one.
    source: Option<SourceId>,
    /// For a page Statac made, the files of the pages whose fields it shows.
    fields: Vec<SourceId>,
}

impl Reader {
    fn of(renderer: &Renderer, page: &Built) -> Reader {
        let site = renderer.site();
        let shown = match page.output {
            Output::Page(page) => Some(page),
            Output::Later(list, _) => Some(
                site.lists
                    .get(list)
                    .page
                    .expect("a list with later pages has a page"),
            ),
            Output::Redirect(at) => Some(site.redirects[at].page),
            Output::Feed(_) => None,
        };
        // A page Statac made has no file of its own to have written one.
        let source = shown.and_then(|shown| site.pages.get(shown).source);
        let mut called = Vec::new();
        // Another page's writing shown here, as a feed or a front page
        // showing whole posts does, can carry its own scripts.
        let mut read: Vec<SourceId> = page
            .reads
            .keys()
            .iter()
            .filter_map(|key| match key {
                Key::Template(id) => {
                    let source = renderer.template_source(*id);
                    if renderer.called_by_name(*id) {
                        called.push(source);
                    }
                    Some(source)
                }
                Key::Body(other) => site.pages.get(*other).source,
                _ => None,
            })
            .chain(source)
            .collect();
        read.sort_unstable();
        read.dedup();
        called.sort_unstable();
        called.dedup();
        let mut fields = Vec::new();
        if shown.is_some() && source.is_none() {
            fields.extend(page.reads.keys().iter().filter_map(|key| match key {
                Key::Field(other, _) => site.pages.get(*other).source,
                _ => None,
            }));
            fields.sort_unstable();
            fields.dedup();
        }
        Reader {
            read,
            called,
            source,
            fields,
        }
    }

    fn calls(&self, source: SourceId) -> bool {
        self.called.binary_search(&source).is_ok()
    }

    /// Whether a snippet the page calls by name loaded or embedded `found`,
    /// as far as the files tell: one holds it as written, and no other
    /// file the page read holds it too, or holds one whose address is
    /// filled in as the page is built.
    fn chose(&self, found: &Found, texts: &mut Texts) -> bool {
        let filled = |address: &str| address.contains("{{") || address.contains("{%");
        let could_be = |held: &Found| match (held, found) {
            (Found::Loaded(address), Found::Loaded(_))
            | (Found::Embedded(address), Found::Embedded(_)) => filled(address),
            _ => false,
        };
        let mut chosen = false;
        for &source in &self.read {
            let held = texts.of(source);
            if self.calls(source) {
                chosen |= held.iter().any(|each| each.found == *found);
            } else if held
                .iter()
                .any(|each| each.found == *found || could_be(&each.found))
            {
                return false;
            }
        }
        chosen
    }

    /// The files JavaScript written into the page is told as written in,
    /// and whether a browser runs it there, for a page with `runs` findings
    /// of it where a browser runs it, each of them one of `pieces`, and
    /// with some where a browser may not for `may_run`.
    ///
    /// What a snippet called by name writes is the author's choice and is
    /// not told, but only what the snippet's own file can be shown to have
    /// written: a piece on the page that the file holds whole, to the
    /// letter, and that no other file the page read holds. Every other
    /// piece is told of some file, as it would be were no snippet called:
    /// of each file that writes some, or where the files hold every piece
    /// on the page to the letter, of those holding one.
    fn wrote(
        &self,
        runs: usize,
        pieces: &[scripts::Print],
        may_run: bool,
        texts: &mut Texts,
    ) -> Vec<(SourceId, bool)> {
        // The called snippets that write nothing a template fills in, and
        // what they write.
        let mut cleared = Vec::new();
        let mut chosen = HashSet::new();
        for &snippet in &self.called {
            let held = texts.pieces(snippet);
            if held.iter().all(|piece| !piece.filled) {
                cleared.push(snippet);
            }
            chosen.extend(
                held.iter()
                    .filter(|piece| !piece.filled)
                    .map(|piece| piece.print),
            );
        }
        let mut all_chosen =
            runs > 0 && runs == pieces.len() && pieces.iter().all(|piece| chosen.contains(piece));
        if all_chosen {
            all_chosen = !self.read.iter().any(|&source| {
                !self.calls(source)
                    && texts
                        .pieces(source)
                        .iter()
                        .any(|piece| pieces.contains(&piece.print))
            });
        }
        let sure = runs > 0 && !all_chosen;
        if !sure && !may_run {
            return Vec::new();
        }
        let mut from: Vec<SourceId> = Vec::new();
        if sure {
            from.extend(
                self.read
                    .iter()
                    .copied()
                    .filter(|&source| !cleared.contains(&source) && texts.writes(source, true)),
            );
            // Where every piece on the page is one some file holds to the
            // letter, a file holding none of them wrote none of them: its
            // own went unshown, or was shown as text.
            let holds = |texts: &mut Texts, source: SourceId, piece: &scripts::Print| {
                texts.pieces(source).iter().any(|held| held.print == *piece)
            };
            let all_held = pieces
                .iter()
                .all(|piece| self.read.iter().any(|&source| holds(texts, source, piece)));
            if all_held {
                let wrote: Vec<SourceId> = from
                    .iter()
                    .copied()
                    .filter(|&source| pieces.iter().any(|piece| holds(texts, source, piece)))
                    .collect();
                if !wrote.is_empty() {
                    from = wrote;
                }
            }
        }
        if from.is_empty() {
            // A cleared snippet's own pieces are run wherever it is called,
            // so only one writing some a browser may not run could have
            // written such a one.
            from.extend(self.read.iter().copied().filter(
                |&source| match cleared.contains(&source) {
                    true => !sure && texts.may_write(source),
                    false => texts.writes(source, false),
                },
            ));
        }
        // Written by a value no file holds as markup, such as a field
        // passed through `safe`: the page's own file holds the value,
        // or for a page Statac made, a file whose fields it shows.
        if from.is_empty() {
            from.extend(
                self.fields
                    .iter()
                    .copied()
                    .filter(|&field| texts.writes(field, false)),
            );
        }
        if from.is_empty() {
            from.extend(self.source.or(self.read.first().copied()));
        }
        from.into_iter().map(|source| (source, sure)).collect()
    }
}

/// The JavaScript the built site carries: each script it publishes, from
/// any folder, whether or not a page loads it; each address elsewhere a
/// page loads one from; each file that writes JavaScript into a page,
/// pages built and every text file published as it is that a browser may
/// read as a document alike; and
/// each thing embedded that may run its own.
pub(super) fn carried(
    renderer: &Renderer,
    built: &[Built],
    remembered: Option<&Remembered>,
) -> Vec<Javascript> {
    let assets = renderer.assets();
    let as_is = published_as_is(renderer);
    let mut published: Vec<assets::Travelled> = assets.travelled();
    published.extend(
        as_is
            .iter()
            .filter(|(_, from, _)| from.starts_with("content/") && assets::is_script(from))
            .map(|(address, from, _)| assets::Travelled {
                address: address.clone(),
                from: from.clone(),
            }),
    );
    let documents: Vec<(Vec<Finding>, String)> = as_is
        .par_iter()
        // A script is reported whole already, as the script it is.
        .filter(|(_, from, _)| !assets::is_script(from) && !never_a_document(from))
        .filter_map(|(_, from, file)| {
            let findings = match (file.open(renderer), remembered) {
                (Some(opened), Some(remembered)) => remembered.found_in(from, opened),
                (Some(opened), None) => scripts::in_html(&text_in(opened.ok()?)?),
                (None, _) => {
                    let AsIs::Bytes(bytes) = file else {
                        return None;
                    };
                    scripts::in_html(&text_of(bytes)?)
                }
            };
            (!findings.is_empty()).then(|| (findings, from.clone()))
        })
        .collect();
    let origin = renderer
        .settings()
        .text("url")
        .map(|url| site_origin(url.value))
        .unwrap_or_default();
    let mut on: BTreeMap<(Carried, bool), usize> = published
        .iter()
        .map(|script| {
            let carried = Carried::Published {
                address: script.address.clone(),
                from: script.from.clone(),
            };
            ((carried, true), 0)
        })
        .collect();
    let by_address: HashMap<&str, Carried> = published
        .iter()
        .map(|script| {
            let carried = Carried::Published {
                address: script.address.clone(),
                from: script.from.clone(),
            };
            (script.address.as_str(), carried)
        })
        .collect();
    let sources = renderer.sources();
    let mut texts = Texts::new(renderer);
    let every = built
        .iter()
        .map(|page| (&page.scripts, Blame::Built(page)))
        .chain(
            documents
                .iter()
                .map(|(findings, from)| (findings, Blame::File(from.clone()))),
        );
    for (findings, blame) in every {
        if findings.is_empty() {
            continue;
        }
        let reader = match &blame {
            Blame::Built(page) => Some(Reader::of(renderer, page)),
            Blame::File(_) => None,
        };
        let mut chosen = |found: &Found| {
            reader
                .as_ref()
                .is_some_and(|reader| reader.chose(found, &mut texts))
        };
        let mut carried: Vec<(Carried, bool)> = Vec::new();
        // How often JavaScript is written in where a browser runs it, and
        // whether it is where one may.
        let (mut runs, mut may_run) = (0, false);
        for finding in findings {
            let sure = finding.sure;
            match &finding.found {
                Found::Written if sure => runs += 1,
                Found::Written => may_run = true,
                Found::Embedded(address) => {
                    if !chosen(&finding.found) {
                        carried.push((Carried::Embedded(address.clone()), sure))
                    }
                }
                Found::Loaded(src) => {
                    let local = (!origin.is_empty())
                        .then(|| src.strip_prefix(origin.as_str()))
                        .flatten()
                        .filter(|path| path.starts_with('/'))
                        .unwrap_or(src.as_str());
                    let path = local.split(['?', '#']).next().unwrap_or_default();
                    match by_address.get(path) {
                        Some(script) => carried.push((script.clone(), sure)),
                        None if chosen(&finding.found) => {}
                        None => carried.push((Carried::Loaded(src.clone()), sure)),
                    }
                }
            }
        }
        match (blame, reader) {
            (Blame::File(from), _) if runs > 0 || may_run => {
                carried.push((Carried::Written(from), runs > 0))
            }
            (Blame::Built(page), Some(reader)) if runs > 0 || may_run => carried.extend(
                reader
                    .wrote(runs, &page.pieces, may_run, &mut texts)
                    .into_iter()
                    .map(|(source, sure)| (Carried::Written(slashed(sources.path(source))), sure)),
            ),
            _ => {}
        }
        // Sure before unsure, so a page carrying one both ways counts as sure.
        carried.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        carried.dedup_by(|later, first| later.0 == first.0);
        for each in carried {
            *on.entry(each).or_default() += 1;
        }
    }
    let unsure: Vec<Carried> = on
        .keys()
        .filter(|(_, sure)| !sure)
        .map(|(carried, _)| carried.clone())
        .collect();
    on.into_iter()
        .filter(|((carried, sure), pages)| !(*sure && *pages == 0 && unsure.contains(carried)))
        .map(|((carried, sure), pages)| Javascript {
            carried,
            sure,
            pages,
        })
        .collect()
}

fn slashed(path: &Path) -> String {
    crate::platform::slashed(path).unwrap_or_else(|lossy| lossy)
}
