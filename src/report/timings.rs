//! `statac build --timings`: where a build spent its time, by stage, then
//! the work of building the pages, then the slowest of each kind named at
//! its file.

use std::time::Duration;

use super::{Report, shown};
use crate::build::{Stage, Timings};
use crate::wording;

/// How many of each kind are named.
const SLOWEST: usize = 5;

/// A section's times are in seconds where its longest is at least this.
const IN_SECONDS: Duration = Duration::from_millis(100);

fn stage_name(stage: Stage) -> &'static str {
    match stage {
        Stage::Before => wording::STAGE_BEFORE,
        Stage::Reading => wording::STAGE_READING,
        Stage::Building => wording::STAGE_BUILDING,
        Stage::Concluding => wording::STAGE_CONCLUDING,
        Stage::Copies => wording::STAGE_COPIES,
        Stage::Cards => wording::STAGE_CARDS,
        Stage::After => wording::STAGE_AFTER,
        Stage::Writing => wording::STAGE_WRITING,
    }
}

#[derive(Copy, Clone)]
enum Align {
    Left,
    Right,
}

impl Report<'_> {
    /// What `build --timings` adds after what the build said; nothing
    /// where nothing was timed.
    pub fn timings(&self, timings: &Timings) -> String {
        let mut sections = Vec::new();
        let whole: Duration = timings.stages.iter().map(|(_, took)| *took).sum();
        let seconds = in_seconds(timings.stages.iter().map(|(_, took)| *took));
        let stages = timings.stages.iter().map(|&(stage, took)| {
            let name = stage_name(stage).to_owned();
            vec![name, time(took, seconds), share(took, whole)]
        });
        sections.push(table(wording::TIMINGS_STAGES, stages.collect(), &NAMED));

        let work = &timings.work;
        if !work.whole.is_zero() {
            let parts = [
                (wording::WORK_SAVING, work.saving),
                (wording::WORK_WRITING, work.writing),
                (wording::WORK_TEMPLATES, work.templates),
                (wording::WORK_LINKS, work.links),
            ];
            let parts = parts.into_iter().filter(|(_, took)| !took.is_zero());
            let parts = parts.chain([(wording::WORK_REST, work.rest())]);
            let seconds = in_seconds([work.whole]);
            let rows = parts.map(|(name, took)| {
                vec![
                    name.to_owned(),
                    time(took, seconds),
                    share(took, work.whole),
                ]
            });
            let heading = wording::timings_work(&time(work.whole, seconds), timings.threads);
            sections.push(table(&heading, rows.collect(), &NAMED));
        }

        let slowest = &timings.templates[..timings.templates.len().min(SLOWEST)];
        let seconds = in_seconds(slowest.iter().map(|t| t.1));
        let rows = slowest.iter().map(|(path, took, runs)| {
            let runs = wording::runs(*runs as usize);
            vec![shown(path), time(*took, seconds), runs]
        });
        sections.push(table(wording::SLOWEST_TEMPLATES, rows.collect(), &NAMED));

        let slowest = &timings.writing[..timings.writing.len().min(SLOWEST)];
        let seconds = in_seconds(slowest.iter().map(|w| w.1));
        let rows = slowest
            .iter()
            .map(|(path, took)| vec![shown(path), time(*took, seconds)]);
        sections.push(table(wording::SLOWEST_WRITING, rows.collect(), &NAMED));

        let pages = &timings.pages;
        let slowest = &pages[..pages.len().min(SLOWEST)];
        let seconds = in_seconds(slowest.iter().map(|p| p.1));
        let rows = slowest.iter().map(|(address, took, layout)| {
            let layout = layout.as_deref().map(shown).unwrap_or_default();
            vec![shown(address), time(*took, seconds), layout]
        });
        if let (Some(median), Some(high)) = (percentile(pages, 50), percentile(pages, 95)) {
            let heading = wording::slowest_pages(&time(median, seconds), &time(high, seconds));
            let rows = rows.collect();
            sections.push(table(
                &heading,
                rows,
                &[Align::Left, Align::Right, Align::Left],
            ));
        }

        let photographs = &timings.photographs;
        let slowest = &photographs[..photographs.len().min(SLOWEST)];
        let seconds = in_seconds(slowest.iter().map(|p| p.1));
        let rows = slowest.iter().map(|(path, took, copies)| {
            vec![
                shown(path),
                time(*took, seconds),
                wording::copies_made(*copies),
            ]
        });
        sections.push(table(wording::SLOWEST_PHOTOGRAPHS, rows.collect(), &NAMED));

        sections.retain(|section| !section.is_empty());
        sections.join("\n")
    }
}

/// A name, then its time, then its share of the whole or how many there
/// were.
const NAMED: [Align; 3] = [Align::Left, Align::Right, Align::Right];

/// `rows` under `heading`, each column as wide as its widest cell, two
/// spaces apart; nothing where there are no rows.
fn table(heading: &str, rows: Vec<Vec<String>>, align: &[Align]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let columns = rows.iter().map(Vec::len).max().unwrap_or_default();
    let widths: Vec<usize> = (0..columns)
        .map(|at| {
            (rows.iter().filter_map(|row| row.get(at)))
                .map(|cell| cell.chars().count())
                .max()
                .unwrap_or_default()
        })
        .collect();
    let mut out = format!("{heading}\n");
    for row in rows {
        let mut line = String::new();
        for (at, cell) in row.iter().enumerate() {
            let pad = widths[at] - cell.chars().count();
            line.push_str("  ");
            match align.get(at).copied().unwrap_or(Align::Left) {
                Align::Left => {
                    line.push_str(cell);
                    line.push_str(&" ".repeat(pad));
                }
                Align::Right => {
                    line.push_str(&" ".repeat(pad));
                    line.push_str(cell);
                }
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

/// Whether times as long as the longest of `times` are shown in seconds.
fn in_seconds(times: impl IntoIterator<Item = Duration>) -> bool {
    times
        .into_iter()
        .max()
        .is_some_and(|longest| longest >= IN_SECONDS)
}

/// `took` in seconds to two places, or milliseconds to one.
fn time(took: Duration, seconds: bool) -> String {
    match seconds {
        true => format!("{:.2} s", took.as_secs_f64()),
        false => format!("{:.1} ms", took.as_secs_f64() * 1000.0),
    }
}

/// `part` as a share of `whole`, in whole percent; a share too small to
/// round up to one is shown as under one.
fn share(part: Duration, whole: Duration) -> String {
    if whole.is_zero() {
        return String::new();
    }
    let percent = part.as_secs_f64() / whole.as_secs_f64() * 100.0;
    match percent {
        _ if part.is_zero() => "0%".into(),
        p if p < 0.5 => "<1%".into(),
        p => format!("{}%", p.round()),
    }
}

/// The time `percent` in 100 of `timed`, the slowest first, took no more
/// than; none where there are none.
fn percentile<T>(timed: &[(String, Duration, T)], percent: usize) -> Option<Duration> {
    let rank = (timed.len() * percent).div_ceil(100).max(1);
    let from_slowest = timed.len().checked_sub(rank)?;
    timed.get(from_slowest).map(|(_, took, _)| *took)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn a_section_keeps_one_unit() {
        assert!(in_seconds([ms(20), ms(100)]));
        assert!(!in_seconds([ms(20), ms(99)]));
        assert_eq!(time(ms(1234), true), "1.23 s");
        assert_eq!(time(ms(30), true), "0.03 s");
        assert_eq!(time(Duration::from_micros(5940), false), "5.9 ms");
    }

    #[test]
    fn a_share_too_small_to_show_is_under_one() {
        assert_eq!(share(ms(1), ms(1000)), "<1%");
        assert_eq!(share(ms(0), ms(1000)), "0%");
        assert_eq!(share(ms(5), ms(1000)), "1%");
        assert_eq!(share(ms(170), ms(1000)), "17%");
    }

    #[test]
    fn percentiles_are_taken_by_rank() {
        let timed: Vec<(String, Duration, ())> =
            (1..=20).rev().map(|n| (n.to_string(), ms(n), ())).collect();
        assert_eq!(percentile(&timed, 50), Some(ms(10)));
        assert_eq!(percentile(&timed, 95), Some(ms(19)));
        assert_eq!(percentile(&timed[..1], 95), Some(ms(20)));
        assert_eq!(percentile::<()>(&[], 50), None);
    }

    #[test]
    fn columns_line_up() {
        let rows = vec![
            vec!["a".into(), "1.0 ms".into(), "3 runs".into()],
            vec!["longer".into(), "10.0 ms".into(), "12 runs".into()],
        ];
        assert_eq!(
            table("Heading:", rows, &NAMED),
            "Heading:\n  a        1.0 ms   3 runs\n  longer  10.0 ms  12 runs\n"
        );
    }
}
