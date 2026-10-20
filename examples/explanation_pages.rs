//! Writes the documentation's generated pages into the docs site's folder:
//! the page for each kind of problem and the page listing them, from the
//! explanations `statac explain` prints, and the page of the starter's
//! files, from their own opening lines: `cargo run --example
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
    format!(
        "---\n# Written from {from} by `cargo run --example explanation_pages`.\n\
         title: {title}\n{summary}{severity}{layout}---\n"
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
    // Only `statac check` reports these two, so only it counts them.
    let counts = match code {
        Code::HookNotRun | Code::LocationPublished => "<code>statac check</code>",
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

/// Where the page of the starter's files goes in the docs site `site`.
pub fn starter_path(site: &Path) -> PathBuf {
    site.join("content/starter.md")
}

/// What is said of a starter file that says nothing of itself, written as
/// a file's opening line is.
fn said_here(path: &str) -> Option<&'static str> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let ending = name.rsplit('.').next().unwrap_or_default();
    Some(match (path, name, ending) {
        (".gitignore", ..) => {
            "The folders git leaves out of the site's history: the built site, and the files \
             Statac keeps between builds"
        }
        ("statac.yaml", ..) => "The site's settings, written from your answers: see /settings/",
        ("content/index.md", ..) => "The home page, where you describe the site: see /new/",
        ("content/about.md", ..) => {
            "A sample page that isn't a post, which explains how to write the next one and how \
             a page joins the menu: see /new/"
        }
        ("content/posts/first-post/index.md", ..) => {
            "The site's first post, which explains how to write the next one"
        }
        ("content/posts/what-a-post-can-hold/index.md", ..) => {
            "A sample post showing everything a page's writing can contain, and how to write \
             each one: see /writing/"
        }
        (_, "example.css", _) => "A file the sample post shows as code",
        ("static/favicon.ico", ..) => {
            "A sample icon, which browsers show in the site's tab. Replace it with your own: \
             see /assets/"
        }
        (_, "background.png", _) => "The picture every card is drawn on",
        (_, "NotoSans-Regular.ttf", _) => "The font used for the card's text",
        (_, "NotoEmoji.ttf", _) => "The font used for the card's emoji",
        (_, "OFL.txt", _) => "The licence Noto Sans is released under",
        (_, "NotoEmoji-OFL.txt", _) => "The licence Noto Emoji is released under",
        (_, "index.md", _) if path.starts_with("content/photos/") => "A sample photograph's page",
        (_, _, "md") if path.starts_with("content/notes/") => "A sample note",
        (_, _, "jpg") => "A sample picture",
        (_, _, "yaml") if name.ends_with(".jpg.yaml") => {
            "The description a reader who can't see the picture is given instead: see /pictures/"
        }
        (_, _, "m4a") => "A sample recording, two seconds long",
        (_, _, "mp4") => "A sample video, two seconds long",
        _ => return None,
    })
}

/// What a starter file's opening says of it: what a snippet takes, where
/// its first line says, and the comment that follows, without its marks.
/// A page's comment is the first line of its front matter.
fn opening(text: &str) -> (Option<&str>, Option<&str>) {
    let mut lines = text.lines().peekable();
    lines.next_if_eq(&"---");
    let takes = lines
        .peek()
        .and_then(|line| line.strip_prefix("{# takes "))
        .and_then(|line| line.strip_suffix("#}"))
        .map(str::trim);
    if takes.is_some() {
        lines.next();
    }
    let line = lines.next().unwrap_or_default();
    let said = if let Some(said) = line.strip_prefix("{#") {
        said.strip_suffix("-#}").or_else(|| said.strip_suffix("#}"))
    } else if let Some(said) = line.strip_prefix("/*") {
        Some(said.strip_suffix("*/").unwrap_or(said))
    } else {
        line.strip_prefix("# ")
    };
    (takes, said.map(str::trim))
}

/// How many bytes at the start of `text`, which opens a word, are to be
/// set as code: a snippet's call, a callout's mark, a template's braces, a
/// command, or a file's name. Markdown or Statac would act on the first
/// three.
fn code_at(text: &str) -> usize {
    let to = |end: &str| text.find(end).map_or(0, |at| at + end.len());
    if text.starts_with("> [!") {
        return to("]");
    }
    if text.starts_with("{{") {
        return to("}}");
    }
    let word = text.find(' ').unwrap_or(text.len());
    if let Some(name) = text.strip_prefix('@') {
        let named = name
            .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .unwrap_or(name.len());
        return match name[named..].starts_with('(') {
            true => to(")"),
            false => 1 + named,
        };
    }
    if let Some(command) = text.strip_prefix("statac ") {
        let named = command
            .find(|c: char| !c.is_ascii_lowercase())
            .unwrap_or(command.len());
        if named > 0 {
            return "statac ".len() + named;
        }
    }
    let name = text[..word].trim_end_matches([',', ':', ';', '.']);
    let a_file = name.contains('/')
        || name
            .rsplit_once('.')
            .is_some_and(|(stem, ending)| !stem.is_empty() && !ending.is_empty());
    if a_file { name.len() } else { 0 }
}

/// A file's words for a table's cell, with everything Markdown or Statac
/// would act on set as code.
fn as_markdown(said: &str) -> String {
    let mut out = String::new();
    let mut rest = said;
    while !rest.is_empty() {
        let code = code_at(rest);
        if code > 0 {
            out.push_str(&format!("`{}`", &rest[..code]));
            rest = &rest[code..];
        }
        let plain = rest.find(' ').map_or(rest.len(), |at| at + 1);
        assert!(
            !rest[..plain].contains(['@', '{', '}', '[', ']', '<', '>', '|', '`', '*', '_']),
            "{said:?} holds something to set as code"
        );
        out.push_str(&rest[..plain]);
        rest = &rest[plain..];
    }
    out
}

/// A file's words apart: what it is, how it is called where they say, and
/// the docs addresses they name.
fn parts(said: &str) -> (&str, Option<&str>, Vec<&str>) {
    let (about, see) = said.rsplit_once(": see ").unwrap_or((said, ""));
    let addresses = see
        .split(' ')
        .filter(|word| word.len() > 1 && word.starts_with('/') && word.ends_with('/'))
        .collect();
    match about.split_once(", called as ") {
        Some((about, called)) => (about, Some(called), addresses),
        None => (about, None, addresses),
    }
}

/// A new site's files and what each says of itself or is said of it here,
/// with the sharing card's where `cards` asks.
fn starter_files(cards: bool) -> Vec<(String, Option<String>, String)> {
    statac::new::files_of_a_new_site(cards)
        .into_iter()
        .map(|(path, bytes)| {
            let text = String::from_utf8(bytes).unwrap_or_default();
            let (takes, said) = opening(&text);
            let said = said
                .or_else(|| said_here(&path))
                .unwrap_or_else(|| panic!("nothing says what {path} is"))
                .to_owned();
            (path, takes.map(str::to_owned), said)
        })
        .collect()
}

/// Every docs address a starter file's opening line names, each once.
pub fn starter_addresses() -> Vec<String> {
    let mut addresses = Vec::new();
    for (_, _, said) in starter_files(true) {
        for address in parts(&said).2 {
            if !addresses.iter().any(|known| known == address) {
                addresses.push(address.to_owned());
            }
        }
    }
    addresses
}

/// The folder a file is listed under: a folder of pages inside `content/`,
/// or else the folder at the top of the site.
pub fn listed_under(path: &str) -> &str {
    let folders: Vec<usize> = path.match_indices('/').map(|(at, _)| at + 1).collect();
    match folders.as_slice() {
        [] => "",
        [_, second, ..] if path.starts_with("content/") => &path[..*second],
        [first, ..] => &path[..*first],
    }
}

/// What a snippet's `takes` line names, in words: a name ending `...`
/// takes any number of values.
fn taken(takes: &str) -> String {
    match takes
        .strip_suffix("...")
        .map(|names| names.rsplit_once(", "))
    {
        Some(Some((before, last))) => format!("`{before}` and any number of `{last}`"),
        Some(None) => format!("any number of `{}`", takes.trim_end_matches('.')),
        None => format!("`{takes}`"),
    }
}

/// The id of the heading a folder's files are listed under: its path with
/// `-` between its folders.
fn folder_id(folder: &str) -> String {
    folder.trim_end_matches('/').replace('/', "-")
}

/// A table of files, each named by its path beneath `folder`, the folder
/// the heading above the table names.
fn table(out: &mut String, folder: &str, files: &[&(String, Option<String>, String)]) {
    out.push_str("| File | What it is | Described in |\n|---|---|---|\n");
    for (path, takes, said) in files {
        let (about, called, addresses) = parts(said);
        let mut about = format!("{}.", as_markdown(about));
        if let Some(takes) = takes {
            about.push_str(&format!(" Takes {}.", taken(takes)));
        }
        if let Some(called) = called {
            about.push_str(&format!(" Called as {}.", as_markdown(called)));
        }
        let links: Vec<String> = addresses
            .iter()
            .map(|address| format!("[{address}]({address})"))
            .collect();
        let name = path.strip_prefix(folder).unwrap_or(path);
        out.push_str(&format!("| `{name}` | {about} | {} |\n", links.join(", ")));
    }
}

/// What a word holds for the search script or a template to put in its
/// place: a number, the words it offers, or the name of a search engine.
const PLACES: [&str; 3] = ["{count}", "{words}", "{engine}"];

/// The page of the starter's files.
pub fn starter_page() -> String {
    let mut out = String::from(
        "---\n# Written from the starter's files by `cargo run --example explanation_pages`.\n\
         title: The starter\n\
         summary: Each file a new site starts with, what it is for, and the words readers see.\n\
         ---\n\
         `statac new` writes these files into a new site, and you can change any of them. \
         If a file has an opening line that describes it, the description below is taken \
         from that line.\n",
    );
    let files = starter_files(false);
    let mut folders: Vec<&str> = vec![""];
    for (path, ..) in &files {
        if !folders.contains(&listed_under(path)) {
            folders.push(listed_under(path));
        }
    }
    for folder in folders {
        match folder {
            "" => out.push_str("\n## The site's folder {#site-folder}\n\n"),
            folder => out.push_str(&format!("\n## `{folder}` {{#{}}}\n\n", folder_id(folder))),
        }
        let inside: Vec<_> = files
            .iter()
            .filter(|(path, ..)| listed_under(path) == folder)
            .collect();
        table(&mut out, folder, &inside);
    }
    out.push_str(
        "\n## What `statac new --cards` adds {#cards}\n\n\
         With `--cards`, `statac new` adds the files that sharing cards are drawn from. \
         This works on a new site or an existing one.\n\n",
    );
    let with_cards = starter_files(true);
    let cards: Vec<_> = with_cards
        .iter()
        .filter(|(path, ..)| !files.iter().any(|(known, ..)| known == path))
        .collect();
    table(&mut out, "", &cards);
    out.push_str(
        "\n## What another language adds {#languages}\n\n\
         For each of the site's languages other than English, `statac new` writes a file \
         named after the language's code.\n\n",
    );
    let (path, text) = statac::new::language_file_of_a_new_site("<code>");
    let (_, said) = opening(&text);
    let said = said.expect("the language file opens by saying what it is");
    table(&mut out, "", &[&(path, None, said.to_owned())]);
    out.push_str(
        "\n## The words {#words}\n\n\
         `data/words.yaml` contains every word the templates and snippets show readers, under \
         each of the site's languages. The words are in English until you translate them. \
         A template uses a word by its name, as in `words.older_posts`.\n\n\
         | Name | In English |\n|---|---|\n",
    );
    for (name, english) in statac::new::words() {
        // A place for the search script or a template to fill is shown as
        // written.
        let plain = PLACES
            .iter()
            .fold(english.to_string(), |text, place| text.replace(place, ""));
        assert!(!plain.contains(['|', '`', '*', '_', '<', '[', '@', '{']));
        let english = (PLACES.iter()).fold(english.to_string(), |text, place| {
            text.replace(place, &format!("`{place}`"))
        });
        out.push_str(&format!("| `{name}` | {english} |\n"));
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
    std::fs::write(starter_path(site), starter_page()).expect("a page can be written");
    println!("Wrote {}.", starter_path(site).display());
}

fn main() {
    let Some(site) = site_given(std::env::args_os().skip(1)) else {
        eprintln!("Usage: cargo run --example explanation_pages -- <the docs site's folder>");
        std::process::exit(2);
    };
    write(&site);
}
