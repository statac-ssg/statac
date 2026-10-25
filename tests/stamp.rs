//! `statac stamp` writes a date over every stamp in the front matter of a
//! site's pages and says each one; `build` and `check` stop at a stamp
//! still unwritten. Each test runs the program on a small site of its own
//! in the system's temporary folder.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const LAYOUT: &str = "<!doctype html>\n<html lang=\"{{ site.language }}\">\n\
                      <title>{{ headline(page) }}</title>\n{{ page.content }}\n</html>\n";

const SETTINGS: &str = "title: Site\nlanguage: en\ntimezone: Europe/London\n";

struct Site(PathBuf);

impl Site {
    fn new(label: &str) -> Site {
        let path =
            std::env::temp_dir().join(format!("statac-stamp-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        let site = Site(path);
        site.write("statac.yaml", SETTINGS);
        site.write("templates/page.html", LAYOUT);
        site.write(
            "content/index.md",
            "---\ntitle: Home\nlayout: page\n---\nThe home page.\n",
        );
        site
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.0.clone(), |path, piece| path.join(piece))
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap()
    }

    /// What the program says of the site on standard output, and its exit
    /// code. Nothing is ever said on standard error.
    fn run(&self, command: &str, options: &[&str]) -> (String, Option<i32>) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .arg(command)
            .arg(&self.0)
            .args(options)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        let said = String::from_utf8(output.stdout).unwrap();
        assert!(output.stderr.is_empty(), "{command}: {said}");
        (said, output.status.code())
    }

    /// `statac stamp` on Tuesday 6 October 2026.
    fn stamp(&self) -> (String, Option<i32>) {
        self.run("stamp", &["--today=2026-10-06"])
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn every_stamp_in_a_site_is_written_and_said() {
    let site = Site::new("every");
    // Line endings as another system writes them, a value in each kind of
    // quotes, values nested and in a list, and several in one file.
    site.write(
        "content/post-1.md",
        "---\r\ntitle: Post 1\r\ndate: \"=today\"\r\nupdated: =now\r\nevent:\r\n  \
         start: =friday 19:00\r\n  days: [=monday, '=tomorrow 08:15']\r\n---\r\n\
         Written =today.\r\n",
    );
    site.write(
        "content/notes/index.md",
        "---\ntitle: Notes\nlayout: page\npages.updated: =yesterday # as handed down\n---\n",
    );
    site.write("content/notes/note-1.md", "# Note 1\n\nA note.\n");
    site.write(
        "content/page-1.html",
        "---\ntitle: Page 1\ndate: =sunday\n---\n<p>=today</p>\n",
    );
    let (said, code) = site.stamp();
    assert_eq!(
        said,
        "Stamped `content/page-1.html`: `date` is `2026-10-11`.\n\
         Stamped `content/post-1.md`: `date` is `2026-10-06`.\n\
         Stamped `content/post-1.md`: `updated` is `2026-10-06 00:00`.\n\
         Stamped `content/post-1.md`: `event.start` is `2026-10-09 19:00`.\n\
         Stamped `content/post-1.md`: `event.days` is `2026-10-12`.\n\
         Stamped `content/post-1.md`: `event.days` is `2026-10-07 08:15`.\n\
         Stamped `content/notes/index.md`: `pages.updated` is `2026-10-05`.\n"
    );
    assert_eq!(code, Some(0));
    assert_eq!(
        site.read("content/post-1.md"),
        "---\r\ntitle: Post 1\r\ndate: 2026-10-06\r\nupdated: 2026-10-06 00:00\r\nevent:\r\n  \
         start: 2026-10-09 19:00\r\n  days: [2026-10-12, 2026-10-07 08:15]\r\n---\r\n\
         Written =today.\r\n"
    );
    assert_eq!(
        site.read("content/notes/index.md"),
        "---\ntitle: Notes\nlayout: page\npages.updated: 2026-10-05 # as handed down\n---\n"
    );
    assert_eq!(
        site.read("content/page-1.html"),
        "---\ntitle: Page 1\ndate: 2026-10-11\n---\n<p>=today</p>\n"
    );

    // What was written is read as the dates it names, and nothing waits.
    let (said, code) = site.run("check", &["--today=2026-10-12"]);
    assert_eq!(code, Some(0), "{said}");
    assert!(!said.contains("error"), "{said}");
    assert_eq!(site.stamp(), ("Nothing to stamp.\n".into(), Some(0)));
}

#[test]
fn a_site_with_no_stamp_is_left_as_it_is() {
    let site = Site::new("none");
    let text = "---\ntitle: Post 1\ndate: 2026-10-04\nformula: =IF(A1)\n---\nWritten =today.\n";
    site.write("content/post-1.md", text);
    let before = fs::metadata(site.path("content/post-1.md")).unwrap();
    assert_eq!(site.stamp(), ("Nothing to stamp.\n".into(), Some(0)));
    let after = fs::metadata(site.path("content/post-1.md")).unwrap();
    assert_eq!(site.read("content/post-1.md"), text);
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    let (said, code) = site.run("check", &[]);
    assert_eq!(code, Some(0), "{said}");
}

#[test]
fn build_and_check_stop_at_a_stamp_not_yet_written() {
    let site = Site::new("stops");
    let text = "---\ntitle: Post 1\ndate: =monday 09:00\n---\nThe first post.\n";
    site.write("content/post-1.md", text);
    let told = "\
error  Not stamped: `date` is still `=monday 09:00`
  content/post-1.md, line 3
   3 │ date: =monday 09:00
     │       ^^^^^^^^^^^^^
  Run `statac stamp` to write the date in its place, or `statac serve`, \
which writes it on every save.
  More: https://statac.dev/e/not-stamped/
";
    for command in ["build", "check"] {
        let (said, code) = site.run(command, &["--today=2026-10-06"]);
        assert_eq!(code, Some(1), "{command}: {said}");
        assert!(said.starts_with(told), "{command}: {said}");
        assert!(said.contains("1 error"), "{command}: {said}");
        assert_eq!(site.read("content/post-1.md"), text, "{command}");
        assert!(!site.path("_site").exists(), "{command}");
    }

    let (said, code) = site.stamp();
    assert_eq!(
        said,
        "Stamped `content/post-1.md`: `date` is `2026-10-12 09:00`.\n"
    );
    assert_eq!(code, Some(0));
    // The post waits for its day, and is published on it.
    let (said, code) = site.run("build", &["--today=2026-10-06"]);
    assert_eq!(code, Some(0), "{said}");
    assert!(!site.path("_site/post-1/index.html").exists());
    let (said, code) = site.run("build", &["--today=2026-10-12"]);
    assert_eq!(code, Some(0), "{said}");
    assert!(site.path("_site/post-1/index.html").exists());
}

#[test]
fn a_value_meant_as_a_stamp_is_told_and_left_as_written() {
    let site = Site::new("refused");
    let text = "---\ntitle: Post 1\ndate: =Tomorrow\nupdated: =today 9am\n---\n";
    site.write("content/post-1.md", text);
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: =today\n---\n",
    );
    let (said, code) = site.stamp();
    assert_eq!(
        said,
        "\
Stamped `content/post-2.md`: `date` is `2026-10-06`.

error  Unknown stamp: `=Tomorrow`
  content/post-1.md, line 3
   3 │ date: =Tomorrow
     │       ^^^^^^^^^
  Statac calls this `=tomorrow`.
  More: https://statac.dev/e/other-spelling/

error  Unknown stamp: `=today 9am`
  content/post-1.md, line 4
   4 │ updated: =today 9am
     │          ^^^^^^^^^^
  Statac calls this `=today 09:00`.
  More: https://statac.dev/e/other-spelling/
"
    );
    assert_eq!(code, Some(1));
    assert_eq!(site.read("content/post-1.md"), text);
    let (said, code) = site.run("check", &["--today=2026-10-06"]);
    assert_eq!(code, Some(1));
    assert!(said.contains("2 errors: 2 other-spelling."), "{said}");
}

#[test]
fn nothing_is_stamped_while_the_settings_have_an_error() {
    let site = Site::new("settings");
    let text = "---\ntitle: Post 1\ndate: =today\n---\n";
    site.write("content/post-1.md", text);
    site.write("statac.yaml", "title: Site\ntimezone: Europe/Londn\n");
    let (said, code) = site.stamp();
    assert_eq!(
        said,
        "\
error  Unknown time zone: `Europe/Londn`
  statac.yaml, line 2
   2 │ timezone: Europe/Londn
     │           ^^^^^^^^^^^^
  Did you mean `Europe/London`?
  More: https://statac.dev/e/unknown-word/

Didn't stamp: `statac.yaml` has an error.
"
    );
    assert_eq!(code, Some(1));
    assert_eq!(site.read("content/post-1.md"), text);

    site.write("statac.yaml", "title: [Site\n");
    let (said, code) = site.stamp();
    assert!(said.starts_with("error  Malformed YAML: "), "{said}");
    assert!(
        said.ends_with("\nDidn't stamp: `statac.yaml` has an error.\n"),
        "{said}"
    );
    assert_eq!(code, Some(1));
    assert_eq!(site.read("content/post-1.md"), text);

    // With nothing waiting, there is nothing to say of the settings.
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\n");
    assert_eq!(site.stamp(), ("Nothing to stamp.\n".into(), Some(0)));
}

/// `Pacific/Kiritimati` and `Pacific/Niue` are twenty-five hours apart, so
/// their days differ at every moment: two sites alike but for their zone
/// are stamped with different days whatever clock this machine is on.
#[test]
fn today_is_the_day_on_the_sites_own_clock() {
    let stamped = |label: &str, zone: &str| {
        let site = Site::new(label);
        site.write("statac.yaml", &format!("title: Site\ntimezone: {zone}\n"));
        site.write(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: =today\nupdated: =now\n---\n",
        );
        let (said, code) = site.run("stamp", &[]);
        assert_eq!(code, Some(0), "{said}");
        let text = site.read("content/post-1.md");
        let line = |at: usize| text.lines().nth(at).unwrap().to_owned();
        (line(2), line(3))
    };
    let (ahead, ahead_now) = stamped("ahead", "Pacific/Kiritimati");
    let (behind, _) = stamped("behind", "Pacific/Niue");
    assert_ne!(ahead, behind);
    // `date: 2026-10-06`, and `updated: 2026-10-06 14:07`.
    let shape = |line: &str| -> String {
        let figure = |c: char| if c.is_ascii_digit() { '0' } else { c };
        line.chars().map(figure).collect()
    };
    assert_eq!(shape(&ahead), "date: 0000-00-00");
    assert_eq!(shape(&ahead_now), "updated: 0000-00-00 00:00");
}

#[cfg(unix)]
#[test]
fn a_page_kept_as_a_link_is_never_written_through() {
    let site = Site::new("link");
    let text = "---\ntitle: Post 1\ndate: =today\n---\n";
    site.write("content/.kept/post-1.md", text);
    std::os::unix::fs::symlink(
        site.path("content/.kept/post-1.md"),
        site.path("content/post-1.md"),
    )
    .unwrap();
    let (said, code) = site.stamp();
    assert_eq!(
        said,
        "\
error  Can't write `content/post-1.md`: it is a link
  More: https://statac.dev/e/cant-write/
"
    );
    assert_eq!(code, Some(1));
    assert_eq!(site.read("content/.kept/post-1.md"), text);
    let link = fs::symlink_metadata(site.path("content/post-1.md")).unwrap();
    assert!(link.is_symlink());
}

#[test]
fn the_help_names_the_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_statac"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(
        help.contains("\n  statac stamp [folder]  Write the date over every stamp"),
        "{help}"
    );
}

#[test]
fn a_folder_that_isnt_there_is_said_to_be_missing() {
    let site = Site::new("missing");
    fs::remove_dir_all(&site.0).unwrap();
    let (said, code) = site.stamp();
    assert!(said.starts_with("error  Can't read `"), "{said}");
    assert!(said.contains("it doesn't exist"), "{said}");
    assert_eq!(code, Some(1));
}

/// What `statac stamp`, run within this program so that the name it
/// writes a file's new text under is known, has to tell.
#[cfg(unix)]
fn told(site: &Site) -> Vec<String> {
    let outcome = statac::stamp::run(&statac::stamp::Options {
        site: site.0.clone(),
        today: statac::date::read("2026-10-06").ok(),
    });
    assert!(outcome.pass.stamped.is_empty());
    let problems = outcome.problems.iter();
    problems.map(|problem| problem.headline()).collect()
}

#[cfg(unix)]
#[test]
fn nothing_outside_the_writing_is_written_through_a_link() {
    use std::os::unix::fs::symlink;
    let text = "---\ntitle: Post 1\ndate: =today\n---\n";
    let kept = "Kept outside.\n";

    // The page's folder is a link.
    let site = Site::new("link-folder");
    site.write("content/.kept/post-1.md", text);
    symlink(site.path("content/.kept"), site.path("content/posts")).unwrap();
    assert_eq!(
        told(&site),
        ["Can't write `content/posts/post-1.md`: it is a link"]
    );
    assert_eq!(site.read("content/.kept/post-1.md"), text);
    assert_eq!(fs::read_dir(site.path("content/.kept")).unwrap().count(), 1);

    // The name the new text would be written under is a link leading out.
    let site = Site::new("link-beside");
    site.write("content/post-1.md", text);
    site.write("outside.txt", kept);
    let beside = site
        .path("content")
        .join(statac::platform::beside("post-1.md"));
    symlink(site.path("outside.txt"), &beside).unwrap();
    assert_eq!(
        told(&site),
        ["Can't write `content/post-1.md`: file exists"]
    );
    assert_eq!(site.read("content/post-1.md"), text);
    assert_eq!(site.read("outside.txt"), kept);
    assert!(fs::symlink_metadata(&beside).unwrap().is_symlink());
}

/// A file's name and a field's name are the site's own text, and a site
/// may be someone else's: neither reaches the terminal as written.
#[cfg(unix)]
#[test]
fn a_name_holding_what_a_terminal_acts_on_is_shown_harmless() {
    let site = Site::new("escapes");
    // An escape, a carriage return and a line feed, in the file's name and
    // in the name of a field nested in it.
    site.write(
        "content/post\u{1b}[2J\r-\n1.md",
        "---\ntitle: Post 1\nevent:\n  \"at\\e[2J\\r\\nfirst\": =today\n---\n",
    );
    let (said, code) = site.stamp();
    assert_eq!(
        said,
        "Stamped `content/post\u{241b}[2J\u{240d}-\u{240a}1.md`: \
         `event.at\u{241b}[2J\u{240d}\u{240a}first` is `2026-10-06`.\n"
    );
    assert_eq!(code, Some(0));
}

#[test]
fn an_id_is_written_for_a_post_new_since_the_site_was_published() {
    let site = Site::new("ids");
    site.write(
        "statac.yaml",
        &format!(
            "{SETTINGS}url: https://example.org\nfeed_key: key\nwrite_feed_ids: true\nfeed: true\n"
        ),
    );
    site.write(
        "templates/feed.xml",
        "{% for post in posts %}{{ feed_id(post) }} {% endfor %}\n",
    );
    let first = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nThe first post.\n";
    site.write("content/post-1.md", first);
    // No build has kept which posts feed readers know.
    let unwritten = "Wrote no feed ids: they start once `statac build` has published the site.\n";
    assert_eq!(site.stamp(), (unwritten.to_owned(), Some(0)));
    assert_eq!(site.read("content/post-1.md"), first);

    let (said, code) = site.run("build", &["--today=2026-10-06"]);
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(site.stamp(), ("Nothing to stamp.\n".to_owned(), Some(0)));

    // A new post, with a stamp of its own, is written once with both.
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\nupdated: =today\n---\nThe second post.\n",
    );
    let (said, code) = site.stamp();
    assert_eq!(
        said,
        "Stamped `content/post-2.md`: `updated` is `2026-10-06`.\n\
         Wrote an `id` into `content/post-2.md`.\n"
    );
    assert_eq!(code, Some(0));
    let second = site.read("content/post-2.md");
    let id = second
        .strip_prefix("---\ntitle: Post 2\ndate: 2026-01-02\nupdated: 2026-10-06\nid: ")
        .and_then(|rest| rest.strip_suffix("\n---\nThe second post.\n"))
        .unwrap_or_else(|| panic!("{second:?}"));
    assert_eq!(id.len(), 36, "{id}");
    assert_eq!(site.read("content/post-1.md"), first);
    assert_eq!(site.stamp(), ("Nothing to stamp.\n".to_owned(), Some(0)));
    assert_eq!(site.read("content/post-2.md"), second);

    // Front matter an `id:` line can't follow is told and left.
    let flow = "---\n{title: Post 3, date: 2026-01-03}\n---\nThe third post.\n";
    site.write("content/post-3.md", flow);
    let (said, code) = site.stamp();
    assert!(
        said.starts_with(
            "error  Can't write `content/post-3.md`: \
             its front matter has no place for an `id:` line\n"
        ),
        "{said}"
    );
    assert_eq!(code, Some(1));
    assert_eq!(site.read("content/post-3.md"), flow);
}

#[test]
fn no_id_is_written_for_a_post_made_from_an_item() {
    let site = Site::new("item-ids");
    site.write(
        "statac.yaml",
        &format!(
            "{SETTINGS}url: https://example.org\nfeed_key: key\nwrite_feed_ids: true\nfeed: true\n"
        ),
    );
    site.write(
        "templates/feed.xml",
        "{% for post in posts %}{{ feed_id(post) }} {% endfor %}\n",
    );
    let page = "---\ntitle: Items\ntype: page\neach: data.items\npermalink: /items/{key}/\n---\n";
    site.write("content/items.md", page);
    site.write(
        "data/items.yaml",
        "one:\n  title: Item 1\n  date: 2026-01-01\n",
    );
    let (said, code) = site.run("build", &["--today=2026-10-06"]);
    assert_eq!(code, Some(0), "{said}");

    // One item moved, and one new.
    let moved = page.replace("/items/", "/elsewhere/");
    site.write("content/items.md", &moved);
    let items =
        "one:\n  title: Item 1\n  date: 2026-01-01\ntwo:\n  title: Item 2\n  date: 2026-01-02\n";
    site.write("data/items.yaml", items);
    let (said, code) = site.stamp();
    assert!(!said.contains("Wrote an `id`"), "{said}");
    assert_eq!(code, Some(0), "{said}");
    assert_eq!(site.read("data/items.yaml"), items);
    assert_eq!(site.read("content/items.md"), moved);
}
