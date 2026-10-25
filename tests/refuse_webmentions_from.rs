//! `refuse_webmentions_from:` in `statac.yaml` turns away the webmentions
//! sent from a URL, or by a sender, at or under each line: a build leaves
//! them out at once, and `statac refresh` removes their files for good.
//! Each test runs the program on a small site of its own in the system's
//! temporary folder; none touches the network.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use image::{ImageFormat, Rgb, RgbImage};

const LAYOUT: &str = "<!doctype html>\n<title>{{ headline(page) }}</title>\n{{ page.content }}\n\
                      {% for mention in page.webmentions.comments %}<a href=\"{{ mention.url }}\">\
                      {{ mention.author }}</a>{% if mention.avatar %}<img src=\"{{ mention.avatar.src }}\">\
                      {% endif %}\n{% endfor %}";

const SETTINGS: &str = "title: Site\nlanguage: en\ntimezone: Europe/London\npages.feed: false\n\
                        url: https://example.org/\nwebmentions: true\n";

/// What `refresh` says where it asks the webmention service nothing.
const NO_TOKEN: &str =
    "Webmentions weren't asked for: no token is in `STATAC_WEBMENTION_TOKEN` or `.statac/token`.\n";

/// One mention of `post-1`, as its file holds it: its id, the page that
/// sent it, the sender's own URL, and the name of its face.
struct Sent<'a> {
    id: u64,
    url: &'a str,
    author_url: Option<&'a str>,
    face: Option<&'a str>,
}

const FACE_1: &str = "0000000000000001.jpg";
const FACE_2: &str = "0000000000000002.jpg";

fn sent<'a>(id: u64, url: &'a str, author_url: Option<&'a str>, face: Option<&'a str>) -> Sent<'a> {
    Sent {
        id,
        url,
        author_url,
        face,
    }
}

impl Sent<'_> {
    fn text(&self) -> String {
        let mut text = "kind: \"replied\"\nauthor: \"A\"\n".to_owned();
        if let Some(author_url) = self.author_url {
            text.push_str(&format!("author_url: \"{author_url}\"\n"));
        }
        if let Some(face) = self.face {
            text.push_str(&format!("face: \"{face}\"\n"));
        }
        text.push_str(&format!(
            "url: \"{}\"\ntarget: \"https://example.org/post-1/\"\n\
             fetched: \"2026-10-05\"\nwords: \"Words.\"\n",
            self.url
        ));
        text
    }
}

fn jpeg() -> Vec<u8> {
    let picture = RgbImage::from_fn(8, 8, |x, y| Rgb([(x * 30) as u8, (y * 30) as u8, 90]));
    let mut bytes = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut bytes, ImageFormat::Jpeg).unwrap();
    bytes.into_inner()
}

fn hash_of(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex()[..16].to_owned()
}

struct Site(PathBuf);

impl Site {
    /// A site with one post, `post-1`, and `mentions` of it kept as
    /// `statac refresh` keeps them, each with its line in the record, and
    /// `refused` listed in `refuse_webmentions_from:`.
    fn new(label: &str, mentions: &[Sent<'_>], refused: &[&str]) -> Site {
        let path = std::env::temp_dir().join(format!(
            "statac-refuse-webmentions-{label}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        let site = Site(path);
        site.refuse(refused);
        site.write("templates/page.html", LAYOUT);
        site.write("templates/home.html", LAYOUT);
        site.write("content/index.md", "---\ntitle: Home\n---\nHome.\n");
        site.write("content/post-1.md", "---\ntitle: Post 1\n---\nPost 1.\n");
        let mut lines = Vec::new();
        let mut faces: Vec<&str> = mentions.iter().filter_map(|sent| sent.face).collect();
        faces.sort_unstable();
        faces.dedup();
        for face in faces {
            let bytes = jpeg();
            site.put(&format!("fetched/faces/{face}"), &bytes);
            lines.push(format!("faces/{face}\t{}\t2026-10-05", hash_of(&bytes)));
        }
        for sent in mentions {
            let text = sent.text();
            site.write(&format!("fetched/mentions/{}.yaml", sent.id), &text);
            lines.push(format!(
                "mentions/{}.yaml\t{}\t2026-10-05",
                sent.id,
                hash_of(text.as_bytes())
            ));
        }
        lines.sort_unstable();
        let lines: String = lines.iter().map(|line| format!("{line}\n")).collect();
        site.write("fetched/record", &format!("statac fetched 1\n{lines}"));
        site
    }

    /// Writes the settings, listing `refused` in `refuse_webmentions_from:`
    /// where there are any.
    fn refuse(&self, refused: &[&str]) {
        let mut settings = SETTINGS.to_owned();
        if !refused.is_empty() {
            settings.push_str("refuse_webmentions_from:\n");
            for line in refused {
                settings.push_str(&format!("  - {line}\n"));
            }
        }
        self.write("statac.yaml", &settings);
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.0.clone(), |path, piece| path.join(piece))
    }

    fn write(&self, relative: &str, text: &str) {
        self.put(relative, text.as_bytes());
    }

    fn put(&self, relative: &str, bytes: &[u8]) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap()
    }

    fn holds(&self, relative: &str) -> bool {
        fs::symlink_metadata(self.path(relative)).is_ok()
    }

    /// The names in `fetched/` and the folders in it, in order.
    fn fetched(&self) -> Vec<String> {
        let mut names = Vec::new();
        for folder in ["", "faces/", "mentions/"] {
            let Ok(entries) = fs::read_dir(self.path(&format!("fetched/{folder}"))) else {
                continue;
            };
            for entry in entries {
                let name = entry.unwrap().file_name().to_string_lossy().into_owned();
                if !matches!(name.as_str(), "faces" | "mentions") || !folder.is_empty() {
                    names.push(format!("{folder}{name}"));
                }
            }
        }
        names.sort_unstable();
        names
    }

    /// What the program says of the site, run with `args` after it on
    /// Tuesday 6 October 2026, and its exit code, with no token for the
    /// webmention service. Nothing is ever said on standard error.
    fn run(&self, args: &[&str]) -> (String, Option<i32>) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(args)
            .arg(format!("--site={}", self.0.display()))
            .arg("--today=2026-10-06")
            .env("NO_COLOR", "1")
            .env_remove("STATAC_WEBMENTION_TOKEN")
            .output()
            .unwrap();
        let said = String::from_utf8(output.stdout).unwrap();
        assert!(output.stderr.is_empty(), "{args:?}: {said}");
        let shown = self.0.display().to_string();
        (said.replace(&shown, "SITE"), output.status.code())
    }

    /// The built `post-1`.
    fn page(&self) -> String {
        let (said, code) = self.run(&["build"]);
        assert_eq!(code, Some(0), "{said}");
        self.read("_site/post-1/index.html")
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The record's lines that aren't the opening, each without its hash.
fn record(site: &Site) -> Vec<String> {
    (site.read("fetched/record").lines().skip(1))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields[1] {
                "absent" => line.to_owned(),
                _ => format!("{}\t{}", fields[0], fields[2]),
            }
        })
        .collect()
}

/// Two replies from `a.example`, sharing a face, and one from
/// `b.example`.
fn three() -> Vec<Sent<'static>> {
    vec![
        sent(
            11,
            "https://a.example/post-1/",
            Some("https://a.example/"),
            Some(FACE_1),
        ),
        sent(
            12,
            "https://a.example/post-2/",
            Some("https://a.example/"),
            Some(FACE_1),
        ),
        sent(
            13,
            "https://b.example/post-3/",
            Some("https://b.example/"),
            Some(FACE_2),
        ),
    ]
}

#[test]
fn a_mention_is_refused_by_its_own_url_however_it_is_spelt() {
    for (n, line) in [
        "https://a.example/post-1/",
        "http://a.example/post-1/",
        "https://www.a.example/post-1",
        "https://A.Example/post-1/",
        "HTTP://WWW.A.EXAMPLE/post-1",
    ]
    .into_iter()
    .enumerate()
    {
        let site = Site::new(&format!("one-{n}"), &three(), &[line]);
        // Left out at once, its file still there.
        let page = site.page();
        assert!(!page.contains("https://a.example/post-1/"), "{line}");
        assert!(page.contains("https://a.example/post-2/"), "{line}");
        assert!(site.holds("fetched/mentions/11.yaml"));

        let (said, code) = site.run(&["refresh"]);
        assert_eq!(
            said,
            format!(
                "Removed `fetched/mentions/11.yaml`.\n\
                 Removed 1 refused webmention.\n{NO_TOKEN}"
            )
        );
        assert_eq!(code, Some(0));
        // The face it shares with a mention kept stays.
        assert_eq!(
            site.fetched(),
            [
                format!("faces/{FACE_1}"),
                format!("faces/{FACE_2}"),
                "mentions/12.yaml".into(),
                "mentions/13.yaml".into(),
                "record".into(),
            ]
        );
        assert_eq!(
            record(&site),
            [
                format!("faces/{FACE_1}\t2026-10-05"),
                format!("faces/{FACE_2}\t2026-10-05"),
                "mentions/11.yaml\tabsent\t2026-10-06".into(),
                "mentions/12.yaml\t2026-10-05".into(),
                "mentions/13.yaml\t2026-10-05".into(),
            ]
        );
        assert!(!site.page().contains("https://a.example/post-1/"));
    }
}

#[test]
fn a_sender_is_refused_with_everything_at_or_under_its_url() {
    let mentions = [
        sent(
            11,
            "https://a.example/post-1/",
            Some("https://a.example/"),
            Some(FACE_1),
        ),
        sent(12, "https://a.example/post-2/", None, Some(FACE_1)),
        // From a page elsewhere, by its sender's URL alone.
        sent(
            13,
            "https://c.example/post-3/",
            Some("https://www.a.example"),
            Some(FACE_2),
        ),
        sent(
            14,
            "https://b.example/post-4/",
            Some("https://b.example/"),
            Some(FACE_2),
        ),
        // The same host on another port is another site.
        sent(15, "https://a.example:8080/post-5/", None, None),
        sent(
            16,
            "https://b.example/post-6/",
            Some("https://a.example.org/"),
            None,
        ),
    ];
    let site = Site::new("sender", &mentions, &["HTTP://A.example"]);
    let (said, code) = site.run(&["refresh"]);
    assert_eq!(
        said,
        format!(
            "Removed `fetched/mentions/11.yaml`.\n\
             Removed `fetched/mentions/12.yaml`.\n\
             Removed `fetched/mentions/13.yaml`.\n\
             Removed `fetched/faces/{FACE_1}`.\n\
             Removed 3 refused webmentions and 1 avatar.\n{NO_TOKEN}"
        )
    );
    assert_eq!(code, Some(0));
    assert_eq!(
        site.fetched(),
        [
            format!("faces/{FACE_2}"),
            "mentions/14.yaml".into(),
            "mentions/15.yaml".into(),
            "mentions/16.yaml".into(),
            "record".into(),
        ]
    );
    assert_eq!(
        record(&site),
        [
            format!("faces/{FACE_2}\t2026-10-05"),
            "mentions/11.yaml\tabsent\t2026-10-06".into(),
            "mentions/12.yaml\tabsent\t2026-10-06".into(),
            "mentions/13.yaml\tabsent\t2026-10-06".into(),
            "mentions/14.yaml\t2026-10-05".into(),
            "mentions/15.yaml\t2026-10-05".into(),
            "mentions/16.yaml\t2026-10-05".into(),
        ]
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!((said.as_str(), code), ("No problems.\n", Some(0)));

    // Nothing more is left to remove.
    let (said, code) = site.run(&["refresh"]);
    assert_eq!((said, code), (NO_TOKEN.to_owned(), Some(0)));
}

#[test]
fn a_url_is_refused_with_what_is_under_it_only_at_a_boundary() {
    let mentions = [
        sent(11, "https://a.example/a/", None, None),
        sent(12, "https://a.example/a/post-2/", None, None),
        sent(13, "https://a.example/ab/post-3/", None, None),
    ];
    let site = Site::new("boundary", &mentions, &["https://a.example/a"]);
    let (said, code) = site.run(&["refresh"]);
    assert_eq!(
        said,
        format!(
            "Removed `fetched/mentions/11.yaml`.\n\
             Removed `fetched/mentions/12.yaml`.\n\
             Removed 2 refused webmentions.\n{NO_TOKEN}"
        )
    );
    assert_eq!(code, Some(0));
    assert_eq!(site.fetched(), ["mentions/13.yaml", "record"]);
}

#[test]
fn check_notes_once_how_many_the_list_leaves_out() {
    let site = Site::new(
        "noted",
        &three(),
        &["https://a.example/", "https://d.example/"],
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(
        said,
        "note  Refused webmentions: 2 in `fetched/mentions/` are left out\n  \
         statac.yaml, line 8\n   \
         8 │   - https://a.example/\n     \
         │   ^^^^^^^^^^^^^^^^^^^^ runs to line 9\n  \
         Run `statac refresh` to delete their files.\n  \
         More: https://statac.dev/e/refused-webmentions/\n\n\
         1 note: 1 refused-webmentions.\n"
    );
    assert_eq!(code, Some(0));
    let (said, _) = site.run(&["build"]);
    assert!(
        said.starts_with("1 note: 1 refused-webmentions. Run `statac check` to see it.\n"),
        "{said}"
    );

    site.write(
        "statac.yaml",
        &format!(
            "{SETTINGS}refuse_webmentions_from: https://b.example/post-3/\n\
             quiet: [refused-webmentions]\n"
        ),
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!((said.as_str(), code), ("1 note, quietened.\n", Some(0)));
    assert!(!site.page().contains("https://b.example/post-3/"));
}

#[test]
fn a_line_that_isnt_a_full_url_is_told_where_it_is() {
    let site = Site::new(
        "not-a-url",
        &three(),
        &["a.example", "https://b.example/", "ftp://c.example/"],
    );
    let (said, code) = site.run(&["check"]);
    // The line that is a URL refuses what it refuses.
    assert_eq!(
        said,
        "note  Refused webmentions: 1 in `fetched/mentions/` is left out\n  \
         statac.yaml, line 8\n   \
         8 │   - a.example\n     \
         │   ^^^^^^^^^^^ runs to line 10\n  \
         Run `statac refresh` to delete its file.\n  \
         More: https://statac.dev/e/refused-webmentions/\n\n\
         error  Wrong kind of value: `refuse_webmentions_from` is a full URL, such as \
         `https://example.org`, not `a.example`\n  \
         statac.yaml, line 8\n   \
         8 │   - a.example\n     \
         │     ^^^^^^^^^\n  \
         Write it as `https://a.example`.\n  \
         More: https://statac.dev/e/wrong-kind-of-value/\n\n\
         error  Wrong kind of value: `refuse_webmentions_from` is a full URL, such as \
         `https://example.org`, not `ftp://c.example/`\n  \
         statac.yaml, line 10\n  \
         10 │   - ftp://c.example/\n     \
         │     ^^^^^^^^^^^^^^^^\n  \
         More: https://statac.dev/e/wrong-kind-of-value/\n\n\
         2 errors: 2 wrong-kind-of-value.\n\
         1 note: 1 refused-webmentions.\n"
    );
    assert_eq!(code, Some(1));
    let (_, code) = site.run(&["refresh"]);
    assert_eq!(code, Some(1));
    assert!(site.holds("fetched/mentions/13.yaml"));
}

#[test]
fn a_list_that_refuses_nothing_kept_changes_nothing_built() {
    let built = |site: &Site| {
        let (said, code) = site.run(&["build"]);
        assert_eq!(code, Some(0), "{said}");
        let said = said.split(" in ").next().unwrap().to_owned();
        let root = site.path("_site");
        let mut files = Vec::new();
        let mut folders = vec![root.clone()];
        while let Some(folder) = folders.pop() {
            for entry in fs::read_dir(folder).unwrap() {
                let path = entry.unwrap().path();
                match path.is_dir() {
                    true => folders.push(path),
                    false => files.push((
                        path.strip_prefix(&root).unwrap().to_owned(),
                        fs::read(&path).unwrap(),
                    )),
                }
            }
        }
        files.sort();
        (said, files)
    };
    let unlisted = Site::new("unlisted", &three(), &[]);
    let listed = Site::new("listed", &three(), &["https://d.example/"]);
    assert!(built(&unlisted) == built(&listed));
}

#[test]
fn an_old_list_of_refused_is_told_with_the_lines_that_carry_it_over() {
    let site = Site::new("retired", &three(), &[]);
    let list = "statac refused 1\n\
                mention\thttps://a.example/post-1/\t2026-10-06\n\
                sender\thttps://b.example/\t2026-10-06\n";
    site.write("fetched/refused", list);
    let told = "error  Retired file: Statac no longer reads `fetched/refused`\n  \
                To keep refusing what it lists, add these lines to `statac.yaml`:\n    \
                refuse_webmentions_from:\n      \
                - https://a.example/post-1/\n      \
                - https://b.example/\n  \
                Delete `fetched/refused` after adding them.\n  \
                More: https://statac.dev/e/retired-file/\n";
    let told = format!("{told}\n1 error: 1 retired-file.\n");
    for command in ["check", "refresh"] {
        assert_eq!(site.run(&[command]), (told.clone(), Some(1)), "{command}");
    }
    assert_eq!(site.read("fetched/refused"), list);
    assert!(site.holds("fetched/mentions/11.yaml"));

    // A line that doesn't read is counted, and its URL left to the author.
    site.write("fetched/refused", &format!("{list}sender\tb.example\n"));
    let (said, _) = site.run(&["check"]);
    assert!(
        said.contains(
            "  Delete `fetched/refused` after adding them, and the URL on the line that \
             couldn't be read.\n"
        ),
        "{said}"
    );

    // With the lines added and the file deleted, the site builds.
    site.refuse(&["https://a.example/post-1/", "https://b.example/"]);
    fs::remove_file(site.path("fetched/refused")).unwrap();
    let (said, code) = site.run(&["refresh"]);
    assert_eq!(code, Some(0), "{said}");
    assert!(
        said.starts_with("Removed `fetched/mentions/11.yaml`.\n"),
        "{said}"
    );
}

#[test]
fn statac_refuse_is_refused_with_the_setting_named() {
    let site = Site::new("command", &three(), &[]);
    let output = Command::new(env!("CARGO_BIN_EXE_statac"))
        .args(["refuse", "https://a.example/post-1/"])
        .arg(&site.0)
        .env("NO_COLOR", "1")
        .output()
        .unwrap();
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(
        said.starts_with(
            "error  Unknown command: `refuse`\n  \
             Statac does this with `refuse_webmentions_from:` in `statac.yaml`.\n  \
             More: https://statac.dev/e/unknown-command/\n"
        ),
        "{said}"
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(site.holds("fetched/mentions/11.yaml"));
}

#[test]
fn a_retired_problems_name_is_told_it_no_longer_exists() {
    let site = Site::new("retired-name", &[], &[]);
    site.write("statac.yaml", &format!("{SETTINGS}quiet: [missing-url]\n"));
    let (said, code) = site.run(&["check"]);
    assert_eq!(
        said,
        "error  Unknown problem name: `missing-url`\n  \
         statac.yaml, line 7\n   \
         7 │ quiet: [missing-url]\n     \
         │         ^^^^^^^^^^^\n  \
         This kind of problem no longer exists.\n  \
         More: https://statac.dev/e/unknown-problem-name/\n\n\
         1 error: 1 unknown-problem-name.\n"
    );
    assert_eq!(code, Some(1));
}

#[cfg(unix)]
#[test]
fn nothing_outside_fetched_is_removed_through_a_link() {
    use std::os::unix::fs::symlink;
    let outside = |site: &Site, from: &str, to: &str| {
        fs::rename(site.path(from), site.path(to)).unwrap();
        symlink(site.path(to), site.path(from)).unwrap();
    };
    let out_of_fetched = |said: &str, link: &str| {
        said.starts_with(&format!(
            "error  Outside the site: the link `{link}` leads out of `fetched/`\n"
        ))
    };

    // A link in a mention's place, out of `fetched/` and within it.
    let site = Site::new("link-mention", &three(), &["https://a.example/post-1/"]);
    let mention = site.read("fetched/mentions/11.yaml");
    outside(&site, "fetched/mentions/11.yaml", "mention.yaml");
    let (said, code) = site.run(&["refresh"]);
    assert!(out_of_fetched(&said, "fetched/mentions/11.yaml"), "{said}");
    assert_eq!(code, Some(1));
    fs::remove_file(site.path("fetched/mentions/11.yaml")).unwrap();
    fs::rename(site.path("mention.yaml"), site.path("fetched/mention.yaml")).unwrap();
    symlink(
        site.path("fetched/mention.yaml"),
        site.path("fetched/mentions/11.yaml"),
    )
    .unwrap();
    let (said, code) = site.run(&["refresh"]);
    assert_eq!(
        (said.as_str(), code),
        (
            format!(
                "error  Can't remove `fetched/mentions/11.yaml`: it is a link\n  \
                 More: https://statac.dev/e/cant-remove/\n{NO_TOKEN}"
            )
            .as_str(),
            Some(1)
        )
    );
    assert_eq!(site.read("fetched/mention.yaml"), mention);
    assert!(site.holds("fetched/mentions/11.yaml"));
    assert!(!site.read("fetched/record").contains("absent"));

    // A link in a face's place, out of `fetched/` and within it.
    let site = Site::new("link-face", &three(), &["https://b.example/post-3/"]);
    let face = format!("fetched/faces/{FACE_2}");
    outside(&site, &face, "face.jpg");
    let (said, code) = site.run(&["refresh"]);
    assert!(out_of_fetched(&said, &face), "{said}");
    assert_eq!(code, Some(1));
    fs::remove_file(site.path(&face)).unwrap();
    fs::rename(site.path("face.jpg"), site.path("fetched/face.jpg")).unwrap();
    symlink(site.path("fetched/face.jpg"), site.path(&face)).unwrap();
    let (said, code) = site.run(&["refresh"]);
    assert_eq!(
        (said.as_str(), code),
        (
            format!(
                "error  Can't remove `{face}`: it is a link\n  \
                 More: https://statac.dev/e/cant-remove/\n{NO_TOKEN}"
            )
            .as_str(),
            Some(1)
        )
    );
    assert!(site.path("fetched/face.jpg").is_file());
    assert!(site.holds("fetched/mentions/13.yaml"));
}
