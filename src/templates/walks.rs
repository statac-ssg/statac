//! The note for a loop over a list of the whole site's pages that so many
//! pages run that the build's work grows with the square of the site.
//!
//! What a build went through is counted as it happens: each output counts
//! the pages of such lists it reached. Which loop did it is then looked
//! for in the templates the output was built from, since the engine
//! doesn't say where a list is walked.

use std::collections::{BTreeMap, HashMap};

use minijinja::machinery::ast::{Expr, ForLoop, Stmt};

use super::check::{full_span, parse_template};
use super::render::Renderer;
use crate::problem::{Kind, Problem};
use crate::reads::{Key, Reads};
use crate::site::{Id, TemplateId};
use crate::source::{Place, Span};

/// How many pages of lists of the whole site the outputs running one loop
/// reach between them before it is noted: about what a site of 2,000 pages
/// reaches with one loop over them all on every page.
pub(super) const MANY: u64 = 5_000_000;

/// How many pages of such lists one output reaches before it counts as
/// running a loop over one: a menu of a few pages chosen from them all,
/// built on every page of a very large site, is no loop over every page.
const LONG: u64 = 500;

/// The filters that hand on some of what they are handed, so a loop over
/// what they give is not one over the whole list.
const NARROWS: &[&str] = &[
    "selectattr",
    "rejectattr",
    "select",
    "reject",
    "tagged",
    "in",
    "where",
    "batch",
    "slice",
];

/// A loop over a whole list of pages, as a template writes it.
#[derive(Clone)]
struct Loop {
    place: Place,
    written: String,
    instead: Option<String>,
}

/// The outputs running one loop: the loop, where a template writes it,
/// how many they are and how many pages they reached between them.
struct Running {
    found: Option<Loop>,
    pages: usize,
    walked: u64,
}

/// Whether a loop's `iter` names a whole list of pages, the filters it
/// goes through aside: `site.posts`, `page.folder.pages | reverse`. Not a
/// slice of one, nor what a filter that hands on some of it gives, nor
/// `page.posts`.
fn whole_list(iter: &Expr<'_>) -> bool {
    match iter {
        // A page's own `posts` are the few its page of a list shows.
        Expr::GetAttr(get) => match (&get.expr, get.name) {
            (Expr::Var(of), "posts") => of.id != "page",
            (_, name) => matches!(name, "pages" | "posts"),
        },
        Expr::Filter(filter) => {
            !NARROWS.contains(&filter.name) && filter.expr.as_ref().is_some_and(whole_list)
        }
        _ => false,
    }
}

/// The list written to choose the pages a loop keeps with `if`, where it
/// keeps those with a field of the page: `site.pages | selectattr("menu")`
/// for `for entry in site.pages if entry.menu`.
fn instead(looped: &ForLoop<'_>, written: &str) -> Option<String> {
    let (Expr::Var(target), Some(Expr::GetAttr(get))) = (&looped.target, &looped.filter_expr)
    else {
        return None;
    };
    match &get.expr {
        Expr::Var(of) if of.id == target.id => {
            Some(format!("{written} | selectattr(\"{}\")", get.name))
        }
        _ => None,
    }
}

fn loops_in(stmts: &[Stmt<'_>], text: &str, source: crate::source::SourceId, out: &mut Vec<Loop>) {
    for stmt in stmts {
        let inner: &[&[Stmt<'_>]] = match stmt {
            Stmt::Template(t) => &[&t.children],
            Stmt::ForLoop(looped) => {
                if whole_list(&looped.iter) {
                    let span = full_span(&looped.iter, text);
                    let (start, end) = (span.start_offset as usize, span.end_offset as usize);
                    let written = &text[start..end];
                    out.push(Loop {
                        place: Place {
                            source,
                            span: Span::new(start, end),
                        },
                        written: written.to_owned(),
                        instead: instead(looped, written),
                    });
                }
                &[&looped.body, &looped.else_body]
            }
            Stmt::IfCond(i) => &[&i.true_body, &i.false_body],
            Stmt::WithBlock(w) => &[&w.body],
            Stmt::SetBlock(s) => &[&s.body],
            Stmt::AutoEscape(a) => &[&a.body],
            Stmt::FilterBlock(f) => &[&f.body],
            Stmt::Block(b) => &[&b.body],
            Stmt::Macro(m) => &[&m.body],
            Stmt::CallBlock(c) => &[&c.macro_decl.body],
            _ => continue,
        };
        for body in inner {
            loops_in(body, text, source, out);
        }
    }
}

impl Renderer {
    /// The loops over a whole list of pages the template `id` writes.
    fn loops_of(&self, id: TemplateId) -> Vec<Loop> {
        let shared = &self.shared;
        let file = &shared.templates.files[id.index()];
        let text = shared.sources.text(file.source);
        let mut loops = Vec::new();
        if let Ok(ast) = parse_template(&file.name, text) {
            loops_in(std::slice::from_ref(&ast), text, file.source, &mut loops);
        }
        loops
    }

    /// A note for each loop over a whole list of pages that the outputs
    /// running it took through [`MANY`] pages or more between them, each
    /// output given as how many it reached and what it read; an output
    /// that reached fewer than [`LONG`] runs none. Where the templates an
    /// output was built from write no such loop, or more than one, the
    /// note is of the build.
    pub fn loops_over_every_page<'a>(
        &self,
        outputs: impl Iterator<Item = (u64, &'a Reads)>,
    ) -> Vec<Problem> {
        let shared = &self.shared;
        let many = shared.templates.many_walked;
        // A test that notes loops sooner counts shorter ones too.
        let long = LONG.min(many.isqrt());
        let walking: Vec<(u64, &Reads)> = outputs.filter(|(walked, _)| *walked >= long).collect();
        if walking.iter().map(|(walked, _)| walked).sum::<u64>() < many {
            return Vec::new();
        }
        let mut known: HashMap<TemplateId, Vec<Loop>> = HashMap::new();
        // By the file and place of the loop, so the notes come in one order.
        let mut told: BTreeMap<Option<(String, u32)>, Running> = BTreeMap::new();
        for (walked, reads) in walking {
            let mut templates: Vec<TemplateId> = (reads.keys().iter())
                .filter_map(|key| match key {
                    Key::Template(id) => Some(*id),
                    _ => None,
                })
                .collect();
            templates.sort_unstable();
            templates.dedup();
            let at = |found: &Loop| {
                let path = shared.sources.path(found.place.source);
                (
                    path.to_string_lossy().replace('\\', "/"),
                    found.place.span.start,
                )
            };
            // One loop is the one; of several, which ran is not known.
            let mut written = templates
                .into_iter()
                .flat_map(|id| known.entry(id).or_insert_with(|| self.loops_of(id)).clone());
            let found = match (written.next(), written.next()) {
                (Some(only), None) => Some(only),
                _ => None,
            };
            let running = told.entry(found.as_ref().map(at)).or_insert(Running {
                found,
                pages: 0,
                walked: 0,
            });
            running.pages += 1;
            running.walked += walked;
        }
        told.into_values()
            .filter(|running| running.walked >= many)
            .map(|running| match running.found {
                Some(found) => Problem::at(
                    Kind::LoopOverEveryPage {
                        written: Some(found.written),
                        pages: running.pages,
                        instead: found.instead,
                    },
                    found.place,
                ),
                None => Problem::nowhere(Kind::LoopOverEveryPage {
                    written: None,
                    pages: running.pages,
                    instead: None,
                }),
            })
            .collect()
    }
}
