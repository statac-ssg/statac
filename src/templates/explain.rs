//! What stopped a template, told as a problem at the exact text: which
//! value was missing, on which page, and the calls that led there.
//!
//! The engine says where it stopped, but not which lookup found nothing.
//! That is found again here: the lookups inside the place it stopped are
//! matched against the names Statac's own objects say they were asked for
//! and didn't have, most recent first. A guard that refused a value says
//! which value it was handed, and the template's text is followed into the
//! list or map holding it.

use std::ops::Range;
use std::sync::Arc;

use minijinja::machinery::ast::{CallArg, CallType, Expr};
use minijinja::{Error, ErrorKind};

use super::check::{full_span, parse_template, visit_exprs};
use super::files::Role;
use super::frame::{Frame, InData, Miss, RAISED, TOLD};
use super::guard::{Callee, REFUSED, Refusal, Role as Held, Step};
use super::render::{Shared, building, met_building};
use super::values::PageValue;
use crate::problem::{
    ArgumentFault, Instead, Kind, Lack, Named, Problem, Relation, Stopped, Unsupported,
};
use crate::source::{Place, SourceId, Span};

/// The words of the error a snippet's failure is wrapped in, so the
/// engine places the wrapping at the call.
pub(super) const IN_SNIPPET: &str = "in a snippet";

/// The problem `error` comes to, with `callers` the places outside any
/// template that led to it, innermost first. Nothing for a problem told
/// already.
pub(super) fn explain(error: &Error, frame: &mut Frame, callers: &[Place]) -> Vec<Problem> {
    let shared = Arc::clone(&frame.shared);
    let mut chain: Vec<&Error> = vec![error];
    let mut next = std::error::Error::source(error);
    while let Some(inner) = next {
        match inner.downcast_ref::<Error>() {
            Some(e) => chain.push(e),
            None => break,
        }
        next = inner.source();
    }
    if chain.iter().any(|e| e.detail() == Some(TOLD)) {
        return Vec::new();
    }
    let located = |e: &Error| -> Option<(SourceId, Range<usize>)> {
        let at = *shared.templates.by_name.get(e.name()?)?;
        let source = shared.templates.files[at].source;
        let len = shared.sources.text(source).len();
        let range = e.range()?;
        let start = range.start.min(len);
        Some((source, start..range.end.clamp(start, len)))
    };
    let Some(failed_at) = chain.iter().rposition(|e| located(e).is_some()) else {
        // The engine doesn't always say where its steps ran out: told
        // at the head of the template the output is built with, so
        // alike for every page built with it.
        if chain.iter().any(|e| e.kind() == ErrorKind::OutOfFuel)
            && let Some(layout) = frame.layout
        {
            let source = shared.templates.files[layout].source;
            let head = crate::content::words::head(&shared.sources, source);
            let kind = super::frame::ran_on(frame, layout);
            return vec![met_building(Problem::at(kind, head), frame)];
        }
        let kind = match frame.raised.take() {
            Some(kind) if error.detail() == Some(RAISED) => kind,
            _ => engine(error),
        };
        // With no template to place it in, it is placed in the page's own
        // file where it has one.
        let problem = match (callers.first(), building(frame)) {
            (Some(&at), _) => Problem::at(kind, at),
            (None, Some((Relation::Building(None), head))) => Problem::placed(kind, head),
            (None, _) => Problem::nowhere(kind),
        };
        return vec![met_building(problem, frame)];
    };
    let failed = chain[failed_at];
    let (source, range) = located(failed).expect("located above");
    let file = shared.templates.by_name[failed.name().expect("located above")];
    let snippet = match &shared.templates.files[file].role {
        Role::Snippet(name) => shared.templates.snippets.get(name),
        Role::Template | Role::Piece(_) | Role::CardText => None,
    };
    let text = shared.sources.text(source);
    let name = failed.name().unwrap_or_default();
    let mut calls: Vec<Place> = chain[..failed_at]
        .iter()
        .rev()
        .filter_map(|e| located(e))
        .map(|(source, range)| place(source, range))
        .collect();
    calls.extend(callers);

    let (kind, at) = if failed.kind() == ErrorKind::OutOfFuel {
        (super::frame::ran_on(frame, file), widen(name, text, &range))
    } else if failed.detail() == Some(RAISED)
        && let Some(kind) = frame.raised.take()
    {
        let span = widen(name, text, &range);
        let written = &text[span.clone()];
        let kind = match kind {
            Kind::NotSupported {
                what: Unsupported::NotYet(what),
            } if written.ends_with(what.rsplit('.').next().unwrap_or_default())
                && written.contains('.') =>
            {
                Kind::NotSupported {
                    what: Unsupported::NotYet(written.to_owned()),
                }
            }
            kind => kind,
        };
        (kind, span)
    } else if let Some(missing) = missing(failed, frame, name, text, &range) {
        match missing {
            Missing::Found(kind, span) => (kind, span),
            Missing::Value {
                span,
                written,
                miss,
                empty,
            } => {
                if !empty
                    && let Some(snippet) = snippet
                    && let Some(given) = not_given(&written, snippet, frame)
                {
                    // The writing a call wraps, line by line, which the
                    // writing can't hand a snippet yet.
                    let kind = match given == "lines" && !callers.is_empty() {
                        true => Kind::NotSupported {
                            what: Unsupported::NotYet(given),
                        },
                        false => Kind::WrongArgument {
                            snippet: snippet.name.clone(),
                            fault: ArgumentFault::NotGiven(given),
                        },
                    };
                    return vec![told(kind, &calls, place(source, span), frame)];
                }
                let lack = lack_of(&shared, miss.as_ref(), empty);
                let instead = match empty {
                    true => Instead::Default(super::check::defaulted(&written)),
                    false => instead(&written, frame, snippet.is_some(), miss.as_ref()),
                };
                (
                    Kind::NoValue {
                        written,
                        lack,
                        instead,
                    },
                    span,
                )
            }
        }
    } else {
        (engine(failed), widen(name, text, &range))
    };
    let at = place(source, at);
    vec![located_problem(kind, at, calls, frame)]
}

/// A problem in a snippet that its call is to blame for, placed at the
/// call.
fn told(kind: Kind, calls: &[Place], at: Place, frame: &Frame) -> Problem {
    match calls.split_first() {
        Some((&call, rest)) => located_problem(kind, call, rest.to_vec(), frame),
        None => located_problem(kind, at, Vec::new(), frame),
    }
}

fn located_problem(kind: Kind, at: Place, calls: Vec<Place>, frame: &Frame) -> Problem {
    let mut problem = Problem::at(kind, at);
    for call in calls {
        problem = problem.met(Relation::CalledFrom, Some(call));
    }
    met_building(problem, frame)
}

fn place(source: SourceId, range: Range<usize>) -> Place {
    Place {
        source,
        span: Span::new(range.start, range.end),
    }
}

/// The engine's own words for why it stopped.
fn engine(error: &Error) -> Kind {
    let detail = match error.detail() {
        Some(detail) if detail != RAISED && detail != REFUSED && detail != IN_SNIPPET => {
            detail.to_owned()
        }
        _ => error.kind().to_string(),
    };
    Kind::TemplateStopped {
        reason: Stopped::Engine(detail),
    }
}

/// The whole of the expression the engine stopped at: it places a lookup
/// from its dot, and a filter from its name.
fn widen(name: &str, text: &str, range: &Range<usize>) -> Range<usize> {
    let Ok(ast) = parse_template(name, text) else {
        return range.clone();
    };
    let mut widest = range.clone();
    visit_exprs(&ast, &mut |expr| {
        let span = expr.span();
        if span.start_offset as usize == range.start && span.end_offset as usize == range.end {
            let full = full_span(expr);
            widest = full.start_offset as usize..full.end_offset as usize;
        }
    });
    widest
}

/// A value the template stopped for having none.
enum Missing {
    /// Told as it is, such as an item a filter can't sort.
    Found(Kind, Range<usize>),
    Value {
        span: Range<usize>,
        written: String,
        miss: Option<Miss>,
        /// Whether it is there and empty.
        empty: bool,
    },
}

/// The missing value the engine or a guard stopped for, found in the
/// template's text.
fn missing(
    failed: &Error,
    frame: &mut Frame,
    name: &str,
    text: &str,
    range: &Range<usize>,
) -> Option<Missing> {
    // A guard's refusal may reach the engine through a filter of its own,
    // as `map("upper")` hands each item to a filter, which keeps the kind of
    // error and not its words.
    let refusal =
        match failed.detail() == Some(REFUSED) || failed.kind() == ErrorKind::UndefinedError {
            true => frame.refusal.take(),
            false => None,
        };
    if let Some(refusal) = refusal
        && let found = culprit_of_refusal(name, text, range, &refusal)
        && (found.is_some() || refusal.by_field.is_some())
    {
        if let Some((field, item)) = &refusal.by_field {
            let (kind, span) = by_field(name, text, range, &refusal, field, item);
            return Some(Missing::Found(kind, span));
        }
        let (span, rest) = found?;
        let written = text[span.clone()].to_owned();
        let written = match rest.is_empty() {
            true => written,
            false => format!("{written}{}", path_words(&rest)),
        };
        let last = written.rsplit('.').next().unwrap_or_default();
        let miss = frame.misses.iter().rev().find(|m| m.name == last).cloned();
        return Some(Missing::Value {
            span,
            written,
            miss,
            empty: refusal.empty,
        });
    }
    if failed.kind() != ErrorKind::UndefinedError {
        return None;
    }
    let (span, miss) = culprit_of_lookup(name, text, range, &frame.misses)
        .unwrap_or_else(|| (widen(name, text, range), None));
    Some(Missing::Value {
        written: text[span.clone()].to_owned(),
        span,
        miss,
        empty: false,
    })
}

/// `[2]` or `.name`, for the part of a value the template doesn't spell
/// out.
fn path_words(path: &[Step]) -> String {
    path.iter()
        .map(|step| match step {
            Step::Index(i) => format!("[{i}]"),
            Step::Key(key) => format!(".{key}"),
        })
        .collect()
}

fn lack_of(shared: &Shared, miss: Option<&Miss>, empty: bool) -> Lack {
    if empty {
        return Lack::Empty;
    }
    if let Some(data) = miss.and_then(|m| m.data.as_ref()) {
        return match data {
            InData::File(file) => Lack::InDataFile(shared.site.data.file(*file).path.clone()),
            InData::Folder(path) => Lack::InDataFolder(path.clone()),
        };
    }
    match miss.and_then(|m| m.page) {
        Some(page) => Lack::OnPage(shared.site.named(page, &shared.sources)),
        None => Lack::Unset,
    }
}

/// What a missing value's suggestion offers: in a snippet handed a page,
/// that page's own field where `page`'s is missing; for the `title` of a
/// page Statac made, which never has one, its headline.
fn instead(written: &str, frame: &Frame, in_snippet: bool, miss: Option<&Miss>) -> Instead {
    if in_snippet
        && let Some(field) = written.strip_prefix("page.")
        && let Some(handed) = &frame.handed
    {
        let field = field.split(['.', '[']).next().unwrap_or(field);
        let offered = handed.values.iter().find_map(|(param, value)| {
            let page = value.downcast_object_ref::<PageValue>()?;
            let has = page.shared.site.pages.get(page.id);
            let found = has.words.field(field).is_some()
                || crate::content::words::PageWord::named(field)
                    .is_some_and(|w| has.words.get(w).is_some())
                || (field == "title" && has.title.is_some())
                || (field == "date" && has.date.is_some());
            found.then(|| format!("{param}.{field}"))
        });
        if let Some(offered) = offered {
            return Instead::Handed(offered);
        }
    }
    if let Some(Miss {
        page: Some(page),
        name,
        ..
    }) = miss
        && name == "title"
        && frame.shared.site.pages.get(*page).place == crate::site::PagePlace::Made
        && let Some(base) = written.strip_suffix(".title")
    {
        return Instead::Headline(format!("headline({base})"));
    }
    Instead::Default(super::check::defaulted(written))
}

/// The name a snippet reads that its call didn't hand it, where that is
/// what was missing.
fn not_given(written: &str, snippet: &super::files::Snippet, frame: &Frame) -> Option<String> {
    let root = written
        .split(['.', '[', ' ', '|'])
        .next()
        .unwrap_or(written)
        .trim();
    let handed = frame.handed.as_ref()?;
    if handed.snippet != snippet.name {
        return None;
    }
    let reads =
        snippet.params.iter().any(|p| p == root) || snippet.wrapped.iter().any(|w| w == root);
    (reads && !handed.values.iter().any(|(h, _)| h == root)).then(|| root.to_owned())
}

/// An item of a filter's, as a page is named where it is one, else by its
/// place in the list.
fn item_named(item: &minijinja::Value, index: Option<usize>) -> Named {
    match item.downcast_object_ref::<PageValue>() {
        Some(page) => page.named(),
        None => match index {
            Some(i) => Named::Item(format!("item {}", i + 1)),
            None => Named::Item("an item".into()),
        },
    }
}

/// The filter `expr` written to take only the items that have `field`.
fn leave_out(text: &str, expr: &Expr<'_>, input: Option<&Expr<'_>>, field: &str) -> Option<String> {
    let input = full_span(input?);
    let input = &text[input.start_offset as usize..input.end_offset as usize];
    let filter = expr.span();
    let filter = &text[filter.start_offset as usize..filter.end_offset as usize];
    Some(format!(
        "{input} | selectattr(\"{field}\", \"defined\") | {filter}"
    ))
}

/// `sort` put `count` items without `field` last, the first of them
/// `first` at `index`: told at the `attribute=` naming it in `template`,
/// the template running the sort. The engine doesn't say where a filter
/// runs, so the sort is found by its `attribute=` as written; a template
/// sorting by the same twice is told at the first.
pub(super) fn sorted_last(
    frame: &Frame,
    template: &str,
    attribute: &str,
    field: &str,
    count: usize,
    (first, index): (&minijinja::Value, usize),
) -> Problem {
    let shared = &frame.shared;
    let first = item_named(first, Some(index));
    let mut found = None;
    if let Some(&at) = shared.templates.by_name.get(template) {
        let source = shared.templates.files[at].source;
        let text = shared.sources.text(source);
        if let Ok(ast) = parse_template(template, text) {
            visit_exprs(&ast, &mut |expr| {
                if found.is_some() {
                    return;
                }
                if let Expr::Filter(f) = expr
                    && f.name == "sort"
                    && let Some(named @ Expr::Const(c)) = kw(&f.args, "attribute")
                    && c.value.as_str() == Some(attribute)
                {
                    let span = full_span(named);
                    let instead = leave_out(text, expr, f.expr.as_ref(), field);
                    found = Some((
                        place(source, span.start_offset as usize..span.end_offset as usize),
                        instead,
                    ));
                }
            });
        }
    }
    let kind = |instead| Kind::SortedLast {
        field: field.to_owned(),
        count,
        first,
        instead,
    };
    match found {
        Some((at, instead)) => Problem::at(kind(instead), at),
        None => met_building(Problem::nowhere(kind(None)), frame),
    }
}

/// An item lacks the field a filter groups, tells items apart or gathers
/// by.
fn by_field(
    name: &str,
    text: &str,
    range: &Range<usize>,
    refusal: &Refusal,
    field: &str,
    item: &minijinja::Value,
) -> (Kind, Range<usize>) {
    let index = match refusal.path.first() {
        Some(Step::Index(i)) => Some(*i),
        _ => None,
    };
    let who = item_named(item, index);
    let mut at = range.clone();
    let mut instead = Instead::Nothing;
    // Of the filters the engine's place covers, the one naming the field.
    let mut named_it = false;
    if let Ok(ast) = parse_template(name, text) {
        visit_exprs(&ast, &mut |expr| {
            if let Expr::Filter(f) = expr
                && f.name == refusal.callee
                && (expr.span().start_offset as usize) < range.end
                && (expr.span().end_offset as usize) > range.start
            {
                let named = kw(&f.args, "attribute").or_else(|| pos(&f.args, 0));
                let names = named.is_some_and(|n| match n {
                    Expr::Const(c) => c
                        .value
                        .as_str()
                        .is_some_and(|s| s.split(',').any(|p| p.trim() == field)),
                    _ => false,
                });
                if named_it && !names {
                    return;
                }
                named_it = names;
                if let Some(named) = named {
                    let span = full_span(named);
                    at = span.start_offset as usize..span.end_offset as usize;
                }
                if let Some(written) = leave_out(text, expr, f.expr.as_ref(), field) {
                    instead = Instead::Selected(written);
                }
            }
        });
    }
    (
        Kind::NoValue {
            written: field.to_owned(),
            lack: Lack::ByField(Box::new(crate::problem::ByField {
                item: who,
                field: field.to_owned(),
                filter: refusal.callee.clone(),
            })),
            instead,
        },
        at,
    )
}

/// Where in the template the missing value a guard refused is written,
/// and the part of it the template doesn't spell out.
fn culprit_of_refusal(
    name: &str,
    text: &str,
    range: &Range<usize>,
    refusal: &Refusal,
) -> Option<(Range<usize>, Vec<Step>)> {
    let ast = parse_template(name, text).ok()?;
    let within = |s: minijinja::machinery::Span| {
        (s.start_offset as usize) <= range.end && (s.end_offset as usize) >= range.start
    };
    let mut target = None;
    visit_exprs(&ast, &mut |expr| {
        if target.is_some() {
            return;
        }
        let hit: Option<&Expr<'_>> = match (refusal.kind, expr) {
            (Callee::Filter, Expr::Filter(f))
                if f.name == refusal.callee && within(expr.span()) =>
            {
                match &refusal.role {
                    Held::Input => f.expr.as_ref(),
                    Held::Arg(n) => pos(&f.args, *n),
                    Held::Kwarg(k) => kw(&f.args, k),
                }
            }
            (Callee::Test, Expr::Test(t)) if t.name == refusal.callee && within(expr.span()) => {
                match &refusal.role {
                    Held::Input => Some(&t.expr),
                    Held::Arg(n) => pos(&t.args, *n),
                    Held::Kwarg(k) => kw(&t.args, k),
                }
            }
            (Callee::Function | Callee::Snippet, Expr::Call(c)) if within(expr.span()) => {
                match c.identify_call() {
                    CallType::Function(n) if n == refusal.callee => match &refusal.role {
                        Held::Arg(n) => pos(&c.args, *n),
                        Held::Kwarg(k) => kw(&c.args, k),
                        Held::Input => None,
                    },
                    _ => None,
                }
            }
            (Callee::Print, _)
                if [expr.span(), full_span(expr)].iter().any(|s| {
                    s.start_offset as usize == range.start && s.end_offset as usize == range.end
                }) =>
            {
                Some(expr)
            }
            _ => None,
        };
        if let Some(hit) = hit {
            target = Some(descend(hit, &refusal.path, refusal.empty));
        }
    });
    target
}

/// What left a missing value missing: `a | default(b)` is missing only
/// where `b` is, so the blame follows each `default` to what it gives
/// instead. An empty value may have come from either, so it isn't
/// followed.
fn left_missing<'e, 'a>(expr: &'e Expr<'a>, empty: bool) -> &'e Expr<'a> {
    match expr {
        Expr::Filter(f) if !empty && matches!(f.name, "default" | "d") => {
            pos(&f.args, 0).map_or(expr, |instead| left_missing(instead, empty))
        }
        _ => expr,
    }
}

/// Follows a path into list and map literals, as far as the template
/// spells them out, and on to what left the value there missing.
fn descend(expr: &Expr<'_>, path: &[Step], empty: bool) -> (Range<usize>, Vec<Step>) {
    let span = full_span(expr);
    let here = span.start_offset as usize..span.end_offset as usize;
    let Some((step, rest)) = path.split_first() else {
        let span = full_span(left_missing(expr, empty));
        return (
            span.start_offset as usize..span.end_offset as usize,
            Vec::new(),
        );
    };
    match (step, expr) {
        (Step::Index(i), Expr::List(list)) if *i < list.items.len() => {
            descend(&list.items[*i], rest, empty)
        }
        (Step::Key(k), Expr::Map(map)) => {
            for (key, value) in map.keys.iter().zip(&map.values) {
                if let Expr::Const(c) = key
                    && c.value.as_str() == Some(k.as_str())
                {
                    return descend(value, rest, empty);
                }
            }
            (here, path.to_vec())
        }
        _ => (here, path.to_vec()),
    }
}

fn pos<'e, 'a>(args: &'e [CallArg<'a>], n: usize) -> Option<&'e Expr<'a>> {
    args.iter()
        .filter_map(|a| match a {
            CallArg::Pos(e) => Some(e),
            _ => None,
        })
        .nth(n)
}

fn kw<'e, 'a>(args: &'e [CallArg<'a>], name: &str) -> Option<&'e Expr<'a>> {
    args.iter().find_map(|a| match a {
        CallArg::Kwarg(k, e) if *k == name => Some(e),
        _ => None,
    })
}

/// Which lookup inside the place the engine stopped found nothing: the one
/// whose name was most recently missed; else the lookup or the name the
/// place is, as for a value a snippet's call left out.
fn culprit_of_lookup(
    name: &str,
    text: &str,
    range: &Range<usize>,
    misses: &[Miss],
) -> Option<(Range<usize>, Option<Miss>)> {
    let ast = parse_template(name, text).ok()?;
    let mut lookups: Vec<(Range<usize>, String)> = Vec::new();
    let mut names: Vec<Range<usize>> = Vec::new();
    visit_exprs(&ast, &mut |expr| {
        let span = full_span(expr);
        let (s, e) = (span.start_offset as usize, span.end_offset as usize);
        if s > range.end || e < range.start {
            return;
        }
        match expr {
            Expr::GetAttr(a) => lookups.push((s..e, a.name.to_owned())),
            Expr::GetItem(i) => {
                if let Expr::Const(c) = &i.subscript_expr {
                    lookups.push((s..e, c.value.as_str().unwrap_or_default().to_owned()));
                }
            }
            Expr::Var(_) => names.push(s..e),
            _ => {}
        }
    });
    for miss in misses.iter().rev() {
        if let Some((span, _)) = lookups.iter().find(|(_, n)| *n == miss.name) {
            return Some((span.clone(), Some(miss.clone())));
        }
    }
    let outermost = lookups.into_iter().map(|(span, _)| span).next();
    outermost
        .or_else(|| names.into_iter().next())
        .map(|span| (span, None))
}

/// Where the template running now runs the filter `name` on `text`.
/// The engine doesn't say where a filter runs, so it is found in the
/// template: its one such, or else the one whose input, a literal, a
/// name or a path of names, holds `text` here, and whose first value
/// written the same way holds `given`, where there is one; failing those,
/// the first.
pub(super) fn filter(
    shared: &Shared,
    state: &minijinja::State,
    name: &str,
    text: &str,
    given: Option<&str>,
) -> Option<Place> {
    let template = state.name();
    let source = shared.templates.files[*shared.templates.by_name.get(template)?].source;
    let written = shared.sources.text(source);
    let ast = parse_template(template, written).ok()?;
    let mut found: Vec<(Range<usize>, bool)> = Vec::new();
    visit_exprs(&ast, &mut |expr| {
        if let Expr::Filter(f) = expr
            && f.name == name
        {
            let span = full_span(expr);
            let holds = f
                .expr
                .as_ref()
                .and_then(|input| value_of(state, input))
                .is_some_and(|value| value.as_str() == Some(text));
            let first = match f.args.first() {
                Some(CallArg::Pos(first)) => value_of(state, first),
                _ => None,
            };
            let holds = holds
                && given
                    .is_none_or(|given| first.is_some_and(|value| value.as_str() == Some(given)));
            found.push((span.start_offset as usize..span.end_offset as usize, holds));
        }
    });
    let range = match found.as_slice() {
        [(only, _)] => only.clone(),
        _ => found
            .iter()
            .find(|(_, holds)| *holds)
            .or(found.first())
            .map(|(range, _)| range.clone())?,
    };
    Some(place(source, range))
}

/// What a literal, a name or a path of names holds where the template
/// runs; none for any other expression.
fn value_of(state: &minijinja::State, expr: &Expr<'_>) -> Option<minijinja::Value> {
    match expr {
        Expr::Const(c) => Some(c.value.clone()),
        Expr::Var(v) => state.lookup(v.id),
        Expr::GetAttr(a) => value_of(state, &a.expr)?.get_attr(a.name).ok(),
        _ => None,
    }
}
