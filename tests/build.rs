//! `statac build` and `statac check` run on small sites in a temporary
//! folder, end to end.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Site {
    root: PathBuf,
}

impl Site {
    /// A site with a home page, a post, a stylesheet and a script, and a
    /// template for each page that prints the build's date.
    fn new(name: &str) -> Site {
        let root = std::env::temp_dir().join(format!("statac-build-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let site = Site { root };
        let template = "<!doctype html>\n\
                        <title>{{ page.title }}</title>\n\
                        {{ css('site.css') }}\n\
                        {{ js('app.js') }}\n\
                        <script>document.documentElement.className = 'js';</script>\n\
                        {{ page.content }}\n\
                        <p>{{ site.today | date }}</p>\n";
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
        );
        site.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        site.write(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\nThe first post.\n",
        );
        site.write("templates/home.html", template);
        site.write("templates/page.html", template);
        site.write("assets/site.css", "body { margin: 0; }\n");
        site.write("assets/app.js", "export const one = 1;\n");
        site
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn out(&self) -> PathBuf {
        self.root.join("_site")
    }

    /// Runs statac with `args` and the site's folder, and gives what it
    /// printed and its exit code.
    fn run(&self, args: &[&str]) -> (String, i32) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(args)
            .arg(&self.root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert!(output.stderr.is_empty(), "{output:?}");
        let said = String::from_utf8(output.stdout).unwrap();
        let shown = self.root.display().to_string();
        // A path below the site is printed with the system's own separator.
        let said = said
            .replace(&shown, "SITE")
            .replace(&format!("SITE{}", std::path::MAIN_SEPARATOR), "SITE/");
        (said, output.status.code().unwrap())
    }

    /// Runs `check`, leaving out the list of JavaScript the site carries.
    fn check_without_javascript(&self, more: &[&str]) -> (String, i32) {
        let args = [&["check", "--today=2026-10-04"], more].concat();
        let (said, code) = self.run(&args);
        let mut kept = String::new();
        let mut listing = false;
        for line in said.split_inclusive('\n') {
            match (listing, line) {
                (_, "JavaScript the site carries:\n") => listing = true,
                (true, line) if line.starts_with("  ") => {}
                (true, "\n") => listing = false,
                _ => {
                    listing = false;
                    kept.push_str(line);
                }
            }
        }
        (kept, code)
    }

    /// Runs statac with `args` in the site's folder, naming no folder, and
    /// gives what it printed and its exit code.
    fn run_in(&self, args: &[&str]) -> (String, i32) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(args)
            .current_dir(&self.root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert!(output.stderr.is_empty(), "{output:?}");
        let said = String::from_utf8(output.stdout).unwrap();
        (said, output.status.code().unwrap())
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Every file under `folder`, by its path with forward slashes, with its bytes.
fn files(folder: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(next) = folders.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let rel = path.strip_prefix(folder).unwrap();
                let rel = rel.to_string_lossy().replace('\\', "/");
                found.push((rel, fs::read(&path).unwrap()));
            }
        }
    }
    found.sort();
    found
}

/// Every file and folder under `folder`, by its path with forward slashes.
fn everything(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(next) = folders.pop() {
        for entry in fs::read_dir(&next).unwrap() {
            let path = entry.unwrap().path();
            let rel = path.strip_prefix(folder).unwrap();
            found.push(rel.to_string_lossy().replace('\\', "/"));
            if path.is_dir() {
                folders.push(path);
            }
        }
    }
    found.sort();
    found
}

fn names(files: &[(String, Vec<u8>)]) -> Vec<&str> {
    files.iter().map(|(name, _)| name.as_str()).collect()
}

/// The last line, with the time it took left out.
fn last_line(said: &str) -> String {
    let line = said.lines().last().unwrap_or_default();
    match line.rsplit_once(" in ") {
        Some((before, _)) => format!("{before} in a moment."),
        None => line.to_owned(),
    }
}

#[test]
fn a_clean_site_is_built_and_says_what_it_wrote() {
    let site = Site::new("clean");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let built = files(&site.out());
    let names = names(&built);
    assert_eq!(names.len(), 6, "{names:?}");
    assert_eq!(names[0], ".statac-built");
    assert!(names[1].starts_with("app.") && names[1].ends_with(".js"));
    assert!(names[2].starts_with("code.") && names[2].ends_with(".css"));
    assert_eq!(names[3], "index.html");
    assert_eq!(names[4], "post-1/index.html");
    assert!(names[5].starts_with("site.") && names[5].ends_with(".css"));

    let post = String::from_utf8(built[4].1.clone()).unwrap();
    assert!(post.contains("<title>Post 1</title>"), "{post}");
    assert!(post.contains("<p>The first post.</p>"), "{post}");
    assert!(post.contains("<p>4 October 2026</p>"), "{post}");
    assert!(post.contains(&format!("href=\"/{}\"", names[5])), "{post}");

    // A build says nothing of the JavaScript; `check` does.
    assert!(!said.contains("JavaScript"), "{said}");
    assert_eq!(
        last_line(&said),
        "Wrote 2 pages and 3 other files to `SITE/_site` in a moment."
    );
    let script = names[1];
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        format!(
            "JavaScript the site carries:\n  \
             `/{script}` from `assets/app.js`, on 2 pages\n  \
             Written in `templates/home.html`, on 1 page\n  \
             Written in `templates/page.html`, on 1 page\n\nNo problems.\n"
        )
    );
}

#[test]
fn a_build_with_an_error_leaves_the_last_site_as_it_was() {
    let site = Site::new("error");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let before = files(&site.out());

    site.write("content/post-2.md", "---\ntitle: Post 2\n---\n");
    site.write(
        "templates/page.html",
        "{{ css('missing.css') }}\n{{ page.content }}\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.starts_with("error  Missing file: there is no `assets/missing.css`\n"),
        "{said}"
    );
    assert!(said.contains("  templates/page.html, line 1\n"), "{said}");
    // Met building two pages, told once with both.
    assert!(
        said.contains(
            "  While building: content/post-1.md\n  And while building 1 more:\n    \
             content/post-2.md\n"
        ),
        "{said}"
    );
    assert!(
        said.ends_with("\nWrote nothing: 1 error stopped the build.\n"),
        "{said}"
    );
    assert!(!said.contains("JavaScript"), "{said}");
    assert_eq!(files(&site.out()), before);
    let mut beside: Vec<_> = fs::read_dir(&site.root)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("statac"))
        .collect();
    beside.sort();
    // The spare the failed build wrote into is kept for the next.
    assert_eq!(beside, [".statac", "statac.yaml"]);
}

#[test]
fn builds_started_together_wait_their_turn_and_publish_what_one_build_does() {
    use std::io::BufRead;
    let alone = Site::new("alone");
    let (said, code) = alone.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let site = Site::new("together");
    // Held here as a build holds it, so both start while another runs.
    let (hold, unheld) = statac::publish::hold(&site.root, || panic!("nothing holds the site"));
    assert!(unheld.is_none());
    let mut builds: Vec<std::process::Child> = (0..2)
        .map(|_| {
            Command::new(env!("CARGO_BIN_EXE_statac"))
                .args(["build", "--today=2026-10-04"])
                .arg(&site.root)
                .env("NO_COLOR", "1")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for build in &mut builds {
        let mut line = String::new();
        let err = build.stderr.as_mut().unwrap();
        std::io::BufReader::new(err).read_line(&mut line).unwrap();
        assert_eq!(line, "Waiting for another build of the site to finish.\n");
    }
    assert!(!site.out().exists());
    let held = std::time::Duration::from_secs(2);
    std::thread::sleep(held);
    drop(hold);
    for build in builds {
        let output = build.wait_with_output().unwrap();
        // The line is said once.
        assert!(output.stderr.is_empty(), "{output:?}");
        let said = String::from_utf8(output.stdout).unwrap();
        assert_eq!(output.status.code(), Some(0), "{said}");
        assert!(last_line(&said).starts_with("Wrote 2 pages"), "{said}");
        assert!(!said.contains("warning"), "{said}");
        // The time said is the build's own, not the wait's.
        let seconds = said.lines().last().unwrap().rsplit_once(" in ").unwrap().1;
        let seconds: f64 = seconds.trim_end_matches(" seconds.").parse().unwrap();
        assert!(seconds < held.as_secs_f64(), "{said}");
    }
    assert_eq!(files(&site.out()), files(&alone.out()));
    // The second build wrote into what the first published.
    assert_eq!(files(&site.root.join(".statac/spare")), files(&site.out()));
}

#[test]
fn check_draws_every_warning_and_writes_nothing() {
    let site = Site::new("check");
    site.write(
        "content/2026-01-01-post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\n---\nThe second post.\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Dates disagree: `date` is `2026-01-02`, the file name `2026-01-01`
  content/2026-01-01-post-2.md, line 3
   3 │ date: 2026-01-02
     │       ^^^^^^^^^^
  Change one so they match; Statac uses the `date` line.
  More: https://statac.dev/e/dates-disagree/

1 warning: 1 dates-disagree.
"
    );
    assert!(!site.out().exists());

    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with(
            "1 warning: 1 dates-disagree. Run `statac check` to see it.\n\n\
             Started `published.txt`"
        ),
        "{said}"
    );
    assert!(site.out().join("post-2/index.html").exists());
}

#[test]
fn a_control_character_in_the_writing_is_left_out_and_said_where_it_was() {
    let site = Site::new("control");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\nThe first\u{b}post.\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Control character: U+000B is left out of the page
  content/post-1.md, line 5
   5 │ The first\u{240b}post.
     │          ^
  Delete it, or type the character you meant.
  More: https://statac.dev/e/control-character/

1 warning: 1 control-character.
"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let page = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    assert!(page.contains("<p>The firstpost.</p>"), "{page}");
}

#[test]
fn a_block_with_a_line_past_five_thousand_characters_is_published_uncoloured_and_told() {
    let site = Site::new("long-line");
    site.write(
        "templates/page.html",
        "<!doctype html>\n{{ css('code.css') }}\n{{ page.content }}\n",
    );
    // A character of two bytes, so the line is measured in characters.
    let post = |letters: usize| {
        let line = format!("let a = \"{}\";", "é".repeat(letters));
        let text = format!("---\ntitle: Post 1\ndate: 2026-01-01\n---\n```js\n{line}\n```\n");
        (line.chars().count(), text)
    };

    let (characters, text) = post(4_989);
    assert_eq!(characters, 5_000);
    site.write("content/post-1.md", &text);
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, "No problems.\n");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let page = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    assert!(
        page.contains("<pre class=\"code\"><code class=\"language-js\"><span class=\"hl-"),
        "{page}"
    );

    let (characters, text) = post(4_990);
    assert_eq!(characters, 5_001);
    site.write("content/post-1.md", &text);
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Code not coloured: a line in this block is 5,001 characters long
  content/post-1.md, line 5
   5 │ ```js
     │    ^^
  Break the line up, or take the language off the block's first line.
  More: https://statac.dev/e/code-not-coloured/

1 warning: 1 code-not-coloured.
"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with("1 warning: 1 code-not-coloured. Run `statac check` to see it.\n"),
        "{said}"
    );
    let page = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    let code = format!(
        "<pre class=\"code\"><code class=\"language-js\">let a = &quot;{}&quot;;</code></pre>",
        "é".repeat(4_990)
    );
    assert!(page.contains(&code), "{page}");
}

#[test]
fn coloured_code_whose_colours_no_template_asks_for_is_told_once_at_the_first_block() {
    let site = Site::new("colours");
    let post = |n: u32, writing: &str| {
        let text = format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\n---\n{writing}\n");
        site.write(&format!("content/post-{n}.md"), &text);
    };
    post(1, "```text\nas it is\n```\n\n```yaml\ntitle: Post 1\n```");
    post(2, "```js\nlet a = 1;\n```");
    post(3, "```rust\nfn main() {}\n```");
    post(4, "```\nlet a = 1;\n```");
    let told = "\
warning  Code colours not loaded: no template asks for `code.css`
  content/post-1.md, line 9
   9 │ ```yaml
     │    ^^^^
  Add `{{ css(\"code.css\") }}` inside a template's `<head>`.
  More: https://statac.dev/e/code-colours-not-loaded/

1 warning: 1 code-colours-not-loaded.
";
    let checked = || {
        let (said, code) = site.check_without_javascript(&[]);
        assert_eq!(code, 0, "{said}");
        said
    };
    assert_eq!(checked(), told);
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with("1 warning: 1 code-colours-not-loaded. Run `statac check` to see it.\n"),
        "{said}"
    );
    let page = fs::read_to_string(site.out().join("post-2/index.html")).unwrap();
    assert!(page.contains("<span class=\"hl-"), "{page}");

    // Asked for by a page that shows no code, and by `asset` as by `css`.
    let home = "<!doctype html>\n<title>{{ page.title }}</title>\n{{ page.content }}\n";
    site.write(
        "templates/home.html",
        &format!("{home}{{{{ css('code.css') }}}}\n"),
    );
    assert_eq!(checked(), "No problems.\n");
    let linked = "<link rel=\"stylesheet\" href=\"{{ asset('code.css') }}\">\n";
    site.write("templates/home.html", &format!("{home}{linked}"));
    assert_eq!(checked(), "No problems.\n");

    // The site's own stylesheet of that name is asked for or not as Statac's is.
    site.write("assets/code.css", ".code { color: black; }\n");
    assert_eq!(checked(), "No problems.\n");
    site.write("templates/home.html", home);
    assert_eq!(checked(), told);

    // A block left uncoloured for a long line is no coloured code.
    for n in 1..=3 {
        post(n, &format!("```js\n{}\n```", "a".repeat(5_001)));
    }
    let said = checked();
    assert!(
        said.ends_with("\n3 warnings: 3 code-not-coloured.\n"),
        "{said}"
    );
}

#[test]
fn a_card_description_is_checked_and_its_files_are_no_templates() {
    let site = Site::new("card");
    site.write(
        "templates/og.yaml",
        "background: background.png\nfont: text.ttf\nshadow: true\n",
    );
    site.write("templates/og/text.ttf", "ttf");
    fs::write(
        site.root.join("templates/og/background.png"),
        [0x89, 0x50, 0xFF],
    )
    .unwrap();
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Field set aside: a card has no `shadow`
  templates/og.yaml, line 3
   3 │ shadow: true
     │ ^^^^^^
  A card reads `background`, `font` and `padding`, \
and `text`, `place`, `size` and `colour` for each text.
  More: https://statac.dev/e/set-aside/

1 warning: 1 set-aside.
"
    );

    site.write("templates/og.yaml", "background: background.png\n");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert!(said.starts_with("error  Missing field: `font`\n"), "{said}");
    assert!(!site.out().exists());

    // With the description gone, its files are still no templates.
    fs::remove_file(site.root.join("templates/og.yaml")).unwrap();
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!((said.as_str(), code), ("No problems.\n", 0));
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(site.out().join("index.html").is_file());
    assert!(!site.out().join("og").exists());
}

#[test]
fn quietened_kinds_are_counted_and_not_drawn() {
    let site = Site::new("quiet");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\nquiet: [dates-disagree]\n",
    );
    site.write(
        "content/2026-01-01-post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\n---\nThe second post.\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!((said.as_str(), code), ("1 warning, quietened.\n", 0));
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with("1 warning, quietened.\n\nStarted `published.txt`"),
        "{said}"
    );
}

#[test]
fn check_shows_every_problem_as_json_with_where_it_starts_and_ends_and_nothing_else() {
    let site = Site::new("json");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\nquiet: [dates-disagree]\n",
    );
    site.write(
        "content/2026-01-01-post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\n---\nThe second post.\n",
    );
    site.write(
        "content/ete.md",
        "---\ntitle: Été\n---\nUn café, ![](https://a.example/b.jpg), [x](/nope/).\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["check", "--json", "--today=2026-10-04"])
        .arg(&site.root)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let said = String::from_utf8(output.stdout).unwrap();
    let root = site.root.display().to_string().replace('\\', "\\\\");
    // A column counts characters and an offset bytes, so `é` is one of
    // the first and two of the second. A quietened problem is there,
    // marked.
    assert_eq!(
        said,
        format!(
            "{{\"statac\":\"{}\",\"format\":1,\"site\":\"{root}\",\"problems\":[\
             {{\"severity\":\"warning\",\"kind\":\"dates-disagree\",\
             \"message\":\"Dates disagree: `date` is `2026-01-02`, the file name `2026-01-01`\",\
             \"file\":\"content/2026-01-01-post-2.md\",\
             \"start\":{{\"line\":3,\"column\":7,\"offset\":24}},\
             \"end\":{{\"line\":3,\"column\":17,\"offset\":34}},\
             \"suggestion\":\"Change one so they match; Statac uses the `date` line.\",\
             \"explanation\":\"https://statac.dev/e/dates-disagree/\",\"quietened\":true,\
             \"related\":[],\"pages\":[]}},\
             {{\"severity\":\"note\",\"kind\":\"undescribed-picture\",\
             \"message\":\"Picture with no description: `https://a.example/b.jpg`\",\
             \"file\":\"content/ete.md\",\
             \"start\":{{\"line\":4,\"column\":14,\"offset\":35}},\
             \"end\":{{\"line\":4,\"column\":37,\"offset\":58}},\
             \"suggestion\":\"Describe it in the brackets.\",\
             \"explanation\":\"https://statac.dev/e/undescribed-picture/\",\"quietened\":false,\
             \"related\":[],\"pages\":[]}},\
             {{\"severity\":\"warning\",\"kind\":\"broken-link\",\
             \"message\":\"Broken link: nothing is published at `/nope/`\",\
             \"file\":\"content/ete.md\",\
             \"start\":{{\"line\":4,\"column\":44,\"offset\":65}},\
             \"end\":{{\"line\":4,\"column\":50,\"offset\":71}},\
             \"suggestion\":null,\"explanation\":\"https://statac.dev/e/broken-link/\",\
             \"quietened\":false,\"related\":[],\"pages\":[]}}]}}\n",
            env!("CARGO_PKG_VERSION")
        )
    );
}

/// What `build --timings` said after the build's own lines: each
/// section's heading and rows, every time and share left out, as they
/// differ from run to run.
fn timings(said: &str) -> Vec<(String, Vec<String>)> {
    let from = said.find("\n\nWhere the time went:").expect(said);
    let is_time = |cell: &str| {
        let number = cell.strip_suffix(" ms").or_else(|| cell.strip_suffix(" s"));
        number.is_some_and(|n| n.parse::<f64>().is_ok())
    };
    let sections = said[from + 2..].trim_end().split("\n\n");
    let sections = sections.map(|section| {
        let mut lines = section.lines();
        let heading = untimed(lines.next().unwrap());
        let rows = lines.map(|line| {
            let cells = line.split("  ").map(str::trim);
            let cells = cells.filter(|cell| !cell.is_empty() && !cell.ends_with('%'));
            cells
                .filter(|cell| !is_time(cell))
                .collect::<Vec<_>>()
                .join(" | ")
        });
        (heading, rows.collect())
    });
    sections.collect()
}

/// `line` with each number, and the unit of each time, as `#`.
fn untimed(line: &str) -> String {
    let mut words = line.split(' ').peekable();
    let mut out = Vec::new();
    while let Some(word) = words.next() {
        if word.parse::<f64>().is_err() {
            out.push(word.to_owned());
            continue;
        }
        let unit = words
            .peek()
            .map(|next| next.trim_end_matches([',', ':', ')']));
        match unit {
            Some("s" | "ms") => {
                let unit = words.next().unwrap();
                out.push(format!("#{}", unit.trim_start_matches(['m', 's'])));
            }
            _ => out.push("#".into()),
        }
    }
    out.join(" ")
}

#[test]
fn build_shows_where_its_time_went_when_asked() {
    let site = Site::new("timings");
    site.write("snippets/byline.html", "<p>{{ page.title }}</p>\n");
    site.write(
        "templates/page.html",
        "<!doctype html>\n<title>{{ page.title }}</title>\n{{ byline() }}\n{{ page.content }}\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("Where the time went"), "{said}");
    let output = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["build", "--timings", "--today=2026-10-04"])
        .arg(&site.root)
        .env("NO_COLOR", "1")
        .env("RAYON_NUM_THREADS", "2")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let said = String::from_utf8(output.stdout).unwrap();
    assert!(
        said.contains(" seconds.\n\nWhere the time went:\n"),
        "{said}"
    );
    let mut sections = timings(&said);
    // Only the stages keep an order that doesn't turn on the times.
    for (_, rows) in &mut sections[2..] {
        rows.sort();
    }
    let rows = |rows: &[&str]| rows.iter().map(|row| row.to_string()).collect();
    assert_eq!(
        sections,
        [
            (
                "Where the time went:".into(),
                rows(&[
                    "Reading the site",
                    "Building the pages",
                    "Concluding",
                    "Writing the rest"
                ])
            ),
            (
                "Building the pages was # of work across # threads:".into(),
                rows(&[
                    "Saving files",
                    "Markdown",
                    "Templates",
                    "Checking links",
                    "The rest"
                ])
            ),
            (
                "Slowest templates, by time in the template itself:".into(),
                rows(&[
                    "snippets/byline.html | 1 run",
                    "templates/home.html | 1 run",
                    "templates/page.html | 1 run"
                ])
            ),
            (
                "Slowest writing:".into(),
                rows(&["content/index.md", "content/post-1.md"])
            ),
            (
                "Slowest pages (median #, 95th percentile #):".into(),
                rows(&["/ | templates/home.html", "/post-1/ | templates/page.html"])
            ),
        ]
    );
    assert!(said.contains("work across 2 threads:"), "{said}");
}

#[test]
fn a_fixed_day_builds_the_same_site_twice() {
    let site = Site::new("same");
    let out = |name: &str| site.root.join(name).display().to_string();
    for name in ["one", "two"] {
        let (said, code) = site.run(&[
            "build",
            "--today=2026-10-04",
            &format!("--out={}", out(name)),
        ]);
        assert_eq!(code, 0, "{said}");
    }
    let one = files(&site.root.join("one"));
    assert!(!one.is_empty());
    assert_eq!(one, files(&site.root.join("two")));

    let (said, code) = site.run(&[
        "build",
        "--today=2026-10-05",
        &format!("--out={}", out("two")),
    ]);
    assert_eq!(code, 0, "{said}");
    assert_ne!(one, files(&site.root.join("two")));
}

/// A site whose pages, feed, sitemap, robots page, redirect and sharing
/// card each write a whole address, with `settings` as its `statac.yaml`.
fn site_writing_whole_addresses(name: &str, settings: &str) -> Site {
    let site = Site::new(name);
    site.write("statac.yaml", settings);
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\nredirect_from: [/old/post-1/]\n---\n\
         The first post.\n",
    );
    for (page, layout) in [("sitemap.md", "sitemap.xml"), ("robots.md", "robots.txt")] {
        site.write(
            &format!("content/{page}"),
            &format!("---\npermalink: /{layout}\nlayout: {layout}\nneeds: url\n---\n"),
        );
    }
    let layout = "<link rel=\"canonical\" href=\"{{ page.url | absolute }}\">\n\
                  <meta property=\"og:url\" content=\"{{ page.url | absolute }}\">\n\
                  <meta property=\"og:image\" content=\"{{ page.card.url | absolute }}\">\n\
                  <a href=\"{{ site.url }}/\">{{ site.host }}</a>\n\
                  <a href=\"{{ url('/post-1/') }}\">Post 1</a>\n\
                  {{ page.content }}\n";
    site.write("templates/home.html", layout);
    site.write("templates/page.html", layout);
    site.write(
        "templates/redirect.html",
        "<link rel=\"canonical\" href=\"{{ to | absolute }}\">\n",
    );
    site.write(
        "templates/feed.xml",
        "<link rel=\"self\" href=\"{{ list.feed | absolute }}\"/>\n\
         {% for post in posts %}<link href=\"{{ post.url | absolute }}\"/>\n{% endfor %}",
    );
    site.write(
        "templates/sitemap.xml",
        "{% for entry in site.pages if entry.url is endingwith('/') %}\
         <loc>{{ entry.url | absolute }}</loc>\n{% endfor %}",
    );
    site.write(
        "templates/robots.txt",
        "{% for entry in site.pages if entry.url is endingwith('/sitemap.xml') %}\
         Sitemap: {{ entry.url | absolute }}\n{% endfor %}",
    );
    site.write(
        "templates/og.yaml",
        "background: background.png\nfont: text.ttf\n\n\
         title.text: \"{{ site.host }}\"\ntitle.place: centre\ntitle.size: up to 75\n\
         title.colour: \"#F2E9D3\"\n",
    );
    fs::create_dir_all(site.root.join("templates/og")).unwrap();
    fs::write(
        site.root.join("templates/og/text.ttf"),
        include_bytes!("../src/cards/fonts/test/test-latin.ttf"),
    )
    .unwrap();
    let mut background = std::io::Cursor::new(Vec::new());
    image::RgbImage::from_pixel(1200, 630, image::Rgb([20, 30, 40]))
        .write_to(&mut background, image::ImageFormat::Png)
        .unwrap();
    fs::write(
        site.root.join("templates/og/background.png"),
        background.into_inner(),
    )
    .unwrap();
    site
}

/// What each file of [`site_writing_whole_addresses`] holds of the site's
/// address, `{at}` standing for the address and `{under}` for its path.
const WHOLE_ADDRESSES: &[(&str, &str)] = &[
    ("index.html", "<link rel=\"canonical\" href=\"{at}/\">"),
    (
        "post-1/index.html",
        "<link rel=\"canonical\" href=\"{at}/post-1/\">",
    ),
    (
        "post-1/index.html",
        "<meta property=\"og:url\" content=\"{at}/post-1/\">",
    ),
    (
        "post-1/index.html",
        "<meta property=\"og:image\" content=\"{at}/post-1/og.",
    ),
    ("post-1/index.html", "<a href=\"{at}/\">{host}</a>"),
    (
        "post-1/index.html",
        "<a href=\"{under}/post-1/\">Post 1</a>",
    ),
    (
        "old/post-1/index.html",
        "<link rel=\"canonical\" href=\"{at}/post-1/\">",
    ),
    ("feed.xml", "<link rel=\"self\" href=\"{at}/feed.xml\"/>"),
    ("feed.xml", "<link href=\"{at}/post-1/\"/>"),
    ("sitemap.xml", "<loc>{at}/</loc>"),
    ("sitemap.xml", "<loc>{at}/post-1/</loc>"),
    ("robots.txt", "Sitemap: {at}/sitemap.xml"),
];

/// A template that only a page left out until the site has an address is
/// built with reads the address freely: it is never built without one.
#[test]
fn a_template_only_a_page_needing_the_address_uses_is_checked_as_with_one() {
    let site = Site::new("needs-url-template");
    site.write(
        "content/robots.md",
        "---\npermalink: /robots.txt\nlayout: robots.txt\nneeds: url\n---\n",
    );
    let robots = "Sitemap: {{ site.url }}/sitemap.xml on {{ site.host }}\n";
    site.write("templates/robots.txt", robots);
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("`content/robots.md` is left out"), "{said}");
    assert!(!said.contains("site.url"), "{said}");

    let (said, code) = site.run(&["build", "--url=https://example.org"]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        fs::read_to_string(site.root.join("_site/robots.txt")).unwrap(),
        "Sitemap: https://example.org/sitemap.xml on example.org\n"
    );

    // Anything else in it is checked as in any template.
    site.write(
        "templates/robots.txt",
        &format!("{robots}{{{{ site.titel }}}}\n"),
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert!(said.contains("Unknown field: `site.titel`"), "{said}");
    assert!(!said.contains("site.url"), "{said}");

    // A page that is built with it has no address to show.
    site.write("templates/robots.txt", robots);
    site.write(
        "content/other.md",
        "---\ntitle: Other\npermalink: /other.txt\nlayout: robots.txt\n---\n",
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("No value: nothing is set for `site.url`"),
        "{said}"
    );
}

#[test]
fn every_whole_address_follows_the_address_given_for_the_run() {
    let with_url = "title: Site\nurl: https://example.org\nlanguage: en\ntimezone: UTC\n\
                    feed_key: key\n";
    let without_url = "title: Site\nlanguage: en\ntimezone: UTC\nfeed_key: key\n";
    // The settings, the address given, and the address, its path and its
    // host as the built site writes them.
    let runs: &[(&str, Option<&str>, [&str; 3])] = &[
        (with_url, None, ["https://example.org", "", "example.org"]),
        (
            with_url,
            Some("https://preview.example.org/"),
            ["https://preview.example.org", "", "preview.example.org"],
        ),
        (
            with_url,
            Some("https://example.org/preview/42/"),
            [
                "https://example.org/preview/42",
                "/preview/42",
                "example.org",
            ],
        ),
        (
            without_url,
            Some("http://localhost:8080/preview"),
            ["http://localhost:8080/preview", "/preview", "localhost"],
        ),
    ];
    let site = site_writing_whole_addresses("whole-addresses", with_url);
    for (settings, given, [at, under, host]) in runs {
        site.write("statac.yaml", settings);
        let option = given.map(|address| format!("--url={address}"));
        let mut args = vec!["build", "--today=2026-10-04"];
        args.extend(option.as_deref());
        let (said, code) = site.run(&args);
        assert_eq!(code, 0, "{given:?}: {said}");
        assert_eq!(
            last_line(&said),
            "Wrote 4 pages, 1 redirect, 1 feed and 5 other files to `SITE/_site` in a moment.",
            "{given:?}"
        );
        for (file, holds) in WHOLE_ADDRESSES {
            let text = fs::read_to_string(site.out().join(file)).unwrap();
            let holds = holds
                .replace("{at}", at)
                .replace("{under}", under)
                .replace("{host}", host);
            assert!(text.contains(&holds), "{given:?}: {file}: {text}");
        }
    }

    // The site with no address is one only for the run that gives it.
    let (said, _) = site.check_without_javascript(&[]);
    for waiting in [
        "the feed of the site's own list is",
        "`content/robots.md` is",
        "`content/sitemap.md` is",
    ] {
        let note =
            format!("note  No site address: {waiting} left out until the site has a `url`\n");
        assert!(said.contains(&note), "{said}");
    }
    let (said, code) = site.check_without_javascript(&["--url=https://preview.example.org"]);
    assert_eq!((said.as_str(), code), ("No problems.\n", 0));
}

#[test]
fn an_old_address_sends_readers_on() {
    let site = Site::new("redirect");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\nredirect_from: [/old/post-1/]\n---\nThe first post.\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.starts_with(
            "error  Missing template: there is no `templates/redirect.html`\n  \
             content/post-1.md, line 3\n"
        ),
        "{said}"
    );
    assert!(!site.out().exists());

    site.write(
        "templates/redirect.html",
        "<meta http-equiv=\"refresh\" content=\"0; url={{ to }}\"><title>{{ title }}</title>\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        fs::read_to_string(site.out().join("old/post-1/index.html")).unwrap(),
        "<meta http-equiv=\"refresh\" content=\"0; url=/post-1/\"><title>Post 1</title>\n"
    );
    assert_eq!(
        last_line(&said),
        "Wrote 2 pages, 1 redirect and 3 other files to `SITE/_site` in a moment."
    );
}

#[test]
fn a_page_sends_readers_on_from_its_own_address() {
    let site = Site::new("redirect-to");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\nurl: https://example.org/\npages.feed: false\n",
    );
    site.write(
        "content/2019-05-01-post-2.md",
        "---\ntitle: Post 2\ntags: [Tag 1]\nredirect_to: /μ/#part-1\n---\nNever shown.\n",
    );
    site.write(
        "content/post-3.md",
        "---\ntitle: Post 3\nredirect_to: https://example.org/elsewhere/\n---\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\npermalink: /μ/\n---\nThe first post.\n",
    );
    site.write(
        "templates/redirect.html",
        "{{ title }} {{ to }} {{ page.title }}\n",
    );
    site.write(
        "templates/home.html",
        "{% for page in site.pages %}{{ page.url }} {% endfor %}\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let read = |path: &str| fs::read_to_string(site.out().join(path)).unwrap();
    assert_eq!(
        read("2019-05-01-post-2/index.html"),
        "Post 2 https://example.org/%CE%BC/#part-1 Post 2\n"
    );
    assert_eq!(
        read("post-3/index.html"),
        "Post 3 https://example.org/elsewhere/ Post 3\n"
    );
    assert_eq!(read("index.html"), "/ /μ/ ");
    assert!(!site.out().join("tags").exists());
    assert_eq!(
        last_line(&said),
        "Wrote 2 pages, 2 redirects and 3 other files to `SITE/_site` in a moment."
    );
}

#[test]
fn a_redirect_to_a_redirect_sends_readers_where_following_ends() {
    let site = Site::new("redirect-chain");
    site.write("content/post-2.md", "---\nredirect_to: /post-3/\n---\n");
    site.write(
        "content/post-3.md",
        "---\nredirect_to: /post-4/#part-3\n---\n",
    );
    site.write(
        "content/post-4.md",
        "---\ntitle: Post 4\nredirect_from: [/old/]\n---\n",
    );
    site.write("content/post-5.md", "---\nredirect_to: /old/#part-5\n---\n");
    site.write("content/post-6.md", "---\nredirect_to: /post-2/\n---\n");
    site.write("templates/redirect.html", "{{ title }} {{ to }}\n");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let read = |path: &str| fs::read_to_string(site.out().join(path)).unwrap();
    assert_eq!(read("post-6/index.html"), "Post 4 /post-4/#part-3\n");
    assert_eq!(read("post-2/index.html"), "Post 4 /post-4/#part-3\n");
    assert_eq!(read("post-3/index.html"), "Post 4 /post-4/#part-3\n");
    assert_eq!(read("post-5/index.html"), "Post 4 /post-4/#part-5\n");
    assert_eq!(read("old/index.html"), "Post 4 /post-4/\n");
}

#[test]
fn a_redirect_is_titled_by_what_the_page_it_sends_to_is_called() {
    let site = Site::new("redirect-title");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\nposts_per_page: 1\n",
    );
    site.write(
        "content/index.md",
        "---\nredirect_from: [/old/]\n---\nThe home page.\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\ntags: [Rust]\n---\n",
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\ntags: [Rust]\n---\n",
    );
    for (name, front) in [
        ("tag", "redirect_to: /tags/rust/"),
        ("later", "redirect_to: /page/2/"),
        ("titled", "title: Moved\nredirect_to: /post-1/"),
        ("elsewhere", "redirect_to: https://example.org/"),
    ] {
        site.write(
            &format!("content/{name}.md"),
            &format!("---\n{front}\n---\n"),
        );
    }
    site.write("templates/redirect.html", "{{ title }}\n");
    site.write("templates/home.html", "{{ headline(page) }}\n");
    site.write("templates/folder.html", "{{ headline(page) }}\n");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let read = |path: &str| fs::read_to_string(site.out().join(path)).unwrap();
    assert_eq!(read("old/index.html"), "Site\n");
    assert_eq!(read("tag/index.html"), "Rust\n");
    assert_eq!(read("later/index.html"), "Site\n");
    assert_eq!(read("titled/index.html"), "Moved\n");
    assert_eq!(read("elsewhere/index.html"), "https://example.org/\n");
}

#[test]
fn a_mistake_many_pages_meet_is_told_once_with_the_pages() {
    let site = Site::new("met-by-many");
    for number in 2..=8 {
        site.write(
            &format!("content/post-{number}.md"),
            &format!("---\ntitle: Post {number}\n---\n"),
        );
    }
    site.write(
        "templates/page.html",
        "{{ css('missing.css') }}\n{{ page.content }}\n",
    );
    let pages = |from: usize, to: usize| -> String {
        (from..=to)
            .map(|number| format!("    content/post-{number}.md\n"))
            .collect()
    };

    // `build` names the first few and counts the rest.
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(said.matches("error  Missing file").count(), 1, "{said}");
    let few = format!(
        "  While building: content/post-1.md\n  And while building 7 more:\n{}    \
         and 2 more. Run `statac check` to see them.\n",
        pages(2, 6)
    );
    assert!(said.contains(&few), "{said}");
    assert!(
        said.ends_with("\nWrote nothing: 1 error stopped the build.\n"),
        "{said}"
    );

    // `check` names every one.
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(said.matches("error  Missing file").count(), 1, "{said}");
    let every = format!(
        "  While building: content/post-1.md\n  And while building 7 more:\n{}  More:",
        pages(2, 8)
    );
    assert!(said.contains(&every), "{said}");
    assert!(said.contains("1 error: 1 missing-file.\n"), "{said}");
}

#[test]
fn a_stylesheet_sass_stopped_on_is_told_once_and_its_link_is_not_broken() {
    let site = Site::new("sass-stopped");
    fs::remove_file(site.root.join("assets/site.css")).unwrap();
    site.write("assets/site.scss", "a { color: $nowhere; }\n");
    site.write(
        "templates/page.html",
        "{{ css('site.css') }}\n<link rel=\"preload\" href=\"{{ asset('site.css') }}\">\n\
         {{ page.content }}\n",
    );

    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(said.matches("error  Sass stopped").count(), 1, "{said}");
    assert!(!said.contains("Broken link"), "{said}");
    assert!(said.contains("1 error: 1 sass-stopped.\n"), "{said}");
}

#[test]
fn a_value_read_as_markdown_on_many_pages_is_told_at_its_place_once() {
    let site = Site::new("markdown-value");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\ntags: [Rust]\nsummary: Area $\\fracc{1}{2}$\n---\n",
    );
    let summary = "{% for post in site.posts %}{{ post.summary | markdown }}{% endfor %}\n";
    site.write("templates/home.html", summary);
    site.write("templates/folder.html", summary);
    site.write("templates/page.html", "{{ page.summary | markdown }}\n");
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        said,
        "\
error  Malformed maths: Statac doesn't know `\\fracc`
  content/post-1.md, line 5
   5 │ summary: Area $\\fracc{1}{2}$
     │                ^^^^^^
  Read as Markdown here: templates/home.html, line 1
   1 │ {% for post in site.posts %}{{ post.summary | markdown }}{% endfor %}
     │                                ^^^^^^^^^^^^^^^^^^^^^^^
  While building: content/index.md
  More: https://statac.dev/e/malformed-maths/

1 error: 1 malformed-maths.
"
    );
}

/// HTML left open, or closing what it never opened, is published exactly
/// as written and told at its tag; a value shown on many pages is told at
/// its place once.
#[test]
fn html_left_unmatched_is_published_as_written_and_told_at_its_tag() {
    let site = Site::new("unmatched-tag");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\nsummary: A summary <span>in words\n---\n\
         <div class=\"box\">\n\nA note in a box.\n\n</section>\n",
    );
    let summaries = "{{ page.content }}\
                     {% for post in site.posts %}{{ post.summary | markdown }}{% endfor %}\n";
    site.write("templates/home.html", summaries);
    site.write("templates/folder.html", summaries);
    site.write(
        "templates/page.html",
        "{{ page.summary | markdown }}\n{{ page.content }}\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Unmatched tag: `<span>` is never closed
  content/post-1.md, line 4
   4 │ summary: A summary <span>in words
     │                    ^^^^^^
  Read as Markdown here: templates/home.html, line 1
   1 │ {{ page.content }}{% for post in site.posts %}{{ post.summary | markdown }}{% endfor %}
     │                                                  ^^^^^^^^^^^^^^^^^^^^^^^
  While building: content/index.md
  Close it with `</span>` where it should end.
  More: https://statac.dev/e/unmatched-tag/

warning  Unmatched tag: `<div>` is never closed
  content/post-1.md, line 6
   6 │ <div class=\"box\">
     │ ^^^^^^^^^^^^^^^^^
  Close it with `</div>` where it should end.
  More: https://statac.dev/e/unmatched-tag/

warning  Unmatched tag: `</section>` closes nothing opened before it
  content/post-1.md, line 10
  10 │ </section>
     │ ^^^^^^^^^^
  Take it out, or open the element above it.
  More: https://statac.dev/e/unmatched-tag/

3 warnings: 3 unmatched-tag.
"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("3 warnings: 3 unmatched-tag."), "{said}");
    let page = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    assert_eq!(
        page,
        "A summary <span>in words\n<div class=\"box\">\n<p>A note in a box.</p>\n</section>\n"
    );
    let home = fs::read_to_string(site.out().join("index.html")).unwrap();
    assert!(
        home.contains("<p>The home page.</p>A summary <span>in words"),
        "{home}"
    );
}

/// A more marker inside an element the writing's HTML holds open over
/// several blocks would cut the excerpt inside it, so it stops.
#[test]
fn a_more_marker_inside_an_element_left_open_stops_at_the_marker() {
    let site = Site::new("more-inside");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n\
         <div class=\"box\">\n\nOne.\n\n<!--more-->\n\nTwo.\n\n</div>\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        said,
        "\
error  Wrong place: `<!--more-->` can't end an excerpt inside `<div>`
  content/post-1.md, line 9
   9 │ <!--more-->
     │ ^^^^^^^^^^^
  Move it outside the `<div>`.
  More: https://statac.dev/e/wrong-place/

1 error: 1 wrong-place.
"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert!(!site.out().exists());
}

#[test]
fn a_layout_that_never_shows_its_pages_writing_is_told_once_counting_them() {
    let site = Site::new("hollow");
    site.write("templates/hollow.html", "<title>{{ page.title }}</title>\n");
    for n in 2..=4 {
        site.write(
            &format!("content/post-{n}.md"),
            &format!("---\ntitle: Post {n}\nlayout: hollow\n---\nPost {n} says this.\n"),
        );
    }
    site.write(
        "content/post-5.md",
        "---\ntitle: Post 5\nlayout: hollow\n---\n<!-- To come. -->\n",
    );
    site.write(
        "content/post-6.md",
        "---\ntitle: Post 6\nlayout: hollow\ndraft: true\n---\nPost 6 says this.\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(
            "\
warning  Writing never shown: 3 pages built by `templates/hollow.html` have writing it never shows
  templates/hollow.html, line 1
   1 │ <title>{{ page.title }}</title>
     │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  While building: content/post-2.md
  And while building 2 more:
    content/post-3.md
    content/post-4.md
  Add `{{ page.content }}` where the writing belongs.
  More: https://statac.dev/e/writing-never-shown/
"
        ),
        "{said}"
    );
    assert!(
        said.contains("1 warning: 1 writing-never-shown.\n"),
        "{said}"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with("1 warning: 1 writing-never-shown.\n"),
        "{said}"
    );
    let post = fs::read_to_string(site.out().join("post-2/index.html")).unwrap();
    assert_eq!(post, "<title>Post 2</title>\n");

    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n\
         quiet: [writing-never-shown]\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("Writing never shown"), "{said}");
    assert!(said.contains("1 warning, quietened.\n"), "{said}");
}

#[test]
fn words_a_language_lacks_are_noted_once_for_the_language_counting_its_pages() {
    let site = Site::new("missing-words");
    let settings = "title: Site\nlanguage: fr\ntimezone: UTC\npages.feed: false\n";
    site.write("statac.yaml", settings);
    site.write("templates/page.html", "{{ page.content }}\n");
    site.write(
        "snippets/markdown/callout.html",
        "<aside><p>{{ title }}</p>{{ body }}</aside>\n",
    );
    for n in 1..=3 {
        site.write(
            &format!("content/post-{n}.md"),
            &format!("---\ntitle: Post {n}\n---\nPost {n} has a note.[^1]\n\n[^1]: A note.\n"),
        );
    }
    site.write(
        "content/post-4.md",
        "---\ntitle: Post 4\n---\n> [!NOTE]\n> Post 4 has a callout.\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
note  Missing words: the footnotes of 3 pages in `fr` have no words for the way back
  While building: content/post-1.md
  And while building 2 more:
    content/post-2.md
    content/post-3.md
  Add them to `languages/fr.yaml`, \
translating the English line `back_to_the_text: back to the text`.
  More: https://statac.dev/e/missing-words/

note  Missing words: the callouts of 1 page in `fr` have no names
  While building: content/post-4.md
  Add them to `languages/fr.yaml`, \
translating the English line `callouts: [Note, Tip, Important, Warning, Caution]`.
  More: https://statac.dev/e/missing-words/

2 notes: 2 missing-words.
"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with("2 notes: 2 missing-words. Run `statac check` to see them.\n"),
        "{said}"
    );

    // The site's own file adds the words to Statac's French.
    site.write(
        "languages/fr.yaml",
        "back_to_the_text: retour au texte\n\
         callouts: [Note, Astuce, Important, Avertissement, Attention]\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!((said.as_str(), code), ("No problems.\n", 0));
    let (_, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0);
    let post = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    assert!(post.contains("aria-label=\"retour au texte\""), "{post}");

    fs::remove_file(site.root.join("languages/fr.yaml")).unwrap();
    site.write(
        "statac.yaml",
        &format!("{settings}quiet: [missing-words]\n"),
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, "2 notes, all quietened.\n");
}

#[test]
fn a_page_called_by_what_the_author_never_wrote_as_a_name_is_noted_once() {
    let site = Site::new("untitled-page");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\nurl: https://example.org/\n\
         feed_key: 0b6d0c4e-2f7a-4d51-9a8e-3c1f5e7a9b20\n",
    );
    site.write(
        "content/post-2.md",
        "---\ndate: 2026-01-02\n---\nPost 2 opens here.\n",
    );
    site.write(
        "templates/page.html",
        "{% set shown = page.content %}<title>{{ headline(page) }}</title>\n",
    );
    site.write(
        "templates/feed.xml",
        "{% for post in posts %}<title>{{ headline(post) }}</title>{% endfor %}\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
note  Untitled page: `content/post-2.md` is called by its opening words, `Post 2 opens here.`
  content/post-2.md, line 1
   1 │ ---
     │ ^^^
  Give it a `title`, or open its writing with a `# heading`. \
A folder whose pages go without titles says `pages.untitled: true`.
  More: https://statac.dev/e/untitled-page/

1 note: 1 untitled-page.
"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with("1 note: 1 untitled-page. Run `statac check` to see it.\n"),
        "{said}"
    );
    let built = fs::read_to_string(site.out().join("post-2/index.html")).unwrap();
    assert_eq!(built, "<title>Post 2 opens here.</title>\n");
}

#[test]
fn a_folder_whose_pages_go_untitled_is_called_by_its_dates_and_never_noted() {
    let site = Site::new("untitled-folder");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\nurl: https://example.org/\n\
         feed_key: 0b6d0c4e-2f7a-4d51-9a8e-3c1f5e7a9b20\n",
    );
    site.write("content/notes/index.md", "---\npages.untitled: true\n---\n");
    site.write(
        "content/notes/note-1.md",
        "---\ndate: 2026-01-02\n---\nNote 1 opens here.\n",
    );
    let named = "{% set shown = page.content %}<title>{{ headline(page) }}</title>\n\
                 <meta property=\"og:title\" content=\"{{ page.og_title }}\">\n";
    site.write("templates/page.html", named);
    site.write("templates/folder.html", named);
    site.write(
        "templates/feed.xml",
        "{% for post in posts %}<title>{{ headline(post) }}</title>{% endfor %}\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, "No problems.\n");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("untitled-page"), "{said}");
    let built = fs::read_to_string(site.out().join("notes/note-1/index.html")).unwrap();
    assert_eq!(
        built,
        "<title>2 January 2026</title>\n<meta property=\"og:title\" content=\"2 January 2026\">\n"
    );
    let feed = fs::read_to_string(site.out().join("feed.xml")).unwrap();
    assert_eq!(feed, "<title>2 January 2026</title><title>Post 1</title>");

    // One page of the folder says it isn't meant to go untitled.
    site.write(
        "content/notes/note-1.md",
        "---\ndate: 2026-01-02\nuntitled: false\n---\nNote 1 opens here.\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
note  Untitled page: `content/notes/note-1.md` is called by its opening words, `Note 1 opens here.`
  content/notes/note-1.md, line 1
   1 │ ---
     │ ^^^
  Give it a `title`, or open its writing with a `# heading`. \
A folder whose pages go without titles says `pages.untitled: true`.
  More: https://statac.dev/e/untitled-page/

1 note: 1 untitled-page.
"
    );

    // A value that is neither stops the build.
    site.write(
        "content/notes/index.md",
        "---\npages.untitled: perhaps\n---\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.starts_with(
            "error  Wrong kind of value: `untitled` is `true` or `false`, not `perhaps`\n"
        ),
        "{said}"
    );
}

#[test]
fn a_long_list_goes_on_onto_later_pages() {
    let site = Site::new("paging");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\nposts_per_page: 1\n",
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: 2026-02-01\n---\nThe second post.\n",
    );
    site.write(
        "templates/home.html",
        "{% for post in page.posts %}{{ post.title }}{% endfor %}|\
         {% if page.next %}{{ page.next.url }}{% endif %}\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let read = |path: &str| fs::read_to_string(site.out().join(path)).unwrap();
    assert_eq!(read("index.html"), "Post 2|/page/2/");
    assert_eq!(read("page/2/index.html"), "Post 1|");
    assert_eq!(
        last_line(&said),
        "Wrote 4 pages and 3 other files to `SITE/_site` in a moment."
    );

    site.write("content/page/2.md", "---\ntitle: Page 2\n---\n");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains(
            "error  Same address: page 2 of `/` and `content/page/2.md` are both published \
             at `/page/2/`\n  content/page/2.md, line 1\n"
        ),
        "{said}"
    );

    // A mistake met on every page of the list is told once.
    fs::remove_file(site.root.join("content/page/2.md")).unwrap();
    site.write("content/index.md", "---\ntitle: Home\nsubtitle:\n---\n");
    site.write("templates/home.html", "{{ page.subtitle }}\n");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(said.matches("error  No value").count(), 1, "{said}");
    assert!(
        said.contains("  While building: content/index.md\n"),
        "{said}"
    );

    // One met only on a later page is told there.
    site.write(
        "templates/home.html",
        "{% if page.number > 1 %}{{ page.subtitle }}{% endif %}\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(said.matches("error  No value").count(), 1, "{said}");
    assert!(
        said.contains("  While building page 2 of `/`: content/index.md\n"),
        "{said}"
    );
}

#[test]
fn javascript_is_found_in_every_file_published_whatever_its_folder() {
    /// A name for the site, its files, and a line `check` must say.
    type Case<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a str);
    let cases: &[Case] = &[
        (
            "static-html",
            &[("static/demo.html", "<p>Demo</p><script>run()</script>\n")],
            "  Written in `static/demo.html`, on 1 page\n",
        ),
        (
            "content-html",
            &[("content/embed.html", "<img src=a.png onerror=\"run()\">\n")],
            "  Written in `content/embed.html`, on 1 page\n",
        ),
        (
            "static-xhtml",
            &[(
                "static/demo.xhtml",
                "<html xmlns=\"http://www.w3.org/1999/xhtml\"><a href=\"javascript:run()\">a</a></html>\n",
            )],
            "  Written in `static/demo.xhtml`, on 1 page\n",
        ),
        (
            "static-htm",
            &[("static/demo.htm", "<script>run()</script>\n")],
            "  Written in `static/demo.htm`, on 1 page\n",
        ),
        (
            "static-shtml",
            &[("static/demo.shtml", "<script>run()</script>\n")],
            "  Written in `static/demo.shtml`, on 1 page\n",
        ),
        (
            "static-no-extension",
            &[("static/demo", "<p>Demo</p><script>run()</script>\n")],
            "  Written in `static/demo`, on 1 page\n",
        ),
        (
            "content-text",
            &[("content/notes.txt", "<img src=x onerror=run()>\n")],
            "  Written in `content/notes.txt`, on 1 page\n",
        ),
        (
            "static-svg",
            &[("static/logo.svg", "<svg onload=\"run()\"></svg>\n")],
            "  Written in `static/logo.svg`, on 1 page\n",
        ),
        (
            "assets-svg",
            &[("assets/icon.svg", "<svg><script>run()</script></svg>\n")],
            "  Written in `assets/icon.svg`, on 1 page\n",
        ),
        (
            "content-js",
            &[("content/extra.js", "run();\n")],
            "  `/extra.js` from `content/extra.js`, on no page\n",
        ),
        (
            "static-js",
            &[
                ("static/vendor.mjs", "export {};\n"),
                (
                    "static/demo.html",
                    "<script type=module src=\"/vendor.mjs\"></script>\n",
                ),
            ],
            "  `/vendor.mjs` from `static/vendor.mjs`, on 1 page\n",
        ),
    ];
    for (name, files, wanted) in cases {
        let site = Site::new(name);
        for (path, text) in *files {
            site.write(path, text);
        }
        let (said, code) = site.run(&["check", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{name}: {said}");
        assert!(said.contains(wanted), "{name}: {said}");
    }
}

/// A file a host sends as a picture, a recording, a font or a PDF is never
/// read as a document, so what it holds is never taken for script.
#[test]
fn javascript_is_never_looked_for_in_a_file_no_browser_reads_as_a_document() {
    let site = Site::new("never-documents");
    let paths = [
        "static/photo-1.jpg",
        "static/photo-2.PNG",
        "static/recording-1.mp3",
        "static/font-1.woff2",
        "static/file-1.pdf",
    ];
    for path in paths {
        site.write(path, "<p>Demo</p><script>run()</script>\n");
    }
    site.write("static/demo.txt", "<p>Demo</p><script>run()</script>\n");
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("  Written in `static/demo.txt`"), "{said}");
    for name in ["photo-1", "photo-2", "recording-1", "font-1", "file-1"] {
        assert!(
            !said.contains(&format!("`static/{name}.")),
            "{name}: {said}"
        );
    }
}

#[test]
fn javascript_a_snippet_called_by_name_writes_is_not_listed_and_the_rest_is() {
    const PLAIN: &str = "<!doctype html>\n<title>{{ page.title }}</title>\n{{ page.content }}\n";
    const CALLING: &str = "@play()\n";
    const PLAY: &str = "<button onclick=\"play()\">Play</button>\n<script>ready()</script>\n";
    const LOADING: &str = "<script src=\"https://example.org/play.js\"></script>\n\
                           <iframe src=\"https://example.org/player\"></iframe>\n";
    /// Where script is written, the files that write it over a site with
    /// none, whose one post is `post-1.md`, and every line `check` lists.
    type Case<'a> = (&'a str, &'a [(&'a str, &'a str)], &'a [&'a str]);
    let cases: &[Case] = &[
        (
            "a snippet the writing calls",
            &[("snippets/play.html", PLAY), ("content/post-1.md", CALLING)],
            &[],
        ),
        (
            "a snippet a template calls",
            &[
                ("snippets/play.html", PLAY),
                ("templates/page.html", "{{ play() }}\n"),
            ],
            &[],
        ),
        (
            "a snippet a called snippet calls",
            &[
                ("snippets/play.html", PLAY),
                ("snippets/player.html", "<div>{{ play() }}</div>\n"),
                ("content/post-1.md", "@player()\n"),
            ],
            &[],
        ),
        (
            "a layout",
            &[("templates/page.html", "<script>run()</script>\n")],
            &["Written in `templates/page.html`, on 1 page"],
        ),
        (
            "a page's own HTML",
            &[("content/post-1.md", "<script>run()</script>\n")],
            &["Written in `content/post-1.md`, on 1 page"],
        ),
        (
            "a piece of the writing replaced",
            &[
                (
                    "snippets/markdown/heading.html",
                    "<h{{ level }} onclick=\"run()\">{{ body }}</h{{ level }}>\n",
                ),
                ("content/post-1.md", "## Heading 1\n"),
            ],
            &["Written in `snippets/markdown/heading.html`, on 1 page"],
        ),
        (
            "a called snippet and a layout on one page",
            &[
                ("snippets/play.html", PLAY),
                ("content/post-1.md", CALLING),
                (
                    "templates/page.html",
                    "{{ page.content }}<script>run()</script>\n",
                ),
            ],
            &["Written in `templates/page.html`, on 1 page"],
        ),
        (
            "a called snippet and the page calling it",
            &[
                ("snippets/play.html", PLAY),
                ("content/post-1.md", "@play()\n\n<script>run()</script>\n"),
            ],
            &["Written in `content/post-1.md`, on 1 page"],
        ),
        (
            "a called snippet, and a layout where a browser may not run it",
            &[
                ("snippets/play.html", PLAY),
                ("content/post-1.md", CALLING),
                (
                    "templates/page.html",
                    "{{ page.content }}<!-- <script>run()</script> -->\n",
                ),
            ],
            &["Written in `templates/page.html`, on 1 page, where a browser may not run it"],
        ),
        (
            "a called snippet loading and embedding from another site",
            &[
                ("snippets/play.html", LOADING),
                ("content/post-1.md", CALLING),
            ],
            &[],
        ),
        (
            "a called snippet and a layout loading and embedding the same",
            &[
                ("snippets/play.html", LOADING),
                ("content/post-1.md", CALLING),
                ("templates/page.html", LOADING),
            ],
            &[
                "`https://example.org/play.js`, on 1 page",
                "`https://example.org/player` embedded, which may run its own, on 1 page",
            ],
        ),
        (
            "a called snippet embedding what it is handed",
            &[
                (
                    "snippets/play.html",
                    "<iframe src=\"{{ address }}\"></iframe>\n",
                ),
                (
                    "content/post-1.md",
                    "@play(\"https://example.org/player\")\n",
                ),
            ],
            &["`https://example.org/player` embedded, which may run its own, on 1 page"],
        ),
        (
            "a called snippet on two pages and their layout",
            &[
                ("snippets/play.html", PLAY),
                ("content/post-1.md", CALLING),
                (
                    "content/post-2.md",
                    "---\ntitle: Post 2\ndate: 2026-01-02\n---\n@play()\n",
                ),
                (
                    "templates/page.html",
                    "{{ page.content }}<script>run()</script>\n",
                ),
            ],
            &["Written in `templates/page.html`, on 2 pages"],
        ),
        (
            "a called snippet and a layout writing the same to the letter",
            &[
                ("snippets/play.html", PLAY),
                ("content/post-1.md", CALLING),
                (
                    "templates/page.html",
                    "{{ page.content }}<script>ready()</script>\n",
                ),
            ],
            &["Written in `templates/page.html`, on 1 page"],
        ),
        (
            "the writing a called snippet wraps",
            &[
                (
                    "snippets/note.html",
                    "<aside>{{ body }}</aside>\n<script>ready()</script>\n",
                ),
                (
                    "content/post-1.md",
                    "@note()\n<script>run()</script>\n@end\n",
                ),
            ],
            &["Written in `content/post-1.md`, on 1 page"],
        ),
        (
            "a value a called snippet is handed",
            &[
                (
                    "snippets/play.html",
                    "<script>ready()</script>{{ text | safe }}\n",
                ),
                (
                    "content/post-1.md",
                    "@play(text=\"<script>run()</script>\")\n",
                ),
            ],
            &["Written in `content/post-1.md`, on 1 page"],
        ),
        (
            "a value a called snippet reads from a data file",
            &[
                (
                    "snippets/play.html",
                    "<script>ready()</script>{{ site.data.extra.shown | safe }}\n",
                ),
                ("data/extra.yaml", "shown: <script>run()</script>\n"),
                ("content/post-1.md", CALLING),
            ],
            &["Written in `content/post-1.md`, on 1 page"],
        ),
        (
            "a called snippet calling one that fills its script in",
            &[
                ("snippets/play.html", "{{ player(id=1) }}{{ id }}\n"),
                ("snippets/player.html", "<script>play({{ id }})</script>\n"),
                ("content/post-1.md", "@play(\"1\")\n"),
            ],
            &["Written in `snippets/player.html`, on 1 page"],
        ),
        (
            "a called snippet named as a piece of the writing is",
            &[
                ("snippets/heading.html", PLAY),
                (
                    "snippets/markdown/heading.html",
                    "<h{{ level }} onclick=\"run()\">{{ body }}</h{{ level }}>\n",
                ),
                ("content/post-1.md", "## Heading 1\n\n@heading()\n"),
            ],
            &["Written in `snippets/markdown/heading.html`, on 1 page"],
        ),
        (
            "a called snippet named as the layout is",
            &[
                ("snippets/page.html", PLAY),
                ("content/post-1.md", "@page()\n"),
                (
                    "templates/page.html",
                    "{{ page.content }}<script>run()</script>\n",
                ),
            ],
            &["Written in `templates/page.html`, on 1 page"],
        ),
        (
            "a field shown as text, beside a layout's own script",
            &[
                (
                    "content/post-2.md",
                    "---\ntitle: Post 2\nsummary: \"<script>shown()</script>\"\n---\nText.\n",
                ),
                (
                    "templates/page.html",
                    "{{ page.summary | default(\"\") }}{{ page.content }}<script>run()</script>\n",
                ),
            ],
            &["Written in `templates/page.html`, on 2 pages"],
        ),
        (
            "a field shown as markup, beside a layout's own script",
            &[
                (
                    "content/post-2.md",
                    "---\ntitle: Post 2\nsummary: \"<script>shown()</script>\"\n---\nText.\n",
                ),
                (
                    "templates/page.html",
                    "{{ page.summary | default(\"\") | safe }}{{ page.content }}<script>run()</script>\n",
                ),
            ],
            &[
                "Written in `content/post-2.md`, on 1 page",
                "Written in `templates/page.html`, on 2 pages",
            ],
        ),
        (
            "a field shown as markup that YAML spells another way, beside a layout's own script",
            &[
                (
                    "content/post-2.md",
                    "---\ntitle: Post 2\nsummary: \"<script>shown(\\\"1\\\")</script>\"\n---\nText.\n",
                ),
                (
                    "templates/page.html",
                    "{{ page.summary | default(\"\") | safe }}{{ page.content }}<script>run()</script>\n",
                ),
            ],
            &[
                "Written in `content/post-2.md`, on 1 page",
                "Written in `templates/page.html`, on 2 pages",
            ],
        ),
        (
            "a layout's script in a branch not taken, beside the writing's own",
            &[
                ("content/post-1.md", "<script>run()</script>\n"),
                (
                    "templates/page.html",
                    "{{ page.content }}{% if page.draft %}<script>draft()</script>{% endif %}\n",
                ),
            ],
            &["Written in `content/post-1.md`, on 1 page"],
        ),
        (
            "a called snippet writing a script, and the page loading one",
            &[
                ("snippets/play.html", PLAY),
                (
                    "content/post-1.md",
                    "@play()\n\n<script src=\"https://example.org/run.js\"></script>\n",
                ),
            ],
            &["`https://example.org/run.js`, on 1 page"],
        ),
        (
            "a called snippet loading from the address it is handed",
            &[
                (
                    "snippets/play.html",
                    "<script>ready()</script><script src=\"{{ address }}\"></script>\n",
                ),
                (
                    "content/post-1.md",
                    "@play(\"https://example.org/play.js\")\n",
                ),
            ],
            &["`https://example.org/play.js`, on 1 page"],
        ),
        (
            "a called snippet loading a script the site publishes",
            &[
                ("snippets/play.html", "<script src=\"/play.js\"></script>\n"),
                ("static/play.js", "play();\n"),
                ("content/post-1.md", CALLING),
            ],
            &["`/play.js` from `static/play.js`, on 1 page"],
        ),
    ];
    for (at, (name, files, listed)) in cases.iter().enumerate() {
        let site = Site::new(&format!("javascript-where-{at}"));
        fs::remove_file(site.root.join("assets/app.js")).unwrap();
        site.write("templates/home.html", PLAIN);
        site.write("templates/page.html", PLAIN);
        for (path, text) in *files {
            let text = match *path {
                "content/post-1.md" => format!("---\ntitle: Post 1\ndate: 2026-01-01\n---\n{text}"),
                "templates/page.html" if !text.contains("page.content") => {
                    format!("{PLAIN}{text}")
                }
                _ => (*text).to_owned(),
            };
            site.write(path, &text);
        }
        let (said, code) = site.run(&["check", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{name}: {said}");
        let wanted: String = match listed {
            [] => String::new(),
            lines => {
                let lines: String = lines.iter().map(|line| format!("  {line}\n")).collect();
                format!("JavaScript the site carries:\n{lines}\n")
            }
        };
        assert_eq!(said, format!("{wanted}No problems.\n"), "{name}");
    }
}

#[test]
fn a_page_showing_a_script_as_code_is_not_told_as_writing_one() {
    let site = Site::new("javascript-shown-as-code");
    fs::remove_file(site.root.join("assets/app.js")).unwrap();
    fs::remove_file(site.root.join("templates/home.html")).unwrap();
    site.write(
        "content/index.md",
        "---\ntitle: Home\nlayout: page\n---\nThe home page.\n",
    );
    site.write(
        "templates/page.html",
        "<!doctype html>\n<title>{{ page.title }}</title>\n<script>run()</script>\n\
         {{ page.content }}\n",
    );
    // Shown as code a block and a span of it, escaped, and in a link's
    // words; none of it reaches the page as markup.
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n\
         A page runs `<script>ready()</script>`, or `<a onclick=\"ready()\">`.\n\n\
         ```\n<script>ready()</script>\n<script src=\"https://example.org/play.js\"></script>\n\
         ```\n\n    \
         <script>ready()</script>\n\n\
         Written \\<script>ready()\\</script> too, and [`<script>`](https://example.org/).\n",
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\n---\n\
         It shows `<script>ready()</script>`.\n\n<script>play()</script>\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "JavaScript the site carries:\n  \
         Written in `content/post-2.md`, on 1 page\n  \
         Written in `templates/page.html`, on 3 pages\n\nNo problems.\n"
    );
}

#[test]
fn a_script_in_one_post_shown_on_another_page_is_blamed_on_the_post() {
    let site = Site::new("shown");
    let template = "<title>{{ page.title }}</title>\n{{ page.content }}\n";
    site.write("templates/page.html", template);
    site.write(
        "templates/home.html",
        "{% for post in site.posts %}{{ post.content }}{% endfor %}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n<script>run()</script>\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains("  Written in `content/post-1.md`, on 2 pages\n"),
        "{said}"
    );
    assert!(!said.contains("Written in `templates/"), "{said}");
}

#[test]
fn a_script_in_a_field_shown_on_a_page_statac_makes_is_blamed_on_its_file() {
    let site = Site::new("made-shown");
    site.write("templates/page.html", "<title>{{ page.title }}</title>\n");
    site.write(
        "templates/folder.html",
        "{% for post in page.posts %}{{ post.summary | safe }}{% endfor %}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\ntags: [Rust]\nsummary: One\n---\n",
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\ntags: [Rust]\nsummary: <script>run()</script>\n\
         ---\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains("  Written in `content/post-2.md`, on 1 page\n"),
        "{said}"
    );
    assert!(!said.contains("Written in `content/post-1.md`"), "{said}");
}

#[test]
fn drafts_and_waiting_posts_are_counted_by_build_and_drawn_by_check() {
    let site = Site::new("drafts");
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndraft: true\n---\nA draft.\n",
    );
    site.write(
        "content/post-3.md",
        "---\ntitle: Post 3\ndate: 2026-10-05\n---\nTomorrow's.\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with(
            "2 notes: 1 draft-left-out, 1 waiting-for-its-day. Run `statac check` to see them.\n"
        ),
        "{said}"
    );
    assert!(said.contains("Wrote 2 pages"), "{said}");
    assert!(!site.out().join("post-2").exists());
    assert!(!site.out().join("post-3").exists());

    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains("note  Draft left out: `content/post-2.md`\n  content/post-2.md, line 3\n"),
        "{said}"
    );
    assert!(
        said.contains(
            "note  Waiting for its day: `content/post-3.md` is dated `2026-10-05`\n  \
             content/post-3.md, line 3\n"
        ),
        "{said}"
    );

    let (said, code) = site.run(&["build", "--today=2026-10-04", "--drafts"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with("Included 1 draft and 1 post waiting for its day as if published.\n\n"),
        "{said}"
    );
    assert!(said.contains("Wrote 4 pages"), "{said}");
    assert!(site.out().join("post-2/index.html").exists());

    // A draft is never quietened.
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\nquiet: [draft-left-out]\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("Can't quieten: `draft-left-out` leaves a page out"),
        "{said}"
    );
}

#[test]
fn the_first_call_built_inside_html_is_said_once_for_the_site() {
    let site = Site::new("call-in-html");
    site.write("snippets/book.html", "<cite>{{ title }}</cite>");
    site.write(
        "content/index.md",
        "---\ntitle: Home\n---\n<p>@book('Post 1')</p>\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n<p>@book('Post 2') @book('Post 3')</p>\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
note  Call in HTML: `@book` is built as a snippet
  content/index.md, line 4
   4 │ <p>@book('Post 1')</p>
     │    ^^^^^^^^^^^^^^^
  To show it as text, write `\\@book`.
  More: https://statac.dev/e/call-in-html/

1 note: 1 call-in-html.
"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with("1 note: 1 call-in-html. Run `statac check` to see it.\n"),
        "{said}"
    );
    let built = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    assert!(
        built.contains("<p><cite>Post 2</cite> <cite>Post 3</cite></p>"),
        "{built}"
    );
}

#[test]
fn a_call_written_as_text_in_a_value_reads_its_references_as_the_rest_does() {
    let site = Site::new("call-as-text");
    site.write(
        "templates/page.html",
        "{{ page.content }}\n{{ page.blurb | markdown }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n\
         blurb: 'A &amp; B @nope(a &amp; b &lt;c&gt; &mdash; &#0;)'\n---\nText.\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let built = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    assert!(
        built.contains("A &amp; B @nope(a &amp; b &lt;c&gt; \u{2014} \u{fffd})"),
        "{built}"
    );
}

#[test]
fn a_name_the_site_has_no_snippet_for_in_html_is_published_as_written() {
    let site = Site::new("no-call-in-html");
    site.write(
        "templates/page.html",
        "{{ page.content }}\n{{ page.blurb | markdown }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n\
         blurb: '<div title=\"@nope(&quot;a&quot;)\">@nope(a &amp; b)</div>'\n---\n\
         <div class=\"x\">Mail @nope(a &amp; b) and @nope(<em>c</em>) \
         <a title=\"@nope(&quot;d&quot;)\">here</a></div>\n",
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let built = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    assert!(
        built.contains(
            "<div class=\"x\">Mail @nope(a &amp; b) and @nope(<em>c</em>) \
             <a title=\"@nope(&quot;d&quot;)\">here</a></div>"
        ),
        "{built}"
    );
    assert!(
        built.contains("<div title=\"@nope(&quot;a&quot;)\">@nope(a &amp; b)</div>"),
        "{built}"
    );
}

#[test]
fn every_link_on_the_built_site_is_followed_and_told_where_it_was_written() {
    let site = Site::new("links");
    site.write(
        "templates/page.html",
        "<title>{{ page.title }}</title>\n\
         <nav><a href=\"/about-us/\">About</a> <a href=\"#top\">Top</a></nav>\n\
         {{ page.content }}\n",
    );
    site.write(
        "templates/home.html",
        "<title>{{ page.title }}</title>\n\
         {% for post in site.posts %}{{ post.content }}{% endfor %}\n\
         {{ page.content }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n\
         Read [post 2](/post-2/), [the notes](#notes) and [post 3](/post-3/#a-phot).\n\n\
         ## Notes {#notes}\n\nSee [the list][list] and [the top](#top).\n\n[list]: /posts/\n",
    );
    site.write(
        "content/post-3/index.md",
        "---\ntitle: Post 3\ndate: 2026-01-03\n---\n## A photo\n\n![A photo](photo-1.jpg)\n",
    );
    site.write("content/post-3/photo-1.jpg", "");
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    for told in [
        "warning  Broken link: nothing is published at `/post-2/`\n  content/post-1.md, line 5\n",
        "warning  Missing anchor: nothing on `/post-3/` has the id `a-phot`\n  \
         content/post-1.md, line 5\n",
        "  Did you mean `#a-photo`?\n",
        "warning  Broken link: nothing is published at `/posts/`\n  content/post-1.md, line 11\n",
        "warning  Broken link: nothing is published at `/about-us/`\n  templates/page.html, line 2\n",
        "4 warnings: 3 broken-link, 1 missing-anchor.\n",
    ] {
        assert!(said.contains(told), "{told}\nin\n{said}");
    }
    // The template's link is told once, though both posts have it.
    assert_eq!(said.matches("`/about-us/`").count(), 1, "{said}");
}

/// A build keeps every address of the author's it publishes in
/// `published.txt`, and one kept that nothing answers at is told there.
/// A run published elsewhere, or with the drafts in, keeps nothing.
#[test]
fn an_address_published_before_that_stops_answering_is_told() {
    let site = Site::new("published");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(
            "Started `published.txt` with the 2 addresses this site publishes, so a later \
             build can say when one stops answering. Commit it with the site.\n"
        ),
        "{said}"
    );
    let kept = || fs::read_to_string(site.root.join("published.txt")).unwrap();
    assert!(kept().ends_with("\n/\n/post-1/\n"), "{}", kept());
    fs::rename(
        site.root.join("content/post-1.md"),
        site.root.join("content/post-one.md"),
    )
    .unwrap();
    for args in [
        &["build", "--url=https://preview.example.org"][..],
        &["build", "--drafts"],
    ] {
        let (said, code) = site.run(args);
        assert_eq!(code, 0, "{said}");
        assert!(!kept().contains("post-one"), "{args:?}: {}", kept());
    }
    let (said, _) = site.check_without_javascript(&[]);
    assert!(
        said.contains(
            "warning  Address gone: `/post-1/` was published before, and nothing answers there \
             now\n  published.txt, line 5\n"
        ),
        "{said}"
    );
    let (said, _) = site.run(&["build", "--today=2026-10-04"]);
    assert!(
        said.contains("Added 1 address to `published.txt`.\n"),
        "{said}"
    );
}

/// A template's link is offered the nearest address the site publishes,
/// even where that is the page being built when it is first met.
#[test]
fn a_templates_broken_link_is_offered_the_page_it_is_first_met_on() {
    let site = Site::new("links-template-nearest");
    site.write(
        "templates/about.html",
        "<a href=\"{{ url('/abot/') }}\">About</a>\n{{ page.content }}\n",
    );
    site.write(
        "content/about.md",
        "---\ntitle: About\nlayout: about\n---\nAbout the site.\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let told = "warning  Broken link: nothing is published at `/abot/`\n";
    assert!(said.contains(told), "{said}");
    assert!(
        said.contains("  Did you mean `{{ url(\"/about/\") }}`?\n"),
        "{said}"
    );
}

/// Writing shown on another page keeps what its addresses mean: one read
/// from the writing's own page is spelled from the top of the site there,
/// in a feed and at an old address too, and on its own page it is as
/// written.
#[test]
fn writing_shown_on_another_page_links_where_it_does_on_its_own() {
    let site = Site::new("links-shown");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\nurl: https://example.org/blog/\n\
         feed_key: 9ec6e8ce-e7dd-8bb6-85ff-b19920a0e8b1\n",
    );
    site.write(
        "templates/home.html",
        "<title>{{ page.title }}</title>\n\
         {% for post in site.posts %}{{ post.excerpt }}{% endfor %}\n\
         {{ page.content }}\n",
    );
    site.write(
        "templates/feed.xml",
        "<feed>{% for post in posts %}<content>{{ post.content | absolute }}</content>\
         {% endfor %}</feed>\n",
    );
    site.write(
        "content/post-2/index.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\nredirect_from: [/old/]\n---\n\
         ![A photo](photo-1.jpg) and [the notes](#notes).\n\n## Notes {#notes}\n",
    );
    site.write("content/post-2/photo-1.jpg", "");
    site.write("templates/redirect.html", "{{ page.excerpt }}\n");
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, "No problems.\n");
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let read = |path: &str| fs::read_to_string(site.out().join(path)).unwrap();
    assert!(
        read("index.html").contains(
            "<p><img src=\"/blog/post-2/photo-1.jpg\" alt=\"A photo\" loading=\"lazy\" \
             decoding=\"async\"> and <a href=\"/blog/post-2/#notes\">the notes</a>.</p>"
        ),
        "{}",
        read("index.html")
    );
    assert!(
        read("post-2/index.html").contains(
            "<p><img src=\"photo-1.jpg\" alt=\"A photo\" loading=\"lazy\" \
             decoding=\"async\"> and <a href=\"#notes\">the notes</a>.</p>"
        ),
        "{}",
        read("post-2/index.html")
    );
    assert!(
        read("old/index.html").contains("<a href=\"/blog/post-2/#notes\">"),
        "{}",
        read("old/index.html")
    );
    let feed = read("feed.xml");
    for whole in [
        "src=&quot;https://example.org/blog/post-2/photo-1.jpg&quot;",
        "href=&quot;https://example.org/blog/post-2/#notes&quot;",
    ] {
        assert!(feed.contains(whole), "{whole}\nin\n{feed}");
    }
}

#[test]
fn a_site_under_a_path_owns_only_that_path() {
    let site = Site::new("links-path");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n\
         url: https://example.org/blog/\nnot_ours: [/video/]\n",
    );
    site.write(
        "templates/home.html",
        "<title>{{ page.title }}</title>\n{{ page.content }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n\
         [Home](/blog/), [this post](https://example.org/blog/post-1/), \
         [a talk](/video/talk/), [elsewhere](https://other.example/) and [home again](/).\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with(
            "warning  Broken link: nothing is published at `/`\n  content/post-1.md, line 5\n"
        ),
        "{said}"
    );
    assert!(
        said.contains("  To link inside the site, write `@url(/)`.\n"),
        "{said}"
    );
    assert!(said.ends_with("1 warning: 1 broken-link.\n"), "{said}");
}

/// The id of a heading in a script with marks is made with the marks in
/// the order typed; a link with them in another order looks the same and
/// lands on nothing. One spelled as the heading is offered the id made
/// from its words.
#[test]
fn an_anchor_is_held_to_the_id_with_its_marks_as_typed() {
    let site = Site::new("links-anchors");
    let typed = "\u{5e9}\u{5c1}\u{5bc}";
    let swapped = "\u{5e9}\u{5bc}\u{5c1}";
    site.write(
        "content/post-1.md",
        &format!(
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\n\
             ## {typed}\n\n## Getting There\n\n\
             [As typed](#{typed}), [swapped](#{swapped}) and [there](#Getting-There).\n"
        ),
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(said.matches("Missing anchor").count(), 2, "{said}");
    assert!(
        said.contains(&format!(
            "warning  Missing anchor: nothing on this page has the id `{swapped}`\n"
        )),
        "{said}"
    );
    assert!(
        said.contains(&format!("  Did you mean `#{typed}`?\n")),
        "{said}"
    );
    assert!(
        said.contains("  Did you mean `#getting-there`?\n"),
        "{said}"
    );
}

const IN_THE_WAY: &str = "  Build into a folder Statac doesn't read, such as `--out=_site`.
  More: https://statac.dev/e/output-in-the-way/

Wrote nothing: 1 error stopped the build.
";

/// A site with one of every folder it is read from, and a `before_build`
/// that leaves a file where it runs.
fn read_from_everywhere(name: &str) -> Site {
    let site = Site::new(name);
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n\
         before_build: echo ran> ran.txt\n",
    );
    for folder in ["snippets", "static", "data", "languages", "fetched"] {
        fs::create_dir_all(site.root.join(folder)).unwrap();
    }
    site
}

#[test]
fn an_output_folder_the_site_is_read_from_is_refused_before_anything_runs() {
    let site = read_from_everywhere("out-read-from");
    let before = everything(&site.root);
    for folder in [
        "content",
        "templates",
        "snippets",
        "assets",
        "static",
        "data",
        "languages",
        "fetched",
    ] {
        let (said, code) = site.run_in(&["build", &format!("--out={folder}")]);
        assert_eq!(code, 1, "{said}");
        assert_eq!(
            said,
            format!(
                "error  Output folder in the way: `{folder}` is the site's `{folder}/` folder\n\
                 {IN_THE_WAY}"
            )
        );
        let (said, code) = site.run_in(&["build", &format!("--out={folder}/built")]);
        assert_eq!(code, 1, "{said}");
        assert_eq!(
            said,
            format!(
                "error  Output folder in the way: `{folder}/built` is inside `{folder}/`\n\
                 {IN_THE_WAY}"
            )
        );
    }
    assert_eq!(everything(&site.root), before);
}

#[test]
fn an_output_folder_reached_through_dots_is_refused_where_it_lands() {
    let site = read_from_everywhere("out-dots");
    let before = everything(&site.root);
    let (said, code) = site.run_in(&["build", "--out=public/../content/./built"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        said,
        format!(
            "error  Output folder in the way: `public/../content/./built` is inside \
             `content/`\n{IN_THE_WAY}"
        )
    );
    let whole = site.root.join("content").join("built");
    let (said, code) = site.run(&["build", &format!("--out={}", whole.display())]);
    assert_eq!(code, 1, "{said}");
    assert!(said.contains("` is inside `content/`\n"), "{said}");
    assert_eq!(everything(&site.root), before);
}

#[test]
fn an_output_folder_in_other_capitals_is_refused_where_the_disk_holds_them_alike() {
    let site = read_from_everywhere("out-capitals");
    site.write("probe", "");
    let alike = site.root.join("PROBE").exists();
    fs::remove_file(site.root.join("probe")).unwrap();
    let (said, code) = site.run_in(&["build", "--out=Content/built"]);
    match alike {
        true => {
            assert_eq!(code, 1, "{said}");
            assert_eq!(
                said,
                format!(
                    "error  Output folder in the way: `Content/built` is inside `content/`\n\
                     {IN_THE_WAY}"
                )
            );
        }
        false => {
            assert_eq!(code, 0, "{said}");
            assert!(site.root.join("Content/built/.statac-built").is_file());
        }
    }
}

#[test]
fn an_output_folder_elsewhere_in_the_site_is_built_into() {
    let site = read_from_everywhere("out-elsewhere");
    for out in ["_site", "public", ".statac/built"] {
        let (said, code) = site.run_in(&["build", &format!("--out={out}")]);
        assert_eq!(code, 0, "{said}");
        assert!(said.contains("Ran `before_build`.\n"), "{said}");
        assert!(
            last_line(&said).ends_with(&format!(" to `{out}` in a moment.")),
            "{said}"
        );
        assert!(site.root.join(out).join(".statac-built").is_file(), "{out}");
    }
}

/// A link naming a page by its file stays a broken link, and is offered
/// the address the page is published at, from the top of the site, with
/// the `#fragment` written. One naming no file of a page is offered what
/// any other link is.
#[test]
fn a_link_naming_a_page_by_its_file_is_offered_the_pages_address() {
    let site = Site::new("by-file");
    let table = [
        ("essay-2.md", Some("/essays/essay-2/")),
        ("essay-3.md", Some("/essays/essay-3/")),
        ("./essay-3.md", Some("/essays/essay-3/")),
        ("index.md", Some("/essays/")),
        ("../about.md", Some("/about/")),
        ("../index.md", Some("/")),
        ("essay-2.md#part-1", Some("/essays/essay-2/#part-1")),
        ("essay-9.md", None),
        ("essay-3.txt", None),
    ];
    let links: Vec<String> = (table.iter())
        .map(|(written, _)| format!("[a link]({written})\n\n"))
        .collect();
    site.write(
        "content/essays/essay-1.md",
        &format!("---\ntitle: Essay 1\n---\n{}", links.concat()),
    );
    site.write(
        "content/essays/essay-2/index.md",
        "---\ntitle: Essay 2\n---\n## Part 1\n\n[Back](../essay-1.md)\n",
    );
    site.write(
        "content/essays/essay-3.md",
        "---\ntitle: Essay 3\n---\nEssay 3.\n",
    );
    site.write(
        "content/essays/index.md",
        "---\ntitle: Essays\nlayout: page\n---\nEssays.\n",
    );
    site.write("content/about.md", "---\ntitle: About\n---\nAbout.\n");
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    let told = |written: &str| {
        let headline = format!("warning  Broken link: nothing is published at `{written}`\n");
        let after = said.split(&headline).nth(1);
        let after = after.unwrap_or_else(|| panic!("{written} isn't told:\n{said}"));
        after.split("\n\n").next().unwrap().to_owned()
    };
    for (written, meant) in table {
        let offered = told(written)
            .lines()
            .find_map(|line| line.strip_prefix("  Did you mean `"))
            .map(|rest| rest.trim_end_matches("`?").to_owned());
        assert_eq!(offered.as_deref(), meant, "{written}");
    }
    assert!(
        told("../essay-1.md").contains("\n  Did you mean `/essays/essay-1/`?\n"),
        "{said}"
    );
}

/// A link a template writes from a value is told at the attribute that
/// wrote it: on each turn of a loop, after the page's writing, and where
/// its value has no quotes, in the template that holds the line.
#[test]
fn a_link_a_template_writes_from_a_value_is_told_at_its_own_line() {
    let site = Site::new("written-from-a-value");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write(
        "templates/base.html",
        "<title>{{ page.title }}</title>\n\
         <nav><a href=\"{{ page.url }}\">Here</a>\n\
         {% for address in [\"/post-1/\", \"/post-2/\"] %}\n\
         <a href=\"{{ address }}\">{{ address }}</a>\n\
         {% endfor %}</nav>\n\
         {% block main %}{% endblock %}\n",
    );
    site.write(
        "templates/page.html",
        "{% extends \"base\" %}\n\
         {% block main %}\n\
         {% for address in [\"/post-1/\", \"/post-3/\", \"/post-4/\"] %}\n\
         <a href=\"{{ address }}\">{{ address }}</a>\n\
         {% endfor %}\n\
         {{ page.content }}\n\
         <a href={{ \"/post-5/\" }}>Five</a>\n\
         {% endblock %}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\nPost 1.\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Broken link: nothing is published at `/post-2/`
  templates/base.html, line 4
   4 │ <a href=\"{{ address }}\">{{ address }}</a>
     │          ^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-3/`
  templates/page.html, line 4
   4 │ <a href=\"{{ address }}\">{{ address }}</a>
     │          ^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-4/`
  templates/page.html, line 4
   4 │ <a href=\"{{ address }}\">{{ address }}</a>
     │          ^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-5/`
  templates/page.html, line 7
   7 │ <a href={{ \"/post-5/\" }}>Five</a>
     │         ^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

4 warnings: 4 broken-link.
"
    );
}

/// A link in markup a field or a data file holds, printed whole, is told
/// once where the file writes it, and a link a template makes and prints
/// whole is told at the print.
#[test]
fn a_link_printed_in_markup_is_told_where_the_markup_is_written() {
    let site = Site::new("printed-markup");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write("data/links.yaml", "nav: <a href=\"/abot/\">About</a>\n");
    site.write(
        "templates/page.html",
        "<a href=\"/{{ page.language }}/\">Home</a>\n\
         {{ site.data.links.nav | safe }}\n\
         {{ page.summary | safe }}\n\
         {{ ('<a href=\"/post-' ~ 2 ~ '/\">') | safe }}Next</a>\n\
         {{ page.content }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\nsummary: '<a href=\"/gone/\">Gone</a>'\n---\n",
    );
    site.write(
        "content/post-3.md",
        "---\ntitle: Post 3\ndate: 2026-01-03\nsummary: Three.\n---\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Broken link: nothing is published at `/gone/`
  content/post-1.md, line 4
   4 │ summary: '<a href=\"/gone/\">Gone</a>'
     │                    ^^^^^^
  While building: content/post-1.md
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/abot/`
  data/links.yaml, line 1
   1 │ nav: <a href=\"/abot/\">About</a>
     │               ^^^^^^
  While building: content/post-1.md
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/en/`
  templates/page.html, line 1
   1 │ <a href=\"/{{ page.language }}/\">Home</a>
     │          ^^^^^^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-2/`
  templates/page.html, line 4
   4 │ {{ ('<a href=\"/post-' ~ 2 ~ '/\">') | safe }}Next</a>
     │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

4 warnings: 4 broken-link.
"
    );
}

/// A link a macro writes is told inside the macro: in its own text, or
/// at its print where it prints the link whole, wherever it is called.
#[test]
fn a_link_a_macro_writes_is_told_inside_the_macro() {
    let site = Site::new("macro-link");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write(
        "templates/page.html",
        "{% macro nav(to) %}\n\
         <a href=\"{{ to }}\">Next</a>\n\
         <p>{{ ('<a href=\"' ~ to ~ 'x/\">') | safe }}More</a></p>\n\
         {% endmacro %}\n\
         <nav>\n  {{ nav(\"/post-2/\") }}\n</nav>\n\
         {{ page.content }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Broken link: nothing is published at `/post-2/`
  templates/page.html, line 2
   2 │ <a href=\"{{ to }}\">Next</a>
     │          ^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-2/x/`
  templates/page.html, line 3
   3 │ <p>{{ ('<a href=\"' ~ to ~ 'x/\">') | safe }}More</a></p>
     │    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

2 warnings: 2 broken-link.
"
    );
}

/// A link a template writes in its own text, to where the page's writing
/// also links, is told at the template, and the writing's at the writing,
/// each once.
#[test]
fn a_templates_link_and_the_writings_to_one_address_are_each_told_where_written() {
    let site = Site::new("one-address-twice");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n\
         url: https://example.org/blog/\n",
    );
    site.write(
        "templates/page.html",
        "<a href=\"{{ url('/abot/') }}\">a</a>\n{{ page.content }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n[1](/blog/abot/)\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Broken link: nothing is published at `/blog/abot/`
  content/post-1.md, line 5
   5 │ [1](/blog/abot/)
     │     ^^^^^^^^^^^
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/blog/abot/`
  templates/page.html, line 1
   1 │ <a href=\"{{ url('/abot/') }}\">a</a>
     │          ^^^^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  More: https://statac.dev/e/broken-link/

2 warnings: 2 broken-link.
"
    );
}

/// A link in the writing, and one in markup a data file holds printed
/// before the writing, to one address, are each told once where written:
/// the data file's isn't taken for the writing's.
#[test]
fn a_writings_link_and_a_data_files_to_one_address_are_each_told_where_written() {
    let site = Site::new("writing-and-data");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write("data/links.yaml", "nav: <a href=\"/abot/\">About</a>\n");
    site.write(
        "templates/page.html",
        "{{ site.data.links.nav | safe }}\n{{ page.content }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\nSee [about](/abot/).\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Broken link: nothing is published at `/abot/`
  content/post-1.md, line 5
   5 │ See [about](/abot/).
     │             ^^^^^^
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/abot/`
  data/links.yaml, line 1
   1 │ nav: <a href=\"/abot/\">About</a>
     │               ^^^^^^
  While building: content/post-1.md
  More: https://statac.dev/e/broken-link/

2 warnings: 2 broken-link.
"
    );
}

/// An address a field holds, printed into a template's attribute, is told
/// at the field; a link a snippet the writing calls writes in its own text
/// is told in the snippet, and the address the writing hands it at the
/// call.
#[test]
fn a_link_is_told_where_its_address_is_written_through_fields_and_the_writings_snippets() {
    let site = Site::new("address-written");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write(
        "templates/page.html",
        "<a href=\"{{ page.following }}\">Next</a>\n{{ page.content }}\n",
    );
    site.write(
        "snippets/card.html",
        "<p><a href=\"/cards/\">Cards</a> <a href=\"{{ to }}\">{{ to }}</a></p>\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\nfollowing: /post-2/\n---\n\
         Before.\n\n@card(to=/post-3/)\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Broken link: nothing is published at `/post-2/`
  content/post-1.md, line 4
   4 │ following: /post-2/
     │            ^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-3/`
  content/post-1.md, line 8
   8 │ @card(to=/post-3/)
     │          ^^^^^^^^
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/cards/`
  snippets/card.html, line 1
   1 │ <p><a href=\"/cards/\">Cards</a> <a href=\"{{ to }}\">{{ to }}</a></p>
     │             ^^^^^^^
  While building: content/post-1.md
  More: https://statac.dev/e/broken-link/

3 warnings: 3 broken-link.
"
    );
}

/// A link a snippet writes from a value is told at the snippet's line,
/// whether the snippet is called in the writing or by a template, and told
/// once where that writing is also shown on another page.
#[test]
fn a_link_a_snippet_writes_from_a_value_is_told_at_the_snippets_line() {
    let site = Site::new("snippet-written-from-a-value");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write(
        "templates/page.html",
        "<title>{{ page.title }}</title>\n\
         {{ page.content }}\n\
         {{ footer() }}\n",
    );
    site.write(
        "templates/shows.html",
        "<title>{{ page.title }}</title>\n\
         {% for p in site.pages %}{% if p.title == \"Post 1\" %}{{ p.content }}{% endif %}\
         {% endfor %}\n",
    );
    site.write(
        "snippets/links.html",
        "{% for address in [\"/post-1/\", \"/post-2/\", \"/post-3/\"] %}\n\
         <a href=\"{{ address }}\">{{ address }}</a>\n\
         {% endfor %}\n\
         <a href=\"{{ \"part/\" }}\">Part</a>\n",
    );
    site.write(
        "snippets/footer.html",
        "<footer>\n<a href=\"{{ \"/post-4/\" }}\">Four</a>\n</footer>\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n@links()\n",
    );
    site.write(
        "content/shows.md",
        "---\ntitle: Shows\nlayout: shows\n---\n",
    );
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Broken link: nothing is published at `/post-4/`
  snippets/footer.html, line 2
   2 │ <a href=\"{{ \"/post-4/\" }}\">Four</a>
     │          ^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-2/`
  snippets/links.html, line 2
   2 │ <a href=\"{{ address }}\">{{ address }}</a>
     │          ^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-3/`
  snippets/links.html, line 2
   2 │ <a href=\"{{ address }}\">{{ address }}</a>
     │          ^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `part/`
  snippets/links.html, line 4
   4 │ <a href=\"{{ \"part/\" }}\">Part</a>
     │          ^^^^^^^^^^^^^
  While building: content/post-1.md
  More: https://statac.dev/e/broken-link/

4 warnings: 4 broken-link.
"
    );
}

/// On a site published under a path, an address written with `@url(...)`
/// or a template's `url(...)` leads under the path and is offered back
/// as another of the same; a plain link from the top of the domain is
/// left as written, and offered as one where the site publishes there. A
/// value handed to a snippet, and a page written in HTML, are read for no
/// call, and offered none.
#[test]
fn a_link_inside_a_site_under_a_path_is_offered_back_as_it_was_written() {
    let site = Site::new("inside");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n\
         url: https://example.org/blog/\n",
    );
    site.write(
        "templates/page.html",
        "<title>{{ page.title }}</title>\n\
         <nav><a href=\"{{ url(\"/\") }}\">Home</a> <a href=\"{{ url(\"/post-2/\") }}\">Next</a>\n\
         <a href=\"/post-1/\">First</a></nav>\n\
         {{ page.content }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n\
         [Home](@url(/)), [the notes](@url(/post-1/#notes)), [post 2](@url(/post-2/)),\n\
         <a href=\"@url(/post-3/)\">post 3</a>, [the notes again](/post-1/#notes)\n\
         and [post 11][eleven].\n\n\
         ## Notes\n\n[eleven]: @url(/post-11/)\n\n@more(/post-1/#more)\n",
    );
    site.write("snippets/more.html", "<a href=\"{{ to }}\">More</a>\n");
    site.write("content/post-4.md", "---\ntitle: Post 4\n---\nPost 4.\n");
    site.write(
        "content/page-1.html",
        "---\ntitle: Page 1\nlayout: bare\n---\n<p><a href=\"/post-1/?a=1\">Post 1</a></p>\n",
    );
    site.write("templates/bare.html", "{{ page.content }}\n");
    let (said, code) = site.check_without_javascript(&[]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "\
warning  Broken link: nothing is published at `/post-1/?a=1`
  content/page-1.html, line 5
   5 │ <p><a href=\"/post-1/?a=1\">Post 1</a></p>
     │             ^^^^^^^^^^^^
  Did you mean `/blog/post-1/`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/blog/post-2/`
  content/post-1.md, line 5
   5 │ [Home](@url(/)), [the notes](@url(/post-1/#notes)), [post 2](@url(/post-2/)),
     │                                                              ^^^^^^^^^^^^^^
  Did you mean `@url(/post-4/)`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/blog/post-3/`
  content/post-1.md, line 6
   6 │ <a href=\"@url(/post-3/)\">post 3</a>, [the notes again](/post-1/#notes)
     │          ^^^^^^^^^^^^^^
  Did you mean `@url(/post-4/)`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-1/#notes`
  content/post-1.md, line 6
   6 │ <a href=\"@url(/post-3/)\">post 3</a>, [the notes again](/post-1/#notes)
     │                                                        ^^^^^^^^^^^^^^
  To link inside the site, write `@url(/post-1/#notes)`.
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/blog/post-11/`
  content/post-1.md, line 11
  11 │ [eleven]: @url(/post-11/)
     │           ^^^^^^^^^^^^^^^
  Did you mean `@url(/post-4/)`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-1/#more`
  content/post-1.md, line 13
  13 │ @more(/post-1/#more)
     │       ^^^^^^^^^^^^^
  Did you mean `/blog/post-1/#more`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/blog/post-2/`
  templates/page.html, line 2
   2 │ <nav><a href=\"{{ url(\"/\") }}\">Home</a> <a href=\"{{ url(\"/post-2/\") }}\">Next</a>
     │                                                 ^^^^^^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `{{ url(\"/post-1/\") }}`?
  More: https://statac.dev/e/broken-link/

warning  Broken link: nothing is published at `/post-1/`
  templates/page.html, line 3
   3 │ <a href=\"/post-1/\">First</a></nav>
     │          ^^^^^^^^
  While building: content/post-1.md
  To link inside the site, write `{{ url(\"/post-1/\") }}`.
  More: https://statac.dev/e/broken-link/

8 warnings: 8 broken-link.
"
    );
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let built = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    for written in [
        "<nav><a href=\"/blog/\">Home</a> <a href=\"/blog/post-2/\">Next</a>\n\
         <a href=\"/post-1/\">First</a></nav>",
        "<p><a href=\"/blog/\">Home</a>, <a href=\"/blog/post-1/#notes\">the notes</a>, \
         <a href=\"/blog/post-2/\">post 2</a>,\n\
         <a href=\"/blog/post-3/\">post 3</a>, <a href=\"/post-1/#notes\">the notes again</a>\n\
         and <a href=\"/blog/post-11/\">post 11</a>.</p>",
    ] {
        assert!(built.contains(written), "{written}\nin\n{built}");
    }
}

/// What `Site::new`'s front matter leaves the writing's first line as.
const FIRST_LINE: usize = 5;

/// Writing of `kind` nested `depth` deep, one level to a line for lists.
fn nested(kind: &str, depth: usize) -> String {
    let marks = |a: &str, b: &str| {
        let open = |n: usize| if n.is_multiple_of(2) { a } else { b };
        let opening: String = (0..depth).map(|n| format!("{}a ", open(n))).collect();
        let closing: String = (0..depth).rev().map(|n| format!("{} a", open(n))).collect();
        format!("{opening}x{closing}\n")
    };
    match kind {
        "quotations" => format!("{} words\n", ">".repeat(depth)),
        "spaced quotations" => format!("{}words\n", "> ".repeat(depth)),
        "lists" => (0..depth)
            .map(|n| format!("{}- item\n", "  ".repeat(n)))
            .collect(),
        "numbered lists" => (0..depth)
            .map(|n| format!("{}1. item\n", "   ".repeat(n)))
            .collect(),
        "emphasis" => marks("*", "_"),
        "strong" => marks("**", "__"),
        // A quotation and a list are one level each.
        "quotations and lists" => format!("{}words\n", "> - ".repeat(depth.div_ceil(2))),
        _ => unreachable!("{kind}"),
    }
}

const NESTED: [&str; 7] = [
    "quotations",
    "spaced quotations",
    "lists",
    "numbered lists",
    "emphasis",
    "strong",
    "quotations and lists",
];

/// The line where the 201st level opens in `nested(kind, 201)`.
fn line_of_the_201st(kind: &str) -> usize {
    match kind {
        "lists" | "numbered lists" => FIRST_LINE + 200,
        _ => FIRST_LINE,
    }
}

fn nested_site(name: &str, writing: &str) -> Site {
    let site = Site::new(name);
    site.write(
        "content/post-1.md",
        &format!("---\ntitle: Post 1\ndate: 2026-01-01\n---\n{writing}"),
    );
    site
}

#[test]
fn writing_nested_two_hundred_deep_is_built() {
    for kind in NESTED {
        let site = nested_site("nested-200", &nested(kind, 200));
        let (said, code) = site.check_without_javascript(&[]);
        assert_eq!((said.as_str(), code), ("No problems.\n", 0), "{kind}");
        let (said, code) = site.run(&["build", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{kind}: {said}");
        assert!(site.out().join("post-1/index.html").exists(), "{kind}");
    }
}

#[test]
fn writing_nested_past_two_hundred_deep_is_told_once_where_the_201st_level_opens() {
    for kind in NESTED {
        for depth in [201, 5000] {
            let site = nested_site("nested-past", &nested(kind, depth));
            let (said, code) = site.check_without_javascript(&[]);
            assert_eq!(code, 1, "{kind} {depth}: {said}");
            let start = format!(
                "error  Nested too deep: the writing here is nested more than 200 deep\n  \
                 content/post-1.md, line {}\n",
                line_of_the_201st(kind)
            );
            assert!(said.starts_with(&start), "{kind} {depth}: {said}");
            assert!(
                said.ends_with("\n1 error: 1 nested-too-deep.\n"),
                "{kind} {depth}: {said}"
            );
            let (said, code) = site.run(&["build", "--today=2026-10-04"]);
            assert_eq!(code, 1, "{kind} {depth}: {said}");
            assert!(!site.out().exists(), "{kind} {depth}");
        }
    }
}

#[test]
fn a_value_read_as_markdown_nested_past_two_hundred_deep_is_told() {
    for kind in ["quotations", "lists", "emphasis"] {
        for depth in [200, 5000] {
            let site = Site::new("nested-value");
            let writing = nested(kind, depth).replace('\n', "\\n");
            site.write(
                "content/post-1.md",
                &format!("---\ntitle: Post 1\ndate: 2026-01-01\nsummary: \"{writing}\"\n---\n"),
            );
            site.write("templates/page.html", "{{ page.summary | markdown }}\n");
            let (said, code) = site.check_without_javascript(&[]);
            if depth == 200 {
                assert_eq!((said.as_str(), code), ("No problems.\n", 0), "{kind}");
                continue;
            }
            assert_eq!(code, 1, "{kind}: {said}");
            assert!(
                said.starts_with(
                    "error  Nested too deep: the writing here is nested more than 200 deep\n"
                ) && said.ends_with("\n1 error: 1 nested-too-deep.\n"),
                "{kind}: {said}"
            );
        }
    }
}

#[test]
fn raw_html_nested_thousands_deep_is_built() {
    let writing = format!(
        "{}words\n{}",
        "<div>\n".repeat(5000),
        "</div>\n".repeat(5000)
    );
    let site = nested_site("nested-html", &writing);
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(site.out().join("post-1/index.html").exists());
}

#[test]
fn threads_building_pages_have_room_whatever_the_system_gives_a_thread() {
    let site = nested_site("nested-room", &nested("numbered lists", 200));
    let output = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["build", "--today=2026-10-04"])
        .arg(&site.root)
        // Far less than the 1.4 MB that writing this deep takes unoptimised.
        .env("RUST_MIN_STACK", "100000")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(site.out().join("post-1/index.html").exists());
}

#[test]
fn every_page_as_a_list_and_chosen_by_a_field_is_as_any_list_is() {
    let site = Site::new("every-page");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write("content/index.md", "---\ntitle: Home\n---\n");
    let pages = [
        ("post-1", "menu: true\nrank: 2\nbook:\n  author: Author 1\n"),
        ("post-2", "menu: false\nrank: 1\n"),
        ("post-3", "menu: yes\nlayout: other\n"),
        ("post-4", "rank: 3\nbook:\n  author: Author 2\n"),
        ("post-5", ""),
    ];
    for (name, more) in pages {
        let text = format!("---\ntitle: {name}\n{more}---\nWords.\n");
        site.write(&format!("content/{name}.md"), &text);
    }
    site.write("templates/page.html", "{{ page.title }}\n");
    site.write("templates/other.html", "{{ page.title }}\n");
    // Each as asked of every page, and of the same pages as a list made
    // in the template, which is chosen from item by item.
    let asked = [
        "selectattr('menu')",
        "rejectattr('menu')",
        "selectattr('menu', 'eq', true)",
        "selectattr('menu', 'eq', 'yes')",
        "selectattr('menu', 'defined')",
        "rejectattr('menu', 'defined')",
        "selectattr('rank', '>', 1)",
        "selectattr('rank', 'in', [1, 3])",
        "selectattr('layout', 'eq', 'other')",
        "rejectattr('layout', 'eq', 'other')",
        "selectattr('book.author', 'eq', 'Author 2')",
        "selectattr('book.author')",
        "selectattr('title', 'eq', 'post-2')",
        "selectattr('url', 'endingwith', '-3/')",
        "selectattr('menu') | selectattr('rank')",
    ];
    let mut home = String::from(
        "{{ site.pages | length }} {{ site.pages[1].title }} {{ site.pages[-1].title }} \
         {{ site.pages[1:3] | map(attribute='title') | join(',') }} \
         {{ (site.pages + site.pages) | length }} {{ site.pages | first is defined }} \
         {{ page in site.pages }} {{ site.pages | reverse | map(attribute='url') | first }}\n",
    );
    for asking in asked {
        let line = format!(
            "{{{{ site.pages | {asking} | map(attribute='url') | join(' ') }}}}\
             ={{{{ site.pages | list | {asking} | map(attribute='url') | join(' ') }}}}\n"
        );
        home.push_str(&line);
    }
    site.write("templates/home.html", &home);
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    let built = fs::read_to_string(site.out().join("index.html")).unwrap();
    let mut lines = built.lines();
    assert_eq!(
        lines.next(),
        Some("6 post-1 post-5 post-1,post-2 12 True True /post-5/")
    );
    for (asking, line) in asked.iter().zip(lines) {
        let (every, list) = line.split_once('=').unwrap();
        assert_eq!(every, list, "{asking}");
    }
    assert_eq!(built.lines().count(), asked.len() + 1);
    assert!(
        built.contains("\n/post-1/ /post-3/=/post-1/ /post-3/\n"),
        "{built}"
    );
    assert!(built.contains("\n/post-4/=/post-4/\n"), "{built}");
}

#[test]
fn every_list_of_pages_a_template_is_handed_is_as_any_list_is() {
    let site = Site::new("lists-of-pages");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\nposts_per_page: 2\n",
    );
    site.write("content/index.md", "---\ntitle: Home\n---\n");
    site.write(
        "content/blog/index.md",
        "---\ntitle: Blog\nlayout: lists\n---\n",
    );
    site.write("content/blog/undated.md", "---\ntitle: Undated\n---\n");
    for n in 1..=5 {
        let text = format!(
            "---\ntitle: Post {n}\ndate: 2026-09-0{n}\ntags: [one{}]\n---\nWords.\n",
            if n % 2 == 0 { ", two" } else { "" }
        );
        site.write(&format!("content/blog/post-{n}.md"), &text);
    }
    site.write("templates/page.html", "{{ page.title }}\n");
    site.write("templates/post.html", "{{ page.title }}\n");
    site.write("templates/folder.html", "{{ page.list.name }}\n");
    // Each list as it is handed, and the same pages as a list made in the
    // template.
    let lists = [
        "site.posts",
        "page.posts",
        "page.list.posts",
        "page.list.pages",
        "page.list.pager",
        "related(page.posts[0])",
        "site.tags[0].posts",
    ];
    let asked = [
        "{{ L | length }}",
        "{{ 'some' if L else 'none' }}",
        "{{ L[0].url }} {{ L[-1].url }} {{ L[1:2] | map(attribute='title') | join }}",
        "{{ L[:2] | join(' ') }}|{{ L[1:] | join(' ') }}|{{ L[-2:] | join(' ') }}",
        "{{ L[::-1] | join(' ') }}",
        "{{ L | first }} {{ L | last }}",
        "{{ L | reverse | join(' ') }}",
        "{{ (L + L) | length }} {{ ([page] + L) | join(' ') }}",
        "{{ L[0] in L }} {{ page in L }}",
        "{% for p in L %}{{ loop.index }}/{{ loop.length }}:{{ p.url }}{% if not loop.last %},{% endif %}{% endfor %}",
        "{% for p in L if p.title is endingwith('2') %}{{ p.url }}{% endfor %}",
        "{{ L | sort(attribute='title', reverse=true) | join(' ') }}",
        "{{ L | map(attribute='title') | join(',') }}",
        "{{ L | selectattr('title', 'eq', 'Post 2') | join(' ') }}",
        "{{ L | batch(2) | map('length') | join(',') }}",
        "{{ L | newest_first | join(' ') }}",
        "{{ L | tagged('two') | join(' ') }}",
        "{{ L | in('blog') | join(' ') }}",
        "{{ L | where('title', 'post 3') | join(' ') }}",
        "{{ L | tagged('one') | tagged('two') | newest_first | join(' ') }}",
        "{{ L | rejectattr('title', 'eq', 'Post 2') | selectattr('date') | join(' ') }}",
        "{{ L | selectattr('language', 'eq', page.language) | join(' ') }}",
        "{{ L | selectattr('url', 'endingwith', '-3/') | join(' ') }}",
        "{{ L | selectattr('date.day', '>', 2) | map(attribute='title') | join(',') }}",
        "{{ L | sort(attribute='title') | join(' ') }}",
        "{{ L | sort(attribute='url', reverse=true) | first }}",
        "{{ L | sort(attribute='date') | join(' ') }}",
        "{{ (L | sort(attribute='title'))[:2] | join(' ') }}",
        "{{ L | selectattr('title', 'eq', 'Post 2') | sort(attribute='title') | length }}",
        "{{ L }}",
        "{{ L | json }}",
    ];
    let mut folder = String::new();
    for list in lists {
        for asking in asked {
            let made = format!("({list} | list)");
            folder.push_str(&asking.replace('L', list));
            folder.push('=');
            folder.push_str(&asking.replace('L', &made));
            folder.push_str(";\n");
        }
    }
    site.write("templates/lists.html", &folder);
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    for built in ["blog/index.html", "blog/page/2/index.html"] {
        let built = fs::read_to_string(site.out().join(built)).unwrap();
        let mut lines = built.lines();
        for list in lists {
            for asking in asked {
                let line = lines.next().unwrap().strip_suffix(';').unwrap();
                let (handed, made) = line.split_at(line.len() / 2);
                assert_eq!(Some(handed), made.strip_prefix('='), "{list}: {asking}");
            }
        }
        assert_eq!(lines.next(), None);
    }
    let first = fs::read_to_string(site.out().join("blog/index.html")).unwrap();
    for line in [
        "5=5",
        "/blog/post-5/ /blog/post-4/=/blog/post-5/ /blog/post-4/",
        "6=6",
        "3=3",
        "/blog/ /blog/page/2/ /blog/page/3/=/blog/ /blog/page/2/ /blog/page/3/",
    ] {
        assert!(first.contains(&format!("\n{line};\n")), "{line}\n{first}");
    }
}

/// A template holding the control characters blame once filled its
/// expressions with is told at the same place as one without.
#[test]
fn a_link_in_a_template_holding_control_characters_is_told_where_it_was_written() {
    for lead in ["", "\u{1}", "\u{2}"] {
        let site = Site::new("control-characters");
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
        );
        site.write(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\n",
        );
        site.write(
            "templates/page.html",
            &format!(
                "{{% set link = '<a href=\"/gone/\">x</a>' %}}\n\
                 {lead}{{{{ link | safe }}}}\n{{{{ page.content }}}}\n"
            ),
        );
        let (said, code) = site.check_without_javascript(&[]);
        assert_eq!(code, 0, "{said}");
        let (shown, margin) = match lead {
            "\u{1}" => ("\u{2401}", " "),
            "\u{2}" => ("\u{2402}", " "),
            _ => ("", ""),
        };
        assert_eq!(
            said,
            format!(
                "\
warning  Broken link: nothing is published at `/gone/`
  templates/page.html, line 2
   2 \u{2502} {shown}{{{{ link | safe }}}}
     \u{2502} {margin}^^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  More: https://statac.dev/e/broken-link/

1 warning: 1 broken-link.
"
            )
        );
    }
}
