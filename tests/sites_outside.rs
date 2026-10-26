//! Reads sites kept outside the repository and checks they read with
//! no errors. `STATAC_SITES` names their folders, parted as `PATH` is;
//! `STATAC_ADDRESSES` names a folder to write each site's addresses into,
//! one file per site, for comparing with another build's.

use std::path::PathBuf;

use statac::ids::{Id, LanguageId};
use statac::problem::Severity;

#[test]
#[ignore = "reads sites outside the repository, named by STATAC_SITES"]
fn sites_outside_read_with_no_errors() {
    let sites = std::env::var_os("STATAC_SITES").expect("STATAC_SITES names the sites");
    let addresses = std::env::var_os("STATAC_ADDRESSES").map(PathBuf::from);
    let mut errors = 0;
    for site in std::env::split_paths(&sites) {
        let (read, ..) = statac::build::read(&site, &statac::content::Run::default());
        let name = site
            .parent()
            .and_then(|p| p.file_name())
            .unwrap_or(site.as_os_str())
            .to_string_lossy()
            .into_owned();
        println!(
            "{name}: {} pages, {} drafts, {} files, {} old addresses, {} problems",
            read.site.pages.len(),
            read.drafts.len(),
            read.site.files.len(),
            read.site.redirects.len(),
            read.problems.len()
        );
        for problem in &read.problems {
            let at = problem.place.map(|place| {
                let line = read
                    .sources
                    .line_column(place.source, place.span.start as usize)
                    .line;
                format!(" ({}:{line})", read.sources.path(place.source).display())
            });
            println!(
                "  {:?} {}{}",
                problem.kind.severity(),
                problem.headline(),
                at.unwrap_or_default()
            );
            errors += usize::from(problem.kind.severity() == Severity::Error);
        }
        if let Some(folder) = &addresses {
            let mut files: Vec<String> = read.claims().into_iter().map(|(file, _)| file).collect();
            files.sort();
            std::fs::create_dir_all(folder).unwrap();
            std::fs::write(folder.join(format!("{name}.txt")), files.join("\n") + "\n").unwrap();
        }
    }
    assert_eq!(errors, 0, "the sites read with errors");
}

/// `STATAC_DATES` names a folder holding a file per site, named as the
/// site's folder above `site/` is, each line a date as written, what was
/// asked of it (`date`, `moment`, `rss` or `example:` and a template's
/// example), and what another build wrote, parted by tabs.
#[test]
#[ignore = "reads sites outside the repository, named by STATAC_SITES and STATAC_DATES"]
fn sites_outside_write_dates_as_expected() {
    use statac::dates::ShowDates;
    let sites = std::env::var_os("STATAC_SITES").expect("STATAC_SITES names the sites");
    let expected = std::env::var_os("STATAC_DATES").expect("STATAC_DATES names the folder");
    let expected = PathBuf::from(expected);
    let mut differ = 0;
    for site in std::env::split_paths(&sites) {
        let name = site
            .parent()
            .and_then(|p| p.file_name())
            .unwrap_or(site.as_os_str())
            .to_string_lossy()
            .into_owned();
        let Ok(lines) = std::fs::read_to_string(expected.join(format!("{name}.txt"))) else {
            println!("{name}: nothing expected");
            continue;
        };
        let mut sources = statac::source::Sources::default();
        let (settings, _) = statac::config::read(&site, &mut sources);
        let (dates, _) = statac::dates::Dates::read(&site, &settings, &mut sources);
        let (mut asked, mut wrong) = (0, 0);
        for line in lines.lines().filter(|line| !line.is_empty()) {
            let [written, what, wanted] = line.splitn(3, '\t').collect::<Vec<_>>()[..] else {
                panic!("{name}: {line}");
            };
            let date = statac::date::read(written).unwrap_or_else(|f| panic!("{written}: {f:?}"));
            let got = match what {
                "date" => dates.format(LanguageId::from_index(0), &date, None).ok(),
                "moment" => Some(dates.moment(&date)),
                "rss" => Some(dates.moment_rss(&date)),
                _ => {
                    let example = what.strip_prefix("example:").expect("an example");
                    dates
                        .format(LanguageId::from_index(0), &date, Some(example))
                        .ok()
                }
            };
            asked += 1;
            if got.as_deref() != Some(wanted) {
                wrong += 1;
                println!("  {written} {what}: {got:?}, expected {wanted:?}");
            }
        }
        println!("{name}: {} of {asked} as expected", asked - wrong);
        differ += wrong;
    }
    assert_eq!(differ, 0, "dates written otherwise than expected");
}

/// Builds every page of each site in `STATAC_SITES` through its own
/// templates, and says how many built with no problems, and which kinds of
/// problem the rest met, with an example of each.
#[test]
#[ignore = "reads sites outside the repository, named by STATAC_SITES"]
fn sites_outside_render() {
    use std::collections::BTreeMap;

    use rayon::prelude::*;
    use statac::problem::Problem;

    let sites = std::env::var_os("STATAC_SITES").expect("STATAC_SITES names the sites");
    for site in std::env::split_paths(&sites) {
        let name = site
            .parent()
            .and_then(|p| p.file_name())
            .unwrap_or(site.as_os_str())
            .to_string_lossy()
            .into_owned();
        let today = statac::date::Date::day_only(2026, 10, 4);
        let (renderer, read) = statac::build::renderer(&site, Some(today));
        let ids: Vec<_> = renderer.site().pages.iter().map(|(id, _)| id).collect();
        let built: Vec<_> = ids.par_iter().map(|&id| renderer.render(id)).collect();
        let clean = built.iter().filter(|b| b.problems.is_empty()).count();
        let sources = renderer.sources();
        let shown = |problem: &Problem| {
            let mut out = problem.headline();
            if let Some(place) = problem.place {
                let line = sources
                    .line_column(place.source, place.span.start as usize)
                    .line;
                let text = sources.spelling(place).lines().next().unwrap_or_default();
                out.push_str(&format!(
                    " ({}:{line}: `{text}`)",
                    sources.path(place.source).display()
                ));
            }
            if let Some(suggestion) = problem.suggestion() {
                out.push_str(&format!(" | {suggestion}"));
            }
            for related in problem.related() {
                out.push_str(&format!(" | {}", related.label));
                if let Some(place) = related.place {
                    let line = sources
                        .line_column(place.source, place.span.start as usize)
                        .line;
                    out.push_str(&format!(" {}:{line}", sources.path(place.source).display()));
                }
            }
            out
        };
        let in_templates = |problem: &&Problem| {
            problem.place.is_some_and(|place| {
                let path = sources.path(place.source);
                path.starts_with("templates") || path.starts_with("snippets")
            })
        };
        let template_problems: Vec<&Problem> = read.iter().filter(in_templates).collect();
        let mut kinds: BTreeMap<&str, (usize, String)> = BTreeMap::new();
        for problem in built
            .iter()
            .flat_map(|b| &b.problems)
            .chain(template_problems.iter().copied())
        {
            let entry = kinds
                .entry(problem.kind.name())
                .or_insert_with(|| (0, shown(problem)));
            entry.0 += 1;
        }
        println!(
            "{name}: {clean} of {} pages built with no problems; {} problems reading templates",
            ids.len(),
            template_problems.len()
        );
        for (kind, (count, example)) in &kinds {
            println!("  {kind} x{count}: {example}");
        }
        if let Some(wanted) = std::env::var_os("STATAC_SHOW") {
            let wanted = wanted.to_string_lossy();
            for (id, page) in renderer.site().pages.iter() {
                if page.path.ends_with(wanted.as_ref()) {
                    let built = &built[ids.iter().position(|&i| i == id).unwrap()];
                    println!("{}", built.html.as_deref().unwrap_or("(not built)"));
                    println!("{:?}", built.reads.keys());
                }
            }
        }
        if std::env::var_os("STATAC_ALL").is_some() {
            for problem in template_problems
                .iter()
                .copied()
                .chain(built.iter().flat_map(|b| &b.problems))
            {
                println!("    {}", shown(problem));
            }
        }
    }
}

/// `STATAC_TEXT` names a folder holding each site's page text as another
/// build wrote it, `<site>/text/<file>.txt`, the site named as its folder
/// above `site/` is. Each block of a page's writing, its punctuation set in
/// the site's language, is looked for in that page's text, and so is the
/// block with nothing set, to show what setting it changed. A block holding
/// raw HTML, a call, maths, a note's marker or a picture is left out, as
/// its text is only known once built.
#[test]
#[ignore = "reads sites outside the repository, named by STATAC_SITES and STATAC_TEXT"]
fn sites_outside_set_punctuation_as_expected() {
    use statac::markdown::{Kind, Node, Typography, set_typography};

    fn words_of(node: &Node, out: &mut String) {
        match &node.kind {
            Kind::Text(text) | Kind::Literal(text) | Kind::Code(text) => out.push_str(text),
            Kind::SoftBreak | Kind::HardBreak => out.push(' '),
            _ => node.children.iter().for_each(|child| words_of(child, out)),
        }
    }

    fn plain_blocks<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
        if matches!(
            node.kind,
            Kind::Paragraph { .. } | Kind::Heading { .. } | Kind::TableCell | Kind::Term
        ) {
            let mut plain = true;
            node.walk(&mut |n| {
                plain &= matches!(
                    n.kind,
                    Kind::Paragraph { .. }
                        | Kind::Heading { .. }
                        | Kind::TableCell
                        | Kind::Term
                        | Kind::Text(_)
                        | Kind::Literal(_)
                        | Kind::Code(_)
                        | Kind::SoftBreak
                        | Kind::HardBreak
                        | Kind::Emphasis
                        | Kind::Strong
                        | Kind::Strikethrough
                        | Kind::Link { .. }
                );
            });
            if plain {
                out.push(node);
            }
            return;
        }
        for child in &node.children {
            plain_blocks(child, out);
        }
    }

    let squash = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    // Every quote, dash and ellipsis as typed, to tell a block that differs
    // only in its punctuation from one that differs in its words.
    let folded = |text: &str| {
        let mut out = String::with_capacity(text.len());
        for c in text.chars() {
            match c {
                '\'' | '"' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201c}' | '\u{201d}'
                | '\u{201e}' | '\u{ab}' | '\u{bb}' | '\u{300c}' | '\u{300d}' | '\u{300e}'
                | '\u{300f}' => {}
                '-' | '\u{2013}' | '\u{2014}' if out.ends_with('-') => {}
                '-' | '\u{2013}' | '\u{2014}' => out.push('-'),
                '\u{2026}' => out.push_str("..."),
                c => out.push(c),
            }
        }
        out
    };
    let plain_text = |node: &Node| {
        let mut out = String::new();
        words_of(node, &mut out);
        out
    };
    let sites = std::env::var_os("STATAC_SITES").expect("STATAC_SITES names the sites");
    let expected = std::env::var_os("STATAC_TEXT").expect("STATAC_TEXT names the folder");
    let expected = PathBuf::from(expected);
    let mut differ = 0;
    for site in std::env::split_paths(&sites) {
        let name = site
            .parent()
            .and_then(|p| p.file_name())
            .unwrap_or(site.as_os_str())
            .to_string_lossy()
            .into_owned();
        let (mut read, ..) = statac::build::read(&site, &statac::content::Run::default());
        let (dates, _) = statac::dates::Dates::read(&site, &read.settings, &mut read.sources);
        let typography = Typography::of(&dates.language(LanguageId::from_index(0)).words);
        let on = read.settings.truth("typography") != Some(false);
        let (mut blocks, mut matched, mut unset, mut shown) = (0, 0, 0, 0);
        let mut punctuation = 0;
        for (_, page) in read.site.pages.iter() {
            let Some(source) = page.source else {
                continue;
            };
            if page.path.ends_with(".html") || page.path.ends_with(".htm") {
                continue;
            }
            let file = expected
                .join(&name)
                .join("text")
                .join(format!("{}.txt", page.address.file()));
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let text = squash(&text);
            let folded_text = folded(&text);
            let writing =
                || statac::markdown::read(&read.sources, source, page.body, &|_| None, "");
            let (as_typed, mut set) = (writing(), writing());
            if on {
                set_typography(&mut set, &read.sources, &typography);
            }
            let (mut before, mut after) = (Vec::new(), Vec::new());
            for top in std::iter::once(&as_typed.root).chain(&as_typed.notes) {
                plain_blocks(top, &mut before);
            }
            for top in std::iter::once(&set.root).chain(&set.notes) {
                plain_blocks(top, &mut after);
            }
            for (typed, set) in before.iter().zip(&after) {
                let (typed, set) = (squash(&plain_text(typed)), squash(&plain_text(set)));
                // A call to a snippet that wraps writing is read here as one
                // to no snippet, which leaves its `@end` among the words.
                if set.is_empty() || typed.contains("@end") {
                    continue;
                }
                blocks += 1;
                unset += usize::from(text.contains(&typed));
                if text.contains(&set) {
                    matched += 1;
                } else if folded_text.contains(&folded(&set)) {
                    punctuation += 1;
                    if shown < 8 {
                        shown += 1;
                        println!("  {}: {set}", page.path);
                    }
                }
            }
        }
        println!(
            "{name}: {matched} of {blocks} blocks as expected, {unset} with nothing set; \
             {punctuation} differ in punctuation alone"
        );
        differ += punctuation;
    }
    assert_eq!(differ, 0, "punctuation set otherwise than expected");
}
