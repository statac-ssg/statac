//! Problems drawn for a reader, and counted.
//!
//! The words come from `wording`; this module only lays them out. Colour is
//! decoration: everything it marks is also said in the text.

use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::Path;
use std::time::Duration;

use crate::build::{Cards, Copies, Javascript, Outcome};
use crate::content::Included;
use crate::fetched::Ages;
use crate::hooks::{Before, Ran};
use crate::problem::{Problem, Published, Quiet, Severity};
use crate::source::{Place, SourceId, Sources};
use crate::stamp::Pass;
use crate::wording;

/// Columns between tab stops.
const TAB: usize = 4;
/// A line wider than this, in columns, is shown as a window around the place.
const LONGEST: usize = 100;
/// The window's width in columns, and how far into it the place starts.
const WINDOW: usize = 80;
const LEAD: usize = 24;
const ELLIPSIS: char = '\u{2026}';

/// Whether to colour what is written to `stream`: only a terminal, and
/// never where `NO_COLOR` is set or the terminal says it can't. Where
/// `CLICOLOR_FORCE` is set to anything but 0, anything else is coloured
/// too, a pipe among them; `NO_COLOR` still wins.
pub fn colour_for(stream: &impl IsTerminal) -> bool {
    let set = |name| std::env::var_os(name).is_some_and(|value| !value.is_empty());
    if set("NO_COLOR") {
        return false;
    }
    if std::env::var_os("CLICOLOR_FORCE").is_some_and(|value| !value.is_empty() && value != "0") {
        return true;
    }
    let dumb = std::env::var_os("TERM").is_some_and(|term| term == "dumb");
    stream.is_terminal() && !dumb && crate::platform::console_reads_escapes()
}

/// What opens a run on a terminal that takes colour: Statac's name as a
/// badge, and the command. Nothing anywhere else, since it says nothing
/// the command line didn't.
pub(crate) fn badge(colour: bool, command: &str) -> String {
    let paint = Paint::new(colour);
    match colour {
        true => format!(
            "{} {}\n\n",
            paint.paint(&paint.brand_chip(), &format!(" {} ", wording::NAME)),
            paint.paint(BOLD, command)
        ),
        false => String::new(),
    }
}

/// A question `statac new` asks, in bold.
pub(crate) fn question(colour: bool, text: &str) -> String {
    format!("{}\n", Paint::new(colour).paint(BOLD, text))
}

/// A hint or choice under a question, indented and dim.
pub(crate) fn hint(colour: bool, text: &str) -> String {
    format!("  {}\n", Paint::new(colour).paint(DIM, text))
}

/// The line an answer is typed on: the default dim, where there is one,
/// then the mark in Statac's orange and a space.
pub(crate) fn answer_line(colour: bool, default: Option<&str>) -> String {
    let paint = Paint::new(colour);
    let default = default.map_or_else(String::new, |shown| format!("{} ", paint.paint(DIM, shown)));
    format!(
        "  {default}{} ",
        paint.paint(&paint.brand_ink(), wording::ASK_MARK)
    )
}

/// How many of the other pages whose builds met a problem are named
/// where not every one is.
pub(crate) const NAMED: usize = 5;

/// Draws problems placed in `sources`, naming files relative to the site's folder.
pub struct Report<'a> {
    sources: &'a Sources,
    root: &'a Path,
    paint: Paint,
}

impl<'a> Report<'a> {
    pub fn new(sources: &'a Sources, root: &'a Path, colour: bool) -> Self {
        Report {
            sources,
            root,
            paint: Paint::new(colour),
        }
    }

    /// A sentence of Statac's own as a terminal that takes colour shows
    /// it, each name in backticks picked out.
    pub(crate) fn said(&self, sentence: &str) -> String {
        self.paint.inline("", sentence)
    }

    /// A sentence saying that errors stopped a build, as `build`'s last
    /// line is coloured where it wrote nothing.
    pub(crate) fn stopped(&self, sentence: &str) -> String {
        self.paint.paint(RED, sentence)
    }

    /// What `build` says: errors in full, and with them what `check`
    /// couldn't show, then the other warnings and notes counted.
    pub(crate) fn build(&self, problems: &[Problem], quiet: &Quiet) -> String {
        let (full, counted): (Vec<&Problem>, Vec<&Problem>) = problems.iter().partition(|p| {
            p.kind.severity() == Severity::Error
                || (p.kind.code().found_by_build_alone() && !quiet.quietens(&p.kind))
        });
        let mut out = self.problems(full);
        let counts = counts(&counted, quiet, &[Severity::Warning, Severity::Note]);
        let shown: usize = counts.iter().map(|(_, shown)| shown).sum();
        let pointer = counts.iter().rposition(|&(_, shown)| shown > 0);
        if !out.is_empty() && !counts.is_empty() {
            out.push('\n');
        }
        for (i, (line, _)) in counts.iter().enumerate() {
            out.push_str(&self.paint.counted(line));
            out.push('.');
            if Some(i) == pointer {
                out.push(' ');
                out.push_str(wording::see_them(shown));
            }
            out.push('\n');
        }
        out
    }

    /// What `statac search` found for `words`. Of one language searched:
    /// how many pages, then each shown at the part that matched best, the
    /// words matched in bold, or where none, the words it may have meant.
    /// Of several: that for each language that found any, under its code,
    /// or where none did, the words the first language offering some may
    /// have meant.
    pub(crate) fn found(&self, words: &str, found: &[(String, crate::search::Results)]) -> String {
        if let [(_, results)] = found {
            return self.results(words, results);
        }
        let blocks: Vec<String> = (found.iter())
            .filter(|(_, results)| results.pages > 0)
            .map(|(code, results)| {
                let heading = self.paint.paint(BOLD, code);
                format!("{heading}\n{}", self.results(words, results))
            })
            .collect();
        if !blocks.is_empty() {
            return blocks.join("\n");
        }
        let mut out = format!("{}\n", self.said(&wording::found(words, 0, false)));
        let meant = (found.iter()).find_map(|(_, results)| results.suggestion.as_deref());
        if let Some(meant) = meant {
            out.push_str(&format!("{}\n", self.said(&wording::meant(meant))));
        }
        out
    }

    /// What one language's search found for `words`.
    fn results(&self, words: &str, found: &crate::search::Results) -> String {
        let (shown, pages) = (&found.shown, found.pages);
        let mut out = self.said(&wording::found(words, pages, found.some));
        out.push('\n');
        if let Some(meant) = &found.suggestion {
            out.push_str(&self.said(&wording::meant(meant)));
            out.push('\n');
        }
        for found in shown.iter() {
            let named = match &found.heading {
                Some(heading) => format!("{} \u{203a} {heading}", found.title),
                None => found.title.clone(),
            };
            out.push('\n');
            out.push_str(&format!("  {}\n", self.paint.paint(BOLD, &named)));
            out.push_str(&format!("  {}\n", self.paint.paint(CYAN, &found.address)));
            if !found.excerpt.is_empty() {
                out.push_str("  ");
                for (text, matched) in &found.excerpt {
                    match matched {
                        true => out.push_str(&self.paint.paint(BOLD, text)),
                        // Each line the text runs to is set in with the first.
                        false => out.push_str(&text.replace('\n', "\n  ")),
                    }
                }
                out.push('\n');
            }
        }
        if pages > shown.len() {
            out.push('\n');
            out.push_str(&wording::more_found(pages - shown.len()));
            out.push('\n');
        }
        out
    }

    /// What `--drafts` brought into the run, ahead of the rest; nothing
    /// where it brought nothing.
    pub(crate) fn included(&self, included: Included) -> String {
        match included.drafts + included.waiting {
            0 => String::new(),
            _ => format!(
                "{}\n\n",
                wording::included(included.drafts, included.waiting)
            ),
        }
    }

    /// The JavaScript the site carries, under its heading; nothing
    /// where it carries none.
    pub fn javascript(&self, javascript: &[Javascript]) -> String {
        if javascript.is_empty() {
            return String::new();
        }
        let mut out = format!("{}\n", wording::JAVASCRIPT);
        for each in javascript {
            out.push_str("  ");
            out.push_str(&shown(&wording::javascript(
                &each.carried,
                each.sure,
                each.pages,
            )));
            out.push('\n');
        }
        out
    }

    /// What `build` did with the copies of photographs: those it made, and
    /// those it removed; nothing where it did neither.
    pub(crate) fn copies(&self, copies: Copies) -> String {
        let mut out = String::new();
        if copies.made > 0 {
            out.push_str(&wording::made_copies(copies.made, copies.from));
            out.push('\n');
        }
        if copies.removed > 0 {
            out.push_str(&wording::removed_copies(copies.removed));
            out.push('\n');
        }
        out
    }

    /// What `build` added to `published.txt`; nothing where it added
    /// nothing.
    pub(crate) fn kept(&self, kept: crate::published::Kept) -> String {
        match kept.added > 0 || kept.started {
            true => format!(
                "{}\n",
                shown(&wording::kept_published(
                    kept.added,
                    kept.started,
                    kept.found
                ))
            ),
            false => String::new(),
        }
    }

    /// What `build` did with the sharing cards: those it drew, and those
    /// it removed; nothing where it did neither.
    pub(crate) fn cards(&self, cards: Cards) -> String {
        let mut out = String::new();
        if cards.drawn > 0 {
            out.push_str(&wording::drew_cards(cards.drawn));
            out.push('\n');
        }
        if cards.removed > 0 {
            out.push_str(&wording::removed_cards(cards.removed));
            out.push('\n');
        }
        out
    }

    /// What became of the site's own commands in a build: each it ran,
    /// and a `before_build` it didn't; nothing where the site sets none.
    pub(crate) fn hooks(&self, ran: Ran) -> String {
        let mut out = String::new();
        match ran.before {
            Before::Unset => {}
            Before::Ran => {
                out.push_str(&wording::ran_hook(crate::config::hooks::BEFORE));
                out.push('\n');
            }
            Before::PassedOver => {
                out.push_str(wording::HOOK_PASSED_OVER);
                out.push('\n');
            }
        }
        if let Some(changed) = ran.after {
            out.push_str(&wording::ran_after(
                changed.added,
                changed.changed,
                changed.removed,
            ));
            out.push('\n');
        }
        out
    }

    /// How old what the site fetched is; nothing where it holds nothing
    /// fetched.
    pub(crate) fn fetched(&self, ages: Ages) -> String {
        let mut parts = Vec::new();
        if ages.stills > 0 {
            parts.push(wording::stills_age(ages.stills, ages.oldest, ages.undated));
        }
        if ages.mentions > 0 {
            parts.push(wording::mentions_age(ages.mentions, ages.newest));
        }
        match parts.is_empty() {
            true => String::new(),
            false => format!("{}\n", wording::fetched_ages(&parts)),
        }
    }

    /// What `build` says of a build into `out` that took `took`: its errors
    /// in full, the rest counted, and what it wrote.
    pub(crate) fn of_build(&self, outcome: &Outcome, out: &Path, took: Duration) -> String {
        let problems = &outcome.problems;
        let mut said = self.build(problems, &outcome.quiet);
        let wrote = self.wrote(problems, outcome.published, out, took);
        let part = format!(
            "{}{}{}{}{}{wrote}",
            self.copies(outcome.copies),
            self.cards(outcome.cards),
            self.hooks(outcome.hooks),
            self.fetched(outcome.fetched),
            self.kept(outcome.kept)
        );
        if !said.is_empty() && !part.is_empty() {
            said.push('\n');
        }
        said.push_str(&part);
        said
    }

    /// `build`'s last line: what it wrote and where, and how long it took,
    /// or that errors stopped it.
    pub(crate) fn wrote(
        &self,
        problems: &[Problem],
        published: Option<Published>,
        out: &Path,
        took: Duration,
    ) -> String {
        let (style, line) = match published {
            Some(published) => (
                GREEN,
                wording::wrote(&published, &out.display().to_string(), took.as_secs_f64()),
            ),
            None => (
                RED,
                wording::wrote_nothing(
                    problems
                        .iter()
                        .filter(|p| p.kind.severity() == Severity::Error)
                        .count(),
                ),
            ),
        };
        format!("{}\n", self.paint.paint(style, &shown(&line)))
    }

    /// What became of the stamps in a site's own files: a line for each
    /// date written, and that none was while the settings have an error.
    /// The names of files and fields are the site's, so they are shown
    /// as any text Statac didn't write is.
    pub(crate) fn stamps(&self, pass: &Pass) -> String {
        let mut out = String::new();
        for stamped in &pass.stamped {
            let line = wording::stamped(&stamped.file, &stamped.field, &stamped.written);
            out.push_str(&format!("{}\n", shown(&line)));
        }
        for file in &pass.ids {
            out.push_str(&format!("{}\n", shown(&wording::wrote_id(file))));
        }
        if pass.unsettled {
            out.push_str(&format!("{}\n", wording::NOT_STAMPED));
        }
        out
    }

    /// What `check` says: every problem not quietened in full, then the count.
    pub fn check(&self, problems: &[Problem], quiet: &Quiet) -> String {
        self.check_with(problems, quiet, &[])
    }

    /// The same, with the JavaScript the site carries between the problems
    /// and the count.
    pub(crate) fn check_with(
        &self,
        problems: &[Problem],
        quiet: &Quiet,
        javascript: &[Javascript],
    ) -> String {
        let shown = problems.iter().filter(|p| !quiet.quietens(&p.kind));
        let mut out = self.drawn(shown, true);
        let carried = self.javascript(javascript);
        if !out.is_empty() && !carried.is_empty() {
            out.push('\n');
        }
        out.push_str(&carried);
        let counts = counts(
            &problems.iter().collect::<Vec<_>>(),
            quiet,
            &[Severity::Error, Severity::Warning, Severity::Note],
        );
        if counts.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&self.paint.paint(GREEN, wording::NO_PROBLEMS));
            out.push('\n');
            return out;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        for (line, _) in counts {
            out.push_str(&self.paint.counted(&line));
            out.push_str(".\n");
        }
        out
    }

    /// Problems in full, a blank line apart, in order of file and position;
    /// problems in no file first.
    pub fn problems<'p>(&self, problems: impl IntoIterator<Item = &'p Problem>) -> String {
        self.drawn(problems, false)
    }

    /// The same, naming every page whose build met a problem where
    /// `every` is set, and the first few where it isn't.
    fn drawn<'p>(&self, problems: impl IntoIterator<Item = &'p Problem>, every: bool) -> String {
        let drawn = self
            .ordered(problems)
            .into_iter()
            .map(|p| self.one(p, every));
        drawn.collect::<Vec<_>>().join("\n")
    }

    /// Problems in the order they are drawn in.
    pub(crate) fn ordered<'p>(
        &self,
        problems: impl IntoIterator<Item = &'p Problem>,
    ) -> Vec<&'p Problem> {
        let mut keyed: Vec<_> = problems
            .into_iter()
            .map(|p| {
                let key = p
                    .place
                    .map(|place| (self.path(place.source), place.span.start, place.span.end));
                (key, p)
            })
            .collect();
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        keyed.into_iter().map(|(_, p)| p).collect()
    }

    /// A file of the site, as a problem names it.
    pub(crate) fn file(&self, source: SourceId) -> String {
        self.path(source)
    }

    /// One problem in full.
    pub fn problem(&self, problem: &Problem) -> String {
        self.one(problem, false)
    }

    fn one(&self, problem: &Problem, every: bool) -> String {
        let severity = problem.kind.severity();
        let style = match severity {
            Severity::Error => RED,
            Severity::Warning => YELLOW,
            Severity::Note => CYAN,
        };
        let mut out = format!(
            "{}  {}\n",
            self.paint.chip(severity, wording::severity(severity)),
            self.paint.inline(BOLD, &shown(&problem.headline())),
        );
        let related = problem.related();
        let built = problem.built().next().and_then(|(_, place)| place);
        let width = problem
            .place
            .iter()
            .chain(related.iter().filter_map(|r| r.place.as_ref()))
            .chain(built.iter())
            .map(|place| self.first_line(*place).to_string().len())
            .max()
            .unwrap_or(0)
            .max(2);
        match problem.place {
            Some(place) if problem.kind.of_a_whole_file() => {
                out.push_str(&format!("  {}\n", self.path(place.source)));
            }
            Some(place) => self.place(&mut out, None, place, width, style),
            None => {}
        }
        for related in &related {
            match related.place {
                Some(place) => self.place(&mut out, Some(&related.label), place, width, BLUE),
                None => out.push_str(&format!("  {}\n", shown(&related.label))),
            }
        }
        self.built(&mut out, problem, every, width);
        if let Some((heading, lines)) = wording::printed(&problem.kind) {
            out.push_str(&format!("  {heading}\n"));
            for line in lines {
                out.push_str(&format!("    {}\n", shown(&line)));
            }
        }
        if let Some(suggestion) = problem.suggestion() {
            let suggestion = self.paint.inline("", &shown(&suggestion));
            out.push_str(&format!("  {suggestion}\n"));
        }
        let more = wording::more(&problem.explanation_link());
        out.push_str(&format!("  {}\n", self.paint.paint(DIM, &more)));
        out
    }

    /// The pages whose builds met the problem: the first as any other
    /// place that bears on it, the others listed beneath, every one or the
    /// first few. The whole first line of a file says no more than the
    /// file's name, so a page placed there is named by its file alone.
    fn built(&self, out: &mut String, problem: &Problem, every: bool, width: usize) {
        let whole = |place: Place| self.whole(place);
        let mut pages = problem.built();
        let Some((named, place)) = pages.next() else {
            return;
        };
        let label = wording::while_building(named);
        match place {
            Some(place) if whole(place) => {
                out.push_str(&format!(
                    "  {}: {}\n",
                    shown(&label),
                    self.path(place.source)
                ));
            }
            Some(place) => self.place(out, Some(&label), place, width, BLUE),
            None => out.push_str(&format!("  {}\n", shown(&label))),
        }
        let others: Vec<_> = pages.collect();
        if others.is_empty() {
            return;
        }
        out.push_str(&format!(
            "  {}\n",
            wording::while_building_more(others.len())
        ));
        let named = match every {
            true => others.len(),
            false => NAMED.min(others.len()),
        };
        for &(page, place) in &others[..named] {
            let at = place.map(|place| match whole(place) {
                true => self.path(place.source),
                false => wording::location(&self.path(place.source), self.first_line(place)),
            });
            let line = match (page.map(wording::named), at) {
                (Some(page), Some(at)) => format!("{}: {at}", shown(&page)),
                (Some(page), None) => shown(&page),
                (None, at) => at.unwrap_or_default(),
            };
            out.push_str(&format!("    {line}\n"));
        }
        if named < others.len() {
            out.push_str(&format!(
                "    {}\n",
                wording::and_more(others.len() - named)
            ));
        }
    }

    /// Whether `place` is the whole first line of its file, which says no
    /// more than the file's name.
    pub(crate) fn whole(&self, place: Place) -> bool {
        place == crate::config::words::head(self.sources, place.source)
    }

    fn path(&self, id: SourceId) -> String {
        let path = self.sources.path(id);
        shown(
            &path
                .strip_prefix(self.root)
                .unwrap_or(path)
                .display()
                .to_string(),
        )
    }

    /// The line a place starts on.
    pub(crate) fn first_line(&self, place: Place) -> usize {
        let text = self.sources.text(place.source);
        let start = text.floor_char_boundary(place.span.start as usize);
        self.sources.line_number(place.source, start)
    }

    /// The location, the line with its number, and carets under the place.
    fn place(
        &self,
        out: &mut String,
        label: Option<&str>,
        place: Place,
        width: usize,
        style: &str,
    ) {
        let (first, last, drawn) = self.marked(place);
        let location = wording::location(&self.path(place.source), first);
        match label {
            Some(label) => out.push_str(&format!("  {}: {location}\n", shown(label))),
            None => out.push_str(&format!("  {location}\n")),
        }

        let gutter = |number: &str| {
            let bar = format!("{number:>width$} \u{2502}");
            self.paint.paint(DIM, &bar)
        };
        let number = gutter(&first.to_string());
        if drawn.text.is_empty() {
            out.push_str(&format!("  {number}\n"));
        } else {
            out.push_str(&format!("  {number} {}\n", drawn.text));
        }
        let carets = "^".repeat(drawn.carets);
        let mut under = format!(
            "  {} {}{}",
            gutter(""),
            " ".repeat(drawn.column),
            self.paint.paint(style, &carets)
        );
        if last > first {
            under.push(' ');
            under.push_str(&wording::runs_to(last));
        }
        out.push_str(&under);
        out.push('\n');
    }

    /// The line a place starts on as it is drawn, with the numbers of the
    /// lines it starts and ends on.
    fn marked(&self, place: Place) -> (usize, usize, Drawn) {
        let id = place.source;
        let text = self.sources.text(id);
        let start = text.floor_char_boundary(place.span.start as usize);
        let end = text.ceil_char_boundary((place.span.end as usize).max(start));
        let first = self.sources.line_number(id, start);
        // A place that ends with a line ending ends on that line, not the next.
        let last = if end > start {
            self.sources.line_number(id, end - 1)
        } else {
            first
        };
        let (line, line_start) = self.line(id, first);
        let from = start.saturating_sub(line_start).min(line.len());
        let to = if last == first {
            end.saturating_sub(line_start).min(line.len())
        } else {
            line.len()
        };
        let hidden = self.hidden(id, line, line_start);
        (
            first,
            last,
            Drawn::masked(line, from, to.max(from), &hidden),
        )
    }

    /// Line `number` of a source, without a byte-order mark it opens with,
    /// and where what is left of it starts.
    fn line(&self, id: SourceId, number: usize) -> (&str, usize) {
        let start = self.sources.line_start(id, number);
        let line = self.sources.line(id, number);
        match line.strip_prefix('\u{feff}') {
            Some(rest) if number == 1 => (rest, start + '\u{feff}'.len_utf8()),
            _ => (line, start),
        }
    }

    /// The secrets on `line`, which starts at `start`, as byte ranges
    /// within it.
    fn hidden(&self, id: SourceId, line: &str, start: usize) -> Vec<std::ops::Range<usize>> {
        let end = start + line.len();
        (self.sources.hidden(id).iter())
            .filter(|span| (span.start as usize) < end && (span.end as usize) > start)
            .map(|span| {
                (span.start as usize).max(start) - start..(span.end as usize).min(end) - start
            })
            .collect()
    }

    /// The line a place starts on as a terminal is shown it, in three
    /// parts: before the place, the place, and after it. Where `beside` is
    /// set, the lines either side come with it, cut to the same columns.
    pub(crate) fn excerpt(&self, place: Place, beside: bool) -> Excerpt {
        let id = place.source;
        let (first, last, drawn) = self.marked(place);
        let text = self.sources.text(id);
        let other = |number: usize| {
            let (line, start) = self.line(id, number);
            let hidden = self.hidden(id, line, start);
            Shown {
                number,
                before: Drawn::beside(line, &hidden, drawn.window),
                marked: None,
                after: String::new(),
            }
        };
        let mut lines = Vec::new();
        if beside && first > 1 {
            lines.push(other(first - 1));
        }
        let Drawn {
            text: line, marked, ..
        } = &drawn;
        lines.push(Shown {
            number: first,
            before: line[..marked.start].to_owned(),
            marked: Some(line[marked.clone()].to_owned()),
            after: line[marked.end..].to_owned(),
        });
        // A file's last line ending opens no line of its own.
        let after = (first < self.sources.line_number(id, text.len()))
            .then(|| self.sources.line_start(id, first + 1))
            .is_some_and(|start| start < text.len());
        if beside && after {
            lines.push(other(first + 1));
        }
        let start = text.floor_char_boundary(place.span.start as usize);
        Excerpt {
            first,
            last,
            column: self.sources.line_column(id, start).column,
            lines,
        }
    }
}

/// Where a place is, and its line, as [`Report::excerpt`] gives them.
pub(crate) struct Excerpt {
    /// The lines the place starts and ends on.
    pub(crate) first: usize,
    pub(crate) last: usize,
    /// Where on its first line the place starts, in characters from 1.
    pub(crate) column: usize,
    pub(crate) lines: Vec<Shown>,
}

/// One line of an excerpt. A line beside the place's own is all `before`.
pub(crate) struct Shown {
    pub(crate) number: usize,
    pub(crate) before: String,
    pub(crate) marked: Option<String>,
    pub(crate) after: String,
}

/// Problems counted by kind, a line for each of `severities` that has any,
/// with how many of each line's problems are shown rather than quietened.
fn counts(problems: &[&Problem], quiet: &Quiet, severities: &[Severity]) -> Vec<(String, usize)> {
    let mut lines = Vec::new();
    for &severity in severities {
        let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
        let mut quietened = 0;
        for problem in problems.iter().filter(|p| p.kind.severity() == severity) {
            if quiet.quietens(&problem.kind) {
                quietened += 1;
            } else {
                *kinds.entry(problem.kind.name()).or_default() += 1;
            }
        }
        let mut kinds: Vec<_> = kinds.into_iter().collect();
        kinds.sort_by_key(|&(_, n)| std::cmp::Reverse(n));
        let shown: usize = kinds.iter().map(|(_, n)| n).sum();
        if shown + quietened > 0 {
            lines.push((wording::count(severity, &kinds, quietened), shown));
        }
    }
    lines
}

/// What a secret is drawn as, the same whatever its length.
const MASK: &str = "\u{2022}\u{2022}\u{2022}";

/// One source line as shown, and where the carets go under it.
#[derive(Debug, PartialEq)]
struct Drawn {
    text: String,
    column: usize,
    carets: usize,
    /// What of `text` the carets are under, in bytes.
    marked: std::ops::Range<usize>,
    /// The columns of the line kept, where it was cut to a window.
    window: Option<(usize, usize)>,
}

impl Drawn {
    /// `from..to` are byte offsets within `line`.
    #[cfg(test)]
    fn new(line: &str, from: usize, to: usize) -> Drawn {
        Drawn::masked(line, from, to, &[])
    }

    /// The same, with each of `hidden` shown as a mask, whatever it holds.
    fn masked(line: &str, from: usize, to: usize, hidden: &[std::ops::Range<usize>]) -> Drawn {
        let (cells, total) = cells(line, hidden);
        let column_of = |offset: usize| {
            cells
                .iter()
                .find(|cell| cell.byte >= offset)
                .map_or(total, |cell| cell.column)
        };
        let (start, end) = (column_of(from), column_of(to));
        let window = (total > LONGEST).then(|| {
            let low = start.saturating_sub(LEAD);
            match low + WINDOW > total {
                true => (total - WINDOW, total),
                false => (low, low + WINDOW),
            }
        });
        let (low, high) = window.unwrap_or((0, total));
        let cut = Cut::of(&cells, total, low, high, from..to);
        let carets = match end.min(cut.kept_to).checked_sub(start) {
            Some(n) if n > 0 => n,
            _ => 1,
        };
        Drawn {
            text: cut.text,
            column: cut.lead + start.saturating_sub(cut.kept_from),
            carets,
            marked: cut.marked,
            window,
        }
    }

    /// A line beside one drawn, unmarked: cut to the same columns where
    /// that one was cut to a window, and drawn as any line is otherwise.
    fn beside(
        line: &str,
        hidden: &[std::ops::Range<usize>],
        window: Option<(usize, usize)>,
    ) -> String {
        let Some((low, high)) = window else {
            return Drawn::masked(line, 0, 0, hidden).text;
        };
        let (cells, total) = cells(line, hidden);
        Cut::of(&cells, total, low, high, 0..0).text
    }
}

/// Each character of `line` as shown, with each of `hidden` shown as a
/// mask, and the columns the whole takes.
fn cells(line: &str, hidden: &[std::ops::Range<usize>]) -> (Vec<Cell>, usize) {
    let mut cells = Vec::new();
    let mut column = 0;
    for (byte, c) in line.char_indices() {
        let secret = hidden.iter().find(|range| range.contains(&byte));
        let (shown, width) = match c {
            _ if secret.is_some_and(|range| range.start == byte) => (MASK.to_owned(), 3),
            _ if secret.is_some() => (String::new(), 0),
            '\t' => {
                let width = TAB - column % TAB;
                (" ".repeat(width), width)
            }
            c if picture(c).is_some() => (picture(c).unwrap_or(c).to_string(), 1),
            c if c.is_control() || invisible(c) => {
                let named = named(c);
                let width = named.chars().count();
                (named, width)
            }
            c => (c.to_string(), self::width(c)),
        };
        cells.push(Cell {
            byte,
            column,
            shown,
        });
        column += width;
    }
    (cells, column)
}

struct Cell {
    byte: usize,
    column: usize,
    shown: String,
}

/// The cells of a line within columns `low..high`, an ellipsis standing
/// for any cut off either side.
struct Cut {
    text: String,
    /// The columns kept.
    kept_from: usize,
    kept_to: usize,
    /// The columns an ellipsis opening `text` takes.
    lead: usize,
    /// What of `text` the bytes `place` of the line are shown as.
    marked: std::ops::Range<usize>,
}

impl Cut {
    fn of(
        cells: &[Cell],
        total: usize,
        low: usize,
        high: usize,
        place: std::ops::Range<usize>,
    ) -> Cut {
        let mut text = String::new();
        let mut kept: Option<(usize, usize)> = None;
        let mut cut_after = false;
        let (mut start, mut end) = (None, None);
        for (i, cell) in cells.iter().enumerate() {
            let next = cells.get(i + 1).map_or(total, |next| next.column);
            if cell.column < low {
                continue;
            }
            if next > high {
                cut_after = true;
                break;
            }
            kept = Some((kept.map_or(cell.column, |(from, _)| from), next));
            if cell.byte >= place.start {
                start.get_or_insert(text.len());
            }
            if cell.byte >= place.end {
                end.get_or_insert(text.len());
            }
            text.push_str(&cell.shown);
        }
        let (kept_from, kept_to) = kept.unwrap_or((low, low));
        let lead = usize::from(kept_from > 0);
        if lead > 0 {
            text.insert(0, ELLIPSIS);
        }
        let opened = lead * ELLIPSIS.len_utf8();
        let start = start.map_or(text.len(), |start| start + opened);
        let end = end.map_or(text.len(), |end| end + opened);
        if cut_after {
            text.push(ELLIPSIS);
        }
        Cut {
            text,
            kept_from,
            kept_to,
            lead,
            marked: start..end,
        }
    }
}

/// A piece of a sentence of Statac's own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Piece<'a> {
    /// A name, written between backticks.
    Name(&'a str),
    Words(&'a str),
}

/// The sentence's names and the words between them, in order. A backtick
/// with none to close it is a character like any other.
pub(crate) fn pieces(sentence: &str) -> Vec<Piece<'_>> {
    let mut pieces = Vec::new();
    let mut rest = sentence;
    while let Some(open) = rest.find('`') {
        let Some(close) = rest[open + 1..].find('`') else {
            break;
        };
        if open > 0 {
            pieces.push(Piece::Words(&rest[..open]));
        }
        pieces.push(Piece::Name(&rest[open + 1..open + 1 + close]));
        rest = &rest[open + 1 + close + 1..];
    }
    if !rest.is_empty() {
        pieces.push(Piece::Words(rest));
    }
    pieces
}

/// Text Statac didn't write, made safe to put on a terminal: a control
/// character is shown as its picture rather than acted on, and an invisible
/// one is named.
pub(crate) fn shown(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if let Some(picture) = picture(c) {
            out.push(picture);
        } else if c.is_control() || invisible(c) {
            out.push_str(&named(c));
        } else {
            out.push(c);
        }
    }
    out
}

/// Characters a terminal draws as nothing but which still act or hide
/// text: Unicode's format characters and other default-ignorable ones
/// (direction overrides and isolates, zero-width spaces, tags, fillers,
/// line separators). The zero-width non-joiner and joiner are left alone
/// because Persian, Indic scripts and emoji spell with them, and so are
/// variation selectors, which only choose a glyph.
fn invisible(c: char) -> bool {
    matches!(
        c as u32,
        0x00AD
            | 0x034F
            | 0x0600..=0x0605
            | 0x061C
            | 0x06DD
            | 0x070F
            | 0x0890..=0x0891
            | 0x08E2
            | 0x115F..=0x1160
            | 0x17B4..=0x17B5
            | 0x180B..=0x180F
            | 0x200B
            | 0x200E..=0x200F
            | 0x2028..=0x202E
            | 0x2060..=0x206F
            | 0x3164
            | 0xFEFF
            | 0xFFA0
            | 0xFFF9..=0xFFFB
            | 0x110BD
            | 0x110CD
            | 0x13430..=0x1343F
            | 0x1BCA0..=0x1BCA3
            | 0x1D173..=0x1D17A
            | 0xE0000..=0xE0FFF
    ) && !matches!(c as u32, 0xE0100..=0xE01EF)
}

fn named(c: char) -> String {
    format!("<U+{:04X}>", c as u32)
}

/// The picture Unicode gives a C0 control character or DEL. C1 has none,
/// so those are named.
fn picture(c: char) -> Option<char> {
    match c {
        '\0'..='\x1f' => char::from_u32(0x2400 + c as u32),
        '\x7f' => Some('\u{2421}'),
        _ => None,
    }
}

/// Columns a character takes on a terminal: two for East Asian wide and
/// fullwidth characters and most emoji, none for marks that combine with
/// the character before, one for the rest.
fn width(c: char) -> usize {
    let c = c as u32;
    let within = |ranges: &[(u32, u32)]| {
        ranges
            .binary_search_by(|&(low, high)| {
                if high < c {
                    std::cmp::Ordering::Less
                } else if low > c {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok()
    };
    if within(ZERO) {
        0
    } else if within(WIDE) {
        2
    } else {
        1
    }
}

const ZERO: &[(u32, u32)] = &[
    (0x0300, 0x036F),
    (0x0483, 0x0489),
    (0x0591, 0x05BD),
    (0x0610, 0x061A),
    (0x064B, 0x065F),
    (0x0E31, 0x0E31),
    (0x0E34, 0x0E3A),
    (0x0E47, 0x0E4E),
    (0x1AB0, 0x1AFF),
    (0x1DC0, 0x1DFF),
    (0x200B, 0x200F),
    (0x2060, 0x2064),
    (0x20D0, 0x20FF),
    (0xFE00, 0xFE0F),
    (0xFE20, 0xFE2F),
    (0xFEFF, 0xFEFF),
    (0xE0100, 0xE01EF),
];

const WIDE: &[(u32, u32)] = &[
    (0x1100, 0x115F),
    (0x231A, 0x231B),
    (0x2329, 0x232A),
    (0x23E9, 0x23EC),
    (0x23F0, 0x23F0),
    (0x23F3, 0x23F3),
    (0x25FD, 0x25FE),
    (0x2614, 0x2615),
    (0x2648, 0x2653),
    (0x267F, 0x267F),
    (0x2693, 0x2693),
    (0x26A1, 0x26A1),
    (0x26AA, 0x26AB),
    (0x26BD, 0x26BE),
    (0x26C4, 0x26C5),
    (0x26CE, 0x26CE),
    (0x26D4, 0x26D4),
    (0x26EA, 0x26EA),
    (0x26F2, 0x26F3),
    (0x26F5, 0x26F5),
    (0x26FA, 0x26FA),
    (0x26FD, 0x26FD),
    (0x2705, 0x2705),
    (0x270A, 0x270B),
    (0x2728, 0x2728),
    (0x274C, 0x274C),
    (0x274E, 0x274E),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2795, 0x2797),
    (0x27B0, 0x27B0),
    (0x27BF, 0x27BF),
    (0x2B1B, 0x2B1C),
    (0x2B50, 0x2B50),
    (0x2B55, 0x2B55),
    (0x2E80, 0x303E),
    (0x3041, 0x33FF),
    (0x3400, 0x4DBF),
    (0x4E00, 0x9FFF),
    (0xA000, 0xA4CF),
    (0xA960, 0xA97F),
    (0xAC00, 0xD7A3),
    (0xF900, 0xFAFF),
    (0xFE10, 0xFE19),
    (0xFE30, 0xFE6F),
    (0xFF00, 0xFF60),
    (0xFFE0, 0xFFE6),
    (0x16FE0, 0x16FE4),
    (0x17000, 0x18CFF),
    (0x1B000, 0x1B2FF),
    (0x1F004, 0x1F004),
    (0x1F0CF, 0x1F0CF),
    (0x1F18E, 0x1F18E),
    (0x1F191, 0x1F19A),
    (0x1F200, 0x1F251),
    (0x1F300, 0x1F64F),
    (0x1F680, 0x1F6FF),
    (0x1F7E0, 0x1F7EB),
    (0x1F90C, 0x1F9FF),
    (0x1FA70, 0x1FAFF),
    (0x20000, 0x2FFFD),
    (0x30000, 0x3FFFD),
];

const BOLD: &str = "1";
const DIM: &str = "2";
const RED: &str = "1;31";
const YELLOW: &str = "1;33";
const CYAN: &str = "1;36";
const GREEN: &str = "32";
const BLUE: &str = "34";

/// Statac's own orange, and the deeper one names in backticks take, which
/// reads on a light ground and a dark one alike: as they are where the
/// terminal says it shows every colour, and by their nearest of the 256
/// where it doesn't.
const BRAND: (&str, &str) = ("2;247;108;20", "5;208");
const IN_BACKTICKS: (&str, &str) = ("2;215;95;0", "5;166");

/// Colour is decoration: with it off, the text alone is written.
#[derive(Copy, Clone)]
struct Paint {
    on: bool,
    /// Whether the terminal says it shows every colour.
    every: bool,
}

impl Paint {
    fn new(on: bool) -> Paint {
        let said = std::env::var_os("COLORTERM");
        Paint {
            on,
            every: said.is_some_and(|said| said == "truecolor" || said == "24bit"),
        }
    }

    fn paint(self, style: &str, text: &str) -> String {
        if self.on && !text.is_empty() && !style.is_empty() {
            format!("\x1b[{style}m{text}\x1b[0m")
        } else {
            text.to_owned()
        }
    }

    fn pick(self, colour: (&'static str, &'static str)) -> &'static str {
        match self.every {
            true => colour.0,
            false => colour.1,
        }
    }

    /// Black on Statac's orange, for its name.
    fn brand_chip(self) -> String {
        format!("1;30;48;{}", self.pick(BRAND))
    }

    /// Statac's orange as a foreground colour.
    fn brand_ink(self) -> String {
        format!("38;{}", self.pick(BRAND))
    }

    /// A problem's severity as a badge, the word on its colour with a
    /// space either side; the word alone without colour.
    fn chip(self, severity: Severity, word: &str) -> String {
        if !self.on {
            return word.to_owned();
        }
        let style = match severity {
            Severity::Error => "1;97;41",
            Severity::Warning => "1;30;43",
            Severity::Note => "1;30;46",
        };
        self.paint(style, &format!(" {word} "))
    }

    /// A sentence in `style`, each name in backticks picked out in the
    /// paler orange. A backtick with none to close it is a character like
    /// any other.
    fn inline(self, style: &str, sentence: &str) -> String {
        if !self.on {
            return sentence.to_owned();
        }
        let named = format!("38;{}", self.pick(IN_BACKTICKS));
        let mut out = String::new();
        for piece in pieces(sentence) {
            match piece {
                Piece::Name(name) => out.push_str(&self.paint(&named, &format!("`{name}`"))),
                Piece::Words(words) => out.push_str(&self.paint(style, words)),
            }
        }
        out
    }

    /// A line counting problems of one severity, its count in the
    /// severity's colour: `1 warning` of `1 warning: 1 broken-link`.
    fn counted(self, line: &str) -> String {
        let Some((count, kinds)) = line.split_once(':') else {
            return line.to_owned();
        };
        let style = match count {
            _ if count.contains("error") => RED,
            _ if count.contains("warning") => YELLOW,
            _ => CYAN,
        };
        format!("{}:{kinds}", self.paint(style, count))
    }
}

mod json;
mod timings;

#[cfg(test)]
mod tests;
