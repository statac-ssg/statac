//! What `statac import` says when it is done, and `IMPORT.md`, the whole
//! account for the author to work through.

use super::Finding;
use super::common::counted;
use super::write::{REPORT, Summary};
use crate::config;

/// The names the report gives the tool and the two folders.
pub(super) struct Shown<'a> {
    pub tool: &'a str,
    pub old: &'a str,
    pub new: &'a str,
}

/// `text` as Markdown code, fenced so no backtick inside ends it, on one
/// line.
pub(super) fn code(text: &str) -> String {
    let text = text.replace(['\n', '\r'], " ");
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        run = if c == '`' { run + 1 } else { 0 };
        longest = longest.max(run);
    }
    let fence = "`".repeat(longest + 1);
    let pad = match text.starts_with('`') || text.ends_with('`') {
        true => " ",
        false => "",
    };
    format!("{fence}{pad}{text}{pad}{fence}")
}

/// The counts of what converted, those that are none left out.
fn converted(summary: &Summary) -> Vec<String> {
    [
        (summary.posts, "post", "posts"),
        (summary.drafts, "draft", "drafts"),
        (summary.pages, "page", "pages"),
        (summary.files, "other file", "other files"),
    ]
    .into_iter()
    .filter(|(n, _, _)| *n > 0)
    .map(|(n, one, many)| counted(n, one, many))
    .collect()
}

/// What is printed once the site is made.
pub(super) fn said(shown: &Shown, summary: &Summary) -> String {
    let mut text = format!(
        "Imported the {} site `{}` into `{}`. This is a best effort: read `{}/{REPORT}` before \
         you publish.\n\n",
        shown.tool, shown.old, shown.new, shown.new
    );
    let converted = converted(summary);
    if !converted.is_empty() {
        text.push_str(&format!("Converted: {}.\n", converted.join(", ")));
    }
    let kept = &summary.kept;
    let urls: Vec<String> = [
        (kept.answered > 0).then(|| format!("{} kept", kept.answered)),
        (kept.lost() > 0).then(|| format!("{} no longer published", kept.lost())),
        (!kept.unkept.is_empty()).then(|| format!("{} that can't be kept", kept.unkept.len())),
    ]
    .into_iter()
    .flatten()
    .collect();
    if !urls.is_empty() {
        text.push_str(&format!("URLs: {}.\n", urls.join(", ")));
    }
    let (things, kinds) = summary.left_for_you();
    if things > 0 {
        text.push_str(&format!(
            "Left for you: {} of {}.\n",
            counted(things, "thing", "things"),
            counted(kinds, "kind", "kinds")
        ));
    }
    if !summary.starter {
        text.push_str(
            "The new site has no templates, so it won't build until you add your own: see \
             https://statac.dev/templates/.\n",
        );
    }
    if summary.errors > 0 {
        let errors = counted(summary.errors, "error", "errors");
        text.push_str(&match summary.starter {
            true => format!(
                "The new site doesn't build yet: {errors}, listed in `{}/{REPORT}`.\n",
                shown.new
            ),
            false => format!(
                "The new site also has {errors}, listed in `{}/{REPORT}`.\n",
                shown.new
            ),
        });
    }
    text.push_str(&match summary.starter {
        true => format!("\nNext: `cd {}`, then `statac check`.\n", shown.new),
        false => format!(
            "\nNext: `cd {}`, add your own templates to `templates/`, then `statac check`.\n",
            shown.new
        ),
    });
    text
}

/// `IMPORT.md`.
pub(super) fn document(shown: &Shown, summary: &Summary) -> String {
    let mut text = format!(
        "This site was imported from {} {tool} site, as well as Statac can. It is a best effort: \
         the content, its settings and its URLs came across, the old site's templates and theme \
         did not, and what is listed below needs your eye before you publish.\n\n",
        match shown.tool.starts_with(['A', 'E', 'I', 'O', 'U']) {
            true => "an",
            false => "a",
        },
        tool = shown.tool
    );
    match summary.starter {
        true => text.push_str(&format!(
            "It was made from `{}` by `statac import`, which changed nothing there. The templates, \
             snippets and stylesheet are Statac's starter, as `statac new` writes it: see \
             https://statac.dev/new/.\n\n",
            shown.old
        )),
        false => {
            text.push_str(&format!(
                "It was made from `{}` by `statac import --no-starter`, which changed nothing \
                 there. It has none of the starter's templates, snippets, stylesheet or pages, \
                 so it won't build until you add templates of your own: see \
                 https://statac.dev/templates/.",
                shown.old
            ));
            if !summary.templates.is_empty() {
                let templates: Vec<String> = summary.templates.iter().map(|t| code(t)).collect();
                let feeds = summary
                    .templates
                    .iter()
                    .any(|t| t.starts_with("templates/feed."));
                text.push_str(&format!(
                    " Its pages{} need {}.",
                    if feeds { " and feeds" } else { "" },
                    crate::wording::listed(&templates)
                ));
            }
            text.push_str("\n\n");
        }
    }
    came_across(&mut text, summary);
    addresses(&mut text, summary);
    left_for_you(&mut text, summary);
    not_converted(&mut text, summary);
    text.push_str(match summary.starter {
        true => {
            "## What to do next\n\n\
             1. Run `statac check` in this folder, and put right what it shows.\n\
             2. Run `statac serve`, and read the site in a browser beside the old one.\n\
             3. Once nothing here is left to do, delete `IMPORT.md`.\n"
        }
        false => {
            "## What to do next\n\n\
             1. Add your own templates to `templates/`.\n\
             2. Run `statac check` in this folder, and put right what it shows.\n\
             3. Run `statac serve`, and read the site in a browser beside the old one.\n\
             4. Once nothing here is left to do, delete `IMPORT.md`.\n"
        }
    });
    text
}

fn came_across(text: &mut String, summary: &Summary) {
    text.push_str("## What came across\n\n");
    let folders: Vec<String> = summary
        .post_folders
        .iter()
        .map(|folder| match folder.is_empty() {
            true => code("content/"),
            false => code(&format!("content/{folder}/")),
        })
        .collect();
    let mut lines = Vec::new();
    if summary.posts > 0 {
        lines.push(format!(
            "{}, into {}.",
            counted(summary.posts, "post", "posts"),
            folders.join(" and ")
        ));
    }
    let one = |n: usize, one: &'static str, many: &'static str| if n == 1 { one } else { many };
    if summary.drafts > 0 {
        lines.push(format!(
            "{}, {}",
            counted(summary.drafts, "draft", "drafts"),
            one(
                summary.drafts,
                "with `draft: true`, so it isn't published by accident.",
                "each with `draft: true`, so none is published by accident."
            )
        ));
    }
    if summary.pages > 0 {
        lines.push(format!(
            "{}, into `content/` where {} in the old site.",
            counted(summary.pages, "page", "pages"),
            one(summary.pages, "it stood", "they stood")
        ));
    }
    if summary.files > 0 {
        lines.push(format!(
            "{}, copied as {}: files published as they are into `static/` at the same path, \
             so their URLs stay, and data files into `data/`.",
            counted(summary.files, "other file", "other files"),
            one(summary.files, "it was", "they were")
        ));
    }
    if lines.is_empty() {
        lines.push("Nothing: the old site had no pages or files Statac could bring across.".into());
    }
    for line in lines {
        text.push_str(&format!("- {line}\n"));
    }
    text.push('\n');
    match (summary.starter, summary.home) {
        (true, false) => text.push_str(
            "The old site's home page had no content of its own, so the starter's home page \
             stays, listing the posts: `content/index.md`.\n\n",
        ),
        (true, true) => text.push_str(
            "The old site's home page is `content/index.md`, built with \
             `templates/home.html`, which lists the posts below its content.\n\n",
        ),
        (false, false) => text.push_str(
            "The old site's home page had no content of its own, so `content/index.md` holds \
             only front matter: write the home page there.\n\n",
        ),
        (false, true) => text.push_str("The old site's home page is `content/index.md`.\n\n"),
    }
    for (path, instead) in &summary.displaced {
        text.push_str(&format!(
            "The starter's {} was left out: the old site's {} is published at its URL.\n\n",
            code(&format!("content/{path}")),
            code(instead)
        ));
    }
    for note in (summary.notes.iter())
        .chain(&terms(summary))
        .chain(&settled(summary))
    {
        text.push_str(note);
        text.push_str("\n\n");
    }
    text.push_str(&format!(
        "The settings written in `{}`:\n\n```yaml\n",
        config::FILE_NAME
    ));
    for line in summary.settings.lines() {
        if line.starts_with('#') || line.starts_with("feed_key:") {
            continue;
        }
        text.push_str(line);
        text.push('\n');
    }
    text.push_str(
        "```\n\nand a `feed_key` made fresh for the site: see \
         https://statac.dev/feeds/#entry-ids.\n\n",
    );
}

/// What became of the old site's lists of terms that Statac has no equal
/// for, a sentence for each list.
fn terms(summary: &Summary) -> Vec<String> {
    let mut notes = Vec::new();
    for (list, names) in &summary.terms.lists {
        // What became of more than one series has a sentence of its own.
        if list == "series" {
            continue;
        }
        let Some((_, example)) = names.first() else {
            continue;
        };
        let mark = example.split('_').next().unwrap_or_default();
        notes.push(match names.len() {
            1 => format!(
                "1 name in the old site's {} is now a tag, with {} before it, as {}, so it can \
                 still be told from the site's own tags. To rename it across the site, change it \
                 on each page that has it.",
                code(list),
                code(&format!("{mark}_")),
                code(example)
            ),
            n => format!(
                "{n} names in the old site's {} are now tags, each with {} before it, as {}, so \
                 they can still be told from the site's own tags. To rename one across the site, \
                 change it on each page that has it.",
                code(list),
                code(&format!("{mark}_")),
                code(example)
            ),
        });
    }
    if summary.terms.several_series > 0 {
        notes.push(format!(
            "{} more than one series, and a page is in one series at most in Statac: the first \
             is its `series`, and each other is a tag with `series_` before it.",
            match summary.terms.several_series {
                1 => "1 page named".to_owned(),
                n => format!("{n} pages named"),
            }
        ));
    }
    notes
}

/// What was changed in the pages' content so Statac reads it as the old
/// site showed it, a sentence for each kind of change made.
fn settled(summary: &Summary) -> Vec<String> {
    let mut notes = Vec::new();
    let tags = summary.settled.tags;
    if tags > 0 {
        notes.push(format!(
            "Statac put a blank line before {} that stood inside other HTML and held a blank \
             line: its Markdown ends a block of HTML at a blank line unless one of those tags \
             opens it, so what followed the blank line would have been read as Markdown.",
            match tags {
                1 => "1 `<pre>`, `<script>`, `<style>` or `<textarea>` tag".to_owned(),
                n => format!("{n} `<pre>`, `<script>`, `<style>` or `<textarea>` tags"),
            }
        ));
    }
    let fences = summary.settled.fences;
    if fences > 0 {
        notes.push(format!(
            "Statac put a blank line before {} that stood inside HTML, so {} is read as code and \
             not as part of the HTML.",
            counted(fences, "code block", "code blocks"),
            match fences {
                1 => "it",
                _ => "each",
            }
        ));
    }
    let moved = summary.settled.markers_moved;
    if moved > 0 {
        notes.push(format!(
            "Statac gave {} a blank line each side, where {} stood on a line of its own inside \
             HTML: Statac ends a summary only at one between two blocks of Markdown. The \
             summary ends where it did, and the lines after the marker, up to the next blank \
             line, are read as Markdown now, where they were part of the HTML.",
            counted(moved, "`<!--more-->` marker", "`<!--more-->` markers"),
            match moved {
                1 => "it",
                _ => "each",
            }
        ));
    }
    let dollars = summary.settled.dollars;
    if dollars > 0 {
        notes.push(format!(
            "Statac put a backslash before {} it would have read as opening maths, as `\\$`, \
             which the old site didn't set as maths, so {} shows as written: see \
             https://statac.dev/writing/#maths.",
            counted(dollars, "dollar sign", "dollar signs"),
            match dollars {
                1 => "it",
                _ => "each",
            }
        ));
    }
    let escaped = summary.settled.escaped;
    if escaped > 0 {
        notes.push(format!(
            "Statac put a backslash before the `@` in {} it would have read as a snippet call, as \
             `\\@name(`, so {} shows as written: see \
             https://statac.dev/writing/#snippet-calls.",
            counted(escaped, "place", "places"),
            match escaped {
                1 => "it",
                _ => "each",
            }
        ));
    }
    notes
}

fn addresses(text: &mut String, summary: &Summary) {
    let kept = &summary.kept;
    text.push_str("## URLs\n\n");
    let all = kept.answered + kept.unanswered.len();
    if all == 0 {
        text.push_str("The old site published no page at a URL Statac could work out.\n\n");
        unanswered(text, summary);
        return;
    }
    let found = match (all, kept.answered) {
        (1, 1) => "looked for the one old URL there: it is published there.".to_owned(),
        (1, _) => "looked for the one old URL there: nothing is published there.".to_owned(),
        (_, 0) => format!("looked for each of the {all} old URLs there: none is published."),
        (_, 1) => {
            format!("looked for each of the {all} old URLs there: 1 is published or redirected.")
        }
        (_, answered) => format!(
            "looked for each of the {all} old URLs there: {answered} are published or redirected."
        ),
    };
    text.push_str(&format!(
        "Statac worked out where the old site published each page, read the new site, and \
         {found}\n\n"
    ));
    let mut ways = Vec::new();
    for (folder, pattern, covered) in &kept.patterns {
        ways.push(format!(
            "{} cascades {} to every page in {}, which puts {} of them at {}.",
            code(&format!("content/{folder}/index.md")),
            code(&format!("pages.permalink: {pattern}")),
            code(&format!("content/{folder}/")),
            covered,
            match covered {
                1 => "its old URL",
                _ => "their old URLs",
            }
        ));
    }
    if kept.own > 0 {
        ways.push(format!(
            "{} a `permalink` of {}, where no pattern fits.",
            match kept.own {
                1 => "1 page carries".to_owned(),
                n => format!("{n} pages carry"),
            },
            match kept.own {
                1 => "its own",
                _ => "their own",
            }
        ));
    }
    if kept.redirected > 0 {
        ways.push(format!(
            "{} `redirect_from`, for the other URLs that led to {}.",
            match kept.redirected {
                1 => "1 page carries".to_owned(),
                n => format!("{n} pages carry"),
            },
            match kept.redirected {
                1 => "it",
                _ => "them",
            }
        ));
    }
    if let Some((_, example)) = kept.sent_on.first() {
        ways.push(match kept.sent_on.len() {
            1 => format!(
                "1 page at the top of `content/`, {}, redirects from the old page of a tag, a \
                 category or a series to the page Statac makes for it, with `permalink` and \
                 `redirect_to`.",
                code(example)
            ),
            n => format!(
                "{n} pages at the top of `content/`, such as {}, redirect from the old page of a \
                 tag, a category or a series to the page Statac makes for it, each with \
                 `permalink` and `redirect_to`.",
                code(example)
            ),
        });
    }
    if let Some((_, example)) = kept.later.pages.first() {
        ways.push(match kept.later.pages.len() {
            1 => format!(
                "1 page at the top of `content/`, {}, redirects from an old paginated page of the \
                 home page to the page of `content/posts/` that holds the same posts, with \
                 `permalink` and `redirect_to`.",
                code(example)
            ),
            n => format!(
                "{n} pages at the top of `content/`, such as {}, redirect from the old paginated \
                 pages of the home page, each to the page of `content/posts/` that holds the \
                 same posts, with `permalink` and `redirect_to`.",
                code(example)
            ),
        });
    }
    if let Some((old, example)) = kept.later.unlike.first() {
        ways.push(match kept.later.unlike.len() {
            1 => format!(
                "1 page at the top of `content/`, {}, redirects from {}, an old paginated page \
                 of the home page, to the page of `content/posts/` with the same number, \
                 which doesn't hold exactly the same posts, as happens where posts share a date.",
                code(example),
                code(old)
            ),
            n => format!(
                "{n} pages at the top of `content/`, such as {}, redirect from old paginated \
                 pages of the home page, each to the page of `content/posts/` with the same \
                 number, which doesn't hold exactly the same posts, as happens where posts \
                 share a date: {}.",
                code(example),
                (kept.later.unlike.iter())
                    .map(|(old, _)| code(old))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        });
    }
    if let Some((_, example)) = kept.list_pages.first() {
        ways.push(match kept.list_pages.len() {
            1 => format!(
                "1 page at the top of `content/`, {}, redirects from an old paginated page of a \
                 list, found in the old site's built output, to the list's paginated page with \
                 the same number, with `permalink` and `redirect_to`.",
                code(example)
            ),
            n => format!(
                "{n} pages at the top of `content/`, such as {}, redirect from old paginated \
                 pages of lists, found in the old site's built output, each to the list's \
                 paginated page with the same number, with `permalink` and `redirect_to`.",
                code(example)
            ),
        });
    }
    match kept.later.left {
        0 => {}
        1 => ways.push(
            "1 old paginated page of the home page held posts that no one page of \
             `content/posts/` holds, and `content/posts/` has no page with the same number, so \
             nothing is published at its URL: it is listed below."
                .to_owned(),
        ),
        n => ways.push(format!(
            "{n} old paginated pages of the home page held posts that no one page of \
             `content/posts/` holds, and `content/posts/` has no page with the same number, so \
             nothing is published at their URLs: they are listed below."
        )),
    }
    if !summary.base.is_empty() {
        ways.push(format!(
            "The site is under a path, {}, as its `url` says, so every URL starts with it, and \
             links in the content were written with it: see https://statac.dev/addresses/.",
            code(&summary.base)
        ));
    }
    if ways.is_empty() {
        ways.push("Every page lands where the old site published it, with nothing added.".into());
    }
    for way in ways {
        text.push_str(&format!("- {way}\n"));
    }
    text.push('\n');
    if let Some(built) = &summary.built {
        text.push_str(&format!(
            "Each URL was also looked for in the old site's own built output, {}.\n\n",
            code(&format!("{built}/"))
        ));
    }
    unanswered(text, summary);
}

/// The old URLs where nothing is published: those Statac worked out, each
/// on a line, then the pages of the old built output at any other URL, by
/// the folder each is in.
fn unanswered(text: &mut String, summary: &Summary) {
    let kept = &summary.kept;
    if !kept.unanswered.is_empty() {
        text.push_str(
            "Nothing in the new site is published at these old URLs. Give the page each should \
             lead to a `redirect_from` naming it, or a `permalink`:\n\n",
        );
        for (address, from) in &kept.unanswered {
            match from.is_empty() {
                true => text.push_str(&format!("- {}\n", code(address))),
                false => text.push_str(&format!("- {}, for {}\n", code(address), code(from))),
            }
        }
        text.push('\n');
    }
    if !kept.unkept.is_empty() {
        text.push_str(match kept.unkept.len() {
            1 => {
                "This old URL can't be kept: the new site can't redirect from it, so it was left \
                  out of its page's `redirect_from`.\n\n"
            }
            _ => {
                "These old URLs can't be kept: the new site can't redirect from them, so each \
                  was left out of its page's `redirect_from`.\n\n"
            }
        });
        for (address, from, why) in &kept.unkept {
            text.push_str(&format!("- {}, for {}: {why}\n", code(address), code(from)));
        }
        text.push('\n');
    }
    for (address, list) in &kept.lists {
        text.push_str(&format!(
            "The old site published a page listing every one of its {} at {}, and the {} has \
             none.\n\n",
            code(list),
            code(address),
            match summary.starter {
                true => "starter",
                false => "new site",
            }
        ));
    }
    if let Some(example) = kept.pictures.first() {
        text.push_str(&match kept.pictures.len() {
            1 => format!(
                "1 image beside a page is shown as resized copies Statac makes, and the file \
                 itself isn't published, so its old URL, {}, is no longer published.",
                code(example)
            ),
            n => format!(
                "{n} images beside pages are shown as resized copies Statac makes, and the files \
                 themselves aren't published, so the old URL of each, such as {}, is no longer \
                 published.",
                code(example)
            ),
        });
        text.push_str(
            " An image's own file is published where a page links to it, or where its sidecar \
             file, such as `photo-1.jpg.yaml`, holds `optimise: false`; `optimise_images: false` \
             in `statac.yaml` leaves every image as it is: see https://statac.dev/pictures/.\n\n",
        );
    }
    if !kept.files.is_empty() {
        text.push_str(match kept.files.len() {
            1 => {
                "Nothing is published at the old URL of the file given another name, and a file \
                  can't redirect:\n\n"
            }
            _ => {
                "Nothing is published at the old URLs of the files given another name, and a \
                  file can't redirect:\n\n"
            }
        });
        for (address, from) in &kept.files {
            text.push_str(&format!("- {}, for {}\n", code(address), code(from)));
        }
        text.push('\n');
    }
    if kept.unworked.is_empty() {
        return;
    }
    text.push_str(&match kept.unworked.len() {
        1 => "The old site's built output also holds 1 page at a URL Statac didn't work out, \
              and nothing in the new site is published there. Give the page it should lead to a \
              `redirect_from` naming it:\n\n"
            .to_owned(),
        n => format!(
            "The old site's built output also holds {n} pages at URLs Statac didn't work out, \
             and nothing in the new site is published there. Give the page each should lead to \
             a `redirect_from` naming it. By folder:\n\n"
        ),
    });
    let folder_of = |address: &str| match address[1..].split_once('/') {
        Some((folder, _)) => format!("/{folder}/"),
        None => "/".to_owned(),
    };
    let mut folders: Vec<(String, Vec<String>)> = Vec::new();
    for address in &kept.unworked {
        let folder = folder_of(address);
        match folders.iter_mut().find(|(known, _)| *known == folder) {
            Some((_, pages)) => pages.push(code(address)),
            None => folders.push((folder, vec![code(address)])),
        }
    }
    for (folder, pages) in folders {
        let named = match pages.as_slice() {
            [one] => one.clone(),
            [one, two] => format!("{one} and {two}"),
            [one, two, three] => format!("{one}, {two} and {three}"),
            [one, two, three, ..] => format!("such as {one}, {two} and {three}"),
            [] => String::new(),
        };
        text.push_str(&format!(
            "- {}: {}, {named}\n",
            code(&folder),
            counted(pages.len(), "page", "pages")
        ));
    }
    text.push('\n');
}

fn left_for_you(text: &mut String, summary: &Summary) {
    text.push_str("## Left for you\n\n");
    if summary.findings.is_empty() {
        text.push_str("Nothing: everything Statac read came across.\n\n");
        return;
    }
    let mut at = 0;
    while at < summary.findings.len() {
        let kind = summary.findings[at].kind;
        let end = summary.findings[at..]
            .iter()
            .position(|finding| !std::ptr::eq(finding.kind, kind))
            .map_or(summary.findings.len(), |n| at + n);
        let of_kind = &summary.findings[at..end];
        text.push_str(&format!("### {}\n\n{}\n\n", kind.heading, kind.sentence));
        for finding in of_kind {
            spots(text, finding);
        }
        text.push('\n');
        at = end;
    }
}

/// Writes every one of a finding's places.
fn spots(text: &mut String, finding: &Finding) {
    let place = |spot: &super::Spot| {
        if spot.file.is_empty() {
            return match spot.times {
                Some(times) => {
                    format!(
                        "{}, {}",
                        code(&spot.text),
                        counted(times, "place", "places")
                    )
                }
                None => spot.text.clone(),
            };
        }
        let mut said = code(&spot.file);
        if let Some(line) = spot.line {
            said.push_str(&format!(", line {line}"));
        }
        if !spot.text.is_empty() {
            said.push_str(&format!(": {}", code(&spot.text)));
        }
        said
    };
    match &finding.about {
        Some(about) => {
            text.push_str(&format!("- {}:\n", code(about)));
            for spot in &finding.spots {
                text.push_str(&format!("  - {}\n", place(spot)));
            }
        }
        None => {
            for spot in &finding.spots {
                text.push_str(&format!("- {}\n", place(spot)));
            }
        }
    }
}

fn not_converted(text: &mut String, summary: &Summary) {
    text.push_str("## Not converted\n\n");
    if summary.left.is_empty() {
        text.push_str("Nothing: Statac read every file of the old site.\n\n");
        return;
    }
    text.push_str("Statac left these behind, and the new site has nothing of them:\n\n");
    for left in &summary.left {
        text.push_str(&format!("- {}: {}.\n", code(&left.path), left.what));
    }
    text.push('\n');
}
