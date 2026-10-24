//! `statac refuse` turns away a webmention kept in `fetched/mentions/`, or
//! everything from a sender, and lists it in `fetched/refused`, so that
//! `statac refresh` never keeps it again. Each test runs the program on a
//! small site of its own in the system's temporary folder; none touches
//! the network.

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

/// One mention of `post-1`, as its file holds it: its id, the page that
/// sent it, the sender's own address, and the name of its face.
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
    /// `statac refresh` keeps them, each with its line in the record.
    fn new(label: &str, mentions: &[Sent<'_>]) -> Site {
        let path =
            std::env::temp_dir().join(format!("statac-refuse-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        let site = Site(path);
        site.write("statac.yaml", SETTINGS);
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
        site.write(
            "fetched/record",
            &format!("statac fetched 1\n{}\n", lines.join("\n")),
        );
        site
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

    /// What the program says of the site, run with `args` after it, and
    /// its exit code. Nothing is ever said on standard error.
    fn run(&self, args: &[&str]) -> (String, Option<i32>) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(args)
            .arg(format!("--site={}", self.0.display()))
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        let said = String::from_utf8(output.stdout).unwrap();
        assert!(output.stderr.is_empty(), "{args:?}: {said}");
        let shown = self.0.display().to_string();
        (said.replace(&shown, "SITE"), output.status.code())
    }

    /// `statac refuse address` on Tuesday 6 October 2026.
    fn refuse(&self, address: &str) -> (String, Option<i32>) {
        self.run(&["refuse", address, "--today=2026-10-06"])
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
fn one_mention_is_refused_by_its_address_however_it_is_spelt() {
    for (n, address) in [
        "https://a.example/post-1/",
        "http://a.example/post-1/",
        "https://www.a.example/post-1",
        "https://A.Example/post-1/",
        "HTTP://WWW.A.EXAMPLE/post-1",
    ]
    .into_iter()
    .enumerate()
    {
        let site = Site::new(&format!("one-{n}"), &three());
        let (said, code) = site.refuse(address);
        assert_eq!(
            said,
            format!(
                "Refused `{address}`: 1 webmention removed.\n\
                 Removed `fetched/mentions/11.yaml`.\n"
            )
        );
        assert_eq!(code, Some(0));
        assert_eq!(
            site.read("fetched/refused"),
            format!("statac refused 1\nmention\t{address}\t2026-10-06\n")
        );
        // The face it shares with a mention kept stays.
        assert_eq!(
            site.fetched(),
            [
                format!("faces/{FACE_1}"),
                format!("faces/{FACE_2}"),
                "mentions/12.yaml".into(),
                "mentions/13.yaml".into(),
                "record".into(),
                "refused".into(),
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
    }
}

#[test]
fn a_sender_is_refused_with_everything_at_or_under_its_address() {
    let mentions = [
        sent(
            11,
            "https://a.example/post-1/",
            Some("https://a.example/"),
            Some(FACE_1),
        ),
        sent(12, "https://a.example/post-2/", None, Some(FACE_1)),
        // From a page elsewhere, by its sender's address alone.
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
    let site = Site::new("sender", &mentions);
    let (said, code) = site.refuse("HTTP://A.example");
    assert_eq!(
        said,
        format!(
            "Refused everything from `HTTP://A.example`: 3 webmentions removed.\n\
             Removed `fetched/mentions/11.yaml`.\n\
             Removed `fetched/mentions/12.yaml`.\n\
             Removed `fetched/mentions/13.yaml`.\n\
             Removed `fetched/faces/{FACE_1}`.\n"
        )
    );
    assert_eq!(code, Some(0));
    assert_eq!(
        site.read("fetched/refused"),
        "statac refused 1\nsender\tHTTP://A.example\t2026-10-06\n"
    );
    assert_eq!(
        site.fetched(),
        [
            format!("faces/{FACE_2}"),
            "mentions/14.yaml".into(),
            "mentions/15.yaml".into(),
            "mentions/16.yaml".into(),
            "record".into(),
            "refused".into(),
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
}

#[test]
fn a_page_with_another_under_it_is_refused_as_a_sender_and_only_at_a_boundary() {
    let mentions = [
        sent(11, "https://a.example/a/", None, None),
        sent(12, "https://a.example/a/post-2/", None, None),
        sent(13, "https://a.example/ab/post-3/", None, None),
    ];
    let site = Site::new("boundary", &mentions);
    let (said, code) = site.refuse("https://a.example/a");
    assert_eq!(
        said,
        "Refused everything from `https://a.example/a`: 2 webmentions removed.\n\
         Removed `fetched/mentions/11.yaml`.\n\
         Removed `fetched/mentions/12.yaml`.\n"
    );
    assert_eq!(code, Some(0));
    assert_eq!(site.fetched(), ["mentions/13.yaml", "record", "refused"]);

    // Two mentions sent from one page are refused as a sender, both.
    let mentions = [
        sent(11, "https://a.example/post-1/", None, None),
        sent(12, "https://a.example/post-1", None, None),
    ];
    let site = Site::new("twice", &mentions);
    let (said, _) = site.refuse("https://a.example/post-1");
    assert_eq!(
        said,
        "Refused everything from `https://a.example/post-1`: 2 webmentions removed.\n\
         Removed `fetched/mentions/11.yaml`.\n\
         Removed `fetched/mentions/12.yaml`.\n"
    );
    assert_eq!(
        site.read("fetched/refused"),
        "statac refused 1\nsender\thttps://a.example/post-1\t2026-10-06\n"
    );
}

#[test]
fn an_address_refused_already_is_said_and_nothing_changes() {
    let site = Site::new("already", &three());
    site.refuse("https://a.example/post-1/");
    let (list, kept) = (site.read("fetched/refused"), site.read("fetched/record"));
    for address in ["https://a.example/post-1/", "http://www.a.example/post-1"] {
        let (said, code) = site.refuse(address);
        assert_eq!(said, format!("`{address}` is refused already.\n"));
        assert_eq!(code, Some(0));
    }
    // A sender takes in what is listed under it, and is listed with it.
    site.refuse("https://a.example/");
    let (said, code) = site.refuse("https://a.example/post-9/");
    assert_eq!(
        (said.as_str(), code),
        ("`https://a.example/post-9/` is refused already.\n", Some(0))
    );
    assert_ne!(site.read("fetched/record"), kept);
    assert_eq!(
        site.read("fetched/refused"),
        list.replace("1\n", "1\nsender\thttps://a.example/\t2026-10-06\n")
    );
}

#[test]
fn nothing_is_refused_where_nothing_kept_is_from_the_address() {
    let site = Site::new("nothing", &three());
    let kept = site.read("fetched/record");
    let (said, code) = site.refuse("https://a.example/post-3/");
    assert_eq!(
        said,
        "Nothing refused: no webmention kept here is from `https://a.example/post-3/`.\n\
         Did you mean `https://a.example/post-1/`?\n"
    );
    assert_eq!(code, Some(1));
    let (said, code) = site.refuse("https://d.example/");
    assert_eq!(
        said,
        "Nothing refused: no webmention kept here is from `https://d.example/`.\n"
    );
    assert_eq!(code, Some(1));
    assert!(!site.holds("fetched/refused"));
    assert_eq!(site.read("fetched/record"), kept);

    // A site with nothing fetched has nothing to refuse, and is left so.
    let site = Site::new("none", &[]);
    fs::remove_dir_all(site.path("fetched")).unwrap();
    let (said, code) = site.refuse("https://a.example/");
    assert_eq!(
        (said.as_str(), code),
        (
            "Nothing refused: no webmention kept here is from `https://a.example/`.\n",
            Some(1)
        )
    );
    assert!(!site.holds("fetched"));
}

#[test]
fn a_mention_that_doesnt_read_is_told_as_check_tells_it_and_nothing_is_refused() {
    let site = Site::new("unread", &three());
    site.write("fetched/mentions/13.yaml", "kind: \"waved\"\n");
    // An error elsewhere in the site is no concern of this command's.
    site.write("content/post-2.md", "---\ntitle: [\n---\n");
    let (said, code) = site.refuse("https://a.example/post-1/");
    assert_eq!(code, Some(1));
    assert!(said.starts_with("error  "), "{said}");
    assert!(
        said.contains("  fetched/mentions/13.yaml, line 1\n"),
        "{said}"
    );
    assert!(!said.contains("post-2"), "{said}");
    assert!(
        said.ends_with("4 errors: 3 missing-field, 1 wrong-kind-of-value.\n"),
        "{said}"
    );
    assert!(site.holds("fetched/mentions/11.yaml"));
    assert!(!site.holds("fetched/refused"));
}

#[test]
fn a_list_that_doesnt_read_is_told_by_check_and_build() {
    let site = Site::new("malformed", &three());
    site.write(
        "fetched/refused",
        "statac refused 1\nsite\thttps://a.example/\t2026-10-06\n",
    );
    for command in ["check", "build"] {
        let (said, code) = site.run(&[command, "--today=2026-10-06"]);
        assert_eq!(code, Some(1), "{said}");
        assert!(
            said.starts_with(
                "error  Malformed record: `site` is neither `mention` nor `sender`\n  \
                 fetched/refused, line 2\n"
            ),
            "{said}"
        );
        assert!(said.contains("  Write `mention` or `sender`.\n"), "{said}");
    }
    let (said, code) = site.refuse("https://a.example/post-1/");
    assert_eq!(code, Some(1));
    assert!(
        said.starts_with("error  Malformed record: `site`"),
        "{said}"
    );
    assert!(site.holds("fetched/mentions/11.yaml"));
}

/// What `build` says of `site`, its time aside, and every file it
/// writes, by its path, in order.
fn built(site: &Site) -> (String, Vec<(PathBuf, Vec<u8>)>) {
    let (said, code) = site.run(&["build", "--today=2026-10-06"]);
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
}

#[test]
fn a_list_that_turns_nothing_away_changes_nothing_built() {
    let unlisted = Site::new("unlisted", &three());
    let listed = Site::new("listed", &three());
    listed.write(
        "fetched/refused",
        "statac refused 1\nsender\thttps://d.example/\t2026-10-06\n",
    );
    assert!(built(&unlisted) == built(&listed));

    // A page shows what is refused no longer.
    let page = || listed.read("_site/post-1/index.html");
    assert!(page().contains("https://a.example/post-1/"));
    listed.refuse("https://a.example/post-1/");
    built(&listed);
    assert!(!page().contains("https://a.example/post-1/"));
    assert!(page().contains("https://a.example/post-2/"));
}

#[cfg(unix)]
#[test]
fn nothing_outside_fetched_is_removed_or_written_through_a_link() {
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
    let site = Site::new("link-mention", &three());
    let mention = site.read("fetched/mentions/11.yaml");
    outside(&site, "fetched/mentions/11.yaml", "mention.yaml");
    let (said, code) = site.refuse("https://a.example/post-1/");
    assert!(out_of_fetched(&said, "fetched/mentions/11.yaml"), "{said}");
    assert_eq!(code, Some(1));
    fs::remove_file(site.path("fetched/mentions/11.yaml")).unwrap();
    fs::rename(site.path("mention.yaml"), site.path("fetched/mention.yaml")).unwrap();
    symlink(
        site.path("fetched/mention.yaml"),
        site.path("fetched/mentions/11.yaml"),
    )
    .unwrap();
    let (said, code) = site.refuse("https://a.example/post-1/");
    assert_eq!(
        (said.as_str(), code),
        (
            "error  Can't remove `fetched/mentions/11.yaml`: it is a link\n  \
             More: https://statac.dev/e/cant-remove/\n",
            Some(1)
        )
    );
    assert_eq!(site.read("fetched/mention.yaml"), mention);
    assert!(site.holds("fetched/mentions/11.yaml"));
    assert!(!site.holds("fetched/refused"));

    // A link in a face's place, out of `fetched/` and within it.
    let site = Site::new("link-face", &three());
    let face = format!("fetched/faces/{FACE_2}");
    outside(&site, &face, "face.jpg");
    let (said, code) = site.refuse("https://b.example/post-3/");
    assert!(out_of_fetched(&said, &face), "{said}");
    assert_eq!(code, Some(1));
    fs::remove_file(site.path(&face)).unwrap();
    fs::rename(site.path("face.jpg"), site.path("fetched/face.jpg")).unwrap();
    symlink(site.path("fetched/face.jpg"), site.path(&face)).unwrap();
    let (said, code) = site.refuse("https://b.example/post-3/");
    assert_eq!(
        (said.as_str(), code),
        (
            format!(
                "error  Can't remove `{face}`: it is a link\n  \
                 More: https://statac.dev/e/cant-remove/\n"
            )
            .as_str(),
            Some(1)
        )
    );
    assert!(site.path("fetched/face.jpg").is_file());
    assert!(site.holds("fetched/mentions/13.yaml"));
    assert!(!site.holds("fetched/refused"));

    // A link in the list's place, out of `fetched/` and within it.
    let site = Site::new("link-list", &three());
    site.write("refused.txt", "statac refused 1\n");
    symlink(site.path("refused.txt"), site.path("fetched/refused")).unwrap();
    let (said, code) = site.refuse("https://a.example/post-1/");
    assert!(out_of_fetched(&said, "fetched/refused"), "{said}");
    assert_eq!(code, Some(1));
    fs::remove_file(site.path("fetched/refused")).unwrap();
    fs::rename(site.path("refused.txt"), site.path("fetched/list.txt")).unwrap();
    symlink(site.path("fetched/list.txt"), site.path("fetched/refused")).unwrap();
    let (said, code) = site.refuse("https://a.example/post-1/");
    assert_eq!(
        (said.as_str(), code),
        (
            "error  Can't write `fetched/refused`: it is a link\n  \
             More: https://statac.dev/e/cant-write/\n",
            Some(1)
        )
    );
    assert_eq!(site.read("fetched/list.txt"), "statac refused 1\n");
    assert!(site.holds("fetched/mentions/11.yaml"));
}

#[test]
fn the_help_names_the_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_statac"))
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(
        help.contains(
            "  statac refuse <url> [folder]\n                         Remove the webmention"
        ),
        "{help}"
    );
}
