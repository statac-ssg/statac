//! `statac refresh` fetches the still of each video a page reads one for
//! into `fetched/stills/`, records it in `fetched/record`, and says each;
//! `build` says how old the stills are. Each test runs on a small site of
//! its own in the system's temporary folder, with a fetcher standing in
//! for the network, which no test touches.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use image::{ImageFormat, Rgb, RgbImage};
use statac::fetch::{Fetched, Refusal};
use statac::refresh::{self, Options, Token};

const LAYOUT: &str = "<!doctype html>\n<title>{{ headline(page) }}</title>\n{{ page.content }}\n";

const SETTINGS: &str = "title: Site\nlanguage: en\ntimezone: Europe/London\npages.feed: false\n";

/// Snippets showing a video in place of its still.
const YOUTUBE: &str = "{# takes address #}\n{% set video = page.youtube(address) %}\n\
                       {% if video.thumbnail %}<img src=\"{{ video.thumbnail.src }}\">{% endif %}\n";
const VIMEO: &str = "{# takes address #}\n{% set video = page.vimeo(address) %}\n\
                     {% if video.thumbnail %}<img src=\"{{ video.thumbnail.src }}\">{% endif %}\n";

const MAXRES: &str = "https://i.ytimg.com/vi/abcdefghijk/maxresdefault.jpg";
const HQ: &str = "https://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg";

struct Site(PathBuf);

impl Site {
    fn new(label: &str) -> Site {
        let path =
            std::env::temp_dir().join(format!("statac-refresh-{label}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        let site = Site(path);
        site.write("statac.yaml", SETTINGS);
        site.write("templates/page.html", LAYOUT);
        site.write("templates/home.html", LAYOUT);
        site.write("snippets/youtube.html", YOUTUBE);
        site.write("snippets/vimeo.html", VIMEO);
        site.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        site
    }

    /// A site with one post showing the YouTube video `abcdefghijk`.
    fn with_a_video(label: &str) -> Site {
        let site = Site::new(label);
        site.write(
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n@youtube(\"https://youtu.be/abcdefghijk\")\n",
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
        self.path(relative).exists()
    }

    /// What `refresh` says, fetching with `answers` on Tuesday 6 October
    /// 2026, and its exit code, with every address asked for in order.
    fn refresh(&self, answers: &Answers<'_>) -> (String, u8, Vec<String>) {
        self.refresh_with(None, answers)
    }

    /// What `refresh` says given the relay's token `token`, as
    /// [`Site::refresh`] does. Nothing it says, and no file in the site,
    /// ever holds the token.
    fn refresh_with(
        &self,
        token: Option<&str>,
        answers: &Answers<'_>,
    ) -> (String, u8, Vec<String>) {
        let options = Options {
            site: self.0.clone(),
            today: statac::date::read("2026-10-06").ok(),
            token: token.and_then(|token| Token::new(token.into())),
        };
        let asked = RefCell::new(Vec::new());
        let fetcher = |address: &str, limit: u64| {
            let expected = match address {
                _ if address.starts_with(RELAY) => 5_000_000,
                _ if address.contains("a.example") => 10_000_000,
                _ => 20_000_000,
            };
            assert_eq!(limit, expected, "{address}");
            asked.borrow_mut().push(address.to_owned());
            answers(address)
        };
        let mut out = Vec::new();
        let code = refresh::run(&options, &fetcher, &mut out, false);
        let said = String::from_utf8(out).unwrap();
        assert!(!said.contains(TOKEN), "{said}");
        assert!(!format!("{options:?}").contains(TOKEN));
        self.holds_no_token(&self.0);
        (said, code, asked.into_inner())
    }

    fn holds_no_token(&self, folder: &std::path::Path) {
        let Ok(entries) = fs::read_dir(folder) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            match path.is_dir() {
                true => self.holds_no_token(&path),
                false => {
                    let bytes = fs::read(&path).unwrap();
                    let text = String::from_utf8_lossy(&bytes);
                    assert!(!text.contains(TOKEN), "{}", path.display());
                }
            }
        }
    }

    /// What the program says of the site, run with `command` on `today`,
    /// and its exit code. Nothing is ever said on standard error.
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

type Answers<'a> = dyn Fn(&str) -> Result<Fetched, Refusal> + 'a;

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let picture = RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x * 3) as u8, (y * 5) as u8, 90])
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut bytes, ImageFormat::Jpeg).unwrap();
    bytes.into_inner()
}

fn answer(address: &str, bytes: &[u8]) -> Result<Fetched, Refusal> {
    Ok(Fetched {
        bytes: bytes.to_vec(),
        from: address.parse().unwrap(),
        content_type: None,
    })
}

fn missing() -> Result<Fetched, Refusal> {
    Err(Refusal::Missing { status: 404 })
}

fn hash_of(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex()[..16].to_owned()
}

#[test]
fn a_still_is_fetched_kept_and_recorded() {
    let site = Site::with_a_video("fetched");
    let still = jpeg(64, 36);
    let (said, code, asked) = site.refresh(&|address| answer(address, &still));
    assert_eq!(
        said,
        "Fetched `fetched/stills/youtube-abcdefghijk.jpg`.\nFetched 1 thumbnail.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [MAXRES]);
    assert_eq!(
        fs::read(site.path("fetched/stills/youtube-abcdefghijk.jpg")).unwrap(),
        still
    );
    assert_eq!(
        site.read("fetched/record"),
        format!(
            "statac fetched 1\nstills/youtube-abcdefghijk.jpg\t{}\t2026-10-06\n",
            hash_of(&still)
        )
    );

    // The page shows it, as copies, and nothing is left to fetch.
    let (said, code) = site.run("check", "2026-10-06");
    assert_eq!((said.as_str(), code), ("No problems.\n", Some(0)));
    let (said, code, asked) = site.refresh(&|_| panic!("nothing is asked for"));
    assert_eq!((said.as_str(), code), ("Nothing to fetch.\n", 0));
    assert!(asked.is_empty());
}

#[test]
fn the_smaller_still_is_taken_where_youtube_has_no_largest() {
    let site = Site::with_a_video("smaller");
    let still = jpeg(48, 36);
    let (said, code, asked) = site.refresh(&|address| match address {
        MAXRES => Err(Refusal::Missing { status: 410 }),
        _ => answer(address, &still),
    });
    assert_eq!(
        said,
        "Fetched `fetched/stills/youtube-abcdefghijk.jpg`.\nFetched 1 thumbnail.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [MAXRES, HQ]);
    assert_eq!(
        fs::read(site.path("fetched/stills/youtube-abcdefghijk.jpg")).unwrap(),
        still
    );
}

#[test]
fn a_video_with_no_still_is_recorded_absent_and_never_asked_for_again() {
    let site = Site::with_a_video("absent");
    let (said, code, asked) = site.refresh(&|_| missing());
    assert_eq!(
        said,
        "No thumbnail: YouTube has no image for `abcdefghijk`.\nFetched no thumbnails.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [MAXRES, HQ]);
    assert!(!site.holds("fetched/stills"));
    assert_eq!(
        site.read("fetched/record"),
        "statac fetched 1\nstills/youtube-abcdefghijk.jpg\tabsent\t2026-10-06\n"
    );

    let (said, code, asked) = site.refresh(&|_| panic!("nothing is asked for"));
    assert_eq!((said.as_str(), code), ("Nothing to fetch.\n", 0));
    assert!(asked.is_empty());
    let (said, _) = site.run("check", "2026-10-06");
    assert!(
        said.starts_with("warning  No thumbnail: YouTube has no image for this video\n"),
        "{said}"
    );
    assert!(
        said.contains("  Show an image of your own in its place.\n"),
        "{said}"
    );

    // Its line deleted, it is asked for again.
    site.write("fetched/record", "statac fetched 1\n");
    let still = jpeg(32, 18);
    let (said, code, _) = site.refresh(&|address| answer(address, &still));
    assert_eq!(
        (said.as_str(), code),
        (
            "Fetched `fetched/stills/youtube-abcdefghijk.jpg`.\nFetched 1 thumbnail.\n",
            0
        )
    );
}

#[test]
fn each_reason_a_still_couldnt_be_fetched_is_said_and_nothing_is_kept() {
    let site = Site::with_a_video("failures");
    let host = || "i.ytimg.com".to_owned();
    let cases = [
        (
            Refusal::NotAnAddress,
            format!("`{MAXRES}` isn\'t an `https://` or `http://` URL"),
        ),
        (
            Refusal::Scheme {
                scheme: "ftp".into(),
            },
            format!("`{MAXRES}` isn\'t an `https://` or `http://` URL"),
        ),
        (
            Refusal::NoHost,
            format!("`{MAXRES}` isn't an `https://` or `http://` URL"),
        ),
        (
            Refusal::Credentials,
            format!("`{MAXRES}` holds a user name or password"),
        ),
        (
            Refusal::UnknownHost { host: host() },
            "nothing answers to the name `i.ytimg.com`".into(),
        ),
        (
            Refusal::PrivateAddress {
                host: host(),
                address: std::net::IpAddr::from([127, 0, 0, 1]),
            },
            "`i.ytimg.com` is at `127.0.0.1`, which isn't on the open web".into(),
        ),
        (
            Refusal::Status { status: 503 },
            "the server answered 503".into(),
        ),
        (
            Refusal::Redirects { limit: 5 },
            "the URL redirected more than 5 times".into(),
        ),
        (
            Refusal::NoRedirectTarget {
                host: host(),
                location: Some(String::new()),
            },
            "the server redirected without saying where".into(),
        ),
        (
            Refusal::TooLarge {
                limit: 20_000_000,
                content_length: None,
            },
            "the answer is larger than 20 MB".into(),
        ),
        (Refusal::TimedOut, "no answer came within 30 seconds".into()),
        (
            Refusal::Tls { host: host() },
            "Statac couldn't make a secure connection to `i.ytimg.com`".into(),
        ),
        (
            Refusal::Failed {
                reason: "Connection refused (os error 111)".into(),
            },
            "connection refused".into(),
        ),
    ];
    for (refusal, reason) in cases {
        let (said, code, asked) = site.refresh(&|_| Err(refusal.clone()));
        assert_eq!(
            said,
            format!("Can't fetch the thumbnail for `abcdefghijk`: {reason}.\nCouldn't fetch 1.\n"),
            "{refusal:?}"
        );
        assert_eq!(code, 1, "{refusal:?}");
        assert_eq!(asked, [MAXRES], "{refusal:?}");
        assert!(!site.holds("fetched"), "{refusal:?}");
    }
}

#[test]
fn an_answer_that_isnt_a_picture_is_not_kept() {
    let site = Site::with_a_video("not-a-picture");
    let cut = jpeg(64, 36)[..200].to_vec();
    for bytes in [b"<!doctype html>".to_vec(), cut] {
        let (said, code, _) = site.refresh(&|address| answer(address, &bytes));
        assert_eq!(
            said,
            "Can't fetch the thumbnail for `abcdefghijk`: the answer isn't an image.\n\
             Couldn't fetch 1.\n"
        );
        assert_eq!(code, 1);
        assert!(!site.holds("fetched"));
    }
}

#[test]
fn a_vimeo_still_is_fetched_from_where_vimeo_says_it_is() {
    let site = Site::new("vimeo");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\n@vimeo(\"https://vimeo.com/123456789\")\n",
    );
    let still = jpeg(64, 36);
    let asked_about = "https://vimeo.com/api/v2/video/123456789.json";
    let named = "https://i.vimeocdn.com/video/1-d_640";
    let json = format!("[{{\"id\": 123456789, \"thumbnail_large\": \"{named}\"}}]");
    let (said, code, asked) = site.refresh(&|address| match address {
        _ if address == asked_about => answer(address, json.as_bytes()),
        _ => answer(address, &still),
    });
    assert_eq!(
        said,
        "Fetched `fetched/stills/vimeo-123456789.jpg`.\nFetched 1 thumbnail.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [asked_about, named]);
    assert_eq!(
        fs::read(site.path("fetched/stills/vimeo-123456789.jpg")).unwrap(),
        still
    );

    // Where Vimeo names no picture, or has no such video.
    site.write("fetched/record", "statac fetched 1\n");
    fs::remove_file(site.path("fetched/stills/vimeo-123456789.jpg")).unwrap();
    let (said, code, _) = site.refresh(&|address| answer(address, b"[{\"id\": 123456789}]"));
    assert_eq!(
        (said.as_str(), code),
        (
            "Can't fetch the thumbnail for `123456789`: the video site named no image.\n\
             Couldn't fetch 1.\n",
            1
        )
    );
    let (said, code, asked) = site.refresh(&|_| missing());
    assert_eq!(
        (said.as_str(), code),
        (
            "No thumbnail: Vimeo has no image for `123456789`.\nFetched no thumbnails.\n",
            0
        )
    );
    assert_eq!(asked, [asked_about]);
}

#[test]
fn only_the_stills_missing_are_asked_for_in_order_of_name() {
    let site = Site::new("several");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\n@youtube(\"bcdefghijkl\")\n\n@youtube(\"abcdefghijk\")\n\n\
         @vimeo(\"123456789\")\n\n@youtube(\"cdefghijklm\")\n",
    );
    // Shown by a draft and a post waiting for its day, which a build
    // leaves out.
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndraft: true\n---\n@youtube(\"defghijklmn\")\n",
    );
    site.write(
        "content/post-3.md",
        "---\ntitle: Post 3\ndate: 2026-12-01\n---\n@youtube(\"efghijklmno\")\n",
    );
    // A playlist has no still.
    site.write(
        "content/post-4.md",
        "---\ntitle: Post 4\n---\n\
         @youtube(\"https://www.youtube.com/embed/videoseries?list=PL0123\")\n",
    );
    site.put("fetched/stills/youtube-cdefghijklm.jpg", &jpeg(32, 18));
    let still = jpeg(64, 36);
    let (said, code, asked) = site.refresh(&|address| match address {
        _ if address.contains("abcdefghijk") => Err(Refusal::TimedOut),
        _ if address.contains("vimeo.com/api") => missing(),
        _ => answer(address, &still),
    });
    assert_eq!(
        said,
        "No thumbnail: Vimeo has no image for `123456789`.\n\
         Can't fetch the thumbnail for `abcdefghijk`: no answer came within 30 seconds.\n\
         Fetched `fetched/stills/youtube-bcdefghijkl.jpg`.\n\
         Fetched `fetched/stills/youtube-defghijklmn.jpg`.\n\
         Fetched `fetched/stills/youtube-efghijklmno.jpg`.\n\
         Fetched 3 thumbnails. 1 already here. Couldn't fetch 1.\n"
    );
    assert_eq!(code, 1);
    assert_eq!(
        asked,
        [
            "https://vimeo.com/api/v2/video/123456789.json",
            MAXRES,
            "https://i.ytimg.com/vi/bcdefghijkl/maxresdefault.jpg",
            "https://i.ytimg.com/vi/defghijklmn/maxresdefault.jpg",
            "https://i.ytimg.com/vi/efghijklmno/maxresdefault.jpg",
        ]
    );
    // What was fetched after a failure is recorded; the still already
    // there has no line, being the author's own.
    let hash = hash_of(&still);
    assert_eq!(
        site.read("fetched/record"),
        format!(
            "statac fetched 1\n\
             stills/vimeo-123456789.jpg\tabsent\t2026-10-06\n\
             stills/youtube-bcdefghijkl.jpg\t{hash}\t2026-10-06\n\
             stills/youtube-defghijklmn.jpg\t{hash}\t2026-10-06\n\
             stills/youtube-efghijklmno.jpg\t{hash}\t2026-10-06\n"
        )
    );
}

#[test]
fn a_line_whose_still_is_gone_is_dropped_when_the_record_is_written() {
    let site = Site::with_a_video("gone");
    site.write(
        "fetched/record",
        "statac fetched 1\n\
         stills/vimeo-123456789.jpg\tabsent\t2026-10-01\n\
         stills/youtube-bcdefghijkl.jpg\t0123456789abcdef\t2026-10-01\n",
    );
    let still = jpeg(32, 18);
    let (_, code, _) = site.refresh(&|address| answer(address, &still));
    assert_eq!(code, 0);
    assert_eq!(
        site.read("fetched/record"),
        format!(
            "statac fetched 1\n\
             stills/vimeo-123456789.jpg\tabsent\t2026-10-01\n\
             stills/youtube-abcdefghijk.jpg\t{}\t2026-10-06\n",
            hash_of(&still)
        )
    );
}

#[test]
fn a_site_with_an_error_is_told_as_check_tells_it_and_nothing_is_fetched() {
    let site = Site::with_a_video("error");
    site.write(
        "fetched/record",
        "statac fetched 1\nstills/youtube-bcdefghijkl.jpg\tabsent\tyesterday\n",
    );
    let (said, code, asked) = site.refresh(&|_| panic!("nothing is asked for"));
    let (checked, _) = site.run("check", "2026-10-06");
    assert_eq!(said.replace(&site.0.display().to_string(), "SITE"), checked);
    assert!(
        said.contains(
            "error  Malformed record: `yesterday` isn't a day\n  fetched/record, line 2\n"
        ),
        "{said}"
    );
    assert_eq!(code, 1);
    assert!(asked.is_empty());
    assert!(!site.holds("fetched/stills"));
    let (said, code) = site.run("build", "2026-10-06");
    assert_eq!(code, Some(1));
    assert!(
        said.contains("Wrote nothing: 1 error stopped the build."),
        "{said}"
    );
}

/// A layout reading a still the site lacks, unguarded, stops the site on
/// it, so refresh fetches the still before it reads the site again.
#[test]
fn a_site_stopped_only_by_a_still_it_lacks_gets_the_still() {
    let site = Site::new("stopped-by-still");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\nlayout: video\n---\n",
    );
    site.write(
        "templates/video.html",
        "{% set v = page.youtube(\"abcdefghijk\") %}<img src=\"{{ v.thumbnail.src }}\" alt=\"\">",
    );
    let (said, code) = site.run("build", "2026-10-06");
    assert_eq!(code, Some(1), "{said}");
    let still = jpeg(64, 36);
    let (said, code, asked) = site.refresh(&|address| answer(address, &still));
    assert_eq!(
        said,
        "Fetched `fetched/stills/youtube-abcdefghijk.jpg`.\nFetched 1 thumbnail.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [MAXRES]);
    let (said, code) = site.run("build", "2026-10-06");
    assert_eq!(code, Some(0), "{said}");

    // A video with no still stays without one, and what stops the site is
    // told as check tells it.
    let site = Site::new("stopped-by-absent-still");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\nlayout: video\n---\n",
    );
    site.write(
        "templates/video.html",
        "{% set v = page.youtube(\"abcdefghijk\") %}<img src=\"{{ v.thumbnail.src }}\" alt=\"\">",
    );
    let (said, code, asked) = site.refresh(&|_| missing());
    assert_eq!(code, 1);
    assert_eq!(asked.len(), 2, "{asked:?}");
    assert!(
        said.contains("error  No value: nothing is set for `v.thumbnail`"),
        "{said}"
    );
    let (again, _, asked) = site.refresh(&|_| panic!("nothing is asked for"));
    assert!(asked.is_empty());
    assert!(
        again.contains("error  No value: nothing is set for `v.thumbnail`"),
        "{again}"
    );
}

#[test]
fn a_site_only_a_draft_stops_gets_the_stills_of_what_it_publishes() {
    let site = Site::with_a_video("draft-error");
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndraft: true\n---\n@youtube(\"bcdefghijkl\")\n\n@absent()\n",
    );
    let still = jpeg(64, 36);
    let (said, code, asked) = site.refresh(&|address| answer(address, &still));
    assert_eq!(
        said,
        "Fetched `fetched/stills/youtube-abcdefghijk.jpg`.\nFetched 1 thumbnail.\n\
         Left the drafts out: the site has 1 error with it. \
         Run `statac check --drafts` to see it.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [MAXRES]);

    let (said, code, _) = site.refresh(&|_| panic!("nothing is asked for"));
    assert_eq!(
        said,
        "Nothing to fetch.\n\
         Left the drafts out: the site has 1 error with it. \
         Run `statac check --drafts` to see it.\n"
    );
    assert_eq!(code, 0);
}

#[test]
fn a_site_with_no_still_missing_has_nothing_to_fetch_and_no_folder_made() {
    let site = Site::new("nothing");
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\nNo video.\n");
    let (said, code, asked) = site.refresh(&|_| panic!("nothing is asked for"));
    assert_eq!((said.as_str(), code), ("Nothing to fetch.\n", 0));
    assert!(asked.is_empty());
    assert!(!site.holds("fetched"));
}

#[test]
fn build_says_how_old_the_stills_are() {
    let site = Site::with_a_video("ages");
    let still = jpeg(32, 18);
    site.refresh(&|address| answer(address, &still));
    let said = |today: &str| {
        let (said, code) = site.run("build", today);
        assert_eq!(code, Some(0), "{said}");
        said.lines()
            .filter(|line| line.starts_with("Fetched"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!(said("2026-10-06"), "Fetched: 1 thumbnail, today.");
    assert_eq!(said("2026-10-07"), "Fetched: 1 thumbnail, yesterday.");
    assert_eq!(said("2026-10-12"), "Fetched: 1 thumbnail, 6 days ago.");

    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\n---\n@youtube(\"bcdefghijkl\")\n\n@youtube(\"cdefghijklm\")\n",
    );
    site.put("fetched/stills/youtube-bcdefghijkl.jpg", &still);
    assert_eq!(
        said("2026-10-12"),
        "Fetched: 2 thumbnails, the oldest 6 days ago; 1 with no date."
    );
    site.refresh(&|address| answer(address, &still));
    assert_eq!(
        said("2026-10-12"),
        "Fetched: 3 thumbnails, the oldest 6 days ago; 1 with no date."
    );
    site.write("fetched/record", "statac fetched 1\n");
    assert_eq!(said("2026-10-12"), "Fetched: 3 thumbnails, with no date.");

    // Neither `check` nor a site with nothing fetched says it.
    let (said, _) = site.run("check", "2026-10-12");
    assert!(!said.contains("Fetched"), "{said}");
}

#[test]
fn a_site_with_nothing_fetched_builds_as_it_did() {
    let site = Site::new("unfetched");
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\nNo video.\n");
    let (said, code) = site.run("build", "2026-10-06");
    assert_eq!(code, Some(0));
    // Besides starting the record of what it published.
    let lines: Vec<&str> = (said.lines())
        .filter(|line| !line.starts_with("Started `published.txt`"))
        .collect();
    assert!(
        lines.len() == 1 && lines[0].starts_with("Wrote 2 pages to `SITE"),
        "{said}"
    );
    assert!(!site.holds("fetched"));
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
            "  statac refresh [folder]\n                         Fetch what the site keeps"
        ),
        "{help}"
    );
}

/// The token the relay is asked with, which nothing ever says or keeps.
const TOKEN: &str = "t0k3n-a1b2c3";

const RELAY: &str = "https://webmention.io/api/mentions.jf2";

const FACE: &str = "https://a.example/a.jpg";

/// The relay's address for the mentions after `since`.
fn relay(since: u64) -> String {
    format!("{RELAY}?token={TOKEN}&since_id={since}&per-page=100&sort-by=created&sort-dir=up")
}

/// The name a face fetched from `address` is kept under.
fn face_name(address: &str) -> String {
    format!("{}.jpg", &blake3::hash(address.as_bytes()).to_hex()[..16])
}

impl Site {
    /// A site with webmentions on and one post, `post-1`.
    fn mentioned(label: &str) -> Site {
        let site = Site::new(label);
        site.write(
            "statac.yaml",
            &format!("{SETTINGS}url: https://example.org/\nwebmentions: true\n"),
        );
        site.write("content/post-1.md", "---\ntitle: Post 1\n---\nPost 1.\n");
        site
    }

    /// What `refresh` says asking the relay with [`TOKEN`].
    fn ask(&self, answers: &Answers<'_>) -> (String, u8, Vec<String>) {
        self.refresh_with(Some(TOKEN), answers)
    }
}

/// `text` as a JSON string.
fn json(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A mention as the relay sends it: a reply from `a.example` to `post-1`,
/// its id written as `id` is, with each of `changes`, a field by name and
/// its JSON, or none to leave it out. `author.` and `content.` name what
/// is inside those.
fn sent(id: &str, changes: &[(&str, Option<&str>)]) -> String {
    let mut fields: Vec<(String, Option<String>)> = [
        ("type", json("entry")),
        ("author.type", json("card")),
        ("author.name", json("A")),
        ("author.url", json("https://a.example/")),
        ("url", json("https://a.example/post-1/")),
        ("published", json("2026-09-03T21:48:39")),
        ("wm-received", json("2026-09-04T04:54:54Z")),
        ("wm-id", id.to_owned()),
        ("wm-source", json("https://a.example/post-1/")),
        ("wm-target", json("https://example.org/post-1/")),
        ("wm-property", json("in-reply-to")),
        ("content.text", json("Words.")),
        ("content.html", json("<p>Words.</p>")),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), Some(value)))
    .collect();
    for (name, value) in changes {
        let value = value.map(str::to_owned);
        match fields.iter_mut().find(|(field, _)| field == name) {
            Some(field) => field.1 = value,
            None => fields.push(((*name).to_owned(), value)),
        }
    }
    let object = |inside: Option<&str>| {
        let parts: Vec<String> = (fields.iter())
            .filter_map(|(name, value)| {
                let name = match (inside, name.split_once('.')) {
                    (None, None) => name.as_str(),
                    (Some(inside), Some((outer, name))) if outer == inside => name,
                    _ => return None,
                };
                Some(format!("{}:{}", json(name), value.as_ref()?))
            })
            .collect();
        format!("{{{}}}", parts.join(","))
    };
    let (author, content) = (object(Some("author")), object(Some("content")));
    let top = object(None);
    format!(
        "{},\"author\":{author},\"content\":{content}}}",
        top.trim_end_matches('}')
    )
}

/// The relay's answer holding `children`.
fn feed(children: &[String]) -> Vec<u8> {
    format!(
        "{{\"type\":\"feed\",\"name\":\"Webmentions\",\"children\":[{}]}}",
        children.join(",")
    )
    .into_bytes()
}

/// A reply with the face [`FACE`].
fn with_face(id: u64) -> String {
    sent(&id.to_string(), &[("author.photo", Some(&json(FACE)))])
}

fn size_of(bytes: &[u8]) -> (u32, u32) {
    let picture = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg).unwrap();
    (picture.width(), picture.height())
}

#[test]
fn webmentions_are_kept_with_their_faces_and_recorded() {
    let site = Site::mentioned("mentions");
    let photo = jpeg(200, 100);
    let like = sent(
        "2028900",
        &[
            ("wm-property", Some(&json("like-of"))),
            ("author.photo", Some(&json(FACE))),
            ("content.text", None),
            ("content.html", None),
            ("published", Some("null")),
        ],
    );
    let sent_back = feed(&[with_face(2028899), like]);
    let (said, code, asked) = site.ask(&|address| match address {
        FACE => answer(address, &photo),
        _ => answer(address, &sent_back),
    });
    let face = face_name(FACE);
    assert_eq!(
        said,
        format!(
            "Fetched `fetched/faces/{face}`.\n\
             Fetched `fetched/mentions/2028899.yaml`.\n\
             Fetched `fetched/mentions/2028900.yaml`.\n\
             Fetched 2 webmentions and 1 avatar.\n"
        )
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [relay(0), FACE.to_owned()]);
    let reply = format!(
        "kind: \"replied\"\nauthor: \"A\"\nauthor_url: \"https://a.example/\"\n\
         face: \"{face}\"\nurl: \"https://a.example/post-1/\"\n\
         target: \"https://example.org/post-1/\"\npublished: \"2026-09-03\"\n\
         fetched: \"2026-10-06\"\nwords: \"Words.\"\n"
    );
    assert_eq!(site.read("fetched/mentions/2028899.yaml"), reply);
    // With no `published`, the day the relay received it.
    let liked = format!(
        "kind: \"liked\"\nauthor: \"A\"\nauthor_url: \"https://a.example/\"\n\
         face: \"{face}\"\nurl: \"https://a.example/post-1/\"\n\
         target: \"https://example.org/post-1/\"\npublished: \"2026-09-04\"\n\
         fetched: \"2026-10-06\"\n"
    );
    assert_eq!(site.read("fetched/mentions/2028900.yaml"), liked);
    let kept = fs::read(site.path(&format!("fetched/faces/{face}"))).unwrap();
    assert_eq!(size_of(&kept), (96, 48));
    assert_eq!(
        site.read("fetched/record"),
        format!(
            "statac fetched 1\n\
             faces/{face}\t{}\t2026-10-06\n\
             mentions/2028899.yaml\t{}\t2026-10-06\n\
             mentions/2028900.yaml\t{}\t2026-10-06\n",
            hash_of(&kept),
            hash_of(reply.as_bytes()),
            hash_of(liked.as_bytes())
        )
    );
    let (said, code) = site.run("check", "2026-10-06");
    assert_eq!((said.as_str(), code), ("No problems.\n", Some(0)));

    // Asked again, from the newest kept.
    let (said, code, asked) = site.ask(&|address| answer(address, &feed(&[])));
    assert_eq!((said.as_str(), code), ("No new webmentions.\n", 0));
    assert_eq!(asked, [relay(2028900)]);
}

#[test]
fn the_relay_is_asked_again_while_it_sends_a_hundred() {
    let site = Site::mentioned("pages");
    let (said, code, asked) = site.ask(&|address| {
        let ids: Vec<u64> = match address {
            _ if address == relay(0) => (1..=100).collect(),
            _ => (101..=103).collect(),
        };
        let children: Vec<String> = ids.iter().map(|id| sent(&id.to_string(), &[])).collect();
        answer(address, &feed(&children))
    });
    assert_eq!(code, 0);
    assert_eq!(asked, [relay(0), relay(100)]);
    assert!(said.ends_with("\nFetched 103 webmentions.\n"), "{said}");
    assert_eq!(said.lines().count(), 104);
    assert!(site.holds("fetched/mentions/103.yaml"));
}

#[test]
fn the_relay_is_asked_fifty_times_at_most() {
    let site = Site::mentioned("fifty");
    let (said, code, asked) = site.ask(&|address| {
        let since: u64 = (address.split("since_id=").nth(1).unwrap())
            .split('&')
            .next()
            .unwrap()
            .parse()
            .unwrap();
        let children: Vec<String> = (since + 1..=since + 100)
            .map(|id| {
                sent(
                    &id.to_string(),
                    &[("wm-property", Some(&json("follow-of")))],
                )
            })
            .collect();
        answer(address, &feed(&children))
    });
    assert_eq!(code, 0);
    assert_eq!(asked.len(), 50);
    assert_eq!(asked[49], relay(4900));
    assert!(
        said.ends_with(
            "Stopped asking for webmentions after 50 asks: run `statac refresh` again for \
             the rest.\nDidn't keep 5,000.\n"
        ),
        "{}",
        &said[said.len() - 200..]
    );
}

#[test]
fn each_mention_not_kept_is_said_recorded_and_never_asked_for_again() {
    let site = Site::mentioned("not-kept");
    let children = [
        sent("11", &[("wm-property", Some(&json("follow-of")))]),
        sent("12", &[("url", Some(&json("javascript:alert(1)")))]),
        // With no `url`, the page that sent it.
        sent(
            "13",
            &[
                ("url", None),
                ("wm-source", Some(&json("https://a.example/post-13/"))),
            ],
        ),
        sent(
            "14",
            &[("wm-target", Some(&json("https://example.com/post-1/")))],
        ),
        sent("15", &[("wm-property", None)]),
        sent(&json("../x"), &[]),
        sent("-1", &[]),
        sent("1e3", &[]),
        sent(&json("16"), &[]),
        sent("99999999999999999999999", &[]),
        sent("0", &[]),
        sent("null", &[]),
        "\"a mention that is no object\"".to_owned(),
    ];
    let (said, code, asked) = site.ask(&|address| answer(address, &feed(&children)));
    assert_eq!(
        said,
        "Webmention `11` not kept: Statac doesn't read `follow-of`.\n\
         Webmention `12` not kept: `javascript:alert(1)` isn't an `https://` or `http://` URL.\n\
         Fetched `fetched/mentions/13.yaml`.\n\
         Webmention `14` not kept: `https://example.com/post-1/` isn't on this site.\n\
         Webmention `15` not kept: it doesn't say what kind it is.\n\
         Fetched 1 webmention. Didn't keep 12.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [relay(0)]);
    assert!(
        site.read("fetched/mentions/13.yaml")
            .contains("url: \"https://a.example/post-13/\"\n")
    );
    let record = site.read("fetched/record");
    let absent: Vec<&str> = (record.lines())
        .filter(|line| line.ends_with("\tabsent\t2026-10-06"))
        .collect();
    assert_eq!(
        absent,
        [
            "mentions/11.yaml\tabsent\t2026-10-06",
            "mentions/12.yaml\tabsent\t2026-10-06",
            "mentions/14.yaml\tabsent\t2026-10-06",
            "mentions/15.yaml\tabsent\t2026-10-06",
        ]
    );
    assert_eq!(record.lines().count(), 6, "{record}");

    let (said, _, asked) = site.ask(&|address| answer(address, &feed(&[])));
    assert_eq!(said, "No new webmentions.\n");
    assert_eq!(asked, [relay(15)]);
}

#[test]
fn without_the_token_the_relay_isnt_asked_and_the_stills_are_fetched() {
    let site = Site::mentioned("no-token");
    let (said, code, asked) = site.refresh(&|_| panic!("nothing is asked for"));
    assert_eq!(
        (said.as_str(), code),
        (
            "Didn't ask for webmentions: no token is in `STATAC_WEBMENTION_TOKEN` or `.statac/token`.\n",
            0
        )
    );
    assert!(asked.is_empty());

    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\n---\n@youtube(\"abcdefghijk\")\n",
    );
    let still = jpeg(64, 36);
    let (said, code, asked) = site.refresh(&|address| answer(address, &still));
    assert_eq!(
        said,
        "Fetched `fetched/stills/youtube-abcdefghijk.jpg`.\nFetched 1 thumbnail.\n\
         Didn't ask for webmentions: no token is in `STATAC_WEBMENTION_TOKEN` or `.statac/token`.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [MAXRES]);

    // With the token, the stills' line is left out where none was missing.
    let (said, code, _) = site.ask(&|address| answer(address, &feed(&[])));
    assert_eq!((said.as_str(), code), ("No new webmentions.\n", 0));
}

#[test]
fn a_token_the_relay_refuses_is_said_and_the_exit_code_is_1() {
    let site = Site::mentioned("refused");
    for status in [401, 403] {
        let (said, code, _) = site.ask(&|_| Err(Refusal::Status { status }));
        assert_eq!(
            (said.as_str(), code),
            (
                "Can't ask for webmentions: the webmention service didn't accept the token.\n",
                1
            )
        );
        assert!(!site.holds("fetched"));
    }
}

#[test]
fn an_answer_that_isnt_the_relays_feed_is_said_and_nothing_is_kept() {
    let site = Site::mentioned("unreadable");
    for body in [
        "{\"error\":\"forbidden\",\"error_description\":\"The token is not valid\"}",
        "{\"type\":\"feed\"}",
        "{\"type\":\"feed\",\"children\":{}}",
        "{\"type\":\"entry\",\"children\":[]}",
        "[]",
        "<!doctype html>",
        "{\"type\":\"feed\",\"children\":[",
        "",
    ] {
        let (said, code, _) = site.ask(&|address| answer(address, body.as_bytes()));
        assert_eq!(
            (said.as_str(), code),
            (
                "Can't ask for webmentions: Statac couldn't read the webmention service's answer.\n",
                1
            ),
            "{body}"
        );
        assert!(!site.holds("fetched"), "{body}");
    }
}

#[test]
fn a_relay_that_cant_be_asked_is_said_without_its_address() {
    let site = Site::mentioned("unasked");
    let host = || "webmention.io".to_owned();
    let cases = [
        (
            Refusal::TimedOut,
            "no answer came within 30 seconds".to_owned(),
        ),
        (
            Refusal::Status { status: 503 },
            "the server answered 503".into(),
        ),
        (
            Refusal::Missing { status: 404 },
            "the server has nothing at that URL".into(),
        ),
        (
            Refusal::NotAnAddress,
            format!("`{RELAY}` isn\'t an `https://` or `http://` URL"),
        ),
        (
            Refusal::Credentials,
            format!("`{RELAY}` holds a user name or password"),
        ),
        (
            Refusal::UnknownHost { host: host() },
            "nothing answers to the name `webmention.io`".into(),
        ),
        (
            Refusal::Tls { host: host() },
            "Statac couldn't make a secure connection to `webmention.io`".into(),
        ),
        (
            Refusal::Failed {
                reason: format!("{} gave no answer (os error 104)", relay(0)),
            },
            format!(
                "{RELAY}?token=•••&since_id=0&per-page=100&sort-by=created&sort-dir=up gave no answer"
            ),
        ),
    ];
    for (refusal, reason) in cases {
        let (said, code, _) = site.ask(&|_| Err(refusal.clone()));
        assert_eq!(
            said,
            format!("Can't ask for webmentions: {reason}.\n"),
            "{refusal:?}"
        );
        assert_eq!(code, 1, "{refusal:?}");
    }

    // What came before is kept, and counted.
    let (said, code, asked) = site.ask(&|address| match address {
        _ if address == relay(0) => {
            let children: Vec<String> = (1..=100).map(|id| sent(&id.to_string(), &[])).collect();
            answer(address, &feed(&children))
        }
        _ => Err(Refusal::TimedOut),
    });
    assert_eq!(code, 1);
    assert_eq!(asked, [relay(0), relay(100)]);
    assert!(
        said.ends_with(
            "Fetched `fetched/mentions/100.yaml`.\n\
             Can't ask for webmentions: no answer came within 30 seconds.\n\
             Fetched 100 webmentions.\n"
        ),
        "{said}"
    );
    assert!(site.read("fetched/record").contains("mentions/100.yaml\t"));
}

#[test]
fn a_face_is_made_again_from_its_pixels_or_left_out() {
    let site = Site::mentioned("faces");
    let face = face_name(FACE);
    let kept = |site: &Site| fs::read(site.path(&format!("fetched/faces/{face}"))).unwrap();

    // Narrower than 96, it keeps its size.
    let (said, code, _) = site.ask(&|address| match address {
        FACE => answer(address, &jpeg(40, 30)),
        _ => answer(address, &feed(&[with_face(1)])),
    });
    assert_eq!(code, 0, "{said}");
    assert_eq!(size_of(&kept(&site)), (40, 30));

    // Its metadata is lost: only the pixels are written.
    let site = Site::mentioned("faces-metadata");
    let mut tagged = jpeg(120, 120);
    let mut exif =
        b"\xff\xe1\x00\x16Exif\x00\x00MM\x00\x2a\x00\x00\x00\x08\x00\x00\x00\x00\x00\x00".to_vec();
    exif.extend(b"\xff\xfe\x00\x14Taken at a.example");
    tagged.splice(2..2, exif);
    assert_eq!(size_of(&tagged), (120, 120));
    let (said, code, _) = site.ask(&|address| match address {
        FACE => answer(address, &tagged),
        _ => answer(address, &feed(&[with_face(1)])),
    });
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Fetched `fetched/faces/"), "{said}");
    let bytes = kept(&site);
    assert_eq!(size_of(&bytes), (96, 96));
    for metadata in [&b"Exif"[..], b"a.example", b"\xff\xe1"] {
        assert!(!bytes.windows(metadata.len()).any(|w| w == metadata));
    }
    assert_ne!(bytes, tagged);

    // A face shared by two mentions is fetched once.
    let site = Site::mentioned("faces-shared");
    let (said, _, asked) = site.ask(&|address| match address {
        FACE => answer(address, &jpeg(200, 200)),
        _ => answer(address, &feed(&[with_face(1), with_face(2)])),
    });
    assert!(
        said.ends_with("Fetched 2 webmentions and 1 avatar.\n"),
        "{said}"
    );
    assert_eq!(asked, [relay(0), FACE.to_owned()]);
    // And one already there is never fetched.
    let (said, _, asked) = site.ask(&|address| match address {
        FACE => panic!("the face is there"),
        _ if address == relay(2) => answer(address, &feed(&[with_face(3)])),
        _ => answer(address, &feed(&[])),
    });
    assert_eq!(
        said,
        "Fetched `fetched/mentions/3.yaml`.\nFetched 1 webmention.\n"
    );
    assert_eq!(asked, [relay(2)]);
    assert!(
        site.read("fetched/mentions/3.yaml")
            .contains(&format!("face: \"{face}\"\n"))
    );
}

#[test]
fn a_face_that_cant_be_fetched_or_read_costs_the_mention_nothing() {
    let wide = {
        let picture = RgbImage::from_fn(4097, 1, |x, _| Rgb([(x % 256) as u8, 0, 0]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        picture.write_to(&mut bytes, ImageFormat::Png).unwrap();
        bytes.into_inner()
    };
    let cases: [(Result<Fetched, Refusal>, &str); 4] = [
        (
            answer(FACE, b"<!doctype html>"),
            "the answer isn't an image",
        ),
        (
            answer(FACE, &jpeg(64, 64)[..300]),
            "the answer isn't an image",
        ),
        (
            answer(FACE, &wide),
            "the image is 4,097 by 1 pixels, too large to open",
        ),
        (missing(), "the server has nothing at that URL"),
    ];
    for (at, (fetched, reason)) in cases.into_iter().enumerate() {
        let site = Site::mentioned(&format!("no-face-{at}"));
        let (said, code, _) = site.ask(&|address| match address {
            FACE => fetched.clone(),
            _ => answer(address, &feed(&[with_face(1)])),
        });
        assert_eq!(
            said,
            format!(
                "Can't fetch the avatar for `1`: {reason}.\n\
                 Fetched `fetched/mentions/1.yaml`.\nFetched 1 webmention.\n"
            )
        );
        assert_eq!(code, 0);
        assert!(!site.read("fetched/mentions/1.yaml").contains("face:"));
        assert!(!site.holds("fetched/faces"));
    }
}

#[test]
fn a_strangers_addresses_are_kept_only_on_the_web() {
    let site = Site::mentioned("hostile-addresses");
    let children = [sent(
        "1",
        &[
            ("author.url", Some(&json("javascript:alert(1)"))),
            ("author.photo", Some(&json("javascript:alert(1)"))),
        ],
    )];
    let (said, code, asked) = site.ask(&|address| answer(address, &feed(&children)));
    assert_eq!(
        said,
        "Webmention `1`: `javascript:alert(1)` isn't an `https://` or `http://` URL, so Statac \
         doesn't keep the sender's URL.\n\
         Can't fetch the avatar for `1`: `javascript:alert(1)` isn't an `https://` or `http://` URL.\n\
         Fetched `fetched/mentions/1.yaml`.\nFetched 1 webmention.\n"
    );
    assert_eq!(code, 0);
    assert_eq!(asked, [relay(0)]);
    let kept = site.read("fetched/mentions/1.yaml");
    assert!(
        !kept.contains("javascript") && !kept.contains("face:"),
        "{kept}"
    );
}

#[test]
fn a_strangers_words_reach_the_page_as_text() {
    let site = Site::mentioned("hostile-words");
    site.write(
        "templates/page.html",
        "<!doctype html>\n<title>{{ headline(page) }}</title>\n\
         {% for mention in page.webmentions.comments %}\
         <p>{% if mention.author %}{{ mention.author }}{% endif %}|{{ mention.url }}|\
         {% if mention.content %}{{ mention.content }}{% endif %}</p>\n{% endfor %}",
    );
    let words = "\"Quoted\" and 'quoted', a \\ backslash: a colon\non a line\r\n# a hash - a dash \
                 = =today @youtube(\"abcdefghijk\") {{ site.title }} {% if page %}x{% endif %} \
                 <script>alert(1)</script> & Grüße, привет, 你好 🙂\u{202E}\u{1b}[31m\u{7}";
    let author = "A \"<b>\" \\ {{ page.title }}";
    let children = [sent(
        "1",
        &[
            ("content.text", Some(&json(words))),
            ("author.name", Some(&json(author))),
            ("url", Some(&json("https://a.example/post-1/?a=1&b=\"2\"<"))),
        ],
    )];
    let (said, code, _) = site.ask(&|address| answer(address, &feed(&children)));
    assert_eq!(code, 0, "{said}");
    let (said, code) = site.run("build", "2026-10-06");
    assert_eq!(code, Some(0), "{said}");
    let page = site.read("_site/post-1/index.html");
    let shown = page.split_once("<p>").unwrap().1;
    assert_eq!(
        shown,
        "A &quot;&lt;b&gt;&quot; \\ {{ page.title }}|https://a.example/post-1/?a=1&amp;b=&quot;2&quot;&lt;|\
         &quot;Quoted&quot; and &#x27;quoted&#x27;, a \\ backslash: a colon on a line # a hash - a dash \
         = =today @youtube(&quot;abcdefghijk&quot;) {{ site.title }} {% if page %}x{% endif %} \
         &lt;script&gt;alert(1)&lt;/script&gt; &amp; Grüße, привет, 你好 🙂[31m</p>\n"
    );
}

#[test]
fn build_says_how_old_the_webmentions_are() {
    let site = Site::mentioned("mention-ages");
    let said = |today: &str| {
        let (said, code) = site.run("build", today);
        assert_eq!(code, Some(0), "{said}");
        said.lines()
            .filter(|line| line.starts_with("Fetched"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    site.ask(&|address| answer(address, &feed(&[sent("1", &[])])));
    assert_eq!(said("2026-10-07"), "Fetched: 1 webmention, yesterday.");
    site.ask(&|address| answer(address, &feed(&[sent("2", &[])])));
    assert_eq!(
        said("2026-10-06"),
        "Fetched: 2 webmentions, the newest today."
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\n---\n@youtube(\"abcdefghijk\")\n",
    );
    site.put("fetched/stills/youtube-abcdefghijk.jpg", &jpeg(32, 18));
    assert_eq!(
        said("2026-10-12"),
        "Fetched: 1 thumbnail, with no date; 2 webmentions, the newest 6 days ago."
    );
    site.write("fetched/record", "statac fetched 1\n");
    assert_eq!(
        said("2026-10-12"),
        "Fetched: 1 thumbnail, with no date; 2 webmentions, with no date."
    );
}

#[test]
fn webmentions_on_a_site_without_url_are_not_asked_for() {
    let site = Site::new("mentions-no-url");
    site.write("statac.yaml", &format!("{SETTINGS}webmentions: true\n"));
    let (said, code, asked) = site.ask(&|_| panic!("nothing is asked for"));
    assert_eq!(
        said,
        "note  No site URL: Statac leaves out webmentions until the site has a `url`\n  \
         statac.yaml, line 5\n   5 │ webmentions: true\n     │              ^^^^\n  \
         Add the site's full URL to `statac.yaml`, such as `url: https://example.org/`.\n  \
         More: https://statac.dev/e/no-site-url/\nNothing to fetch.\n"
    );
    assert_eq!(code, 0);
    assert!(asked.is_empty());
}

#[test]
fn a_refused_mention_and_anything_from_a_refused_sender_is_not_kept() {
    let site = Site::mentioned("refused-list");
    let settings = site.read("statac.yaml");
    site.write(
        "statac.yaml",
        &format!(
            "{settings}refuse_webmentions_from:\n  - https://a.example/\n  \
             - https://b.example/post-12/\n"
        ),
    );
    let from = |id: &str, url: &str, author: &str| {
        sent(
            id,
            &[
                ("url", Some(&json(url))),
                ("wm-source", Some(&json(url))),
                ("author.url", Some(&json(author))),
                ("author.photo", Some(&json(FACE))),
            ],
        )
    };
    let children = [
        from("12", "http://b.example/post-12", "https://b.example/"),
        from("13", "https://a.example/post-13/", "https://a.example/"),
        // By its sender's address alone.
        from("14", "https://d.example/post-14/", "https://www.A.example"),
        from("15", "https://c.example/post-15/", "https://c.example/"),
    ];
    let (said, code, asked) = site.ask(&|address| match address {
        FACE => answer(address, &jpeg(48, 48)),
        _ => answer(address, &feed(&children)),
    });
    assert_eq!(
        said,
        format!(
            "Webmention `12` not kept: `refuse_webmentions_from` refuses `https://b.example/post-12/`.\n\
             Webmention `13` not kept: `refuse_webmentions_from` refuses `https://a.example/`.\n\
             Webmention `14` not kept: `refuse_webmentions_from` refuses `https://a.example/`.\n\
             Fetched `fetched/faces/{}`.\n\
             Fetched `fetched/mentions/15.yaml`.\n\
             Fetched 1 webmention and 1 avatar. Didn't keep 3.\n",
            face_name(FACE)
        )
    );
    assert_eq!(code, 0);
    // The face is fetched for the mention kept, and for none refused.
    assert_eq!(asked, [relay(0), FACE.to_owned()]);
    let record = site.read("fetched/record");
    let absent: Vec<&str> = (record.lines())
        .filter(|line| line.contains("\tabsent\t"))
        .collect();
    assert_eq!(
        absent,
        [
            "mentions/12.yaml\tabsent\t2026-10-06",
            "mentions/13.yaml\tabsent\t2026-10-06",
            "mentions/14.yaml\tabsent\t2026-10-06",
        ]
    );
    assert!(!site.holds("fetched/mentions/12.yaml"));
}

#[test]
fn a_mention_kept_and_then_refused_is_removed_for_good() {
    let site = Site::mentioned("refused-later");
    let other = sent(
        "10",
        &[
            ("url", Some(&json("https://c.example/post-10/"))),
            ("author.url", Some(&json("https://c.example/"))),
        ],
    );
    let refused = sent("11", &[]);
    let both = [other.clone(), refused.clone()];
    site.ask(&|address| answer(address, &feed(&both)));
    assert!(site.holds("fetched/mentions/11.yaml"));
    let settings = site.read("statac.yaml");
    site.write(
        "statac.yaml",
        &format!("{settings}refuse_webmentions_from: [https://a.example/post-1/]\n"),
    );

    let (said, code, asked) = site.ask(&|address| answer(address, &feed(&[])));
    assert_eq!(
        said,
        "Removed `fetched/mentions/11.yaml`.\n\
         Removed 1 refused webmention.\n\
         No new webmentions.\n"
    );
    assert_eq!((code, asked), (0, vec![relay(11)]));
    assert!(!site.holds("fetched/mentions/11.yaml"));
    assert!(site.holds("fetched/mentions/10.yaml"));
    assert!(
        (site.read("fetched/record").lines())
            .any(|line| line == "mentions/11.yaml\tabsent\t2026-10-06")
    );

    // Taken out of the list, it isn't asked for again, nor kept if sent.
    site.write("statac.yaml", &settings);
    let (said, _, asked) =
        site.ask(&|address| answer(address, &feed(std::slice::from_ref(&refused))));
    assert_eq!(said, "No new webmentions.\n");
    assert_eq!(asked, [relay(11)]);
    assert!(!site.holds("fetched/mentions/11.yaml"));
}
