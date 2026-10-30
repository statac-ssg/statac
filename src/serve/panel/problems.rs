//! Problems in their parts, for the page to lay out for itself: each
//! place with its lines, the pages whose builds met it, and what `statac
//! explain` says of its kind. And how each round went, told the same way.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;

use super::pieced;
use crate::address::on_the_wire;
use crate::json;
use crate::markdown::{self, Document, Kind, Node};
use crate::problem::Quiet;
use crate::problem::{Code, Kind as ProblemKind, Named, Problem, Severity, Word, unanchored};
use crate::report::{Excerpt, NAMED, Report, shown};
use crate::serve::session::{Read, Took};
use crate::source::{Place, Sources, Span};
use crate::wording;

/// What turns a round's problems into their parts.
pub(super) struct Parts<'a> {
    pub(crate) report: &'a Report<'a>,
    pub(crate) quiet: &'a Quiet,
    /// The site as the round read it, where it was, to find where the
    /// pages a problem was met building are published.
    pub(crate) read: Option<&'a Read>,
    pub(crate) addresses: &'a OnceLock<Addresses>,
}

impl Parts<'_> {
    /// One problem, as JSON.
    pub(super) fn of(&self, problem: &Problem) -> String {
        let report = self.report;
        let kind = &problem.kind;
        let null = || "null".to_owned();
        let (file, line, column, excerpt) = match problem.place {
            None => (null(), null(), null(), "[]".to_owned()),
            Some(place) if kind.of_a_whole_file() => (
                json::string(&report.file(place.source)),
                null(),
                null(),
                "[]".to_owned(),
            ),
            Some(place) => {
                let excerpt = report.excerpt(place, true);
                (
                    json::string(&report.file(place.source)),
                    excerpt.first.to_string(),
                    excerpt.column.to_string(),
                    lines(&excerpt),
                )
            }
        };
        let related: Vec<String> = (problem.related().iter())
            .map(|related| {
                let (file, line, excerpt) = match related.place {
                    Some(place) => {
                        let excerpt = report.excerpt(place, false);
                        let file = json::string(&report.file(place.source));
                        (file, excerpt.first.to_string(), lines(&excerpt))
                    }
                    None => (null(), null(), "[]".to_owned()),
                };
                format!(
                    "{{\"label\":{},\"file\":{file},\"line\":{line},\"excerpt\":{excerpt}}}",
                    pieced(&shown(&related.label)),
                )
            })
            .collect();
        let pages: Vec<_> = problem.built().collect();
        let named = pages.len().min(1 + NAMED);
        let built: Vec<String> = (pages[..named].iter())
            .map(|&(page, place)| {
                let said = match (page, place) {
                    (Some(page), _) => shown(&wording::named(page)),
                    (None, Some(place)) => format!("`{}`", report.file(place.source)),
                    (None, None) => String::new(),
                };
                let href = (self.read).and_then(|read| {
                    self.addresses
                        .get_or_init(|| Addresses::of(read))
                        .href(page, place)
                });
                let (file, line) = match place {
                    Some(place) => (
                        json::string(&report.file(place.source)),
                        match report.whole(place) {
                            true => null(),
                            false => report.first_line(place).to_string(),
                        },
                    ),
                    None => (null(), null()),
                };
                format!(
                    "{{\"page\":{},\"href\":{},\"file\":{file},\"line\":{line}}}",
                    pieced(&said),
                    href.as_deref().map_or_else(null, json::string),
                )
            })
            .collect();
        let printed = match wording::printed(kind) {
            Some((heading, said)) => {
                let said: Vec<String> =
                    said.iter().map(|line| json::string(&shown(line))).collect();
                format!(
                    "{{\"heading\":{},\"lines\":[{}]}}",
                    pieced(&heading),
                    said.join(",")
                )
            }
            None => null(),
        };
        let suggestion = problem.suggestion();
        format!(
            "{{\"severity\":{},\"kind\":{},\"headline\":{},\"file\":{file},\"line\":{line},\
             \"column\":{column},\"excerpt\":{excerpt},\"related\":[{}],\"built\":[{}],\
             \"builtMore\":{},\"printed\":{printed},\"suggestion\":{},\"suggests\":{},\
             \"explanation\":{},\"quietened\":{}}}",
            json::string(wording::severity(kind.severity())),
            json::string(kind.name()),
            pieced(&shown(&problem.headline())),
            related.join(","),
            built.join(","),
            pages.len() - named,
            suggestion.map_or_else(null, |said| pieced(&shown(&said))),
            suggests(kind).map_or_else(null, |name| json::string(&shown(name))),
            json::string(&problem.explanation_link()),
            self.quiet.quietens(kind),
        )
    }
}

/// An excerpt's lines, each as its number and its pieces: the words, and
/// the place as `{"mark": ...}`, empty where the place is.
pub(super) fn lines(excerpt: &Excerpt) -> String {
    let lines: Vec<String> = (excerpt.lines.iter())
        .map(|line| {
            let text = |words: &str| format!("{{\"text\":{}}}", json::string(words));
            let mut pieces = Vec::new();
            if !line.before.is_empty() {
                pieces.push(text(&line.before));
            }
            if let Some(marked) = &line.marked {
                pieces.push(format!("{{\"mark\":{}}}", json::string(marked)));
            }
            if !line.after.is_empty() {
                pieces.push(text(&line.after));
            }
            let runs = match line.marked.is_some() && excerpt.last > excerpt.first {
                true => format!(",\"runs\":{}", pieced(&wording::runs_to(excerpt.last))),
                false => String::new(),
            };
            format!(
                "{{\"line\":{},\"text\":[{}]{runs}}}",
                line.number,
                pieces.join(",")
            )
        })
        .collect();
    format!("[{}]", lines.join(","))
}

/// The name a problem's suggestion offers in its place, where it is one
/// of the page's or the site's that a template writes, which the panel
/// lists.
fn suggests(kind: &ProblemKind) -> Option<&str> {
    let name = match kind {
        ProblemKind::UnknownWord {
            word: Word::Field,
            nearest: Some(name),
            ..
        }
        | ProblemKind::AlwaysFalse { meant: name, .. } => name,
        _ => return None,
    };
    (name.starts_with("page.") || name.starts_with("site.")).then_some(name.as_str())
}

/// Where each page a round built is published, by how a problem met
/// building it names it: by what names it and where, as the round marks
/// such a problem. None where two pages are named alike.
pub(super) struct Addresses(HashMap<(Option<Named>, Option<Place>), Option<String>>);

impl Addresses {
    pub(super) fn of(read: &Read) -> Addresses {
        let renderer = &read.renderer;
        let (site, sources) = (renderer.site(), renderer.sources());
        let base = renderer.settings().base();
        let mut addresses = HashMap::new();
        let mut add = |named: Option<Named>, head: Option<Place>, address: &str| {
            let href = on_the_wire(&format!("{base}{address}"));
            addresses
                .entry((named, head))
                .and_modify(|held: &mut Option<String>| {
                    if held.as_ref() != Some(&href) {
                        *held = None;
                    }
                })
                .or_insert_with(|| Some(href.clone()));
        };
        for (id, page) in site.pages.iter() {
            // A page of the author's is named by its place alone, and a
            // list's page Statac made by its list.
            let named = match (page.source, page.speaks_for) {
                (Some(_), _) => None,
                (None, Some(_)) => Some(site.named(id, sources)),
                (None, None) => continue,
            };
            add(named, site.head(id, sources), page.address.as_str());
        }
        for &(id, number) in read.pages.values() {
            let page = site.pages.get(id);
            let later = (page.speaks_for)
                .filter(|_| number > 1)
                .and_then(|list| site.lists.get(list).later_address(number));
            if let Some(address) = later {
                let named = Named::later(number, format!("{base}{}", page.address.as_str()));
                add(Some(named), site.head(id, sources), address.as_str());
            }
        }
        Addresses(addresses)
    }

    fn href(&self, named: Option<&Named>, head: Option<Place>) -> Option<String> {
        if let Some(Named::Feed(feed)) = named {
            return Some(on_the_wire(&feed.address));
        }
        self.0.get(&(named.cloned(), head)).cloned().flatten()
    }
}

/// What `statac explain` says of a kind, as the panel shows it.
struct Explained {
    /// Its heading.
    title: String,
    /// Its heading, its opening and how to fix it, as JSON.
    json: String,
}

/// The heading of the section of an explanation that says how to fix it.
const FIX: &str = "How to fix it";

fn explained(code: Code) -> &'static Explained {
    static EXPLAINED: OnceLock<HashMap<Code, Explained>> = OnceLock::new();
    let every = EXPLAINED.get_or_init(|| {
        let every = Code::ALL.iter();
        every.map(|&code| (code, explain(code))).collect()
    });
    &every[&code]
}

/// The heading of the explanation of a kind.
pub(super) fn title(code: Code) -> &'static str {
    &explained(code).title
}

/// The explanation of each of `codes`, once each, by its kind's name.
pub(super) fn explanations(codes: impl IntoIterator<Item = Code>) -> String {
    let mut seen = Vec::new();
    let mut said = Vec::new();
    for code in codes {
        if !seen.contains(&code) {
            seen.push(code);
            said.push(format!(
                "{}:{}",
                json::string(code.name()),
                explained(code).json
            ));
        }
    }
    format!("{{{}}}", said.join(","))
}

/// Where the docs are published. A link in an explanation to a URL
/// starting with `/` is to a page of theirs.
const DOCS: &str = "https://statac.dev/";

/// An explanation's heading, the blocks before its first section, and
/// those of the section saying how to fix it.
fn explain(code: Code) -> Explained {
    let written = code.explanation().replace("\r\n", "\n");
    let title = (written.lines().next())
        .and_then(|line| line.strip_prefix("# "))
        .map(unanchored)
        .unwrap_or_default();
    let [opening, fix] = parts(&written);
    Explained {
        json: format!(
            "{{\"title\":{},\"opening\":[{opening}],\"fix\":[{fix}]}}",
            json::string(title),
        ),
        title: title.to_owned(),
    }
}

/// The blocks before an explanation's first section, and those of the
/// section saying how to fix it, each as JSON, read as the docs read
/// them. A paragraph is its pieces: its words, and the code, bold,
/// emphasis and links among them, a link's URL as the docs publish it. A
/// block of code is as written, and a list is its items, each its own
/// blocks. Anything else is as written, which no explanation needs.
fn parts(written: &str) -> [String; 2] {
    let document = read(written);
    sections(&document).map(|blocks| {
        let blocks: Vec<String> = blocks.iter().map(|node| block(node, written)).collect();
        blocks.join(",")
    })
}

/// An explanation read as Markdown. It calls no snippet.
fn read(written: &str) -> Document {
    let mut sources = Sources::default();
    let source = sources.add("explanation.md", written.to_owned());
    let whole = Span::new(0, written.len());
    markdown::read(&sources, source, whole, &|_| None, "")
}

/// The blocks before an explanation's first section, and those of the
/// section saying how to fix it.
fn sections(document: &Document) -> [Vec<&Node>; 2] {
    let mut parts: [Vec<&Node>; 2] = Default::default();
    // The part being read, where it is one the panel shows.
    let mut part = Some(0);
    for node in &document.root.children {
        match node.kind {
            Kind::Heading { level: 1, .. } => {}
            Kind::Heading { level: 2, .. } => part = (words(node) == FIX).then_some(1),
            _ => {
                if let Some(part) = part {
                    parts[part].push(node);
                }
            }
        }
    }
    parts
}

/// The words of a heading.
fn words(node: &Node) -> String {
    let mut words = String::new();
    node.walk(&mut |node| {
        if let Kind::Text(text) | Kind::Literal(text) | Kind::Code(text) = &node.kind {
            words.push_str(text);
        }
    });
    words
}

/// A block of an explanation, as JSON.
fn block(node: &Node, written: &str) -> String {
    match &node.kind {
        Kind::Paragraph { .. } => format!("{{\"said\":{}}}", said(&node.children, written)),
        Kind::CodeBlock { text, .. } => {
            let text = text.strip_suffix('\n').unwrap_or(text);
            format!("{{\"code\":{}}}", json::string(text))
        }
        Kind::List { start } => {
            let items: Vec<String> = (node.children.iter())
                .map(|item| {
                    let blocks: Vec<String> = (item.children.iter())
                        .map(|node| block(node, written))
                        .collect();
                    format!("[{}]", blocks.join(","))
                })
                .collect();
            let start = start.map_or_else(String::new, |start| format!(",\"start\":{start}"));
            format!("{{\"list\":[{}]{start}}}", items.join(","))
        }
        _ => format!(
            "{{\"said\":[{{\"text\":{}}}]}}",
            json::string(&written[node.place.span.range()])
        ),
    }
}

/// The pieces of a paragraph of an explanation, as JSON: its words, and
/// the code, bold, emphasis and links among them.
fn said(nodes: &[Node], written: &str) -> String {
    let mut pieces = Vec::new();
    let mut words = String::new();
    for node in nodes {
        let piece = match &node.kind {
            Kind::Text(text) | Kind::Literal(text) => {
                words.push_str(text);
                continue;
            }
            Kind::SoftBreak => {
                words.push(' ');
                continue;
            }
            Kind::Code(code) => format!("{{\"code\":{}}}", json::string(code)),
            Kind::Strong => format!("{{\"strong\":{}}}", said(&node.children, written)),
            Kind::Emphasis => format!("{{\"em\":{}}}", said(&node.children, written)),
            Kind::Link { destination, .. } => {
                let href = match destination.strip_prefix('/') {
                    Some(path) if !path.starts_with('/') => format!("{DOCS}{path}"),
                    _ => destination.clone(),
                };
                format!(
                    "{{\"link\":{},\"href\":{}}}",
                    said(&node.children, written),
                    json::string(&href)
                )
            }
            _ => {
                words.push_str(&written[node.place.span.range()]);
                continue;
            }
        };
        if !words.is_empty() {
            pieces.push(format!("{{\"text\":{}}}", json::string(&words)));
            words.clear();
        }
        pieces.push(piece);
    }
    if !words.is_empty() {
        pieces.push(format!("{{\"text\":{}}}", json::string(&words)));
    }
    format!("[{}]", pieces.join(","))
}

/// The problems not quietened, counted by severity, as JSON.
pub(super) fn counts(problems: &[Problem], quiet: &Quiet) -> String {
    let count = |severity| {
        let of = |problem: &&Problem| problem.kind.severity() == severity;
        let shown = |problem: &&Problem| !quiet.quietens(&problem.kind);
        problems.iter().filter(of).filter(shown).count()
    };
    format!(
        "{{\"error\":{},\"warning\":{},\"note\":{}}}",
        count(Severity::Error),
        count(Severity::Warning),
        count(Severity::Note)
    )
}

/// How long each part of a round took, in milliseconds, as JSON.
pub(super) fn took(took: &Took) -> String {
    let ms = |part: Duration| (part.as_secs_f64() * 10_000.0).round() / 10.0;
    format!(
        "{{\"before\":{},\"reading\":{},\"comparing\":{},\"building\":{},\"concluding\":{},\
         \"writing\":{}}}",
        ms(took.before),
        ms(took.reading),
        ms(took.comparing),
        ms(took.building),
        ms(took.concluding),
        ms(took.writing),
    )
}

/// The problems counted by kind, as the members of a JSON list: errors,
/// then warnings, then notes, the kind with most first within each. A
/// kind whose problems all suggest the same, its quietened ones only
/// where it has no others, has that suggestion.
pub(super) fn kinds(problems: &[Problem], quiet: &Quiet) -> String {
    let mut counted: HashMap<Code, [Vec<&Problem>; 2]> = HashMap::new();
    for problem in problems {
        let quietened = usize::from(quiet.quietens(&problem.kind));
        counted.entry(problem.kind.code()).or_default()[quietened].push(problem);
    }
    let mut counted: Vec<_> = counted.into_iter().collect();
    counted.sort_by_key(|(code, [heard, hushed])| {
        let count = heard.len() + hushed.len();
        (code.severity(), std::cmp::Reverse(count), code.name())
    });
    let kinds: Vec<String> = (counted.into_iter())
        .map(|(code, [heard, hushed])| {
            let (count, quietened) = (heard.len() + hushed.len(), hushed.len());
            let told = if heard.is_empty() { hushed } else { heard };
            let mut said = told.iter().map(|problem| problem.suggestion());
            let first = said.next().flatten();
            let shared = first.filter(|first| said.all(|other| other.as_ref() == Some(first)));
            format!(
                "{{\"kind\":{},\"severity\":{},\"title\":{},\"count\":{count},\
                 \"quietened\":{quietened},\"suggestion\":{}}}",
                json::string(code.name()),
                json::string(wording::severity(code.severity())),
                json::string(title(code)),
                shared.map_or_else(|| "null".to_owned(), |said| pieced(&shown(&said))),
            )
        })
        .collect();
    kinds.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether `block` and `said` draw a kind of node, rather than show it
    /// as written.
    fn drawn(kind: &Kind) -> bool {
        matches!(
            kind,
            Kind::Paragraph { .. }
                | Kind::CodeBlock { .. }
                | Kind::List { .. }
                | Kind::Item
                | Kind::Text(_)
                | Kind::Literal(_)
                | Kind::SoftBreak
                | Kind::Code(_)
                | Kind::Strong
                | Kind::Emphasis
                | Kind::Link { .. }
        )
    }

    #[test]
    fn an_explanation_keeps_its_links_bold_lists_and_code() {
        let written = "# Kind\n\
            \n\
            Words with `code`, **bold**, *emphasis*,\n\
            a [page](/writing/#maths) and [another](https://statac.dev/e/broken-link/).\n\
            \n\
            - One item.\n\
            - Another item,\n  \
              over two lines.\n\
            \n\
            ## What you'll see {#what-youll-see}\n\
            \n\
            Not shown.\n\
            \n\
            ## How to fix it {#fix}\n\
            \n\
            **A cause.** What to do.\n\
            \n\
            3. Third.\n\
            4. Fourth.\n\
            \n\
            ```yaml\n\
            title: Site\n\
            ```\n\
            \n\
            > Shown as written.\n\
            \n\
            ## Example\n\
            \n\
            Not shown either.\n";
        let [opening, fix] = parts(written);
        assert_eq!(
            opening,
            r#"{"said":[{"text":"Words with "},{"code":"code"},{"text":", "},{"strong":[{"text":"bold"}]},{"text":", "},{"em":[{"text":"emphasis"}]},{"text":", a "},{"link":[{"text":"page"}],"href":"https://statac.dev/writing/#maths"},{"text":" and "},{"link":[{"text":"another"}],"href":"https://statac.dev/e/broken-link/"},{"text":"."}]},{"list":[[{"said":[{"text":"One item."}]}],[{"said":[{"text":"Another item, over two lines."}]}]]}"#
        );
        assert_eq!(
            fix,
            r#"{"said":[{"strong":[{"text":"A cause."}]},{"text":" What to do."}]},{"list":[[{"said":[{"text":"Third."}]}],[{"said":[{"text":"Fourth."}]}]],"start":3},{"code":"title: Site"},{"said":[{"text":"\u003e Shown as written."}]}"#
        );
    }

    #[test]
    fn the_panel_draws_every_explanation_and_each_link_goes_to_the_docs() {
        for &code in Code::ALL {
            let name = code.name();
            let written = code.explanation().replace("\r\n", "\n");
            let document = read(&written);
            let [opening, fix] = sections(&document);
            assert!(!opening.is_empty() && !fix.is_empty(), "{name}");
            for node in opening.into_iter().chain(fix) {
                node.walk(&mut |node| assert!(drawn(&node.kind), "{name}: {:?}", node.kind));
            }
            let json = &explained(code).json;
            for (at, _) in json.match_indices("\"href\":") {
                let href = &json[at + "\"href\":".len()..];
                assert!(href.starts_with(&format!("\"{DOCS}")), "{name}: {href}");
            }
        }
    }
}
