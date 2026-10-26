//! Writes the documentation's reference pages into the docs site's folder:
//! a page for each group of names Statac describes, from the files in
//! `descriptions/`, and the page listing them: `cargo run --example
//! reference_pages -- <the docs site's folder>`.

use std::path::{Path, PathBuf};

use statac::described::{Described, Group};

#[allow(dead_code, reason = "only finding the docs site's folder is shared")]
#[path = "explanation_pages.rs"]
mod explanation_pages;

const DOCS: &str = "https://statac.dev";

/// The groups the page listing them puts under each heading.
const SECTIONS: &[(&str, &[Group])] = &[
    (
        "Settings and front matter",
        &[Group::Settings, Group::Fields],
    ),
    (
        "Templates",
        &[
            Group::Handed,
            Group::Page,
            Group::Site,
            Group::List,
            Group::Date,
            Group::Webmentions,
            Group::Video,
            Group::Image,
            Group::File,
            Group::Card,
            Group::Heading,
            Group::Search,
            Group::SearchField,
            Group::Loop,
            Group::Filters,
            Group::Tests,
            Group::Functions,
        ],
    ),
];

/// Where the reference pages go in the docs site `site`.
pub fn folder(site: &Path) -> PathBuf {
    site.join("content/reference")
}

/// Every page, by its file's name in [`folder`], with its text.
pub fn pages() -> Vec<(String, String)> {
    let mut pages: Vec<(String, String)> = (Group::ALL.iter())
        .map(|&group| (format!("{}.md", group.name()), page(group)))
        .collect();
    pages.push(("index.md".to_owned(), index()));
    pages
}

/// What the line naming the source of every page this writes ends with,
/// by which a page it wrote is told from one written by hand.
const WRITTEN_BY: &str = "by `cargo run --example reference_pages`.";

/// Whether `text` is a page this wrote.
fn written_here(text: &str) -> bool {
    text.strip_prefix("---\n# Written from ")
        .and_then(|rest| rest.lines().next())
        .is_some_and(|line| line.ends_with(WRITTEN_BY))
}

fn front_matter(title: &str, summary: Option<&str>, layout: Option<&str>, from: &str) -> String {
    for text in [Some(title), summary].into_iter().flatten() {
        assert!(
            !text.contains([':', '#', '"']),
            "{text:?} would need quoting in YAML"
        );
    }
    let summary = summary.map_or_else(String::new, |summary| format!("summary: {summary}\n"));
    let layout = layout.map_or_else(String::new, |layout| format!("layout: {layout}\n"));
    format!("---\n# Written from {from} {WRITTEN_BY}\ntitle: {title}\n{summary}{layout}---\n")
}

/// The id of the heading for the name `name`: its letters, figures and
/// underscores, with a hyphen for anything else between them.
pub fn id(name: &str) -> String {
    let mut id = String::new();
    for c in name.to_lowercase().chars() {
        match c.is_alphanumeric() || c == '_' {
            true => id.push(c),
            false if !id.is_empty() && !id.ends_with('-') => id.push('-'),
            false => {}
        }
    }
    id.trim_end_matches('-').to_owned()
}

/// A link to `more` as the docs site links its own pages, from its top.
fn linked(more: &str) -> String {
    let at = more.strip_prefix(DOCS).unwrap_or(more);
    format!("[Read more]({at})")
}

fn entry(described: &Described) -> String {
    let more = (described.more.as_deref())
        .map_or_else(String::new, |more| format!("\n{}\n", linked(more)));
    format!(
        "\n## `{}` {{#{}}}\n\n{}\n{more}",
        described.name,
        id(&described.name),
        described.text
    )
}

fn page(group: Group) -> String {
    let mut out = front_matter(
        group.title(),
        None,
        None,
        &format!("descriptions/{}.md", group.name()),
    );
    out.push_str(&format!("\n{}\n", group.about()));
    for described in group.described() {
        out.push_str(&entry(described));
    }
    out
}

/// The first sentence of `text`.
fn opening(text: &str) -> &str {
    text.find(". ").map_or(text, |at| &text[..=at])
}

fn index() -> String {
    let mut out = front_matter(
        "Reference",
        Some(
            "Every setting, front matter field, template name, filter and function \
             Statac has, each in a line or two.",
        ),
        Some("page"),
        "descriptions/",
    );
    out.push_str(
        "\nEach of Statac's names has a line or two here. An editor shows the same \
         words when you point at the name, and the debug panel shows them beside it.\n",
    );
    for (heading, groups) in SECTIONS {
        out.push_str(&format!("\n## {heading}\n\n"));
        for group in *groups {
            out.push_str(&format!(
                "- [{}](/reference/{}/): {}\n",
                group.title(),
                group.name(),
                opening(group.about())
            ));
        }
    }
    out
}

/// Writes every reference page into the docs site `site`, and removes a
/// page there it wrote whose group is gone. A page written by hand beside
/// them is left alone.
pub fn write(site: &Path) {
    let folder = folder(site);
    std::fs::create_dir_all(&folder).expect("the folder can be made");
    let wanted: Vec<(String, String)> = pages();
    for entry in std::fs::read_dir(&folder).expect("the folder can be read") {
        let path = entry.expect("the folder can be read").path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let ours = std::fs::read_to_string(&path).is_ok_and(|text| written_here(&text));
        if ours && !wanted.iter().any(|(wanted, _)| *wanted == name) {
            std::fs::remove_file(&path).expect("an old page can be removed");
        }
    }
    for (name, text) in &wanted {
        std::fs::write(folder.join(name), text).expect("a page can be written");
    }
    println!("Wrote {} pages to {}.", wanted.len(), folder.display());
}

fn main() {
    let Some(site) = explanation_pages::site_given(std::env::args_os().skip(1)) else {
        eprintln!("Usage: cargo run --example reference_pages -- <the docs site's folder>");
        std::process::exit(2);
    };
    write(&site);
}
