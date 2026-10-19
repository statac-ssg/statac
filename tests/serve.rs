//! A save in a served site reaches a waiting browser, as a reload or a
//! stylesheet swapped in, and the terminal says what happened. Each test
//! serves its own small site from the system's temporary folder on a free
//! port of this machine, and speaks HTTP over a plain socket, under the
//! system's own watching and under polling.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use image::{ImageFormat, Rgb, RgbImage};
use statac::fetch::{Fetched, Refusal};
use statac::refresh::Token;
use statac::serve::live::Hub;
use statac::serve::watch::{How, Settle};
use statac::serve::{self, Fetcher, Options};

/// Generous, for a machine busy with other tests.
const BOUND: Duration = Duration::from_secs(10);

const LAYOUT: &str = "\
<!doctype html>
<html lang=\"{{ site.language }}\">
<head>
<title>{{ page.title }}</title>
{{ css(\"site.css\") }}
</head>
<body>
{{ page.content }}
</body>
</html>
";

struct Site(PathBuf);

impl Site {
    fn new(label: &str) -> Site {
        let unique = format!(
            "statac-serve-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let site = Site(std::env::temp_dir().join(unique));
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: en\ntimezone: Europe/London\n",
        );
        site.write("templates/page.html", LAYOUT);
        site.write(
            "content/index.md",
            "---\nlayout: page\n---\n# Home\n\nThe home page.\n",
        );
        site.write("content/post-1.md", "# Post 1\n\nThe first post.\n");
        site.write("assets/site.css", "body { color: black; }\n");
        site
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.0.clone(), |path, piece| path.join(piece))
    }

    fn write(&self, relative: &str, text: &str) {
        let path = self.path(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// What serve wrote to its terminal, kept for the test to read.
#[derive(Clone, Default)]
struct Said(Arc<Mutex<Vec<u8>>>);

impl Write for Said {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Said {
    fn text(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }

    /// Everything said from `mark` on, once it holds `wanted`.
    fn after(&self, mark: usize, wanted: &str) -> String {
        let began = Instant::now();
        loop {
            let text = self.text();
            if text[mark..].contains(wanted) {
                return text[mark..].to_owned();
            }
            assert!(
                began.elapsed() < BOUND,
                "{wanted:?} was never said:\n{text}"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

struct Served {
    site: Site,
    port: u16,
    hub: Arc<Hub>,
    said: Said,
    polling: Option<Duration>,
}

fn served(label: &str, how: How) -> Served {
    served_as(label, how, true, |_| {})
}

/// A site served once `ready` has made it what the test wants, showing
/// drafts or not.
fn served_as(label: &str, how: How, drafts: bool, ready: impl FnOnce(&Site)) -> Served {
    served_fetching(label, how, drafts, None, None, ready).0
}

/// [`served_as`], fetching with `fetcher` and `token`, with what serve
/// said as it opened.
fn served_fetching(
    label: &str,
    how: How,
    drafts: bool,
    fetcher: Option<Fetcher>,
    token: Option<Token>,
    ready: impl FnOnce(&Site),
) -> (Served, String) {
    let site = Site::new(label);
    ready(&site);
    let options = Options {
        site: site.0.clone(),
        port: 0,
        watch: how,
        // Wide enough that a stalled machine doesn't split a burst of saves.
        settle: Settle {
            quiet: Duration::from_millis(250),
            longest: Duration::from_secs(2),
        },
        today: None,
        drafts,
        fetcher,
        token,
    };
    let mut said = Said::default();
    let mut serving = serve::start(&options, &mut said, false)
        .unwrap_or_else(|| panic!("serve didn't start:\n{}", said.text()));
    let opened = said.text();
    let (port, hub, polling) = (serving.port(), serving.hub().clone(), serving.polling());
    {
        let mut said = said.clone();
        std::thread::spawn(move || while serving.round(&mut said) {});
    }
    // The system may still be reporting the site's own making; a test
    // starts once that has settled.
    let mut generation = hub.generation();
    let mut quiet_since = Instant::now();
    while quiet_since.elapsed() < Duration::from_millis(700) {
        std::thread::sleep(Duration::from_millis(50));
        if hub.generation() != generation {
            generation = hub.generation();
            quiet_since = Instant::now();
        }
    }
    let served = Served {
        site,
        port,
        hub,
        said,
        polling,
    };
    (served, opened)
}

/// A bare HTTP/1.1 GET, answered with the status line and the body.
fn get(port: u16, target: &str) -> (String, String) {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    write!(
        stream,
        "GET {target} HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let text = String::from_utf8(raw).unwrap();
    let (head, body) = text.split_once("\r\n\r\n").unwrap();
    (head.lines().next().unwrap().to_owned(), body.to_owned())
}

/// What stands between `before` and `after` in `text`.
fn between<'a>(text: &'a str, before: &str, after: &str) -> &'a str {
    let from = text
        .find(before)
        .unwrap_or_else(|| panic!("no {before:?} in:\n{text}"))
        + before.len();
    &text[from..from + text[from..].find(after).unwrap()]
}

impl Served {
    fn page(&self, address: &str) -> String {
        let (status, body) = get(self.port, address);
        assert!(status.contains("200"), "{address}: {status}");
        body
    }

    /// The generation a page was answered at, from the client added to it.
    fn page_generation(&self) -> u64 {
        between(&self.page("/post-1/"), "data-live=\"", "\"")
            .parse()
            .unwrap()
    }

    /// The address the pages link the stylesheet at.
    fn stylesheet(&self) -> String {
        between(&self.page("/post-1/"), "rel=\"stylesheet\" href=\"", "\"").to_owned()
    }

    /// Holds a page's wait open, then makes the change, and gives the
    /// answer and how long it took.
    fn change_and_wait(&self, change: impl FnOnce()) -> (String, Duration) {
        let port = self.port;
        let since = self.page_generation();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (_, body) = get(port, &format!("/.statac-live/wait?since={since}"));
            let _ = tx.send((body, Instant::now()));
        });
        // The wait must be held before the change, or this would test a
        // page catching up instead.
        let began = Instant::now();
        while self.hub.waiting() == 0 {
            assert!(began.elapsed() < BOUND, "the wait was never held");
            std::thread::sleep(Duration::from_millis(5));
        }
        let changed_at = Instant::now();
        change();
        let (body, answered_at) = rx.recv_timeout(BOUND).expect("the page heard nothing");
        (body, answered_at - changed_at)
    }
}

fn each_way(label: &str, test: impl Fn(&str, Served)) {
    each_way_as(label, |_| {}, test);
}

/// [`each_way`], each site made what the test wants by `ready`.
fn each_way_as(label: &str, ready: impl Fn(&Site), test: impl Fn(&str, Served)) {
    let ways = [
        ("native", How::Native),
        (
            "poll",
            How::Poll {
                every: Duration::from_millis(100),
            },
        ),
    ];
    for (way, how) in ways {
        let served = served_as(&format!("{label}-{way}"), how, true, &ready);
        assert_eq!(
            served.polling.is_some(),
            way == "poll",
            "{way}:\n{}",
            served.said.text()
        );
        test(way, served);
    }
}

#[test]
fn serve_says_where_the_site_is() {
    each_way("start", |way, served| {
        let said = served.said.text();
        let folder = served.site.path(".statac/serve");
        let mut wanted = format!(
            "Serving `{}` at http://localhost:{}/\n",
            folder.display(),
            served.port
        );
        if way == "poll" {
            wanted.push_str("Watching by polling every 100 ms.\n");
        }
        wanted.push_str("Press Ctrl+C to stop.\n");
        // The system may report the site's own making as a first save.
        assert!(said.contains(&wanted), "{way}:\n{said}");
        assert!(
            said.starts_with(&format!(
                "Wrote 2 pages and 2 other files to `{}` in ",
                folder.display()
            )),
            "{way}:\n{said}"
        );
        assert!(served.page("/").contains("<p>The home page.</p>"));
        assert!(folder.join("post-1").join("index.html").is_file());
    });
}

#[test]
fn a_saved_stylesheet_is_swapped_in_not_reloaded() {
    each_way("css", |way, served| {
        let old = served.stylesheet();
        let mark = served.said.text().len();
        let (body, took) = served.change_and_wait(|| {
            served
                .site
                .write("assets/site.css", "body { color: rebeccapurple; }\n");
        });
        println!("{way}: a stylesheet swap reached the page in {took:?}");
        let new = served.stylesheet();
        assert_ne!(old, new, "{way}");
        assert!(body.contains("\"kind\":\"styles\""), "{way}: {body}");
        assert!(
            body.contains(&format!("\"styles\":[[\"{old}\",\"{new}\"]]")),
            "{way}: {body}"
        );
        assert!(
            body.contains("\"changed\":[\"assets/site.css\"]"),
            "{way}: {body}"
        );
        assert!(served.page(&new).contains("rebeccapurple"), "{way}");
        let said = served.said.after(mark, "\n");
        assert!(
            said.starts_with("`assets/site.css` changed. Built ")
                && said.ends_with(" seconds. Browser told to swap 1 stylesheet.\n"),
            "{way}: {said}"
        );
    });
}

#[test]
fn a_saved_page_reloads() {
    each_way("page", |way, served| {
        let mark = served.said.text().len();
        let (body, took) = served.change_and_wait(|| {
            served
                .site
                .write("content/post-1.md", "# Post 1\n\nThe first post, edited.\n");
        });
        println!("{way}: a reload reached the page in {took:?}");
        assert!(body.contains("\"kind\":\"reload\""), "{way}: {body}");
        assert!(
            body.contains("\"changed\":[\"content/post-1.md\"]"),
            "{way}: {body}"
        );
        assert!(served.page("/post-1/").contains("The first post, edited."));
        let said = served.said.after(mark, "\n");
        assert!(
            said.starts_with("`content/post-1.md` changed. Built ")
                && said.ends_with(" seconds. Browser told to reload.\n"),
            "{way}: {said}"
        );
    });
}

#[test]
fn a_save_through_a_temporary_file_is_one_change_to_the_real_file() {
    each_way("atomic", |way, served| {
        let post = served.site.path("content/post-1.md");
        let temporary = served.site.path("content/.post-1.md.tmp");
        let (body, _) = served.change_and_wait(|| {
            std::fs::write(&temporary, "# Post 1\n\nSaved in one step.\n").unwrap();
            std::fs::rename(&temporary, &post).unwrap();
        });
        assert!(
            body.contains("\"changed\":[\"content/post-1.md\"]"),
            "{way}: {body}"
        );
        assert!(served.page("/post-1/").contains("Saved in one step."));
    });
}

#[test]
fn a_burst_of_saves_is_one_round() {
    each_way("burst", |way, served| {
        let before = served.hub.generation();
        let mark = served.said.text().len();
        for n in 10..20 {
            served.site.write(
                &format!("content/post-{n}.md"),
                &format!("# Post {n}\n\nOne of a burst.\n"),
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        let said = served.said.after(mark, "\n");
        assert!(
            said.starts_with("`content/post-10.md` and 9 other files changed. Built "),
            "{way}: {said}"
        );
        // Nothing more once the burst has settled.
        std::thread::sleep(Duration::from_secs(1));
        assert_eq!(served.hub.generation(), before + 1, "{way}");
        let (_, body) = get(
            served.port,
            &format!("/.statac-live/wait?since={before}&timeout=0"),
        );
        for n in 10..20 {
            assert!(
                body.contains(&format!("content/post-{n}.md")),
                "{way}: {body}"
            );
        }
        assert!(served.page("/post-19/").contains("One of a burst."));
    });
}

#[test]
fn a_build_that_stops_keeps_the_last_pages_and_says_why() {
    each_way("stopped", |way, served| {
        let mark = served.said.text().len();
        let (body, _) = served.change_and_wait(|| {
            served.site.write(
                "content/post-1.md",
                "---\ndate: 4/10/2026\n---\n# Post 1\n\nNever shown.\n",
            );
        });
        assert!(body.contains("\"kind\":\"said\""), "{way}: {body}");
        assert!(
            body.contains(
                "\"status\":{\"stopped\":true,\
                 \"line\":\"1 error stopped the build. This page is as last built.\",\
                 \"problems\":\"error  Unclear date: `4/10/2026` could be 4 October or 10 April"
            ),
            "{way}: {body}"
        );
        assert!(served.page("/post-1/").contains("The first post."), "{way}");
        // Windows names the file with its own slash.
        let said = served
            .said
            .after(mark, "unclear-date/\n\n")
            .replace('\\', "/");
        assert_eq!(
            said,
            "\
`content/post-1.md` changed. 1 error stopped the build. The browser keeps the pages as last built.

error  Unclear date: `4/10/2026` could be 4 October or 10 April
  content/post-1.md, line 2
   2 \u{2502} date: 4/10/2026
     \u{2502}       ^^^^^^^^^
  Write it as `2026-10-04` or `2026-04-10`.
  More: https://statac.dev/e/unclear-date/

",
            "{way}"
        );

        let mark = served.said.text().len();
        let (body, _) = served.change_and_wait(|| {
            served
                .site
                .write("content/post-1.md", "# Post 1\n\nShown again.\n");
        });
        assert!(body.contains("\"kind\":\"reload\""), "{way}: {body}");
        assert!(body.contains("\"stopped\":false"), "{way}: {body}");
        assert!(served.page("/post-1/").contains("Shown again."), "{way}");
        let said = served.said.after(mark, "unclear-date.\n");
        assert!(
            said.starts_with("`content/post-1.md` changed. Built ")
                && said.ends_with(
                    " seconds. Browser told to reload.\n\
                     Gone since the last build: 1 unclear-date.\n"
                ),
            "{way}: {said}"
        );
    });
}

#[test]
fn a_page_answered_after_a_round_is_not_told_of_it() {
    let served = served("settled", How::Native);
    let before = served.hub.generation();
    let mark = served.said.text().len();
    served.site.write(
        "content/index.md",
        "---\nlayout: page\n---\n# Home\n\nChanged.\n",
    );
    served.said.after(mark, "Browser told to reload.\n");
    let since = served.page_generation();
    assert_eq!(since, before + 1);
    let (_, body) = get(
        served.port,
        &format!("/.statac-live/wait?since={since}&timeout=300"),
    );
    assert!(body.contains("\"kind\":\"none\""), "{body}");
}

#[test]
fn reading_the_site_is_not_a_change() {
    let served = served("reads", How::Native);
    let before = served.hub.generation();
    for _ in 0..5 {
        served.page("/post-1/");
        served.page(&served.stylesheet());
        std::fs::read(served.site.path("content/post-1.md")).unwrap();
    }
    std::thread::sleep(Duration::from_secs(1));
    assert_eq!(served.hub.generation(), before);
}

#[test]
fn a_miss_is_answered_404() {
    let served = served("miss", How::Native);
    let (status, body) = get(served.port, "/post-3/");
    assert!(status.contains("404"), "{status}");
    assert!(body.contains("Nothing is published at this address."));
    assert!(body.contains("/.statac-live/client.js"));
    let (status, _) = get(served.port, "/%2e%2e/statac.yaml");
    assert!(status.contains("400"), "{status}");
    let (status, body) = get(served.port, "/.statac-live/client.js");
    assert!(status.contains("200") && body.contains("statac-live"));
}

#[test]
fn a_stamp_saved_is_written_and_built_in_one_round() {
    each_way("stamp", |way, served| {
        let before = served.hub.generation();
        let mark = served.said.text().len();
        served.site.write(
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: =yesterday\n---\nThe second post.\n",
        );
        let stamped = "Stamped `content/post-2.md`: `date` is `";
        served.said.after(mark, stamped);
        // Serve's own write to the file is no save to build for.
        std::thread::sleep(Duration::from_millis(1500));
        assert_eq!(served.hub.generation(), before + 1, "{way}");
        let said = served.said.text()[mark..].to_owned();
        assert!(
            said.starts_with("`content/post-2.md` changed. Built ")
                && said.contains(" seconds. Browser told to reload.\nStamped `"),
            "{way}: {said}"
        );
        assert_eq!(said.matches(" changed. ").count(), 1, "{way}: {said}");
        let date = between(&said, stamped, "`.\n");
        let shape: String = date
            .chars()
            .map(|c| if c.is_ascii_digit() { '0' } else { c })
            .collect();
        assert_eq!(shape, "0000-00-00", "{way}: {said}");
        assert_eq!(
            std::fs::read_to_string(served.site.path("content/post-2.md")).unwrap(),
            format!("---\ntitle: Post 2\ndate: {date}\n---\nThe second post.\n"),
            "{way}"
        );
        assert!(served.page("/post-2/").contains("The second post."));

        // The file saved again as it was stamped is no change either;
        // saved with something new, it is.
        let mark = served.said.text().len();
        served.site.write(
            "content/post-2.md",
            &format!("---\ntitle: Post 2\ndate: {date}\n---\nThe second post, edited.\n"),
        );
        let said = served.said.after(mark, "\n");
        assert!(
            said.starts_with("`content/post-2.md` changed. Built "),
            "{way}: {said}"
        );
        assert!(served.page("/post-2/").contains("The second post, edited."));
    });
}

/// A file's name and a field's name are the site's own text: neither
/// reaches the terminal as written in the line for a stamp.
#[cfg(unix)]
#[test]
fn a_stamps_line_shows_a_name_a_terminal_would_act_on_harmless() {
    let served = served("stamp-escapes", How::Native);
    let mark = served.said.text().len();
    served.site.write(
        "content/post\u{1b}[2J\r-\n2.md",
        "---\ntitle: Post 2\nevent:\n  \"at\\e[2J\\r\\nfirst\": =yesterday\n---\nThe second post.\n",
    );
    let stamped = "Stamped `content/post\u{241b}[2J\u{240d}-\u{240a}2.md`: \
                   `event.at\u{241b}[2J\u{240d}\u{240a}first` is `";
    let said = served.said.after(mark, stamped);
    let line = said.lines().find(|line| line.starts_with("Stamped `"));
    assert!(!line.unwrap().contains(['\u{1b}', '\r']), "{said:?}");
}

/// The panel's answer for the page at `address`.
fn panel(served: &Served, address: &str) -> String {
    let asked = address.replace('/', "%2F");
    served.page(&format!("/.statac-live/panel?page={asked}"))
}

#[test]
fn the_panel_is_handed_what_there_is_to_say_of_a_page_and_the_site() {
    let served = served_as("panel", How::Native, true, |site| {
        site.write(
            "content/post-2.md",
            "---\ntitle: Post <2>\ndraft: true\n---\nA draft.\n",
        );
        site.write(
            "content/post-3.md",
            "---\ntitle: Post 3\ndate: 2999-01-01\n---\nSee [nothing](/post-9/).\n",
        );
    });
    let said = served.said.text();
    assert!(
        said.contains(
            "Showing 1 draft and 1 post waiting for its day, which `statac build` leaves out.\n"
        ),
        "{said}"
    );
    let plain = panel(&served, "/post-1/");
    assert!(
        plain.starts_with(
            "{\"v\":2,\"problems\":[],\"stopped\":null,\"explanations\":{},\
             \"round\":{\"state\":\"built\",\"at\":\""
        ) && plain.contains("\"counts\":{\"error\":0,\"warning\":1,\"note\":1}}")
            && plain.contains(
                "\"site\":{\"kinds\":[\
                 {\"kind\":\"broken-link\",\"severity\":\"warning\",\"title\":\"Broken link\",\
                 \"count\":1,\"quietened\":0,\"suggestion\":[{\"text\":\"Did you mean \"},\
                 {\"code\":\"/post-1/\"},{\"text\":\"?\"}]},\
                 {\"kind\":\"no-site-address\",\"severity\":\"note\",\"title\":\"No site address\",\
                 \"count\":1,\"quietened\":0,\"suggestion\":[{\"text\":\"Add the site\\u0027s address \
                 to \"},{\"code\":\"statac.yaml\"},{\"text\":\", as \"},\
                 {\"code\":\"url: https://example.org/\"},{\"text\":\".\"}]}],\"gone\":null,\
                 \"drafts\":[{\"text\":\"Post \\u003c2\\u003e\",\"href\":\"/post-2/\"}],\
                 \"waiting\":[{\"text\":\"Post 3\",\"href\":\"/post-3/\",\"day\":\"2999-01-01\"}],\
                 \"written\":[]},"
            ),
        "{plain}"
    );
    // What builds the page, then every name its template can write, with
    // what each holds here and where that is written.
    assert!(
        plain.contains(
            "\"page\":{\"title\":\"Post 1\",\"draft\":false,\"waiting\":null,\
             \"file\":\"content/post-1.md\",\"address\":\"/post-1/\","
        ) && plain.contains(
            "\"layout\":{\"name\":\"page\",\"file\":\"templates/page.html\",\"extends\":[],\
             \"chosenAt\":null,\"handedDown\":false}"
        ) && plain.contains(
            "{\"name\":\"page.summary\",\"holds\":\"text\",\"value\":\"no value\",\
             \"empty\":true,\"has\":[],\"from\":null,"
        ) && plain.contains("{\"name\":\"page.url\",\"holds\":\"text\",\"value\":\"/post-1/\","),
        "{plain}"
    );
    let draft = panel(&served, "/post-2/");
    assert!(
        draft.contains("\"problems\":[],\"stopped\":null,")
            && draft.contains(
                "\"page\":{\"title\":\"Post \\u003c2\\u003e\",\"draft\":true,\"waiting\":null,"
            ),
        "{draft}"
    );
    // By the file's own name too, with the page's own problem.
    let waiting = panel(&served, "/post-3/index.html");
    assert!(
        waiting
            .contains("\"page\":{\"title\":\"Post 3\",\"draft\":false,\"waiting\":\"2999-01-01\",")
            && waiting.contains(
                "\"problems\":[{\"severity\":\"warning\",\"kind\":\"broken-link\",\
                 \"headline\":[{\"text\":\"Broken link: nothing is published at \"},\
                 {\"code\":\"/post-9/\"}],\"file\":\"content/post-3.md\","
            ),
        "{waiting}"
    );
    // An address with no page at it is told of the site alone.
    let nowhere = panel(&served, "/site.css");
    assert!(
        nowhere.contains("\"site\":{\"kinds\":[{\"kind\":\"broken-link\",")
            && nowhere.ends_with("\"page\":null,\"names\":[]}"),
        "{nowhere}"
    );

    // A round that stops says why, over what was last built; a stamp
    // written in the same round is said too.
    let mark = served.said.text().len();
    served.site.write(
        "content/post-1.md",
        "---\ndate: 4/10/2026\nupdated: =yesterday\n---\n# Post 1\n\nNever shown.\n",
    );
    served.said.after(mark, "unclear-date/\n\n");
    let stopped = panel(&served, "/post-2/");
    assert!(
        stopped.contains("\"stopped\":{\"at\":\"")
            && stopped.contains(
                "\"errors\":[{\"severity\":\"error\",\"kind\":\"unclear-date\",\
             \"headline\":[{\"text\":\"Unclear date: \"},{\"code\":\"4/10/2026\"},\
             {\"text\":\" could be 4 October or 10 April\"}],\
             \"file\":\"content/post-1.md\",\"line\":2,"
            )
            && stopped.contains(
                "\"suggestion\":[{\"text\":\"Write it as \"},{\"code\":\"2026-10-04\"},\
             {\"text\":\" or \"},{\"code\":\"2026-04-10\"},{\"text\":\".\"}],"
            )
            && stopped.contains("\"page\":{\"title\":\"Post \\u003c2\\u003e\",\"draft\":true,")
            && stopped.contains(
                "\"waiting\":[{\"text\":\"Post 3\",\"href\":\"/post-3/\",\"day\":\"2999-01-01\"}],\
                 \"written\":[[{\"text\":\"Stamped \"},{\"code\":\"content/post-1.md\"},\
                 {\"text\":\": \"},{\"code\":\"updated\"},{\"text\":\" is \"},{\"code\":\""
            ),
        "{stopped}"
    );
}

#[test]
fn the_panel_is_handed_each_problem_in_parts_and_the_rounds() {
    let served = served_as("panel-parts", How::Native, true, |site| {
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: en\ntimezone: Europe/London\nquiet: [broken-link]\n",
        );
        site.write(
            "templates/page.html",
            &LAYOUT.replace("</body>", "<a href=\"/post-8/\">Post 8</a>\n</body>"),
        );
        site.write(
            "content/post-2.md",
            "# Post 2\n\nSee [Post 9](/post-9/)\tthen.\n\nThe end.\n",
        );
    });
    // The page's own problem, quietened, with the lines either side of
    // its own as the terminal shows them.
    let built = panel(&served, "/post-2/");
    let link = "{\"severity\":\"warning\",\"kind\":\"broken-link\",\
        \"headline\":[{\"text\":\"Broken link: nothing is published at \"},{\"code\":\"/post-9/\"}],\
        \"file\":\"content/post-2.md\",\"line\":3,\"column\":14,\"excerpt\":[\
        {\"line\":2,\"text\":[]},\
        {\"line\":3,\"text\":[{\"text\":\"See [Post 9](\"},{\"mark\":\"/post-9/\"},{\"text\":\")  then.\"}]},\
        {\"line\":4,\"text\":[]}],\
        \"related\":[],\"built\":[],\"builtMore\":0,\"printed\":null,\
        \"suggestion\":[{\"text\":\"Did you mean \"},{\"code\":\"/post-1/\"},{\"text\":\"?\"}],\
        \"suggests\":null,\"explanation\":\"https://statac.dev/e/broken-link/\",\"quietened\":true}";
    let at = between(&built, "\"round\":{\"state\":\"built\",\"at\":\"", "\"").to_owned();
    assert!(
        built.contains(&format!(
            "\"problems\":[{link}],\"stopped\":null,\"explanations\":{{\
             \"broken-link\":{{\"title\":\"Broken link\",\"opening\":[{{\"said\":[\
             {{\"text\":\"A link, or a page\\u0027s \"}},{{\"code\":\"redirect_to\"}},\
             {{\"text\":\", points at an address on this site, but nothing is published at \
             that address. A reader who follows it lands on a missing page.\"}}]}}],\
             \"fix\":[{{\"said\":[{{\"text\":\"Change the link to an address the site publishes."
        )) && built.contains(&format!(
            "\"round\":{{\"state\":\"built\",\"at\":\"{at}\",\"builtAt\":\"{at}\",\"changed\":[],\
             \"built\":3,\"outputs\":3,\"took\":{{\"before\":"
        )) && built.contains(&format!(
            "\"counts\":{{\"error\":0,\"warning\":0,\"note\":0}}}},\"rounds\":[\
             {{\"state\":\"built\",\"at\":\"{at}\",\"said\":[{{\"text\":\"Wrote 3 pages and 2 \
             other files to \"}},{{\"code\":\""
        )) && built.contains(
            "\"site\":{\"kinds\":[{\"kind\":\"broken-link\",\"severity\":\"warning\",\
             \"title\":\"Broken link\",\"count\":2,\"quietened\":2,\"suggestion\":[\
             {\"text\":\"Did you mean \"},{\"code\":\"/post-1/\"},{\"text\":\"?\"}]}],\"gone\":null,\
             \"drafts\":[],\"waiting\":[],\"written\":[]},\"root\":\""
        ),
        "{built}"
    );
    let root = between(&built, ",\"root\":\"", "\"").replace("\\\\", "\\");
    assert!(std::path::Path::new(&root).is_absolute(), "{root}");
    assert_eq!(
        std::fs::canonicalize(&root).unwrap(),
        std::fs::canonicalize(&served.site.0).unwrap()
    );

    // The whole site's, in order, one met building a page.
    let template = "{\"severity\":\"warning\",\"kind\":\"broken-link\",\
        \"headline\":[{\"text\":\"Broken link: nothing is published at \"},{\"code\":\"/post-8/\"}],\
        \"file\":\"templates/page.html\",\"line\":9,\"column\":10,\"excerpt\":[\
        {\"line\":8,\"text\":[{\"text\":\"{{ page.content }}\"}]},\
        {\"line\":9,\"text\":[{\"text\":\"\\u003ca href=\\\"\"},{\"mark\":\"/post-8/\"},\
        {\"text\":\"\\\"\\u003ePost 8\\u003c/a\\u003e\"}]},\
        {\"line\":10,\"text\":[{\"text\":\"\\u003c/body\\u003e\"}]}],\"related\":[],\
        \"built\":[{\"page\":[{\"code\":\"content/index.md\"}],\"href\":\"/\",\
        \"file\":\"content/index.md\",\"line\":null}],\"builtMore\":0,";
    for asked in ["", "?kind=broken-link"] {
        let every = served.page(&format!("/.statac-live/problems{asked}"));
        assert!(
            every.starts_with(&format!("{{\"v\":2,\"problems\":[{link},{template}"))
                && every
                    .contains("\"quietened\":true}],\"more\":0,\"explanations\":{\"broken-link\":"),
            "{asked}: {every}"
        );
    }
    assert_eq!(
        served.page("/.statac-live/problems?kind=unknown-word"),
        "{\"v\":2,\"problems\":[],\"more\":0,\"explanations\":{}}"
    );

    // A round that stops has every error in its parts, and the page's
    // problems are those of the round before.
    let mark = served.said.text().len();
    let broken = std::fs::read_to_string(served.site.path("templates/page.html")).unwrap();
    served.site.write(
        "templates/page.html",
        &broken.replace("{{ page.title }}", "{{ page.sumary }}"),
    );
    served.said.after(mark, "unknown-word/\n");
    let stopped = panel(&served, "/post-2/");
    let then = between(&stopped, "\"stopped\":{\"at\":\"", "\"").to_owned();
    assert!(
        stopped.contains(&format!(
            "\"problems\":[{link}],\"stopped\":{{\"at\":\"{then}\",\"errors\":[\
             {{\"severity\":\"error\",\"kind\":\"unknown-word\",\
             \"headline\":[{{\"text\":\"Unknown field: \"}},{{\"code\":\"page.sumary\"}}],\
             \"file\":\"templates/page.html\",\"line\":4,\"column\":11,\"excerpt\":[\
             {{\"line\":3,\"text\":[{{\"text\":\"\\u003chead\\u003e\"}}]}},\
             {{\"line\":4,\"text\":[{{\"text\":\"\\u003ctitle\\u003e{{{{ \"}},{{\"mark\":\"page.sumary\"}},\
             {{\"text\":\" }}}}\\u003c/title\\u003e\"}}]}},\
             {{\"line\":5,\"text\":[{{\"text\":\"{{{{ css(\\\"site.css\\\") }}}}\"}}]}}],\
             \"related\":[],\"built\":[],\"builtMore\":0,\"printed\":null,\
             \"suggestion\":[{{\"text\":\"Did you mean \"}},{{\"code\":\"page.summary\"}},\
             {{\"text\":\"?\"}}],\"suggests\":\"page.summary\",\
             \"explanation\":\"https://statac.dev/e/unknown-word/\",\"quietened\":false}}]}},\
             \"explanations\":{{\"broken-link\":{{\"title\":\"Broken link\","
        )) && stopped.contains("\"unknown-word\":{\"title\":\"Unknown word\",\"opening\":[")
            && stopped.contains(&format!(
                "\"round\":{{\"state\":\"stopped\",\"at\":\"{then}\",\"builtAt\":\"{at}\",\
                 \"changed\":[\"templates/page.html\"],\"built\":3,\"outputs\":3,"
            ))
            && stopped.contains(&format!(
                "\"counts\":{{\"error\":1,\"warning\":0,\"note\":0}}}},\"rounds\":[\
                 {{\"state\":\"stopped\",\"at\":\"{then}\",\
                 \"said\":[{{\"text\":\"1 error stopped the build.\"}}],\
                 \"counts\":{{\"error\":1,\"warning\":0,\"note\":0}},\"first\":false}},\
                 {{\"state\":\"built\",\"at\":\"{at}\",\"said\":[{{\"text\":\"Wrote 3 pages"
            ))
            && stopped.contains(
                "\"counts\":{\"error\":0,\"warning\":0,\"note\":0},\"first\":true}],\"site\":"
            )
            && stopped.matches("\"first\":true").count() == 1
            && stopped.contains("\"site\":{\"kinds\":[{\"kind\":\"broken-link\","),
        "{stopped}"
    );

    // Mended, what stopped it is gone, and the rounds are three.
    let mark = served.said.text().len();
    served.site.write("templates/page.html", &broken);
    served
        .said
        .after(mark, "Gone since the last build: 1 unknown-word.\n");
    let mended = panel(&served, "/post-2/");
    let now = between(&mended, "\"round\":{\"state\":\"built\",\"at\":\"", "\"").to_owned();
    assert!(
        mended.contains(
            "\"stopped\":null,\"explanations\":{\"broken-link\":{\"title\":\"Broken link\","
        ) && mended.contains(&format!("\"builtAt\":\"{now}\""))
            && mended.contains(&format!(
                "\"rounds\":[{{\"state\":\"built\",\"at\":\"{now}\",\
                 \"said\":[{{\"text\":\"Built "
            ))
            && mended.contains(&format!(
                "{{\"state\":\"stopped\",\"at\":\"{then}\",\
                 \"said\":[{{\"text\":\"1 error stopped the build.\"}}],"
            ))
            && mended.matches("\"first\":false").count() == 2
            && mended.contains("\"first\":true}],\"site\":")
            && mended
                .contains("\"gone\":[{\"text\":\"Gone since the last build: 1 unknown-word.\"}],"),
        "{mended}"
    );
}

#[test]
fn the_panel_is_handed_a_kinds_suggestion_where_each_of_its_problems_has_it() {
    let served = served_as("panel-kinds", How::Native, true, |site| {
        for n in 2..=3 {
            site.write(
                &format!("content/post-{n}.md"),
                &format!("---\ntitle: Post {n}\nneeds: url\n---\nPost {n}.\n"),
            );
        }
        site.write("content/about.md", "# About\n\nAbout the site.\n");
        site.write(
            "content/links.md",
            "# Links\n\nSee [one](/post-9/) and [another](/abut/).\n",
        );
    });
    let site = panel(&served, "/post-1/");
    assert!(
        site.contains(
            "{\"kind\":\"broken-link\",\"severity\":\"warning\",\"title\":\"Broken link\",\
             \"count\":2,\"quietened\":0,\"suggestion\":null}"
        ) && site.contains(
            "{\"kind\":\"no-site-address\",\"severity\":\"note\",\"title\":\"No site address\",\
             \"count\":2,\"quietened\":0,\"suggestion\":[{\"text\":\"Add the site\\u0027s address to \"},\
             {\"code\":\"statac.yaml\"},{\"text\":\", as \"},\
             {\"code\":\"url: https://example.org/\"},{\"text\":\".\"}]}"
        ),
        "{site}"
    );
    let suggestions = served.page("/.statac-live/problems?kind=broken-link");
    assert!(
        suggestions.contains("{\"code\":\"/post-1/\"}")
            && suggestions.contains("{\"code\":\"/about/\"}"),
        "{suggestions}"
    );
}

#[test]
fn the_panel_is_handed_the_pages_facts_and_what_its_build_read_of_each_name() {
    let served = served_as("panel-page", How::Native, true, |site| {
        site.write(
            "templates/base.html",
            "<title>{{ page.title }} - {{ site.title }}</title>\n{% block main %}{% endblock %}\n",
        );
        site.write(
            "templates/post.html",
            "{% extends \"base\" %}{% block main %}{{ page.content }}\
             {% if page.next %}{{ page.next.title }}{% endif %}{% endblock %}",
        );
        site.write(
            "content/posts/index.md",
            "---\ntitle: Posts\nlayout: page\npages.layout: post\n---\n",
        );
        for n in 2..=4 {
            site.write(
                &format!("content/posts/post-{n}.md"),
                &format!("---\ntitle: Post {n}\ndate: 2026-10-0{n}\n---\nPost {n}.\n"),
            );
        }
    });
    // A post built with a layout its folder hands down, which extends
    // another, between two others in each of its lists.
    let post = panel(&served, "/posts/post-3/");
    assert!(
        post.contains(
            "\"page\":{\"title\":\"Post 3\",\"draft\":false,\"waiting\":null,\
             \"file\":\"content/posts/post-3.md\",\"address\":\"/posts/post-3/\",\
             \"language\":{\"tag\":\"en\",\"direction\":\"ltr\"},\
             \"layout\":{\"name\":\"post\",\"file\":\"templates/post.html\",\
             \"extends\":[\"templates/base.html\"],\
             \"chosenAt\":{\"file\":\"content/posts/index.md\",\"line\":4},\"handedDown\":true},\
             \"lists\":[{\"said\":[{\"text\":\"The site\\u0027s own list\"}],\"at\":2,\"of\":3,\
             \"previous\":{\"text\":\"Post 4\",\"href\":\"/posts/post-4/\"},\
             \"next\":{\"text\":\"Post 2\",\"href\":\"/posts/post-2/\"}},\
             {\"said\":[{\"text\":\"The folder \"},{\"code\":\"content/posts/\"}],\"at\":2,\"of\":3,\
             \"previous\":{\"text\":\"Post 4\",\"href\":\"/posts/post-4/\"},\
             \"next\":{\"text\":\"Post 2\",\"href\":\"/posts/post-2/\"}}],\"feeds\":[],\
             \"reads\":{\"files\":[\"content/posts/post-2.md\",\"content/posts/post-3.md\",\
             \"statac.yaml\",\"templates/base.html\",\"templates/post.html\"],\
             \"also\":[[{\"text\":\"The order of the pages in their lists\"}]]}},\
             \"names\":[{\"group\":\"page\",\"count\":"
        ),
        "{post}"
    );
    // A call is written with its brackets, empty where it takes nothing.
    assert!(
        post.contains(
            "{\"name\":\"page.audio\",\"written\":\"page.audio(\u{2026})\",\"holds\":\"call\",\
             \"value\":\"called, not shown\","
        ) && post.contains(
            "{\"name\":\"page.search_box\",\"written\":\"page.search_box()\",\"holds\":\"call\","
        ),
        "{post}"
    );
    // What the layout writes, and what it doesn't, with the line each
    // value is written on.
    let title = "{\"name\":\"page.title\",\"holds\":\"text\",\"value\":\"Post 3\",\
        \"empty\":false,\"has\":[],\"from\":{\"file\":\"content/posts/post-3.md\",\"line\":2,\
        \"excerpt\":[{\"line\":2,\"text\":[{\"text\":\"title: \"},{\"mark\":\"Post 3\"}]}]},\
        \"used\":true}";
    assert!(
        post.contains(title)
            && post.contains(
                "{\"name\":\"page.date\",\"holds\":\"date\",\"value\":\"3 October 2026\",\
                 \"empty\":false,\"has\":[\"day\",\"month\",\"weekday\",\"year\"],\
                 \"from\":{\"file\":\"content/posts/post-3.md\",\"line\":3,\"excerpt\":[\
                 {\"line\":3,\"text\":[{\"text\":\"date: \"},{\"mark\":\"2026-10-03\"}]}]},\
                 \"used\":false}"
            )
            && post.contains(
                "{\"name\":\"page.summary\",\"holds\":\"text\",\"value\":\"no value\",\
                 \"empty\":true,\"has\":[],\"from\":null,\"used\":false}"
            ),
        "{post}"
    );
    // Answered from the same writing, but asked for only as itself.
    let used = |name: &str, value: &str, used: bool| {
        format!(
            "{{\"name\":\"page.{name}\",\"holds\":\"HTML\",\"value\":\"{value}\",\
             \"empty\":false,\"has\":[],\"from\":null,\"used\":{used}}}"
        )
    };
    let writing = "\\u003cp\\u003ePost 3.\\u003c/p\\u003e";
    assert!(
        post.contains(&used("content", writing, true))
            && post.contains(&used("excerpt", writing, false)),
        "{post}"
    );

    // Built again for another page's sake alone, the post keeps what its
    // build asked for.
    let mark = served.said.text().len();
    served.site.write(
        "content/posts/post-4.md",
        "---\ntitle: Post 4\ndate: 2026-10-04\n---\nPost 4, again.\n",
    );
    let said = served.said.after(mark, "Browser told to reload.\n");
    assert!(said.contains(" changed. Built 1 of "), "{said}");
    let kept = panel(&served, "/posts/post-3/");
    assert!(
        kept.contains(&used("content", writing, true))
            && kept.contains(&used("excerpt", writing, false))
            && kept.contains(title),
        "{kept}"
    );

    // The page speaking for the folder names its own layout and is in no
    // list; it lists its posts, which its layout never asks for.
    let list = panel(&served, "/posts/");
    assert!(
        list.contains(
            "\"page\":{\"title\":\"Posts\",\"draft\":false,\"waiting\":null,\
             \"file\":\"content/posts/index.md\",\"address\":\"/posts/\",\
             \"language\":{\"tag\":\"en\",\"direction\":\"ltr\"},\
             \"layout\":{\"name\":\"page\",\"file\":\"templates/page.html\",\"extends\":[],\
             \"chosenAt\":{\"file\":\"content/posts/index.md\",\"line\":3},\"handedDown\":false},\
             \"lists\":[],\"feeds\":[],\
             \"reads\":{\"files\":[\"assets/site.css\",\"content/posts/index.md\",\
             \"statac.yaml\",\"templates/page.html\"],\"also\":[]}}"
        ) && list.contains(
            "{\"name\":\"page.title\",\"holds\":\"text\",\"value\":\"Posts\",\"empty\":false,\
             \"has\":[],\"from\":{\"file\":\"content/posts/index.md\",\"line\":2,\"excerpt\":[\
             {\"line\":2,\"text\":[{\"text\":\"title: \"},{\"mark\":\"Posts\"}]}]},\"used\":true}"
        ) && list.contains(
            "{\"name\":\"page.posts\",\"holds\":\"pages\",\"value\":\"3 pages\",\
             \"empty\":false,\"has\":[],\"from\":null,\"used\":false}"
        ),
        "{list}"
    );

    // An address with no page at it has none to tell of.
    let nowhere = panel(&served, "/site.css");
    assert!(
        nowhere.ends_with("\"page\":null,\"names\":[]}"),
        "{nowhere}"
    );
}

#[test]
fn asked_for_no_drafts_serve_shows_what_a_build_publishes() {
    let served = served_as("no-drafts", How::Native, false, |site| {
        site.write("templates/folder.html", LAYOUT);
        site.write(
            "content/post-2.md",
            "---\ntitle: Post 2\ndraft: true\n---\nA draft.\n",
        );
        site.write(
            "content/post-3.md",
            "---\ntitle: Post 3\ndate: 2999-01-01\n---\nLater.\n",
        );
        site.write(
            "content/later/index.md",
            "---\ntitle: Later\npages.draft: true\n---\n",
        );
        site.write("content/later/post-4.md", "# Post 4\n");
        site.write(
            "content/items.md",
            "---\ntitle: Items\neach: data.items\npermalink: /items/{key}/\n---\n",
        );
        site.write(
            "data/items.yaml",
            "item-1:\n  title: Item 1\nitem-2:\n  draft: true\n",
        );
    });
    let said = served.said.text();
    assert!(!said.contains("Showing"), "{said}");
    assert!(
        said.contains("4 notes: 3 draft-left-out, 1 waiting-for-its-day."),
        "{said}"
    );
    let (status, _) = get(served.port, "/post-2/");
    assert!(status.contains("404"), "{status}");
    // Each page left out is listed by its file, opened at the start
    // of a page's own file and at the item of one a file holds many of.
    let answer = panel(&served, "/post-1/");
    assert!(
        answer.contains(
            "\"drafts\":[\
             {\"text\":\"content/later/post-4.md\",\"file\":\"content/later/post-4.md\",\
             \"line\":null},\
             {\"text\":\"content/post-2.md\",\"file\":\"content/post-2.md\",\"line\":null},\
             {\"text\":\"data/items.yaml: item-2\",\"file\":\"data/items.yaml\",\"line\":4}],\
             \"waiting\":[{\"text\":\"content/post-3.md\",\"file\":\"content/post-3.md\",\
             \"line\":null,\"day\":\"2999-01-01\"}],"
        ),
        "{answer}"
    );
}

/// What the panel's search finds for `words`, as a query carries them,
/// asked from the page at `address`.
fn search(served: &Served, words: &str, address: &str) -> String {
    let asked = address.replace('/', "%2F");
    served.page(&format!(
        "/.statac-live/search?words={words}&typing=0&page={asked}"
    ))
}

#[test]
fn the_panel_searches_the_site_as_serve_shows_it_saying_why_each_page_ranks() {
    let served = served_as("search", How::Native, true, |site| {
        site.write(
            "content/post-2.md",
            "---\ntitle: Post 2\ndraft: true\n---\nA draft.\n",
        );
        site.write(
            "content/post-3.md",
            "---\ntitle: Post 3\ndate: 2999-01-01\n---\nA post.\n",
        );
    });
    // A draft is found, and each word says where it stands and counts.
    let draft = search(&served, "draft", "/post-1/");
    assert!(
        draft.starts_with(
            "{\"drafts\":true,\"languages\":[{\"code\":\"en\",\"pages\":1,\"some\":false,\
             \"suggestion\":null,\"parts\":6,\"average\":10,\"results\":[\
             {\"title\":\"Post 2\",\"day\":null,\"heading\":null,\"href\":\"/post-2/\",\
             \"file\":\"content/post-2.md\",\"draft\":true,\"waiting\":null,\
             \"excerpt\":[{\"text\":\"A \",\"marked\":false},{\"text\":\"draft\",\"marked\":true},\
             {\"text\":\".\",\"marked\":false}],\"matched\":1,\"score\":"
        ) && draft.contains(
            ",\"length\":12,\"words\":[{\"word\":\"draft\",\"forms\":[\"draft\"],\
             \"starting\":false,\"places\":[{\"place\":\"text\",\"times\":1,\"counts\":1}],\
             \"holding\":1,\"score\":"
        ),
        "{draft}"
    );
    // A post waiting for its day is found, with the day.
    let post = search(&served, "post", "/post-1/");
    assert!(
        post.contains(
            "{\"title\":\"Post 3\",\"day\":\"1 January 2999\",\"heading\":null,\
             \"href\":\"/post-3/\",\"file\":\"content/post-3.md\",\"draft\":false,\
             \"waiting\":\"2999-01-01\","
        ) && post.contains(
            "\"places\":[{\"place\":\"title\",\"times\":1,\"counts\":5},\
             {\"place\":\"text\",\"times\":1,\"counts\":1}],\"holding\":4,"
        ),
        "{post}"
    );
    // A space as a form writes it; a word no page holds adds nothing.
    let some = search(&served, "draft+nowhere", "/post-1/");
    assert!(
        some.contains("\"pages\":1,\"some\":true,\"suggestion\":null,")
            && some.contains(
                "{\"word\":\"nowhere\",\"forms\":[],\"starting\":false,\"places\":[],\
                 \"holding\":0,\"score\":0}]}"
            ),
        "{some}"
    );
    let meant = search(&served, "drafx", "/post-1/");
    assert!(
        meant.contains("\"pages\":0,\"some\":false,\"suggestion\":\"draft\",")
            && meant.ends_with("\"results\":[]}]}"),
        "{meant}"
    );
    let blank = search(&served, "%20%20", "/post-1/");
    assert_eq!(
        blank,
        "{\"drafts\":true,\"languages\":[{\"code\":\"en\",\"pages\":0,\"some\":false,\
         \"suggestion\":null,\"parts\":0,\"average\":0,\"results\":[]}]}"
    );
}

#[test]
fn the_panel_searches_the_language_of_the_page_it_is_on_first() {
    let served = served_as("search-languages", How::Native, true, |site| {
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: [en, fr]\ntimezone: Europe/London\n",
        );
        site.write(
            "content/fr/index.md",
            "---\nlayout: page\n---\n# Accueil\n\nBienvenue.\n",
        );
        site.write("content/fr/post-1.md", "# Post 1\n\nUn post.\n");
    });
    let codes = |answer: &str| -> Vec<String> {
        let codes = answer.split("{\"code\":\"").skip(1);
        codes.map(|rest| rest[..2].to_owned()).collect()
    };
    let english = search(&served, "post", "/post-1/");
    assert_eq!(codes(&english), ["en", "fr"], "{english}");
    let french = search(&served, "post", "/fr/post-1/");
    assert_eq!(codes(&french), ["fr", "en"], "{french}");
    assert!(
        french.contains("\"heading\":\"Post 1\",\"href\":\"/fr/post-1/#post-1\",")
            && french.contains("\"heading\":null,\"href\":\"/post-1/\","),
        "{french}"
    );
}

/// A file's name is the site's own text: it reaches neither the terminal
/// nor a page as written.
#[cfg(unix)]
#[test]
fn a_changed_files_name_is_shown_harmless_in_the_terminal_and_to_the_page() {
    let served = served("name-escapes", How::Native);
    let before = served.hub.generation();
    let mark = served.said.text().len();
    served.site.write(
        "content/post\u{1b}[2J<b>.md",
        "# Post 2\n\nThe second post.\n",
    );
    let said = served.said.after(mark, " changed. ");
    assert!(
        said.starts_with("`content/post\u{241b}[2J<b>.md` changed. "),
        "{said:?}"
    );
    assert!(!said.contains('\u{1b}'), "{said:?}");
    let (_, body) = get(
        served.port,
        &format!("/.statac-live/wait?since={before}&timeout=0"),
    );
    assert!(
        body.contains("\"changed\":[\"content/post\u{241b}[2J\\u003cb\\u003e.md\"]"),
        "{body}"
    );
    assert!(!body.contains('\u{1b}') && !body.contains('<'), "{body}");
    let panel = panel(&served, "/");
    assert!(!panel.contains('\u{1b}') && !panel.contains('<'), "{panel}");
}

#[cfg(unix)]
#[test]
fn a_command_writing_into_a_folder_it_watches_starts_no_round_of_its_own() {
    each_way_as(
        "hook-loop",
        |site| {
            site.write(
                "statac.yaml",
                "title: Site\nlanguage: en\ntimezone: Europe/London\n\
             before_build: echo ran >> assets/ran.txt && cp styles/site.css assets/site.css\n\
             before_build_watches: [styles/, assets/]\n",
            );
            site.write("styles/site.css", "body { color: black; }\n");
        },
        |way, served| {
            let said = served.said.text();
            assert!(
                said.contains(
                    "Ran `before_build`: it runs again when a file `before_build_watches` \
                 names changes.\n"
                ),
                "{way}:\n{said}"
            );
            let ran = || {
                std::fs::read_to_string(served.site.path("assets/ran.txt"))
                    .unwrap()
                    .lines()
                    .count()
            };
            let (before, runs) = (served.hub.generation(), ran());
            let mark = served.said.text().len();
            served
                .site
                .write("styles/site.css", "body { color: rebeccapurple; }\n");
            let said = served.said.after(mark, "Ran `before_build`.\n");
            assert!(
                said.starts_with("`styles/site.css` changed. Built "),
                "{way}: {said}"
            );
            // What the command wrote is built in the same round, and brings
            // no round after it.
            std::thread::sleep(Duration::from_millis(1500));
            assert_eq!(served.hub.generation(), before + 1, "{way}");
            assert_eq!(ran(), runs + 1, "{way}");
            let said = served.said.text()[mark..].to_owned();
            assert_eq!(said.matches(" changed. ").count(), 1, "{way}: {said}");
            assert!(served.page(&served.stylesheet()).contains("rebeccapurple"));

            // A save of the author's in the same folder still runs it.
            let mark = served.said.text().len();
            served.site.write("assets/other.css", "p { margin: 0; }\n");
            served.said.after(mark, "Ran `before_build`.\n");
            std::thread::sleep(Duration::from_millis(1500));
            assert_eq!(served.hub.generation(), before + 2, "{way}");
            assert_eq!(ran(), runs + 2, "{way}");
        },
    );
}

#[cfg(unix)]
#[test]
fn the_command_after_a_build_runs_once_and_serve_says_so() {
    let served = served_as("after", How::Native, true, |site| {
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: en\ntimezone: Europe/London\n\
             after_build: test \"$STATAC_SERVE\" = yes && echo made >> \"$STATAC_OUT/made.txt\"\n\
             after_build_makes: [made.txt]\n",
        );
    });
    let said = served.said.text();
    assert!(
        said.contains("Ran `after_build`, which added 1 file.\n")
            && said.contains(
                "`after_build` ran once: what it made stays as it is until serve is started again.\n"
            ),
        "{said}"
    );
    let mark = served.said.text().len();
    served
        .site
        .write("content/post-1.md", "# Post 1\n\nEdited.\n");
    let said = served.said.after(mark, "Browser told to reload.\n");
    assert!(!said.contains("after_build"), "{said}");
    assert_eq!(served.page("/made.txt"), "made\n");
}

#[test]
fn an_id_is_written_into_a_new_post_once_the_site_has_been_published() {
    const SETTINGS: &str = "title: Site\nlanguage: en\ntimezone: Europe/London\n\
                            url: https://example.org\nfeed_key: key\nwrite_feed_ids: true\n";
    let served = served_as("ids", How::Native, true, |site| {
        site.write("statac.yaml", SETTINGS);
        site.write(
            "templates/feed.xml",
            "{% for post in posts %}{{ feed_id(post) }} {% endfor %}\n",
        );
        site.write(
            "content/post-1.md",
            "---\ndate: 2026-01-01\n---\n# Post 1\n\nThe first post.\n",
        );
    });
    let unwritten = "Wrote no feed ids: they start once `statac build` has published the site.\n";
    let said = served.said.text();
    assert!(said.contains(unwritten), "{said}");
    let first = std::fs::read_to_string(served.site.path("content/post-1.md")).unwrap();
    assert!(!first.contains("id:"), "{first}");

    let built = statac::build::run(&statac::build::Options {
        mode: statac::build::Mode::Build,
        site: served.site.0.clone(),
        out: None,
        today: None,
        drafts: false,
        url: None,
    });
    assert!(built.published.is_some());
    let before = served.hub.generation();
    let mark = served.said.text().len();
    served.site.write(
        "content/post-2.md",
        "---\ndate: 2026-01-02\n---\n# Post 2\n\nThe second post.\n",
    );
    let said = served
        .said
        .after(mark, "Wrote an `id` into `content/post-2.md`.\n");
    assert!(
        said.starts_with("`content/post-2.md` changed. Built "),
        "{said}"
    );
    // Serve's own write to the file is no save to build for.
    std::thread::sleep(Duration::from_millis(1500));
    assert_eq!(served.hub.generation(), before + 1);
    let second = std::fs::read_to_string(served.site.path("content/post-2.md")).unwrap();
    assert!(
        second.starts_with("---\ndate: 2026-01-02\nid: ")
            && second.ends_with("\n---\n# Post 2\n\nThe second post.\n"),
        "{second}"
    );
    let id = between(&second, "id: ", "\n");
    assert!(served.page("/feed.xml").contains(&format!("urn:uuid:{id}")));
    assert!(panel(&served, "/post-2/").contains(
        "\"written\":[[{\"text\":\"Wrote an \"},{\"code\":\"id\"},\
         {\"text\":\" into \"},{\"code\":\"content/post-2.md\"},{\"text\":\".\"}]]"
    ));
    assert_eq!(
        std::fs::read_to_string(served.site.path("content/post-1.md")).unwrap(),
        first
    );
    let said = served.said.text();
    assert_eq!(said.matches(unwritten).count(), 1, "{said}");
}

#[test]
fn a_round_waits_for_a_build_holding_the_site_and_says_so_once() {
    let served = served("waits", How::Native);
    let (hold, unheld) = statac::publish::hold(&served.site.0, || {});
    assert!(unheld.is_none());
    let mark = served.said.text().len();
    served
        .site
        .write("content/post-1.md", "# Post 1\n\nSaved again.\n");
    let waiting = "Waiting for another build of the site to finish.\n";
    let said = served.said.after(mark, waiting);
    assert_eq!(said, waiting);
    drop(hold);
    let said = served
        .said
        .after(mark, "`content/post-1.md` changed. Built ");
    assert_eq!(said.matches(waiting).count(), 1, "{said}");
    assert!(served.page("/post-1/").contains("Saved again."));
}

const YOUTUBE: &str = "{# takes address #}\n{% set video = page.youtube(address) %}\n\
                       {% if video.still %}<img src=\"{{ video.still.src }}\">{% endif %}\n";

const MAXRES: &str = "https://i.ytimg.com/vi/abcdefghijk/maxresdefault.jpg";

const RELAY: &str = "https://webmention.io/api/mentions.jf2";

const FACE: &str = "https://a.example/a.jpg";

/// A reply from `a.example` to `post-1`, with the face [`FACE`], as the
/// relay sends it.
const SENT: &str = "{\"type\":\"feed\",\"children\":[{\"type\":\"entry\",\
    \"author\":{\"type\":\"card\",\"name\":\"A\",\"url\":\"https://a.example/\",\
    \"photo\":\"https://a.example/a.jpg\"},\"url\":\"https://a.example/post-1/\",\
    \"published\":\"2026-09-03T21:48:39\",\"wm-received\":\"2026-09-04T04:54:54Z\",\
    \"wm-id\":1,\"wm-source\":\"https://a.example/post-1/\",\
    \"wm-target\":\"https://example.org/post-1/\",\"wm-property\":\"in-reply-to\",\
    \"content\":{\"text\":\"Words.\",\"html\":\"<p>Words.</p>\"}}]}";

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

/// A fetcher standing in for the network, answering with `answers`, and
/// every address it was asked for, in order.
fn standing_in(
    answers: impl Fn(&str) -> Result<Fetched, Refusal> + Send + Sync + 'static,
) -> (Fetcher, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let kept = asked.clone();
    let fetcher = Fetcher::new(move |address: &str, _limit: u64| {
        kept.lock().unwrap().push(address.to_owned());
        answers(address)
    });
    (fetcher, asked)
}

/// A site with a post showing the YouTube video `abcdefghijk`.
fn with_a_video(site: &Site) {
    site.write("snippets/youtube.html", YOUTUBE);
    site.write(
        "content/post-2.md",
        "# Post 2\n\n@youtube(\"https://youtu.be/abcdefghijk\")\n",
    );
}

/// A site with webmentions on, `post-1` showing its own with their faces.
fn with_webmentions(site: &Site) {
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: Europe/London\n\
         url: https://example.org/\nwebmentions: true\n",
    );
    site.write(
        "templates/mentioned.html",
        "<!doctype html>\n<title>{{ page.title }}</title>\n\
         {% for m in page.webmentions.said %}{{ m.words }}\
         {% if m.face %}<img src=\"{{ m.face.src }}\" alt=\"\">{% endif %}{% endfor %}\n\
         {{ page.content }}\n",
    );
    site.write(
        "content/post-1.md",
        "---\nlayout: mentioned\n---\n# Post 1\n\nThe first post.\n",
    );
}

/// `text` with the site's folder, the port and how long each round took
/// written the same every time.
fn steady(served: &Served, text: &str) -> String {
    // The folder served is shown as the system writes a path.
    let text = text
        .replace(&served.site.0.display().to_string(), "SITE")
        .replace("SITE\\.statac\\serve", "SITE/.statac/serve")
        .replace(&format!(":{}/", served.port), ":PORT/");
    let mut out = String::new();
    for line in text.split_inclusive('\n') {
        match (line.find(" in "), line.find(" seconds.")) {
            (Some(from), Some(to)) if from < to => {
                out.push_str(&line[..from]);
                out.push_str(" in TIME");
                out.push_str(&line[to..]);
            }
            _ => out.push_str(line),
        }
    }
    out
}

/// [`steady`], for what is said while the system may still be reporting
/// the site's own making: a round that built nothing new is left out, and
/// a round's files are not named, since those reports may join them.
fn steady_from_the_start(served: &Served, text: &str) -> String {
    let mut out = String::new();
    for line in steady(served, text).split_inclusive('\n') {
        if line.contains("No built file changed.") {
            continue;
        }
        match line.find(" changed. Built ") {
            Some(at) => out.push_str(&format!("FILES{}", &line[at..])),
            None => out.push_str(line),
        }
    }
    out
}

/// Every file under `folder`, by its path with forward slashes, with its
/// bytes.
fn files(folder: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(next) = folders.pop() {
        for entry in std::fs::read_dir(&next).into_iter().flatten() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let relative = path.strip_prefix(folder).unwrap();
                let relative = relative.to_string_lossy().replace('\\', "/");
                found.push((relative, std::fs::read(&path).unwrap()));
            }
        }
    }
    found.sort();
    found
}

/// Holds what the session's folder holds to a fresh build of the site as
/// it stands.
fn same_as_a_build(served: &Served) {
    let out = served.site.0.with_extension("fresh");
    let built = statac::build::run(&statac::build::Options {
        mode: statac::build::Mode::Build,
        site: served.site.0.clone(),
        out: Some(out.clone()),
        today: None,
        drafts: true,
        url: None,
    });
    assert!(built.published.is_some());
    let (held, fresh) = (files(&served.site.path(".statac/serve")), files(&out));
    let _ = std::fs::remove_dir_all(&out);
    let names = |files: &[(String, Vec<u8>)]| -> Vec<String> {
        files.iter().map(|(name, _)| name.clone()).collect()
    };
    assert_eq!(names(&held), names(&fresh));
    for ((name, held), (_, fresh)) in held.iter().zip(&fresh) {
        assert!(held == fresh, "{name}");
    }
}

/// Waits until no round has been made for a while, and gives how many
/// rounds were made since `generation`.
fn rounds_since(served: &Served, generation: u64) -> u64 {
    let mut last = served.hub.generation();
    let mut quiet_since = Instant::now();
    while quiet_since.elapsed() < Duration::from_millis(1500) {
        std::thread::sleep(Duration::from_millis(50));
        if served.hub.generation() != last {
            last = served.hub.generation();
            quiet_since = Instant::now();
        }
    }
    last - generation
}

#[test]
fn a_still_is_fetched_after_the_first_round_and_built_as_a_build_would() {
    let still = jpeg(64, 36);
    let (fetcher, asked) = standing_in(move |address| answer(address, &still));
    let (served, opened) = served_fetching(
        "still",
        How::Native,
        true,
        Some(fetcher),
        None,
        with_a_video,
    );
    assert_eq!(
        steady(&served, &opened),
        "\
1 warning: 1 no-still. Run `statac check` to see it.

Wrote 3 pages and 2 other files to `SITE/.statac/serve` in TIME seconds.
Serving `SITE/.statac/serve` at http://localhost:PORT/
Fetching what's new from other sites. `--offline` turns this off.
Press Ctrl+C to stop.
"
    );
    let said = served.said.after(opened.len(), "Browser told to reload.\n");
    assert_eq!(
        steady_from_the_start(&served, &said),
        "\
Fetched `fetched/stills/youtube-abcdefghijk.jpg`.
Fetched 1 still.
FILES changed. Built 1 of 3 files in TIME seconds. Browser told to reload.
Gone since the last build: 1 no-still.
"
    );
    assert_eq!(*asked.lock().unwrap(), [MAXRES]);
    assert!(served.page("/post-2/").contains("<img src="));
    same_as_a_build(&served);

    // A video added later is never fetched for.
    let mark = served.said.text().len();
    served.site.write(
        "content/post-3.md",
        "# Post 3\n\n@youtube(\"https://youtu.be/bcdefghijkl\")\n",
    );
    let said = served.said.after(mark, "Browser told to reload.\n");
    assert!(
        said.contains("No still: no picture has been fetched"),
        "{said}"
    );
    let generation = served.hub.generation();
    assert_eq!(rounds_since(&served, generation), 0);
    assert_eq!(*asked.lock().unwrap(), [MAXRES]);
    assert!(!served.said.text()[mark..].contains("Fetch"));
}

#[test]
fn a_webmention_and_its_face_are_fetched_in_one_round_and_built_as_a_build_would() {
    let photo = jpeg(200, 100);
    let (fetcher, asked) = standing_in(move |address| match address {
        FACE => answer(address, &photo),
        _ => answer(address, SENT.as_bytes()),
    });
    let token = Token::new("t0k3n".into());
    let (served, opened) = served_fetching(
        "mention",
        How::Native,
        true,
        Some(fetcher),
        token,
        with_webmentions,
    );
    assert_eq!(
        steady(&served, &opened),
        "\
Wrote 2 pages and 2 other files to `SITE/.statac/serve` in TIME seconds.
Serving `SITE/.statac/serve` at http://localhost:PORT/
Fetching what's new from other sites. `--offline` turns this off.
Press Ctrl+C to stop.
"
    );
    let said = served.said.after(opened.len(), "Browser told to reload.\n");
    assert_eq!(
        steady_from_the_start(&served, &said),
        "\
Fetched `fetched/faces/0a523cc7bab84b06.jpg`.
Fetched `fetched/mentions/1.yaml`.
Fetched 1 webmention and 1 face.
FILES changed. Built 1 of 2 files in TIME seconds. Browser told to reload.
"
    );
    // The three files are one round, and neither what fetching wrote nor
    // what that round wrote starts another.
    assert_eq!(rounds_since(&served, served.hub.generation()), 0);
    assert_eq!(
        steady_from_the_start(&served, &served.said.text()),
        steady_from_the_start(&served, &format!("{opened}{said}"))
    );
    let asked = asked.lock().unwrap().clone();
    assert_eq!(asked.len(), 2);
    assert!(asked[0].starts_with(RELAY) && asked[1] == FACE, "{asked:?}");
    assert!(served.page("/post-1/").contains("Words.<img src="));
    same_as_a_build(&served);
}

#[test]
fn a_fetch_that_fails_is_said_and_serving_goes_on() {
    let (fetcher, _) = standing_in(|_| Err(Refusal::Status { status: 503 }));
    let (served, opened) = served_fetching(
        "failed",
        How::Native,
        true,
        Some(fetcher),
        None,
        with_a_video,
    );
    let said = served.said.after(opened.len(), "couldn't be fetched.\n");
    assert_eq!(
        steady_from_the_start(&served, &said),
        "Can't fetch the still for `abcdefghijk`: the server answered 503.\n\
         1 couldn't be fetched.\n"
    );
    let mark = served.said.text().len();
    served
        .site
        .write("content/post-1.md", "# Post 1\n\nEdited.\n");
    served.said.after(mark, "Browser told to reload.\n");
}

#[test]
fn a_fault_while_fetching_is_said_in_a_line_and_serving_goes_on() {
    let (fetcher, _) = standing_in(|_| panic!("a fault"));
    let (served, opened) = served_fetching(
        "fault",
        How::Native,
        true,
        Some(fetcher),
        None,
        with_a_video,
    );
    let said = served.said.after(opened.len(), "Fetching stopped");
    assert_eq!(
        steady_from_the_start(&served, &said),
        "Fetching stopped: Statac met a fault of its own.\n"
    );
    let mark = served.said.text().len();
    served
        .site
        .write("content/post-1.md", "# Post 1\n\nEdited.\n");
    served.said.after(mark, "Browser told to reload.\n");
}

#[test]
fn fetching_waits_for_the_first_round_with_no_error() {
    let still = jpeg(64, 36);
    let (fetcher, asked) = standing_in(move |address| answer(address, &still));
    let (served, opened) =
        served_fetching("waits", How::Native, true, Some(fetcher), None, |site| {
            with_a_video(site);
            site.write("templates/page.html", "{{ page.title }\n");
        });
    assert!(!opened.contains("Fetch"), "{opened}");
    assert!(asked.lock().unwrap().is_empty());

    let mark = served.said.text().len();
    served.site.write("templates/page.html", LAYOUT);
    let said = served.said.after(mark, "Fetched 1 still.\n");
    let said = steady(&served, &said);
    assert!(
        said.starts_with("`templates/page.html` changed. Built 3 of 3 files in TIME seconds."),
        "{said}"
    );
    assert!(
        said.ends_with(
            "Fetching what's new from other sites. `--offline` turns this off.\n\
             Fetched `fetched/stills/youtube-abcdefghijk.jpg`.\n\
             Fetched 1 still.\n"
        ),
        "{said}"
    );
    assert_eq!(*asked.lock().unwrap(), [MAXRES]);
}

#[test]
fn offline_nothing_is_fetched_and_serve_says_so() {
    let (served, opened) = served_fetching("offline", How::Native, true, None, None, with_a_video);
    assert_eq!(
        steady(&served, &opened),
        "\
1 warning: 1 no-still. Run `statac check` to see it.

Wrote 3 pages and 2 other files to `SITE/.statac/serve` in TIME seconds.
Serving `SITE/.statac/serve` at http://localhost:PORT/
Offline: nothing is fetched from other sites.
Press Ctrl+C to stop.
"
    );
    assert_eq!(
        steady_from_the_start(&served, &served.said.text()),
        steady_from_the_start(&served, &opened)
    );
    assert!(!served.site.path("fetched").exists());
}

#[test]
fn a_site_with_nothing_to_fetch_is_served_as_it_was_without_fetching() {
    let (fetcher, asked) = standing_in(|_| panic!("nothing is asked for"));
    let (fetching, opened) =
        served_fetching("nothing", How::Native, true, Some(fetcher), None, |_| {});
    let (offline, before) = served_fetching("nothing", How::Native, true, None, None, |_| {});
    assert_eq!(steady(&fetching, &opened), steady(&offline, &before));
    assert_eq!(
        steady(&fetching, &opened),
        "\
Wrote 2 pages and 2 other files to `SITE/.statac/serve` in TIME seconds.
Serving `SITE/.statac/serve` at http://localhost:PORT/
Press Ctrl+C to stop.
"
    );
    assert_eq!(fetching.said.text(), opened);
    assert!(asked.lock().unwrap().is_empty());
    assert_eq!(
        files(&fetching.site.path(".statac/serve")),
        files(&offline.site.path(".statac/serve"))
    );
}

#[test]
fn webmentions_without_the_token_are_said_not_to_be_asked_for() {
    let (fetcher, asked) = standing_in(|_| panic!("nothing is asked for"));
    let (served, opened) = served_fetching(
        "token",
        How::Native,
        true,
        Some(fetcher),
        None,
        with_webmentions,
    );
    assert!(
        opened.ends_with(
            "Fetching what's new from other sites. `--offline` turns this off.\n\
             Press Ctrl+C to stop.\n"
        ),
        "{opened}"
    );
    let said = served.said.after(opened.len(), "`.statac/token`.\n");
    assert_eq!(
        steady_from_the_start(&served, &said),
        "Webmentions weren't asked for: no token is in `STATAC_WEBMENTION_TOKEN` or `.statac/token`.\n"
    );
    assert!(asked.lock().unwrap().is_empty());
}
