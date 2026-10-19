//! Building pages with the site's templates and snippets.
//!
//! The engine underneath is MiniJinja, changed only to hand Statac each
//! body it holds back to lay out (`indent`). Three things close the
//! gaps where it would publish a missing value as `none`, `null`,
//! `[undefined]` or nothing: semi-strict undefined; every filter, test,
//! function and the formatter registered again behind a guard (`guard`);
//! and a check of every template when it is read (`check`), which also
//! finds the mistakes it can before any page is built. Nothing outside
//! this module knows which engine is underneath.

mod card;
mod check;
mod data;
mod dedent;
mod engine;
mod explain;
mod files;
mod frame;
mod guard;
mod indent;
mod landing;
mod layout;
mod media;
mod own;
mod pieces;
mod render;
mod shows;
#[cfg(test)]
mod tests;
mod values;
mod walks;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::Arc;

use minijinja::{AutoEscape, Environment};

use crate::config::Settings;
use crate::problem::{Problem, Severity};
use crate::site::{Site, TemplateId};
use crate::source::Sources;
use check::{Checked, Field, Rec, Schema, Ty};
use files::{File, Role, Snippet};
pub(crate) use files::{SNIPPETS, TEMPLATES};
pub(crate) use values::{Answering, SEARCH_BOX, of_the_front, writes};

pub use card::{Asked, Gathered};
pub use indent::Own;
pub use layout::Chosen;
pub use render::{REDIRECT, Rendered, Renderer, developed};
pub use shows::{Explained, Group, Held, Holds, Member, Neighbour, Reached, Row};

/// The site's templates and snippets, read and checked, ready to build
/// pages with.
pub struct Templates {
    env: Environment<'static>,
    files: Vec<File>,
    by_name: HashMap<String, usize>,
    snippets: BTreeMap<String, Snippet>,
    /// The file of each piece of the writing the site writes itself.
    pieces: BTreeMap<crate::markdown::Piece, usize>,
    /// Files with a problem told when they were read, which build nothing.
    unusable: BTreeSet<usize>,
    /// The template each file extends, where it names one.
    extends: Vec<Option<String>>,
    /// The cards the site describes, their texts among the files.
    cards: card::Cards,
    /// Where the files' lines are written.
    landings: Arc<landing::Landings>,
    /// How many of the engine's steps building one output may take.
    steps: u64,
    /// How many pages of lists of the whole site the outputs running one
    /// loop reach before it is noted.
    many_walked: u64,
}

impl Templates {
    /// Whether `error` is the engine not finding a template of the site's
    /// that a problem told when it was read keeps from it.
    fn unusable_asked_for(&self, error: &minijinja::Error) -> bool {
        error.kind() == minijinja::ErrorKind::TemplateNotFound
            && self.unusable.iter().any(|&at| {
                // As the engine words it.
                let unfound = format!("template {:?} does not exist", self.files[at].name);
                error.detail() == Some(unfound.as_str())
            })
    }

    /// Whether `name` is a template a page can be built with.
    fn is_layout(&self, name: &str) -> bool {
        self.by_name
            .get(name)
            .is_some_and(|&at| self.files[at].role == Role::Template)
    }

    /// The template named `name`, and those it extends, in turn.
    fn chain(&self, name: &str) -> Vec<usize> {
        let mut chain = Vec::new();
        let mut next = self.by_name.get(name).copied();
        while let Some(at) = next {
            if chain.contains(&at) {
                break;
            }
            chain.push(at);
            next = self.extends[at]
                .as_ref()
                .and_then(|parent| self.by_name.get(parent).copied());
        }
        chain
    }

    /// Where the file named `name` writes its lines when rendered.
    fn placing(&self, name: &str) -> landing::Place {
        landing::Place::new(&self.landings, self.by_name.get(name).copied())
    }

    /// The file whose text, as the engine holds it, has `address` in
    /// memory inside it, and how far into the text that is.
    fn holds(&self, address: usize) -> Option<(usize, usize)> {
        self.files.iter().enumerate().find_map(|(at, file)| {
            let template = self.env.get_template(&file.name).ok()?;
            let offset = address.checked_sub(template.source().as_ptr() as usize)?;
            (offset < template.source().len()).then_some((at, offset))
        })
    }

    fn id(at: usize) -> TemplateId {
        use crate::site::Id;
        TemplateId::from_index(at)
    }

    /// Whether what Statac hands `piece` as `name` is shown: by the site's
    /// file for the piece where that reads the name, and where the site
    /// has no file for it by Statac's own writing of the piece, which shows
    /// what the footnotes are handed and nothing a callout is.
    fn piece_shows(&self, piece: crate::markdown::Piece, name: &str) -> bool {
        let Some(&file) = self.pieces.get(&piece) else {
            return piece == crate::markdown::Piece::Footnotes;
        };
        frame::read(crate::reads::Key::Template(Templates::id(file)));
        self.env
            .get_template(&self.files[file].name)
            .is_ok_and(|template| template.undeclared_variables(false).contains(name))
    }
}

/// Reads the templates and snippets of the site at `root`, checks them
/// and the texts of `cards` against what the site has, and makes them
/// ready to build with.
pub fn read(
    root: &Path,
    sources: &mut Sources,
    site: &Site,
    settings: &Settings,
    cards: crate::cards::Cards,
) -> (Templates, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut files = files::gather(root, sources, &mut problems);
    let cards = card::gather(root, cards, sources, &mut files);
    problems.extend(card::check_pages(site, &cards));
    let snippet_names: BTreeSet<String> = files
        .iter()
        .filter_map(|f| match &f.role {
            Role::Snippet(name) => Some(name.clone()),
            Role::Template | Role::Piece(_) | Role::CardText => None,
        })
        .collect();
    let pieces = files
        .iter()
        .enumerate()
        .filter_map(|(at, f)| match f.role {
            Role::Piece(piece) => Some((piece, at)),
            _ => None,
        })
        .collect();
    let mut callables = snippet_names.clone();
    callables.extend(guard::FUNCTIONS.iter().map(|f| (*f).to_owned()));
    callables.extend(own::FUNCTIONS.iter().map(|f| (*f).to_owned()));

    let mut snippets = BTreeMap::new();
    let mut extends = Vec::with_capacity(files.len());
    let mut blocks = Vec::with_capacity(files.len());
    let mut told = Vec::new();
    for (at, file) in files.iter().enumerate() {
        let text = sources.text(file.source);
        let ast = check::parse_template(&file.name, text).ok();
        extends.push(ast.as_ref().and_then(extended));
        blocks.push(ast.as_ref().map(blocks_of));
        let Role::Snippet(name) = &file.role else {
            continue;
        };
        // The writing's `@url(...)` is Statac's own, and calls no snippet.
        if name == crate::markdown::IN_SITE {
            let kind = crate::problem::Kind::ReservedName { name: name.clone() };
            let head = crate::content::words::head(sources, file.source);
            told.push((at, Problem::at(kind, head)));
        }
        let (wrapped, params): (Vec<String>, Vec<String>) = ast
            .as_ref()
            .map(|ast| check::free_names(ast, &callables))
            .unwrap_or_default()
            .into_iter()
            .filter(|n| {
                !matches!(
                    n.as_str(),
                    "page" | "site" | "loop" | "self" | "super" | "caller" | "varargs" | "kwargs"
                ) && !files::SUPPLIED.contains(&n.as_str())
            })
            .partition(|n| files::WRAPPED.contains(&n.as_str()));
        let takes = match files::takes_line(file.source, text) {
            None => None,
            Some(Err(problem)) => {
                told.push((at, problem));
                None
            }
            Some(Ok(_)) if ast.is_none() => None,
            Some(Ok(line)) => {
                let wrong = files::line_against(&line, file.source, &params);
                let usable = wrong.is_empty();
                told.extend(wrong.into_iter().map(|p| (at, p)));
                usable.then_some(line.takes)
            }
        };
        snippets.insert(
            name.clone(),
            Snippet {
                name: name.clone(),
                file: at,
                params,
                wrapped,
                takes,
                readable: ast.is_some(),
            },
        );
    }

    told.extend(never_shown(&files, &extends, &blocks, sources));

    let mut schema = schema(site, settings);
    schema.snippets = snippets.clone();
    schema.templates = files
        .iter()
        .filter(|f| f.role == Role::Template)
        .map(|f| f.name.clone())
        .collect();
    let checked: Vec<Checked> = files
        .iter()
        .map(|f| Checked {
            name: &f.name,
            source: f.source,
            role: &f.role,
        })
        .collect();
    // A template only pages left out until the site has an address are
    // built with is never built without one, and is checked as with one.
    let waiting = layouts_only_needing_url(site);
    let with_url = (!waiting.is_empty()).then(|| {
        let mut with_url = self::schema(site, settings);
        with_url.snippets = snippets.clone();
        with_url.templates.clone_from(&schema.templates);
        for name in ["url", "host"] {
            let always = Field {
                ty: Ty::Text,
                always: true,
            };
            with_url.site.insert(name.to_owned(), always);
        }
        (with_url, waiting)
    });
    let with_url = with_url.as_ref().map(|(schema, waiting)| (schema, waiting));
    told.extend(check::check_site(&schema, with_url, &checked, sources));

    let mut unusable = BTreeSet::new();
    for (at, problem) in told {
        if problem.kind.severity() == Severity::Error {
            unusable.insert(at);
        }
        problems.push(cards.placed(problem));
    }

    let steps = lowered(root).unwrap_or(STEPS);
    let mut env = Environment::new();
    // The engine stops a run on the step after its last.
    env.set_fuel(Some(steps + 1));
    // On by default in a debug build only, where it adds to the engine's
    // words, which are read here; off, a test runs as a release does.
    env.set_debug(false);
    env.set_syntax(check::reading());
    guard::install(&mut env);
    own::install(&mut env);
    env.set_held(indent::held);
    env.set_auto_escape_callback(escaping);
    env.set_path_join_callback(|name, _| files::named(name).into());
    let mut by_name = HashMap::new();
    let mut facts = Vec::with_capacity(files.len());
    for (at, file) in files.iter().enumerate() {
        by_name.insert(file.name.clone(), at);
        let mut text = sources.text(file.source).to_owned();
        // A card's text is drawn, and where a JSON file's lines break
        // doesn't matter.
        let read = match escaping(&file.name) {
            AutoEscape::Html => dedent::facts(&text, true),
            AutoEscape::None if !file.name.starts_with(card::PREFIX) => dedent::facts(&text, false),
            _ => None,
        };
        if let Some(read) = &read {
            // The same length, so every place in the text stays where it is.
            let mut bytes = std::mem::take(&mut text).into_bytes();
            for &at in &read.keeps {
                bytes[at] = b'+';
            }
            for &at in &read.trims {
                bytes[at] = b'-';
            }
            text = String::from_utf8(bytes).expect("a space written as a plus");
        }
        facts.push(read);
        // A file the engine can't read was told about when it was checked.
        let _ = env.add_template_owned(file.name.clone(), text);
    }
    for snippet in snippets.values() {
        render::register_snippet(&mut env, snippet);
    }
    let mut templates = Templates {
        env,
        files,
        by_name,
        snippets,
        pieces,
        unusable,
        extends,
        cards,
        landings: Arc::default(),
        steps,
        many_walked: lowered_walked(root).unwrap_or(walks::MANY),
    };
    templates.landings = Arc::new(landings(&templates, &facts, sources));
    (templates, problems)
}

/// Where the lines of the site's files are written, from what is known of
/// each. What tells an indent meant for the template from one meant for
/// the page is the markup: elsewhere an indent may mean something, and
/// is kept.
fn landings(
    templates: &Templates,
    facts: &[Option<dedent::Facts>],
    sources: &Sources,
) -> landing::Landings {
    let markup = |at: usize| matches!(escaping(&templates.files[at].name), AutoEscape::Html);
    let held = |at: usize| {
        let file = &templates.files[at];
        let template = templates.env.get_template(&file.name).ok()?;
        Some(landing::Held {
            base: template.source().as_ptr() as usize,
            source: sources.text(file.source),
            facts: facts[at].as_ref()?,
        })
    };
    let mut landings = landing::Landings::default();
    for at in 0..templates.files.len() {
        if let Some(held) = held(at) {
            landings.add(&held, markup(at));
        }
    }
    landings.sort();
    for (at, file) in templates.files.iter().enumerate() {
        let chain = templates.chain(&file.name);
        if chain.iter().all(|&link| markup(link))
            && let Some(chain) = chain.into_iter().map(held).collect::<Option<Vec<_>>>()
        {
            landings.chain(at, &chain);
        }
    }
    landings
}

/// How many of the engine's steps building one output may take: enough
/// for any site, and few enough that a loop that never ends is stopped
/// in seconds.
pub const STEPS: u64 = 100_000_000;

/// The steps a test lowers [`STEPS`] to for the site at a root.
#[cfg(not(test))]
fn lowered(_: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
use tests::lowered;

/// What a test lowers [`walks::MANY`] to for the site at a root.
#[cfg(not(test))]
fn lowered_walked(_: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
use tests::lowered_walked;

/// How what a template prints is escaped, by the kind of file it writes:
/// not at all for plain text, as JSON values for JSON, and as markup for
/// HTML, the XML family and anything else.
fn escaping(name: &str) -> AutoEscape {
    // A card's texts are drawn, not written into a page.
    if name.starts_with(card::PREFIX) {
        return AutoEscape::None;
    }
    let last = name.rsplit('/').next().unwrap_or(name);
    match last.rsplit_once('.').map(|(_, ext)| ext) {
        Some("txt") => AutoEscape::None,
        Some("json" | "webmanifest") => AutoEscape::Custom(guard::JSON.into()),
        _ => AutoEscape::Html,
    }
}

/// The template a template extends, where it names one outright.
fn extended(ast: &minijinja::machinery::ast::Stmt<'_>) -> Option<String> {
    use minijinja::machinery::ast::{Expr, Stmt};
    let Stmt::Template(template) = ast else {
        return None;
    };
    template.children.iter().find_map(|stmt| match stmt {
        Stmt::Extends(e) => match &e.name {
            Expr::Const(c) => c.value.as_str().map(|name| files::named(name).to_owned()),
            _ => None,
        },
        _ => None,
    })
}

/// A template's blocks by name, each with whether it is at the top of the
/// file, where a template that extends another fills that one's blocks,
/// and where its name is written.
type Blocks = Vec<(String, bool, usize)>;

/// The blocks a template has.
fn blocks_of(ast: &minijinja::machinery::ast::Stmt<'_>) -> Blocks {
    use minijinja::machinery::ast::Stmt;
    fn walk(stmts: &[Stmt<'_>], top: bool, out: &mut Blocks) {
        for stmt in stmts {
            let inner: &[Stmt<'_>] = match stmt {
                Stmt::Block(b) => {
                    out.push((b.name.to_owned(), top, b.span().start_offset as usize));
                    &b.body
                }
                Stmt::ForLoop(f) => &f.body,
                Stmt::IfCond(i) => {
                    walk(&i.false_body, false, out);
                    &i.true_body
                }
                Stmt::WithBlock(w) => &w.body,
                Stmt::AutoEscape(a) => &a.body,
                Stmt::FilterBlock(f) => &f.body,
                _ => continue,
            };
            walk(inner, false, out);
        }
    }
    let mut out = Vec::new();
    if let Stmt::Template(t) = ast {
        walk(&t.children, true, &mut out);
    }
    out
}

/// A block filled by a template that extends another, where no template
/// it extends has a block of that name. Where one of them can't be read,
/// its blocks are unknown and nothing is told.
fn never_shown(
    files: &[File],
    extends: &[Option<String>],
    blocks: &[Option<Blocks>],
    sources: &Sources,
) -> Vec<(usize, Problem)> {
    let by_name: HashMap<&str, usize> = files
        .iter()
        .enumerate()
        .map(|(at, f)| (f.name.as_str(), at))
        .collect();
    let mut problems = Vec::new();
    for (at, file) in files.iter().enumerate() {
        let Some(parent) = extends[at].as_deref() else {
            continue;
        };
        let Some(&first) = by_name.get(parent) else {
            continue;
        };
        let mut ancestors = vec![first];
        while let Some(next) = extends[*ancestors.last().expect("one at least")]
            .as_deref()
            .and_then(|name| by_name.get(name).copied())
            .filter(|next| !ancestors.contains(next) && *next != at)
        {
            ancestors.push(next);
        }
        let Some(shown) = ancestors
            .iter()
            .map(|&a| blocks[a].as_deref())
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        let shown: Vec<&str> = shown
            .into_iter()
            .flatten()
            .map(|(name, _, _)| name.as_str())
            .collect();
        let text = sources.text(file.source);
        for (block, top, start) in blocks[at].iter().flatten() {
            if !top || shown.contains(&block.as_str()) {
                continue;
            }
            let name_at = text[*start..]
                .find(block.as_str())
                .map_or(*start, |found| start + found);
            let nearest = crate::suggest::nearest(block, shown.iter().copied()).map(str::to_owned);
            problems.push((
                at,
                Problem::at(
                    crate::problem::Kind::TextNeverShown {
                        extended: files::template_path(parent),
                        block: block.clone(),
                        nearest,
                    },
                    crate::source::Place {
                        source: file.source,
                        span: crate::source::Span::new(name_at, name_at + block.len()),
                    },
                ),
            ));
        }
    }
    problems
}

/// Every name a template may read on a page and on the site.
/// The templates named as the layout of a page left out until the site
/// has an address, and of no page that is built. The templates a page
/// naming none is built with are left out, being any page's.
fn layouts_only_needing_url(site: &Site) -> BTreeSet<String> {
    use crate::content::words::PageWord;
    let name = |layout: &str| files::named(layout).to_owned();
    let mut waiting: BTreeSet<String> = site
        .layouts_needing_url
        .iter()
        .map(|layout| name(layout))
        .filter(|layout| !matches!(layout.as_str(), "home" | "folder" | "page"))
        .collect();
    for (_, page) in site.pages.iter() {
        for word in [PageWord::Layout, PageWord::Type] {
            if let Some(named) = page.words.text(word) {
                waiting.remove(&name(named.value));
            }
        }
    }
    waiting
}

fn schema(site: &Site, settings: &Settings) -> Schema {
    use crate::content::words::PageWord;
    let mut schema = Schema::new();
    let pages = Ty::List(Box::new(Ty::Record(Rec::Page)));
    let total = site.pages.len();
    let every = |count: usize| total > 0 && count == total;

    let page = &mut schema.page;
    for word in PageWord::all() {
        page.insert(
            word.name().to_owned(),
            Field {
                ty: Ty::Any,
                always: false,
            },
        );
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for (_, p) in site.pages.iter() {
        for field in &p.words.fields {
            *counts.entry(&field.name).or_default() += 1;
        }
    }
    for (name, count) in counts {
        let kind = settings.declarations.kind(name);
        page.insert(
            name.to_owned(),
            Field {
                ty: check::declared_ty(kind),
                always: every(count),
            },
        );
    }
    for name in settings.declarations.names() {
        // A nested field is reached through the field it is nested in.
        let ty = match name.split_once('.') {
            Some(_) => Ty::Any,
            None => check::declared_ty(settings.declarations.kind(name)),
        };
        let head = name.split('.').next().unwrap_or(name);
        page.entry(head.to_owned())
            .or_insert(Field { ty, always: false });
    }
    let titled = site.pages.iter().filter(|(_, p)| p.title.is_some()).count();
    let dated = site.pages.iter().filter(|(_, p)| p.date.is_some()).count();
    let summarised = site
        .pages
        .iter()
        .filter(|(_, p)| p.words.get(PageWord::Summary).is_some())
        .count();
    let in_series = site
        .pages
        .iter()
        .filter(|(_, p)| p.words.get(PageWord::Series).is_some())
        .count();
    let lists = Ty::List(Box::new(Ty::Record(Rec::List)));
    for (name, ty, always) in [
        ("title", Ty::Text, every(titled)),
        ("url", Ty::Text, true),
        ("slug", Ty::Text, true),
        ("date", Ty::Record(Rec::Date), every(dated)),
        ("updated", Ty::Record(Rec::Date), false),
        ("summary", Ty::Text, every(summarised)),
        ("content", Ty::Html, true),
        ("words", Ty::Number, true),
        ("excerpt", Ty::Html, false),
        ("excerpt_marked", Ty::Bool, true),
        ("maths", Ty::Bool, true),
        ("opening", Ty::Text, false),
        ("toc", Ty::List(Box::new(Ty::Any)), true),
        ("language", Ty::Text, true),
        ("direction", Ty::Text, true),
        ("tags", lists.clone(), true),
        ("series", Ty::Record(Rec::List), every(in_series)),
        ("posts", pages.clone(), false),
        ("list", Ty::Record(Rec::List), false),
        ("folder", Ty::Record(Rec::List), true),
        ("source", Ty::Text, true),
        ("number", Ty::Number, false),
        ("next", Ty::Record(Rec::Page), false),
        ("previous", Ty::Record(Rec::Page), false),
        ("translations", pages.clone(), true),
        ("og_title", Ty::Text, true),
        ("card", Ty::Any, false),
        ("webmentions", Ty::Record(Rec::Webmentions), true),
    ] {
        page.insert(name.to_owned(), Field { ty, always });
    }
    // The post a page was made for; a page made for none may write a field
    // of its own by that name.
    let made_for_posts = site.pages.iter().any(|(_, p)| {
        p.made
            .as_ref()
            .is_some_and(|made| matches!(made.item, crate::site::Item::Post { .. }))
    });
    let post = Field {
        ty: Ty::Record(Rec::Page),
        always: false,
    };
    match made_for_posts {
        true => {
            page.insert("post".to_owned(), post);
        }
        false => {
            page.entry("post".to_owned()).or_insert(post);
        }
    }
    for name in values::PAGE_NOT_YET {
        page.insert(
            (*name).to_owned(),
            Field {
                ty: Ty::Any,
                always: true,
            },
        );
    }
    for name in [values::SEARCH_BOX].iter().chain(media::CALLS) {
        page.insert(
            (*name).to_owned(),
            Field {
                ty: Ty::Callable,
                always: true,
            },
        );
    }

    let site_fields = &mut schema.site;
    // On a site in several languages, a setting written for some of them
    // alone is there on their pages only.
    let tags: Vec<&str> = match site.languages.len() {
        0 | 1 => Vec::new(),
        _ => site
            .languages
            .iter()
            .filter_map(|(_, language)| language.tag.as_deref())
            .collect(),
    };
    for name in ["title", "url", "language"] {
        let everywhere = match name {
            "language" => true,
            "title" if !tags.is_empty() => tags
                .iter()
                .all(|tag| settings.text_for(Some(tag), name).is_some()),
            _ => settings.text(name).is_some(),
        };
        let somewhere = name == "title"
            && tags
                .iter()
                .any(|tag| settings.text_for(Some(tag), name).is_some());
        if everywhere || somewhere {
            site_fields.insert(
                name.to_owned(),
                Field {
                    ty: Ty::Text,
                    always: everywhere,
                },
            );
        }
    }
    for (name, ty) in [
        ("pages", pages.clone()),
        ("posts", pages),
        ("tags", lists.clone()),
        ("folders", lists.clone()),
        ("series", lists),
        ("today", Ty::Record(Rec::Date)),
        ("feed", Ty::Text),
        ("data", Ty::Any),
        ("languages", Ty::Any),
        ("webmentions", Ty::Bool),
    ] {
        site_fields.insert(name.to_owned(), Field { ty, always: true });
    }
    if settings.text("url").is_some() {
        site_fields.insert(
            "host".to_owned(),
            Field {
                ty: Ty::Text,
                always: true,
            },
        );
    }
    for own in &settings.own {
        site_fields.insert(
            own.name.clone(),
            Field {
                ty: check::declared_ty(settings.declarations.kind(&own.name)),
                always: true,
            },
        );
    }
    schema
}
