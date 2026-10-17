//! The site's own commands, `before_build` and `after_build`, run by
//! `statac build` on small sites in a temporary folder. Each command is one
//! both `sh` and `cmd` read, but for how a value Statac hands over is
//! spelled.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct Site {
    root: PathBuf,
}

const SETTINGS: &str = "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n";

impl Site {
    /// A site with a home page that links what its settings are to name.
    fn new(name: &str, settings: &str) -> Site {
        let root = std::env::temp_dir().join(format!("statac-hooks-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let site = Site { root };
        site.write("statac.yaml", &format!("{SETTINGS}{settings}"));
        site.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        site.write(
            "templates/home.html",
            "<!doctype html>\n<title>{{ page.title }}</title>\n{{ page.content }}\n",
        );
        site.write("static/kept.txt", "kept\n");
        site
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// The text of a file in the site's folder, without the line ending a
    /// shell closed it with; none where there is no such file.
    fn read(&self, path: &str) -> Option<String> {
        let text = fs::read_to_string(self.root.join(path)).ok()?;
        Some(text.trim_end().to_owned())
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
        (said, output.status.code().unwrap())
    }

    fn build(&self) -> (String, i32) {
        self.run(&["build", "--today=2026-10-04"])
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A value Statac hands a command, as the system's shell spells it.
fn handed(name: &str) -> String {
    match cfg!(windows) {
        true => format!("%{name}%"),
        false => format!("${name}"),
    }
}

/// A command that prints on both streams and ends with exit code 3.
fn stops() -> &'static str {
    match cfg!(windows) {
        true => "echo partway& echo bad 1>&2& exit /b 3",
        false => "echo partway; echo bad >&2; exit 3",
    }
}

#[test]
fn before_build_runs_before_the_site_is_read() {
    let site = Site::new("before", "before_build: echo made> static/made.txt\n");
    let (said, code) = site.build();
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("Ran `before_build`.\n"), "{said}");
    assert_eq!(site.read("_site/made.txt").as_deref(), Some("made"));
}

#[test]
fn a_command_is_handed_where_it_is_and_what_it_is() {
    let line = format!(
        "echo {} {}> told.txt",
        handed("STATAC_HOOK"),
        handed("STATAC_DRAFTS")
    );
    let site = Site::new("handed", &format!("before_build: {line}\n"));
    let (said, code) = site.build();
    assert_eq!(code, 0, "{said}");
    assert_eq!(site.read("told.txt").as_deref(), Some("before_build no"));
    let (said, code) = site.run(&["build", "--today=2026-10-04", "--drafts"]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(site.read("told.txt").as_deref(), Some("before_build yes"));
}

#[test]
fn after_build_acts_on_the_built_site_and_what_it_adds_is_published() {
    let line = format!("echo extra> \"{}/extra.txt\"", handed("STATAC_OUT"));
    let site = Site::new("after", &format!("after_build: {line}\n"));
    // Into no output, into an output with no spare, and into the spare
    // that the first build's command wrote in.
    for build in 1..=3 {
        let (said, code) = site.build();
        assert_eq!(code, 0, "build {build}: {said}");
        assert!(
            said.contains("Ran `after_build`, which added 1 file.\n"),
            "build {build}: {said}"
        );
        assert_eq!(site.read("_site/extra.txt").as_deref(), Some("extra"));
        assert_eq!(site.read("_site/kept.txt").as_deref(), Some("kept"));
    }
}

/// A file in the folder Statac hands a command, by its name there, as the
/// system's shell spells its path.
fn in_out(name: &str) -> String {
    let out = handed("STATAC_OUT");
    match cfg!(windows) {
        true => format!("\"{out}\\{name}\""),
        false => format!("\"{out}/{name}\""),
    }
}

#[test]
fn after_build_is_told_by_the_files_it_added_changed_and_removed() {
    let added =
        |n: usize| (1..=n).map(|n| format!("echo added> {}", in_out(&format!("added-{n}.txt"))));
    // The same length as what the build wrote, so only its bytes change,
    // and its time where the disk's times have moved on.
    let changed = format!("echo KEPT> {}", in_out("kept.txt"));
    let removed = (1..=3).map(|n| in_out(&format!("removed-{n}.txt")));
    let removed = match cfg!(windows) {
        true => format!("del {}", removed.collect::<Vec<_>>().join(" ")),
        false => format!("rm {}", removed.collect::<Vec<_>>().join(" ")),
    };
    // (what the command does, what is said of it)
    let cases = [
        (added(2).collect::<Vec<_>>(), "added 2 files"),
        (
            added(2).chain([changed.clone()]).collect(),
            "added 2 files and changed 1",
        ),
        (
            added(2).chain([removed.clone()]).collect(),
            "added 2 files and removed 3",
        ),
        (
            added(2).chain([changed.clone(), removed.clone()]).collect(),
            "added 2 files, changed 1 and removed 3",
        ),
        (vec![changed.clone()], "added 0 files and changed 1"),
    ];
    for (commands, told) in cases {
        let line = commands.join(" && ");
        let site = Site::new("changed", &format!("after_build: '{line}'\n"));
        let ending = if cfg!(windows) { "\r\n" } else { "\n" };
        site.write("static/kept.txt", &format!("kept{ending}"));
        for n in 1..=3 {
            site.write(&format!("static/removed-{n}.txt"), "removed\n");
        }
        // Into no output, into an output with no spare, and into the spare
        // that the first build's command wrote in.
        for build in 1..=3 {
            let (said, code) = site.build();
            assert_eq!(code, 0, "build {build}: {said}");
            assert!(
                said.contains(&format!("Ran `after_build`, which {told}.\n")),
                "build {build}: {said}"
            );
        }
    }
}

#[test]
fn an_after_build_that_stops_publishes_nothing() {
    let site = Site::new("stops", "");
    let (said, code) = site.build();
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("after_build"), "{said}");

    site.write(
        "statac.yaml",
        &format!("{SETTINGS}after_build: {}\n", stops()),
    );
    site.write("static/kept.txt", "changed\n");
    let (said, code) = site.build();
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("error  Hook stopped: `after_build` ended with exit code 3\n"),
        "{said}"
    );
    assert!(said.contains("  statac.yaml, line 5\n"), "{said}");
    assert!(
        said.contains("  What it printed:\n    partway\n    bad\n  More: "),
        "{said}"
    );
    assert!(
        said.contains("Wrote nothing: 1 error stopped the build."),
        "{said}"
    );
    assert_eq!(site.read("_site/kept.txt").as_deref(), Some("kept"));
}

#[test]
fn a_before_build_the_shell_cant_find_stops_the_build_unread() {
    let site = Site::new("missing", "before_build: statac-no-such-command\n");
    // Nothing of the site is read, so its own mistakes aren't told yet.
    site.write("content/post-1.md", "---\ntitle: [\n---\n");
    let (said, code) = site.build();
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("error  Hook stopped: `before_build` ended with exit code "),
        "{said}"
    );
    assert!(said.contains("statac-no-such-command"), "{said}");
    assert!(!said.contains("post-1.md"), "{said}");
    if cfg!(unix) {
        assert!(
            said.contains(
                "  The shell ends with 127 when it can't find a command. Is the command \
                 installed, and on the path Statac was run with?\n"
            ),
            "{said}"
        );
    }
    assert!(!site.root.join("_site").exists());
}

#[test]
fn a_command_that_prints_nothing_is_said_to_have() {
    let line = match cfg!(windows) {
        true => "exit /b 2",
        false => "exit 2",
    };
    let site = Site::new("silent", &format!("before_build: {line}\n"));
    let (said, code) = site.build();
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("ended with exit code 2\n  statac.yaml, line 5\n"),
        "{said}"
    );
    assert!(said.contains("  It printed nothing.\n"), "{said}");
}

#[test]
fn check_runs_neither_command_and_says_so_once() {
    let settings = "before_build: echo made> made.txt\nafter_build: echo made> made.txt\n";
    let site = Site::new("check", settings);
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said.matches("note  Hook not run: `check` doesn't run `before_build` or `after_build`\n")
            .count(),
        1,
        "{said}"
    );
    assert!(said.contains("  statac.yaml, line 5\n"), "{said}");
    assert_eq!(site.read("made.txt"), None);

    site.write(
        "statac.yaml",
        &format!("{SETTINGS}{settings}quiet: [hook-not-run]\n"),
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("Hook not run"), "{said}");
    assert!(said.contains("1 note, quietened."), "{said}");
}

#[test]
fn a_before_build_in_settings_with_an_error_is_not_run() {
    let site = Site::new(
        "unsure",
        "before_build: echo made> made.txt\ntags.nothing: true\n",
    );
    let (said, code) = site.build();
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("error  Unknown setting: `tags.nothing`"),
        "{said}"
    );
    assert!(
        said.contains("Didn't run `before_build`: `statac.yaml` has an error.\n"),
        "{said}"
    );
    assert_eq!(site.read("made.txt"), None);
}

#[test]
fn an_after_build_naming_the_output_folder_is_warned_of_at_the_name() {
    let handed = format!("\"{}\"", handed("STATAC_OUT"));
    let site = Site::new("named", "after_build: echo ./_site/made.txt\n");
    let headline = "warning  Output folder named: `_site` holds the site built last time while \
                    `after_build` runs\n  statac.yaml, line 5\n";
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains(headline), "{said}");
    assert!(
        said.contains(
            "   5 \u{2502} after_build: echo ./_site/made.txt\n     \u{2502}                     ^^^^^\n"
        ),
        "{said}"
    );
    assert!(
        said.contains(&format!("  Write it as `echo {handed}/made.txt`.\n")),
        "{said}"
    );
    // A warning: the build goes on, and counts it.
    let (said, code) = site.build();
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("1 warning: 1 output-folder-named."), "{said}");

    // Another folder asked for is the one looked for.
    let out = site.root.join("public");
    let out = format!("--out={}", out.display());
    let (said, code) = site.run(&["build", "--today=2026-10-04", &out]);
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("output-folder-named"), "{said}");

    for quiet in [
        format!("echo {handed}/made.txt"),
        "echo _sites my_site.sh".to_owned(),
    ] {
        site.write("statac.yaml", &format!("{SETTINGS}after_build: {quiet}\n"));
        let (said, code) = site.run(&["check", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{said}");
        assert!(!said.contains("Output folder named"), "{quiet}: {said}");
    }
}

#[test]
fn the_folder_given_with_out_is_the_one_an_after_build_is_warned_of() {
    let site = Site::new("outnamed", "after_build: echo public/ _site\n");
    let run = |out: &str| {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(["build", "--today=2026-10-04", out])
            .current_dir(&site.root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        String::from_utf8(output.stdout).unwrap()
    };
    let said = run("--out=./public/");
    assert!(said.contains("1 warning: 1 output-folder-named."), "{said}");
    let said = run("--out=elsewhere");
    assert!(!said.contains("output-folder-named"), "{said}");
}

#[test]
fn a_command_written_on_a_page_is_never_run() {
    let site = Site::new("page", "");
    site.write(
        "content/index.md",
        "---\ntitle: Home\nbefore_build: echo made> made.txt\n\
         after_build: echo made> made.txt\npages.before_build: echo made> made.txt\n---\nHome.\n",
    );
    site.write(
        "content/notes/index.md",
        "---\ntitle: Notes\npages.after_build: echo made> made.txt\n---\n",
    );
    site.write(
        "data/commands.yaml",
        "before_build: echo made> made.txt\nafter_build: echo made> made.txt\n",
    );
    let (said, _) = site.build();
    assert!(!said.contains("Ran `"), "{said}");
    assert_eq!(site.read("made.txt"), None);
    assert_eq!(site.read("_site/made.txt"), None);
}

#[test]
fn a_link_into_what_after_build_makes_lands() {
    let out = handed("STATAC_OUT");
    let site = Site::new("makes", "");
    site.write(
        "content/index.md",
        "---\ntitle: Home\n---\n[The index](/search.json) and [more](/more/a.txt).\n",
    );
    let (said, code) = site.build();
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("2 broken-link"), "{said}");

    site.write(
        "statac.yaml",
        &format!(
            "{SETTINGS}after_build: echo made> \"{out}/search.json\"\n\
             after_build_makes: [search.json, /more/]\n"
        ),
    );
    let (said, code) = site.build();
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("broken-link"), "{said}");
    assert_eq!(site.read("_site/search.json").as_deref(), Some("made"));
    // What is named and not made is told in full, since its links were
    // let pass and `check` runs no command to find it.
    assert!(
        said.contains(
            "warning  Nothing made: `after_build` left nothing at `/more/`\n  statac.yaml, line 6\n"
        ),
        "{said}"
    );
    assert!(!said.contains("statac check"), "{said}");
}

#[test]
fn what_after_build_makes_is_held_to_the_addresses_the_site_publishes() {
    let site = Site::new(
        "claimed",
        "after_build: echo made\nafter_build_makes: [kept.txt]\n",
    );
    let (said, code) = site.build();
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains(
            "error  Same address: `static/kept.txt` and what `after_build` makes at \
             `kept.txt` are both published at `/kept.txt`\n  statac.yaml, line 6\n"
        ),
        "{said}"
    );
}

#[test]
fn a_setting_about_a_command_needs_the_command() {
    let site = Site::new(
        "alone",
        "after_build_makes: [search/]\nbefore_build_watches: [templates/]\n",
    );
    let (said, code) = site.build();
    assert_eq!(code, 1, "{said}");
    for (about, command) in [
        ("after_build_makes", "after_build"),
        ("before_build_watches", "before_build"),
    ] {
        assert!(
            said.contains(&format!(
                "error  Wrong place: `{about}` is for a site that sets `{command}`\n"
            )),
            "{said}"
        );
        assert!(
            said.contains(&format!(
                "  Add the `{command}:` line it describes, or take it out.\n"
            )),
            "{said}"
        );
    }
}

#[test]
fn what_before_build_watches_is_in_the_site() {
    let site = Site::new(
        "watches",
        "before_build: echo made\nbefore_build_watches: [templates/, static/kept.txt, styles/]\n",
    );
    let (said, code) = site.build();
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("error  Missing file: there is no `styles/`\n  statac.yaml, line 6\n"),
        "{said}"
    );
    assert_eq!(said.matches("error  ").count(), 1, "{said}");

    site.write(
        "statac.yaml",
        &format!("{SETTINGS}before_build: echo made\nbefore_build_watches: [../styles/]\n"),
    );
    let (said, code) = site.build();
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains(
            "error  Wrong kind of value: `before_build_watches` is a file or a folder in \
             the site, such as `templates/` or `assets/site.css`, not `../styles/`\n"
        ),
        "{said}"
    );
}
