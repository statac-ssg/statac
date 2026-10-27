//! Writes the documentation's generated pages into the docs site's folder:
//! the page for each kind of problem and the page listing them, from the
//! explanations `statac explain` prints: `cargo run --example
//! explanation_pages -- <the docs site's folder>`.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use statac::problem::{Code, Severity};

/// Where the problems' pages go in the docs site `site`.
pub fn folder(site: &Path) -> PathBuf {
    site.join("content/e")
}

/// Every page, by its file's name in [`folder`], with its text.
pub fn pages() -> Vec<(String, String)> {
    let mut pages: Vec<(String, String)> = Code::ALL
        .iter()
        .map(|&code| (format!("{}.md", code.name()), page(code)))
        .collect();
    pages.push(("index.md".to_owned(), index()));
    pages
}

/// The kind's readable name: its explanation's heading.
fn title(code: Code) -> &'static str {
    let text = code.explanation();
    let heading = text.lines().next().unwrap_or_default();
    heading
        .strip_prefix("# ")
        .unwrap_or_else(|| panic!("{} opens with no heading", code.name()))
}

fn front_matter(
    title: &str,
    summary: Option<&str>,
    severity: Option<Severity>,
    layout: Option<&str>,
    redirect_from: &[String],
    from: &str,
) -> String {
    for text in [Some(title), summary].into_iter().flatten() {
        assert!(
            !text.contains([':', '#', '"']),
            "{text:?} would need quoting in YAML"
        );
    }
    let summary = summary.map_or_else(String::new, |summary| format!("summary: {summary}\n"));
    let severity = match severity {
        Some(Severity::Error) => "severity: error\n",
        Some(Severity::Warning) => "severity: warning\n",
        Some(Severity::Note) => "severity: note\n",
        None => "",
    };
    let layout = layout.map_or_else(String::new, |layout| format!("layout: {layout}\n"));
    let redirect_from = match redirect_from {
        [] => String::new(),
        from => format!("redirect_from: [{}]\n", from.join(", ")),
    };
    format!(
        "---\n# Written from {from} by `cargo run --example explanation_pages`.\n\
         title: {title}\n{summary}{severity}{layout}{redirect_from}---\n"
    )
}

fn page(code: Code) -> String {
    let name = code.name();
    // A checkout may give the explanations either line ending.
    let text = code.explanation().replace("\r\n", "\n");
    let body = text
        .split_once('\n')
        .map_or("", |(_, rest)| rest)
        .trim_start_matches('\n');
    // The page alone says what the terminal adds to the message it shows.
    let seen = "\n## What you'll see {#what-youll-see}\n\n```text\n";
    let shown = body.find(seen).expect("each explanation shows its message") + seen.len();
    let after = shown + body[shown..].find("```\n").expect("the message closes") + 4;
    let body = format!(
        "{}\nIn the terminal, the message also shows the file and line, if there is one, \
         and marks the exact text.\n{}",
        &body[..after],
        &body[after..]
    );
    // Only `statac check` reports these, so only it counts them.
    let counts = match code {
        Code::HookNotRun | Code::LocationPublished | Code::NoHomePage | Code::NoListPage => {
            "<code>statac check</code>"
        }
        _ => "Statac",
    };
    let quieten = match code.quietenable() {
        true => format!(
            "<code>quiet: [{name}]</code> in <code>statac.yaml</code>, once you have \
             understood it. {counts} still counts it."
        ),
        false => match code.severity() {
            Severity::Error => "It stops the build, so it can't be quietened.".to_owned(),
            _ => "It leaves a page out of the site, so it can't be quietened.".to_owned(),
        },
    };
    format!(
        "{}\n{}\n\n## Its name\n\n<table class=\"named\">\n\
         \x20 <tr><th scope=\"row\">Name</th><td><code>{name}</code></td></tr>\n\
         \x20 <tr><th scope=\"row\">In the terminal</th>\
         <td><code>statac explain {name}</code></td></tr>\n\
         \x20 <tr><th scope=\"row\">To stop showing it</th><td>{quieten}</td></tr>\n\
         </table>\n",
        front_matter(
            title(code),
            None,
            Some(code.severity()),
            None,
            &code
                .former_names()
                .map(|former| format!("/e/{former}/"))
                .collect::<Vec<_>>(),
            &format!("explanations/{name}.md")
        ),
        body.trim_end()
    )
}

fn index() -> String {
    let mut out = front_matter(
        "Problems explained",
        Some("Every error, warning and note Statac reports, what each means and how to fix it."),
        None,
        Some("page"),
        &[],
        "explanations/",
    );
    out.push_str(
        "\nEvery kind of problem Statac reports has a page here. The page's \
         name is the last part of the problem's `More:` link, and running \
         `statac explain` with that name shows the same page in the terminal.\n",
    );
    let groups = [
        (
            Severity::Error,
            "Errors",
            "An error stops the build: nothing is published until it is fixed.",
        ),
        (
            Severity::Warning,
            "Warnings",
            "A warning says what Statac did instead, and the build goes on.",
        ),
        (
            Severity::Note,
            "Notes",
            "A note tells you something about the published page. It doesn't mean anything is wrong.",
        ),
    ];
    for (severity, heading, about) in groups {
        let mut kinds: Vec<Code> = Code::ALL
            .iter()
            .copied()
            .filter(|code| code.severity() == severity)
            .collect();
        if kinds.is_empty() {
            continue;
        }
        kinds.sort_by_key(|&code| title(code).to_lowercase());
        out.push_str(&format!("\n## {heading}\n\n{about}\n\n"));
        for code in kinds {
            out.push_str(&format!("- [{}](/e/{}/)\n", title(code), code.name()));
        }
    }
    out
}

/// The docs site's folder, where it is the one argument given and holds
/// a site's settings.
pub fn site_given(mut arguments: impl Iterator<Item = OsString>) -> Option<PathBuf> {
    let site = PathBuf::from(arguments.next()?);
    (arguments.next().is_none() && site.join("statac.yaml").is_file()).then_some(site)
}

/// Writes every generated page into the docs site `site`, and removes a
/// problem's page there whose kind is gone.
pub fn write(site: &Path) {
    let folder = folder(site);
    std::fs::create_dir_all(&folder).expect("the folder can be made");
    let wanted: Vec<(String, String)> = pages();
    for entry in std::fs::read_dir(&folder).expect("the folder can be read") {
        let path = entry.expect("the folder can be read").path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if !wanted.iter().any(|(wanted, _)| *wanted == name) {
            std::fs::remove_file(&path).expect("an old page can be removed");
        }
    }
    for (name, text) in &wanted {
        std::fs::write(folder.join(name), text).expect("a page can be written");
    }
    println!("Wrote {} pages to {}.", wanted.len(), folder.display());
}

fn main() {
    let Some(site) = site_given(std::env::args_os().skip(1)) else {
        eprintln!("Usage: cargo run --example explanation_pages -- <the docs site's folder>");
        std::process::exit(2);
    };
    write(&site);
}
