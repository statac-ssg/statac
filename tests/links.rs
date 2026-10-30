//! `statac refresh` checks the links a site's pages hold to other sites,
//! where `check_external_links: true` asks, and records each answer in
//! `fetched/links`; `check` tells each broken one where it was written,
//! and counts the rest; `build` says how old the answers are. Each test
//! runs on a small site of its own in the system's temporary folder, with
//! a looker standing in for the network, which no test touches.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use statac::fetch::{Ending, Fetched, Looked, Method, Refusal};
use statac::refresh::{self, Options};

const SETTINGS: &str = "title: Site\nlanguage: en\ntimezone: Europe/London\n\
                        url: https://site.example/\ncheck_external_links: true\n";

/// A layout writing a link of its own to another site, and one to the
/// site's own host, around the page's content.
const LAYOUT: &str = "<!doctype html>\n<title>{{ page.title }}</title>\n\
                      <a href=\"https://elsewhere.example/about/\">About</a>\n\
                      <a href=\"https://site.example/\">Home</a>\n\
                      {{ page.content }}\n";

struct Site(PathBuf);

impl Site {
    fn new(label: &str, settings: &str) -> Site {
        let path =
            std::env::temp_dir().join(format!("statac-links-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        let site = Site(path);
        site.write("statac.yaml", settings);
        site.write("templates/page.html", LAYOUT);
        site.write("templates/home.html", LAYOUT);
        site.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        site.write(
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n\
             Read [the news](https://news.example/today/#top), [the old page](http://old.example/page),\n\
             [the lost one](https://lost.example/gone/) and [the wall](https://walled.example/).\n\n\
             ![A chart](https://images.example/chart.png)\n\n\
             Fetch [the notes](ftp://files.example/notes.txt), or read [post 2](/post-2/).\n",
        );
        site.write(
            "content/post-2.md",
            "---\ntitle: Post 2\n---\nSee [the news](https://news.example/today/) too.\n",
        );
        site.write(
            "content/draft.md",
            "---\ntitle: Draft\ndraft: true\n---\nSee [nowhere](https://nowhere.example/).\n",
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

    fn read(&self, relative: &str) -> Option<String> {
        fs::read_to_string(self.path(relative)).ok()
    }

    /// What `refresh` says on `today`, asking with `looker`, and its exit
    /// code.
    fn refresh(&self, today: &str, looker: &dyn refresh::Look) -> (String, u8) {
        let options = Options {
            site: self.0.clone(),
            today: statac::date::read(today).ok(),
            token: None,
        };
        let fetcher =
            |address: &str, _: u64| -> Result<Fetched, Refusal> { panic!("fetched {address}") };
        let mut out = Vec::new();
        let code = refresh::run(&options, &fetcher, looker, &mut out, false);
        (String::from_utf8(out).unwrap(), code)
    }

    /// What the program says of the site, run with `command` on `today`,
    /// and its exit code.
    fn run(&self, command: &str, today: &str) -> (String, Option<i32>) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .arg(command)
            .arg(&self.0)
            .arg(format!("--today={today}"))
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        let said = String::from_utf8(output.stdout).unwrap();
        assert!(output.stderr.is_empty(), "{command}: {said}");
        let shown = self.0.display().to_string();
        (said.replace(&shown, "SITE"), output.status.code())
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A looker standing in for the network, answering each site as the
/// links on the test site need, and remembering each URL asked.
struct StandIn {
    asked: Mutex<Vec<String>>,
}

impl StandIn {
    fn new() -> StandIn {
        StandIn {
            asked: Mutex::default(),
        }
    }

    fn asked(&self) -> Vec<String> {
        let mut asked = self.asked.lock().unwrap().clone();
        asked.sort_unstable();
        asked.dedup();
        asked
    }
}

impl refresh::Look for StandIn {
    fn look(&self, address: &str, _: Method) -> Looked {
        self.asked.lock().unwrap().push(address.to_owned());
        let status = |status| Ending::Status {
            status,
            challenge: false,
            retry_after: None,
        };
        let (redirects, end) = match address {
            "http://old.example/page" => (
                vec![(301, "https://new.example/page".parse().unwrap())],
                status(200),
            ),
            "https://lost.example/gone/" => (Vec::new(), status(404)),
            "https://walled.example/" => (Vec::new(), status(403)),
            "https://nowhere.example/" => (
                Vec::new(),
                Ending::Refused(Refusal::UnknownHost {
                    host: "nowhere.example".into(),
                }),
            ),
            _ => (Vec::new(), status(200)),
        };
        Looked { redirects, end }
    }

    fn pause(&self) -> Duration {
        Duration::ZERO
    }
}

/// What asks about links for a site that doesn't check them, which never
/// asks.
struct Unasked;

impl refresh::Look for Unasked {
    fn look(&self, address: &str, _: Method) -> Looked {
        panic!("asked about {address}")
    }

    fn pause(&self) -> Duration {
        Duration::ZERO
    }
}

#[test]
fn refresh_checks_each_link_a_reader_follows_to_another_site_once() {
    let site = Site::new("refreshed", SETTINGS);
    let looker = StandIn::new();
    let (said, code) = site.refresh("2026-10-06", &looker);
    assert_eq!(code, 0);
    assert_eq!(
        said,
        "Checking 6 external links on 6 sites.\n\
         Checked 6 external links: 3 work, 2 broken, 1 didn't answer. \
         Run `statac check` to see the broken ones.\n"
    );
    // Not the image, the FTP link, the site's own pages, nor the fragment.
    assert_eq!(
        looker.asked(),
        [
            "http://old.example/page",
            "https://elsewhere.example/about/",
            "https://lost.example/gone/",
            "https://news.example/today/",
            "https://nowhere.example/",
            "https://walled.example/",
        ]
    );
    assert_eq!(
        site.read("fetched/links").unwrap(),
        "statac links 1\n\
         http://old.example/page\tmoved https://new.example/page\t2026-10-06\n\
         https://elsewhere.example/about/\tok\t2026-10-06\n\
         https://lost.example/gone/\t404\t2026-10-06\n\
         https://news.example/today/\tok\t2026-10-06\n\
         https://nowhere.example/\tno-host\t2026-10-06\n\
         https://walled.example/\tunanswered 403\t2026-10-06\n"
    );

    // Nothing is due again for days.
    let (said, _) = site.refresh("2026-10-07", &Unasked);
    assert_eq!(said, "No external links are due to be checked.\n");
}

#[test]
fn check_tells_each_broken_link_where_it_was_written_and_counts_the_rest() {
    let site = Site::new("checked", SETTINGS);
    site.refresh("2026-10-06", &StandIn::new());
    let (said, code) = site.run("check", "2026-10-09");
    assert_eq!(code, Some(0), "{said}");
    // `check` leaves the draft out, and its link with it.
    assert_eq!(
        said,
        "note  Draft left out: `content/draft.md`
  content/draft.md, line 3
   3 │ draft: true
     │        ^^^^
  Run with `--drafts` to build it as if published.
  More: https://statac.dev/e/draft-left-out/

note  External link moved: `http://old.example/page` redirects permanently to \
         `https://new.example/page`
  content/post-1.md, line 4
   4 │ Read [the news](https://news.example/today/#top), [the old page](http://old.example/page),
     │                                                                  ^^^^^^^^^^^^^^^^^^^^^^^
  Link to `https://new.example/page` instead.
  More: https://statac.dev/e/external-link-moved/

warning  Broken external link: `https://lost.example/gone/` answered 404 Not Found
  content/post-1.md, line 5
   5 │ [the lost one](https://lost.example/gone/) and [the wall](https://walled.example/).
     │                ^^^^^^^^^^^^^^^^^^^^^^^^^^
  `statac refresh` checked it 3 days ago. Change or remove the link.
  More: https://statac.dev/e/broken-external-link/

External links:
  5 checked by `statac refresh`, the oldest 3 days ago: 3 work, 1 broken, 1 didn't answer.

1 warning: 1 broken-external-link.
2 notes: 1 draft-left-out, 1 external-link-moved.
"
    );
    let (said, _) = site.run("check", "2026-10-09");
    let drafts = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["check", "--drafts"])
        .arg(&site.0)
        .arg("--today=2026-10-09")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let drafts = String::from_utf8(drafts.stdout).unwrap();
    assert!(!said.contains("nowhere.example"), "{said}");
    assert!(
        drafts.contains(
            "warning  Broken external link: nothing answers to the name `nowhere.example`
  content/draft.md, line 5
   5 │ See [nowhere](https://nowhere.example/).
     │               ^^^^^^^^^^^^^^^^^^^^^^^^
"
        ),
        "{drafts}"
    );

    // A broken link a layout writes is told in the layout.
    let record = site.read("fetched/links").unwrap();
    site.write(
        "fetched/links",
        &record.replace(
            "https://elsewhere.example/about/\tok",
            "https://elsewhere.example/about/\t410",
        ),
    );
    let (said, _) = site.run("check", "2026-10-09");
    assert!(
        said.contains(
            "warning  Broken external link: `https://elsewhere.example/about/` answered 410 Gone
  templates/page.html, line 3
   3 │ <a href=\"https://elsewhere.example/about/\">About</a>
     │          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  While building: content/post-1.md
"
        ),
        "{said}"
    );
}

#[test]
fn build_says_how_old_the_answers_are_and_counts_what_check_tells() {
    let site = Site::new("built", SETTINGS);
    site.refresh("2026-10-06", &StandIn::new());
    let (said, code) = site.run("build", "2026-10-16");
    assert_eq!(code, Some(0), "{said}");
    // The draft isn't built, so its link isn't counted.
    assert!(
        said.starts_with(
            "1 warning: 1 broken-external-link.\n\
             2 notes: 1 draft-left-out, 1 external-link-moved. Run `statac check` to see them.\n"
        ),
        "{said}"
    );
    assert!(
        said.contains("Fetched: 5 external links checked, the oldest 10 days ago.\n"),
        "{said}"
    );
}

#[test]
fn links_not_checked_yet_and_those_the_list_leaves_out_are_counted() {
    let settings = format!("{SETTINGS}unchecked_external_links:\n  - https://walled.example\n");
    let site = Site::new("counted", &settings);
    let (said, _) = site.run("check", "2026-10-06");
    assert!(
        said.contains(
            "External links:
  4 not checked yet. Run `statac refresh` to check them.
  1 left out by `unchecked_external_links`.
"
        ),
        "{said}"
    );
    let looker = StandIn::new();
    site.refresh("2026-10-06", &looker);
    assert!(!looker.asked().iter().any(|url| url.contains("walled")));
    assert!(!site.read("fetched/links").unwrap().contains("walled"));
    let (said, _) = site.run("check", "2026-10-06");
    assert!(
        said.contains(
            "External links:
  4 checked by `statac refresh`, the oldest today: 3 work, 1 broken.
  1 left out by `unchecked_external_links`.
"
        ),
        "{said}"
    );
}

#[test]
fn a_site_that_doesnt_ask_is_never_checked_and_its_record_never_told() {
    let site = Site::new(
        "unasked",
        "title: Site\nlanguage: en\ntimezone: Europe/London\n",
    );
    let (said, code) = site.refresh("2026-10-06", &Unasked);
    assert_eq!((said.as_str(), code), ("Nothing to fetch.\n", 0));
    assert_eq!(site.read("fetched/links"), None);
    site.write(
        "fetched/links",
        "statac links 1\nhttps://lost.example/gone/\t404\t2026-10-06\n",
    );
    let (said, _) = site.run("check", "2026-10-06");
    assert!(!said.contains("external"), "{said}");
    assert!(!said.contains("External"), "{said}");
}

#[test]
fn a_line_of_the_record_not_as_statac_writes_it_stops_the_build_there() {
    let site = Site::new("malformed", SETTINGS);
    site.write(
        "fetched/links",
        "statac links 1\nhttps://lost.example/gone/\tbroken\t2026-10-06\n",
    );
    let (said, code) = site.run("check", "2026-10-06");
    assert_eq!(code, Some(1));
    assert!(
        said.contains(
            "error  Malformed record: `broken` isn't an answer Statac records
  fetched/links, line 2
   2 │ https://lost.example/gone/  broken  2026-10-06
     │                             ^^^^^^
  Delete the line. The next `statac refresh` checks the link again.
"
        ),
        "{said}"
    );
}
