//! A session rebuilding what each save touches leaves what a build of the
//! same files leaves: after every edit in a sequence, the session's folder
//! equals a fresh build's byte for byte, and the two tell the same
//! problems.

use std::fs;
use std::path::{Path, PathBuf};

use image::{ImageFormat, Rgb, RgbImage};
use statac::build::{self, Mode, Options, Outcome};
use statac::report::Report;
use statac::serve::session::{Round, Session};

/// A site in two languages with paged lists, tags, a feed, a snippet, a
/// data file, a stylesheet, an old address and a photograph.
struct Site {
    top: PathBuf,
}

const SETTINGS: &str = "title: Site\nlanguage: [en, de]\nurl: https://example.org\n\
                        feed_key: key\ntimezone: UTC\nposts_per_page: 2\n";

const BASE: &str = "<!doctype html>\n<title>{{ headline(page) }}</title>\n{{ css('site.css') }}\n\
                    <nav>{% for tag in site.tags %}<a href=\"{{ tag.url }}\">{{ tag.name }}</a>\
                    {% endfor %}</nav>\n";

const HOME: &str = "{{ page.content }}\n\
                    {% for post in page.posts %}<a href=\"{{ post.url }}\">{{ post.title }}</a>\
                    {{ post.excerpt }}{% endfor %}\n\
                    {% if page.next %}<a href=\"{{ page.next.url }}\">Older</a>{% endif %}\n\
                    <ul>{% for person in site.data.people %}<li>{{ person.name }}</li>\
                    {% endfor %}</ul>\n";

const PAGE: &str = "{{ page.content }}\n\
                    {% for tag in page.tags %}<a href=\"{{ tag.url }}\">{{ tag.name }}</a>\
                    {% endfor %}\n\
                    {% if page.previous %}<a href=\"{{ page.previous.url }}\">\
                    {{ page.previous.title }}</a>{% endif %}\n\
                    {% if page.next %}<a href=\"{{ page.next.url }}\">{{ page.next.title }}</a>\
                    {% endif %}\n\
                    {% for other in page.translations %}<a href=\"{{ other.url }}\">\
                    {{ other.language }}</a>{% endfor %}\n";

const FEED: &str = "<feed>{% for post in posts %}<entry>{{ post.url }} {{ post.title }} \
                    {{ post.content }}</entry>{% endfor %}</feed>\n";

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let picture = RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x * 3) as u8, (y * 5) as u8, 90])
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    picture.write_to(&mut bytes, ImageFormat::Jpeg).unwrap();
    bytes.into_inner()
}

fn post(title: &str, date: &str, tags: &str, text: &str) -> String {
    format!("---\ntitle: {title}\ndate: {date}\ntags: [{tags}]\n---\n{text}\n")
}

impl Site {
    fn new(name: &str) -> Site {
        let top = std::env::temp_dir().join(format!("statac-serve-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&top);
        let site = Site { top };
        site.write("statac.yaml", SETTINGS);
        site.write("templates/home.html", &format!("{BASE}{HOME}"));
        site.write("templates/folder.html", &format!("{BASE}{HOME}"));
        site.write("templates/page.html", &format!("{BASE}{PAGE}"));
        site.write("templates/feed.xml", FEED);
        site.write(
            "templates/redirect.html",
            "<a href=\"{{ to }}\">{{ title }}</a>\n",
        );
        site.write("snippets/aside.html", "<aside>{{ text }}</aside>\n");
        site.write("assets/site.css", "body { margin: 0; }\n");
        site.write("data/people.yaml", "- name: Person 1\n- name: Person 2\n");
        site.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        site.write(
            "content/de/index.md",
            "---\ntitle: Start\n---\nDie Startseite.\n",
        );
        for n in 1..=5 {
            let text = format!("Post {n} says this.\n\nAnd [the first](/post-1/).");
            let date = format!("2026-01-0{n}");
            site.write(
                &format!("content/post-{n}.md"),
                &post(&format!("Post {n}"), &date, "one", &text),
            );
        }
        site.write(
            "content/de/post-1.md",
            &post("Beitrag 1", "2026-01-01", "eins", "Der erste Beitrag."),
        );
        site.write(
            "content/post-6/index.md",
            "---\ntitle: Post 6\ndate: 2026-01-06\nredirect_from: /old-6/\n---\n\
             A photograph.\n\n![](photo-1.jpg)\n\n@aside(text='Beside the writing.')\n",
        );
        site.put("content/post-6/photo-1.jpg", &jpeg(48, 32));
        site
    }

    fn root(&self) -> PathBuf {
        self.top.join("site")
    }

    fn write(&self, path: &str, text: &str) {
        self.put(path, text.as_bytes());
    }

    fn put(&self, path: &str, bytes: &[u8]) {
        let path = self.root().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn remove(&self, path: &str) {
        fs::remove_file(self.root().join(path)).unwrap();
    }

    fn options(&self) -> Options {
        Options {
            mode: Mode::Build,
            site: self.root(),
            out: Some(self.top.join("fresh")),
            today: statac::date::read("2026-10-04").ok(),
            drafts: false,
            url: None,
        }
    }

    /// What a build of the site as it stands leaves, and says.
    fn fresh(&self, options: &Options) -> (Vec<(String, Vec<u8>)>, String) {
        let outcome = build::run(options);
        (files(&options.out()), self.said(&outcome))
    }

    /// Every problem drawn in full, the JavaScript carried and what was
    /// published.
    fn said(&self, outcome: &Outcome) -> String {
        let root = self.root();
        let report = Report::new(outcome.sources(), &root, false);
        format!(
            "{}{}{:?}",
            report.check(&outcome.problems, &outcome.quiet),
            report.javascript(&outcome.javascript),
            outcome.published,
        )
    }

    /// Holds what `round` left in the session's folder, and said, to a
    /// fresh build of the same files.
    fn same(&self, session: &Session, round: &Round, after: &str) {
        self.same_as(&self.options(), session, round, after);
    }

    /// [`Site::same`], the build being asked as `options` has it.
    fn same_as(&self, options: &Options, session: &Session, round: &Round, after: &str) {
        let (fresh, said) = self.fresh(options);
        assert_eq!(self.said(&round.outcome), said, "after {after}");
        let held = files(session.folder());
        let names = |files: &[(String, Vec<u8>)]| -> Vec<String> {
            files.iter().map(|(name, _)| name.clone()).collect()
        };
        assert_eq!(names(&held), names(&fresh), "after {after}");
        for ((name, held), (_, fresh)) in held.iter().zip(&fresh) {
            let text = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
            assert!(
                held == fresh,
                "{name} after {after}:\n{}\n{}",
                text(held),
                text(fresh)
            );
        }
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.top);
    }
}

/// Every file under `folder`, by its path with forward slashes, with its
/// bytes.
fn files(folder: &Path) -> Vec<(String, Vec<u8>)> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(next) = folders.pop() {
        for entry in fs::read_dir(&next).into_iter().flatten() {
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

/// An edit to a site, and what it is called where a test fails after it.
type Edit<'a> = (&'a str, &'a dyn Fn(&Site));

/// Starts a session on `site`, holds it to a build, and then holds it to
/// one again after each of `edits`, made in turn. Gives how many outputs
/// each round built, of how many.
fn through(site: &Site, edits: &[Edit]) -> Vec<(usize, usize)> {
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    site.same(&session, &round, "the first build");
    let mut built = vec![(round.built, round.outputs)];
    for (after, edit) in edits {
        edit(site);
        let round = session.rebuild();
        site.same(&session, &round, after);
        built.push((round.built, round.outputs));
    }
    built
}

#[test]
fn the_first_round_is_a_whole_build_and_one_with_no_edit_builds_nothing() {
    let site = Site::new("nothing");
    let built = through(&site, &[("no edit", &|_| {})]);
    assert_eq!(built[0].0, built[0].1);
    assert_eq!(built[1].0, 0, "{built:?}");
}

#[test]
fn code_colours_no_template_asks_for_are_told_as_a_build_tells_them() {
    let site = Site::new("colours");
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    site.same(&session, &round, "the first build");
    let coloured = |n: u32| {
        let text = format!("Post {n} says this.\n\n```rust\nfn main() {{}}\n```");
        post(&format!("Post {n}"), &format!("2026-01-0{n}"), "one", &text)
    };
    let plain = post("Post 2", "2026-01-02", "one", "Post 2 says this.");
    let asking = format!("{BASE}{{{{ css('code.css') }}}}\n{PAGE}");
    let not_asking = format!("{BASE}{PAGE}");
    // Each edit, and the files the warning then names: its own, and the
    // other pages'.
    let edits: [(&str, &str, &str, &[&str]); 6] = [
        (
            "a coloured block",
            "content/post-2.md",
            &coloured(2),
            &["post-2.md"],
        ),
        (
            "a second page's",
            "content/post-4.md",
            &coloured(4),
            &["post-2.md"],
        ),
        ("a layout asking", "templates/page.html", &asking, &[]),
        (
            "none asking",
            "templates/page.html",
            &not_asking,
            &["post-2.md"],
        ),
        (
            "the first block gone",
            "content/post-2.md",
            &plain,
            &["post-4.md"],
        ),
        (
            "a block on a page built already",
            "content/post-1.md",
            &coloured(1),
            &["post-1.md"],
        ),
    ];
    for (after, path, text, named) in edits {
        site.write(path, text);
        let round = session.rebuild();
        assert!(round.built < round.outputs, "after {after}");
        let said = site.said(&round.outcome);
        let told: Vec<&str> = said
            .lines()
            .skip_while(|line| !line.contains("Code colours not loaded"))
            .take_while(|line| !line.contains("Add `{{ css"))
            .filter_map(|line| line.split("content/").nth(1))
            .map(|file| file.split(',').next().unwrap_or(file))
            .collect();
        assert_eq!(told, named, "after {after}:\n{said}");
        site.same(&session, &round, after);
    }
}

#[test]
fn a_body_and_a_title_reach_only_what_shows_them() {
    let site = Site::new("writing");
    let edited = post("Post 2", "2026-01-02", "one", "Post 2 says something else.");
    let retitled = post(
        "Post 2 again",
        "2026-01-02",
        "one",
        "Post 2 says something else.",
    );
    let built = through(
        &site,
        &[
            ("a body", &|site| site.write("content/post-2.md", &edited)),
            ("a title", &|site| {
                site.write("content/post-2.md", &retitled)
            }),
        ],
    );
    let outputs = built[0].1;
    assert!(built[1].0 > 0 && built[1].0 < outputs, "{built:?}");
    assert!(built[2].0 > 0 && built[2].0 < outputs, "{built:?}");
}

#[test]
fn a_tag_added_and_removed() {
    let site = Site::new("tag");
    let tagged = post("Post 3", "2026-01-03", "one, two", "Post 3 says this.");
    let untagged = post("Post 3", "2026-01-03", "one", "Post 3 says this.");
    through(
        &site,
        &[
            ("a tag added", &|site| {
                site.write("content/post-3.md", &tagged)
            }),
            ("a tag removed", &|site| {
                site.write("content/post-3.md", &untagged)
            }),
        ],
    );
}

#[test]
fn a_post_added_and_deleted() {
    let site = Site::new("post");
    // Named to be read before the posts there are, so every file read
    // after it is numbered anew.
    let early = post("Post 0", "2026-01-07", "one", "Post 0 says this.");
    through(
        &site,
        &[
            ("a post added", &|site| {
                site.write("content/a-post.md", &early)
            }),
            ("a post deleted", &|site| site.remove("content/a-post.md")),
            ("an older post deleted", &|site| {
                site.remove("content/post-2.md")
            }),
        ],
    );
}

#[test]
fn a_template_a_snippet_and_a_stylesheet() {
    let site = Site::new("templates");
    through(
        &site,
        &[
            ("a template", &|site| {
                site.write(
                    "templates/page.html",
                    &format!("{BASE}{PAGE}<footer></footer>\n"),
                )
            }),
            ("a snippet", &|site| {
                site.write("snippets/aside.html", "<aside><p>{{ text }}</p></aside>\n")
            }),
            ("a stylesheet", &|site| {
                site.write("assets/site.css", "body { margin: 1rem; }\n")
            }),
            ("a template added", &|site| {
                site.write("templates/post.html", &format!("{BASE}{PAGE}"))
            }),
        ],
    );
}

#[test]
fn a_rule_narrowing_pictures_is_told_of_pages_kept_as_of_pages_built() {
    let site = Site::new("narrowed");
    let told = |round: &Round| {
        let problems = round.outcome.problems.iter();
        let names = problems.map(|problem| problem.kind.code().name());
        names
            .filter(|&name| name == "pictures-out-of-proportion")
            .count()
    };
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    site.same(&session, &round, "the first build");
    assert_eq!(told(&round), 0);

    site.write("assets/more.css", "img { max-width: 100%; }\n");
    let round = session.rebuild();
    site.same(&session, &round, "a rule narrowing pictures");
    assert_eq!(told(&round), 1);

    // The page showing the photograph is kept as it was built.
    let round = session.rebuild();
    site.same(&session, &round, "no edit");
    assert_eq!((told(&round), round.built), (1, 0));

    site.write(
        "content/post-6/index.md",
        "---\ntitle: Post 6\ndate: 2026-01-06\n---\nNo photograph.\n",
    );
    let round = session.rebuild();
    site.same(&session, &round, "the photograph no longer shown");
    assert_eq!(told(&round), 0);

    site.write(
        "content/post-6/index.md",
        "---\ntitle: Post 6\ndate: 2026-01-06\n---\n![](photo-1.jpg)\n",
    );
    let round = session.rebuild();
    site.same(&session, &round, "the photograph shown again");
    assert_eq!(told(&round), 1);

    site.write(
        "assets/more.css",
        "img { max-width: 100%; height: auto; }\n",
    );
    let round = session.rebuild();
    site.same(&session, &round, "the height left to the browser");
    assert_eq!(told(&round), 0);
}

#[test]
fn a_data_file_a_picture_and_what_describes_it() {
    let site = Site::new("data");
    let built = through(
        &site,
        &[
            ("a data file", &|site| {
                site.write("data/people.yaml", "- name: Person 1\n- name: Person 3\n")
            }),
            ("a description", &|site| {
                site.write("content/post-6/photo-1.jpg.yaml", "alt: A photograph\n")
            }),
            ("a description changed", &|site| {
                site.write("content/post-6/photo-1.jpg.yaml", "alt: The photograph\n")
            }),
            ("a picture", &|site| {
                site.put("content/post-6/photo-1.jpg", &jpeg(64, 32))
            }),
            ("a description removed", &|site| {
                site.remove("content/post-6/photo-1.jpg.yaml")
            }),
        ],
    );
    assert!(built[2].0 < built[2].1, "{built:?}");
}

#[test]
fn the_settings_and_a_translation() {
    let site = Site::new("settings");
    let translated = post("Beitrag 2", "2026-01-02", "eins", "Der zweite Beitrag.");
    through(
        &site,
        &[
            ("the settings", &|site| {
                site.write(
                    "statac.yaml",
                    &SETTINGS.replace("title: Site", "title: Site again"),
                )
            }),
            ("a translation", &|site| {
                site.write("content/de/post-2.md", &translated)
            }),
            ("a translation removed", &|site| {
                site.remove("content/de/post-1.md")
            }),
            ("pages a list shows", &|site| {
                site.write(
                    "statac.yaml",
                    &SETTINGS.replace("posts_per_page: 2", "posts_per_page: 4"),
                )
            }),
        ],
    );
}

#[test]
fn an_error_introduced_and_fixed_leaves_the_last_site_until_it_is() {
    let site = Site::new("error");
    let broken = post(
        "Post 4",
        "2026-01-04",
        "one",
        "Post 4 calls for nothing the site has.\n\n@nothing()",
    );
    let fixed = post(
        "Post 4",
        "2026-01-04",
        "one",
        "Post 4 [leads somewhere](/post-5/).",
    );
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    site.same(&session, &round, "the first build");
    let before = files(session.folder());

    site.write("content/post-4.md", &broken);
    let round = session.rebuild();
    assert!(round.outcome.stopped());
    assert!(round.written.is_empty() && round.removed.is_empty());
    assert!(files(session.folder()) == before);
    site.same(&session, &round, "an error");

    site.write(
        "templates/page.html",
        &format!("{BASE}{PAGE}{{{{ page.nothing.at.all }}}}\n"),
    );
    let round = session.rebuild();
    assert!(round.outcome.stopped());
    assert!(files(session.folder()) == before);
    site.same(&session, &round, "a second error");

    site.write("templates/page.html", &format!("{BASE}{PAGE}"));
    site.write("content/post-4.md", &fixed);
    let round = session.rebuild();
    assert!(!round.outcome.stopped());
    // The post, and the pages and the feed that show its writing.
    assert_eq!(
        round.written,
        [
            "feed.xml",
            "page/2/index.html",
            "post-4/index.html",
            "tags/one/index.html"
        ]
    );
    site.same(&session, &round, "the fix");
}

#[test]
fn what_a_folder_hands_down_a_draft_and_an_old_address() {
    let site = Site::new("handed");
    let note = "---\ntitle: Note 1\ndate: 2026-02-01\n---\nA note.\n";
    let folder = |tags: &str| format!("---\ntitle: Notes\ntags: [{tags}]\n---\nThe notes.\n");
    let draft =
        "---\ntitle: Post 3\ndate: 2026-01-03\ntags: [one]\ndraft: true\n---\nUnfinished.\n";
    let moved = "---\ntitle: Post 6\ndate: 2026-01-06\nredirect_from: [/old-6/, /older-6/]\n---\n\
                 A photograph.\n\n![](photo-1.jpg)\n\n@aside(text='Beside the writing.')\n";
    through(
        &site,
        &[
            ("a folder added", &|site| {
                site.write("content/notes/index.md", &folder("one"));
                site.write("content/notes/note-1.md", note);
            }),
            ("what the folder hands down", &|site| {
                site.write("content/notes/index.md", &folder("two"))
            }),
            ("a draft", &|site| site.write("content/post-3.md", draft)),
            ("an old address", &|site| {
                site.write("content/post-6/index.md", moved)
            }),
        ],
    );
}

#[test]
fn a_folders_own_permalink_added_changed_and_removed() {
    let site = Site::new("folder-permalink");
    let folder = |permalink: &str| format!("---\ntitle: Notes\n{permalink}---\nThe notes.\n");
    for n in 1..=3 {
        site.write(
            &format!("content/notes/note-{n}.md"),
            &format!("---\ntitle: Note {n}\ndate: 2026-02-0{n}\n---\nNote {n}.\n"),
        );
    }
    site.put("content/notes/photo-1.jpg", &jpeg(48, 32));
    site.write("content/notes/index.md", &folder(""));
    through(
        &site,
        &[
            ("a permalink added", &|site| {
                site.write("content/notes/index.md", &folder("permalink: /writing/\n"))
            }),
            ("a permalink changed to a file", &|site| {
                site.write(
                    "content/notes/index.md",
                    &folder("permalink: /notes.html\n"),
                )
            }),
            ("the home page's added", &|site| {
                site.write(
                    "content/index.md",
                    "---\ntitle: Home\npermalink: /start/\n---\nThe home page.\n",
                )
            }),
            ("a permalink removed", &|site| {
                site.write("content/notes/index.md", &folder(""));
                site.write(
                    "content/index.md",
                    "---\ntitle: Home\n---\nThe home page.\n",
                )
            }),
        ],
    );
}

#[test]
fn a_field_naming_a_record_or_a_page_is_held_to_the_names_there_now() {
    let site = Site::new("named");
    let declared = "fields.first: one of data.items\nfields.second: one of folder.notes\n";
    site.write("statac.yaml", &format!("{SETTINGS}{declared}"));
    site.write("data/items.yaml", "item-1:\n  title: Item 1\n");
    site.write("content/notes/note-1.md", "# Note 1\n");
    site.write(
        "content/post-5.md",
        "---\ntitle: Post 5\ndate: 2026-01-05\nfirst: item-1\nsecond: note-1\n---\nPost 5.\n",
    );
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    site.same(&session, &round, "the first build");
    let edits: [(bool, Edit); 6] = [
        (
            true,
            ("a record renamed", &|site| {
                site.write("data/items.yaml", "item-2:\n  title: Item 1\n")
            }),
        ),
        (
            false,
            ("the record named again", &|site| {
                site.write("data/items.yaml", "item-1:\n  title: Item 1\n")
            }),
        ),
        (
            true,
            ("a page renamed", &|site| {
                site.remove("content/notes/note-1.md");
                site.write("content/notes/note-2.md", "# Note 1\n");
            }),
        ),
        (
            false,
            ("a draft of the name", &|site| {
                site.write("content/notes/note-1.md", "---\ndraft: true\n---\n")
            }),
        ),
        (
            true,
            ("the table written as a list", &|site| {
                site.write("data/items.yaml", "- title: Item 1\n")
            }),
        ),
        (
            false,
            ("the table written by name", &|site| {
                site.write("data/items.yaml", "item-1:\n  title: Item 1\n")
            }),
        ),
    ];
    for (stops, (after, edit)) in edits {
        edit(&site);
        let round = session.rebuild();
        assert_eq!(round.outcome.stopped(), stops, "after {after}");
        site.same(&session, &round, after);
    }
}

#[test]
fn the_middle_of_redirects_followed_on_changed() {
    let site = Site::new("followed");
    site.write("content/moved-1.md", "---\nredirect_to: /moved-2/\n---\n");
    site.write("content/moved-2.md", "---\nredirect_to: /post-1/\n---\n");
    through(
        &site,
        &[
            ("where the middle sends readers", &|site| {
                site.write("content/moved-2.md", "---\nredirect_to: /post-2/\n---\n")
            }),
            (
                "the middle sending readers on from an old address",
                &|site| site.write("content/moved-2.md", "---\nredirect_to: /old-6/\n---\n"),
            ),
            ("the middle published", &|site| {
                site.write("content/moved-2.md", "---\ntitle: Moved 2\n---\n")
            }),
        ],
    );
}

#[test]
fn a_link_is_held_to_what_the_site_publishes_now() {
    let site = Site::new("links");
    let other = "---\ntitle: Page 1\n---\nA page.\n";
    let round_trip: &[Edit] = &[
        // Every post links the first, and reads nothing of it.
        ("the page linked removed", &|site| {
            site.remove("content/post-1.md")
        }),
        ("the page linked put back", &|site| {
            let text = "Post 1 says this.\n\nAnd [the first](/post-1/).";
            site.write(
                "content/post-1.md",
                &post("Post 1", "2026-01-01", "one", text),
            )
        }),
        ("a page added", &|site| {
            site.write("content/page-1.md", other)
        }),
    ];
    through(&site, round_trip);
}

#[test]
fn an_address_inside_the_site_moves_with_the_sites_path() {
    let site = Site::new("inside");
    let text = "[The first](@url(/post-1/)), <a href=\"@url(/post-1/)\">again</a>.\n\n\
                @aside(text='Beside the writing.')";
    site.write(
        "content/post-2.md",
        &post("Post 2", "2026-01-02", "one", text),
    );
    site.write(
        "snippets/aside.html",
        "<aside>{{ text }} <a href=\"{{ url('/post-1/') }}\">The first</a></aside>\n",
    );
    let under = |path: &'static str| {
        move |site: &Site| {
            let url = format!("url: https://example.org{path}");
            site.write(
                "statac.yaml",
                &SETTINGS.replace("url: https://example.org", &url),
            )
        }
    };
    through(
        &site,
        &[
            ("the site put under a path", &under("/blog/")),
            ("the site's path changed", &under("/notes/")),
            ("the site put back at the top", &under("")),
        ],
    );
}

#[test]
fn what_a_kept_page_told_is_told_again_where_it_is_written() {
    let site = Site::new("told");
    let nowhere = post(
        "Post 4",
        "2026-01-04",
        "one",
        "Post 4 [leads nowhere](/post-9/).",
    );
    let early = post("Post 0", "2026-01-07", "two", "Post 0 says this.");
    let built = through(
        &site,
        &[
            ("a link that lands nowhere", &|site| {
                site.write("content/post-4.md", &nowhere)
            }),
            // Read before the page that told it, so its file is numbered
            // anew.
            ("a post added before it", &|site| {
                site.write("content/a-post.md", &early)
            }),
            ("a body elsewhere", &|site| {
                site.write("content/de/index.md", "---\ntitle: Start\n---\nAnders.\n")
            }),
        ],
    );
    assert!(built[3].0 < built[3].1, "{built:?}");
}

#[test]
fn a_layout_that_stops_showing_the_writing_and_a_page_that_loses_its_own() {
    let site = Site::new("hollow");
    let hollow = format!("{BASE}{{{{ page.title }}}}\n");
    let whole = format!("{BASE}{PAGE}");
    let empty = "---\ntitle: Post 2\ndate: 2026-01-02\ntags: [one]\n---\n";
    let written = post("Post 2", "2026-01-02", "one", "Post 2 says this.");
    let (mut session, _) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    let layout = "built by `templates/page.html`";
    let edits: [(Edit, Option<&str>); 4] = [
        (
            ("a layout that shows no writing", &|site| {
                site.write("templates/page.html", &hollow)
            }),
            Some("7 pages"),
        ),
        (
            ("a page left with no writing", &|site| {
                site.write("content/post-2.md", empty)
            }),
            Some("6 pages"),
        ),
        (
            ("a page given writing", &|site| {
                site.write("content/post-2.md", &written)
            }),
            Some("7 pages"),
        ),
        (
            ("a layout that shows the writing", &|site| {
                site.write("templates/page.html", &whole)
            }),
            None,
        ),
    ];
    for ((after, edit), pages) in edits {
        edit(&site);
        let round = session.rebuild();
        site.same(&session, &round, after);
        let said = site.said(&round.outcome);
        match pages {
            Some(pages) => assert!(
                said.contains(&format!("Writing never shown: {pages} {layout}")),
                "after {after}: {said}"
            ),
            None => assert!(!said.contains(layout), "after {after}: {said}"),
        }
    }
}

#[test]
fn a_round_says_what_it_wrote_and_removed() {
    let site = Site::new("written");
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    // Every file but the marker that says whose the folder is.
    let held: Vec<String> = files(session.folder())
        .into_iter()
        .map(|(name, _)| name)
        .filter(|name| name != ".statac-built")
        .collect();
    assert_eq!(round.written, held);
    assert_eq!(session.folder(), site.root().join(".statac").join("serve"));

    site.write(
        "content/post-5.md",
        &post("Post 5", "2026-01-05", "one", "Post 5 edited."),
    );
    let round = session.rebuild();
    assert!(
        round.written.contains(&"post-5/index.html".to_owned()),
        "{:?}",
        round.written
    );
    assert!(
        !round.written.contains(&"post-3/index.html".to_owned()),
        "{:?}",
        round.written
    );
    assert!(round.removed.is_empty(), "{:?}", round.removed);

    site.remove("content/post-5.md");
    let round = session.rebuild();
    // Its tag's posts fit on one page fewer without it.
    assert_eq!(
        round.removed,
        ["post-5/index.html", "tags/one/page/3/index.html"]
    );
    site.same(&session, &round, "a post removed");
}

/// A folder outside the site, holding a file of someone else's.
#[cfg(unix)]
fn outside(site: &Site) -> PathBuf {
    let folder = site.top.join("outside");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("index.html"), "someone else's").unwrap();
    folder
}

#[cfg(unix)]
fn untouched(folder: &Path) {
    let held = files(folder);
    assert_eq!(
        held.len(),
        1,
        "{:?}",
        held.iter().map(|(name, _)| name).collect::<Vec<_>>()
    );
    assert_eq!(
        held[0],
        ("index.html".to_owned(), b"someone else's".to_vec())
    );
}

#[cfg(unix)]
#[test]
fn nothing_is_written_through_a_link_standing_for_the_folder() {
    use std::os::unix::fs::symlink;
    for linked in [".statac/serve", ".statac"] {
        let site = Site::new(&format!("linked-{}", linked.len()));
        let outside = outside(&site);
        let link = site.root().join(linked);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink(&outside, &link).unwrap();
        let (mut session, round) = Session::start(&Options {
            out: None,
            ..site.options()
        });
        let said = site.said(&round.outcome);
        assert!(round.outcome.stopped(), "{said}");
        assert!(said.contains("Not Statac's folder"), "{said}");
        assert!(said.contains("is a link"), "{said}");
        assert!(round.written.is_empty());
        untouched(&outside);
        let round = session.rebuild();
        assert!(round.outcome.stopped());
        untouched(&outside);
    }
}

/// Starts a session, puts `link` in the built site in place of what is
/// there, leading to a folder outside the site or to the file in it, then
/// makes each of `edits` in turn: every round after must stop, having
/// written and removed nothing outside, until the session builds the site
/// whole again.
#[cfg(unix)]
fn with_a_link_put_in(name: &str, link: &str, to_file: bool, edit: &dyn Fn(&Site)) {
    use std::os::unix::fs::symlink;
    let site = Site::new(name);
    let outside = outside(&site);
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped());
    let link = session.folder().join(link);
    match link.is_dir() {
        true => fs::remove_dir_all(&link).unwrap(),
        false => fs::remove_file(&link).unwrap(),
    }
    let to = match to_file {
        true => outside.join("index.html"),
        false => outside.clone(),
    };
    symlink(to, &link).unwrap();

    edit(&site);
    let round = session.rebuild();
    let said = site.said(&round.outcome);
    assert!(round.outcome.stopped(), "{said}");
    assert!(said.contains("it is a link"), "{said}");
    untouched(&outside);

    // Built whole again, the folder is the session's own once more.
    let round = session.rebuild();
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    untouched(&outside);
    assert!(!fs::symlink_metadata(&link).is_ok_and(|at| at.is_symlink()));
    site.same(&session, &round, "a link removed");
}

#[cfg(unix)]
#[test]
fn nothing_is_written_through_a_link_put_in_place_of_a_folder() {
    let edited = post("Post 2", "2026-01-02", "one", "Post 2 says something else.");
    with_a_link_put_in("link-folder-write", "post-2", false, &|site| {
        site.write("content/post-2.md", &edited)
    });
}

#[cfg(unix)]
#[test]
fn nothing_is_removed_through_a_link_put_in_place_of_a_folder() {
    with_a_link_put_in("link-folder-remove", "post-2", false, &|site| {
        site.remove("content/post-2.md")
    });
}

#[cfg(unix)]
#[test]
fn nothing_is_written_through_a_link_put_in_place_of_a_file() {
    let edited = post("Post 2", "2026-01-02", "one", "Post 2 says something else.");
    with_a_link_put_in("link-file-write", "post-2/index.html", true, &|site| {
        site.write("content/post-2.md", &edited)
    });
}

#[cfg(unix)]
#[test]
fn nothing_is_removed_through_a_link_put_in_place_of_a_file() {
    with_a_link_put_in("link-file-remove", "post-2/index.html", true, &|site| {
        site.remove("content/post-2.md")
    });
}

#[test]
fn an_output_built_to_the_bytes_it_had_is_not_written_again() {
    let site = Site::new("unchanged");
    let (mut session, _) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    // A comment in a template reaches no page.
    site.write(
        "templates/page.html",
        &format!("{BASE}{PAGE}{{# Nothing a page shows. #}}"),
    );
    let round = session.rebuild();
    assert!(round.built > 0);
    assert!(round.written.is_empty(), "{:?}", round.written);
    assert!(round.removed.is_empty(), "{:?}", round.removed);
    site.same(&session, &round, "a comment in a template");

    // Still the session's to write and to remove.
    site.remove("content/post-5.md");
    let round = session.rebuild();
    assert!(round.removed.contains(&"post-5/index.html".to_owned()));
    site.same(&session, &round, "a post removed");
}

#[test]
fn a_round_says_whether_its_pages_differ_only_by_a_stylesheets_name() {
    let site = Site::new("restyled");
    let (mut session, _) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    site.write("assets/site.css", "body { margin: 2rem; }\n");
    let round = session.rebuild();
    assert!(round.restyled);
    assert!(round.written.iter().any(|file| file.ends_with(".css")));
    assert!(round.written.contains(&"index.html".to_owned()));
    site.same(&session, &round, "a stylesheet");

    site.write("assets/site.css", "body { margin: 3rem; }\n");
    site.write(
        "content/post-5.md",
        &post("Post 5", "2026-01-05", "one", "Post 5 edited."),
    );
    let round = session.rebuild();
    assert!(!round.restyled);
    site.same(&session, &round, "a stylesheet and a post");
}

/// The name the session's folder holds the site's own stylesheet under,
/// or its script where `ending` is `.js`.
fn published(session: &Session, ending: &str) -> String {
    let held = files(session.folder());
    let own = |name: &str| name.starts_with("site.") || name.starts_with("app.");
    let mut named = (held.iter()).filter(|(name, _)| own(name) && name.ends_with(ending));
    let (name, _) = named.next().expect("a file published under that ending");
    assert!(named.next().is_none(), "one file ends {ending}");
    name.clone()
}

/// The text of the file at `file` in the session's folder.
fn page(session: &Session, file: &str) -> String {
    fs::read_to_string(session.folder().join(file)).unwrap()
}

#[test]
fn a_stylesheet_edited_where_a_page_shows_its_address_twice() {
    let site = Site::new("style-twice");
    let preload = "<link rel=\"preload\" href=\"{{ asset('site.css') }}\" as=\"style\">\n";
    site.write("templates/page.html", &format!("{BASE}{preload}{PAGE}"));
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    site.same(&session, &round, "the first build");
    let was = published(&session, ".css");
    assert_eq!(page(&session, "post-2/index.html").matches(&was).count(), 2);

    site.write("assets/site.css", "body { margin: 2rem; }\n");
    let round = session.rebuild();
    site.same(&session, &round, "a stylesheet");
    let now = published(&session, ".css");
    assert_ne!(was, now);
    let post = page(&session, "post-2/index.html");
    assert_eq!(post.matches(&now).count(), 2, "{post}");
    assert!(!post.contains(&was), "{post}");
    assert!(round.restyled);
}

#[test]
fn a_stylesheet_edited_where_a_pages_writing_holds_the_name_it_had() {
    let site = Site::new("style-named");
    let (mut session, _) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    let was = published(&session, ".css");
    let text = format!("The stylesheet was `{was}` and /{was} when this was written.");
    site.write(
        "content/post-2.md",
        &post("Post 2", "2026-01-02", "one", &text),
    );
    let round = session.rebuild();
    site.same(&session, &round, "a post naming the stylesheet");
    assert_eq!(page(&session, "post-2/index.html").matches(&was).count(), 3);

    site.write("assets/site.css", "body { margin: 2rem; }\n");
    let round = session.rebuild();
    site.same(&session, &round, "a stylesheet");
    let now = published(&session, ".css");
    assert_ne!(was, now);
    // Its writing still says what it said.
    let post = page(&session, "post-2/index.html");
    assert_eq!(post.matches(&was).count(), 2, "{post}");
    assert_eq!(post.matches(&now).count(), 1, "{post}");
    // A page that differs by more than the name is no page restyled.
    site.write("assets/site.css", "body { margin: 3rem; }\n");
    let round = session.rebuild();
    site.same(&session, &round, "a stylesheet again");
    assert_eq!(page(&session, "post-2/index.html").matches(&was).count(), 2);
}

#[test]
fn a_part_of_a_sass_stylesheet_edited() {
    let site = Site::new("style-sass");
    site.remove("assets/site.css");
    site.write(
        "assets/site.scss",
        "@use 'parts';\nbody { margin: parts.$gap; }\n",
    );
    site.write("assets/_parts.scss", "$gap: 1rem;\n");
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    site.same(&session, &round, "the first build");
    let was = published(&session, ".css");
    assert!(page(&session, &was).contains("1rem"));

    site.write("assets/_parts.scss", "$gap: 2rem;\n");
    let round = session.rebuild();
    site.same(&session, &round, "a part of a stylesheet");
    let now = published(&session, ".css");
    assert_ne!(was, now);
    assert!(page(&session, &now).contains("2rem"));
    assert!(page(&session, "post-2/index.html").contains(&now));
    assert!(round.restyled);

    // A part that says the same builds the same stylesheet.
    site.write("assets/_parts.scss", "$gap: 2rem;\n// Said again.\n");
    let round = session.rebuild();
    site.same(&session, &round, "a part saying the same");
    assert_eq!(published(&session, ".css"), now);
    assert!(round.written.is_empty(), "{:?}", round.written);
}

#[test]
fn a_script_edited() {
    let site = Site::new("script");
    site.write("assets/app.js", "console.log(1);\n");
    site.write(
        "templates/page.html",
        &format!("{BASE}{{{{ js('app.js') }}}}\n{PAGE}"),
    );
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    site.same(&session, &round, "the first build");
    let was = published(&session, ".js");
    assert!(page(&session, "post-2/index.html").contains(&was));

    site.write("assets/app.js", "console.log(2);\n");
    let round = session.rebuild();
    site.same(&session, &round, "a script");
    let now = published(&session, ".js");
    assert_ne!(was, now);
    let post = page(&session, "post-2/index.html");
    assert!(post.contains(&now) && !post.contains(&was), "{post}");
    // The home page links no script, and stands.
    assert!(!round.written.contains(&"index.html".to_owned()));
    assert!(round.written.contains(&"post-2/index.html".to_owned()));
    assert!(!round.restyled);
}

#[test]
fn javascript_written_in_a_called_snippet_is_listed_by_neither() {
    let site = Site::new("called");
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    site.same(&session, &round, "the first build");
    let listed = |round: &Round| {
        let root = site.root();
        let report = Report::new(round.outcome.sources(), &root, false);
        report.javascript(&round.outcome.javascript)
    };
    assert_eq!(listed(&round), "");

    site.write(
        "snippets/aside.html",
        "<aside onclick=\"run()\">{{ text }}</aside>\n<script>run()</script>\n",
    );
    let round = session.rebuild();
    site.same(&session, &round, "a script in a called snippet");
    assert_eq!(listed(&round), "");

    site.write(
        "snippets/markdown/link.html",
        "<a href=\"{{ address }}\" onclick=\"run()\">{{ body }}</a>\n",
    );
    let round = session.rebuild();
    site.same(&session, &round, "a script in a piece of the writing");
    assert!(
        listed(&round).contains("  Written in `snippets/markdown/link.html`, on "),
        "{}",
        listed(&round)
    );
    assert!(!listed(&round).contains("aside"), "{}", listed(&round));

    site.remove("snippets/markdown/link.html");
    site.write(
        "content/post-6/index.md",
        "---\ntitle: Post 6\ndate: 2026-01-06\nredirect_from: /old-6/\n---\n\
         @aside(text='Beside the writing.')\n\n<script>run()</script>\n",
    );
    let round = session.rebuild();
    site.same(&session, &round, "a script in the page calling the snippet");
    assert_eq!(
        listed(&round),
        "JavaScript the site carries:\n  Written in `content/post-6/index.md`, on 2 pages\n"
    );
}

#[test]
fn a_stylesheet_edited_with_a_post() {
    let site = Site::new("style-and-post");
    let (mut session, _) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    let was = published(&session, ".css");
    let shorter = post("Post 5", "2026-01-05", "one", "Less.");
    let longer = post(
        "Post 3",
        "2026-01-03",
        "one",
        "Post 3 says this, and now a good deal more than it said.",
    );
    site.write("assets/site.css", "body { margin: 2rem; }\n");
    site.write("content/post-5.md", &shorter);
    site.write("content/post-3.md", &longer);
    let round = session.rebuild();
    site.same(&session, &round, "a stylesheet and two posts");
    let now = published(&session, ".css");
    assert_ne!(was, now);
    assert!(!round.restyled);
    assert!(round.removed.contains(&was), "{:?}", round.removed);
    for file in ["post-3/index.html", "post-5/index.html", "index.html"] {
        assert!(round.written.contains(&file.to_owned()), "{file}");
        assert!(page(&session, file).contains(&now), "{file}");
    }
    assert!(page(&session, "post-5/index.html").contains("Less."));

    // The stylesheet put back with the posts as they are.
    site.write("assets/site.css", "body { margin: 0; }\n");
    let round = session.rebuild();
    site.same(&session, &round, "the stylesheet put back");
    assert_eq!(published(&session, ".css"), was);
    assert!(round.restyled);
}

/// How many lines the file at `path` holds; none where it isn't there.
#[cfg(unix)]
fn lines(path: &Path) -> usize {
    fs::read_to_string(path).map_or(0, |text| text.lines().count())
}

#[cfg(unix)]
#[test]
fn the_command_before_a_build_runs_again_only_for_what_it_watches() {
    let site = Site::new("before");
    let commands = "before_build: echo ran >> ran.txt && cp styles/site.css assets/site.css\n\
                    before_build_watches: [styles/, tokens.txt]\n";
    site.write("statac.yaml", &format!("{SETTINGS}{commands}"));
    site.write("styles/site.css", "body { margin: 1rem; }\n");
    site.write("tokens.txt", "one\n");
    let ran = || lines(&site.root().join("ran.txt"));
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    assert!(session.ran_before() && session.watches());
    assert_eq!(ran(), 1);
    // What the command wrote into the site is told, to be no save.
    let wrote = |round: &Round| -> Vec<String> {
        round.hooked.iter().map(|(file, _)| file.clone()).collect()
    };
    assert_eq!(wrote(&round), ["assets/site.css", "ran.txt"]);

    let changed = |files: &[&str]| -> Vec<String> { files.iter().map(|f| f.to_string()).collect() };
    site.write(
        "content/post-5.md",
        &post("Post 5", "2026-01-05", "one", "Post 5 edited."),
    );
    let round = session.rebuild_for(&changed(&["content/post-5.md"]));
    assert_eq!(ran(), 1, "a file it doesn't watch");
    assert!(round.hooked.is_empty());

    site.write("styles/site.css", "body { margin: 2rem; }\n");
    let round = session.rebuild_for(&changed(&["styles/site.css"]));
    assert_eq!(ran(), 2, "a file in a folder it watches");
    assert_eq!(wrote(&round), ["assets/site.css", "ran.txt"]);
    assert!(round.written.iter().any(|file| file.ends_with(".css")));
    let held = files(session.folder());
    assert!(
        held.iter()
            .any(|(name, bytes)| name.ends_with(".css") && bytes == b"body { margin: 2rem; }\n")
    );

    let round = session.rebuild_for(&changed(&["tokens.txt"]));
    assert_eq!(ran(), 3, "a file it watches");
    assert!(!round.outcome.stopped());
    // The command's own writing, come back as a save, runs nothing.
    session.rebuild_for(&changed(&["tokens.txt.old", "stylesheets/site.css"]));
    assert_eq!(ran(), 3, "names that only open as a watched one does");
}

#[cfg(unix)]
#[test]
fn a_command_that_stops_stops_its_round_and_the_next_builds() {
    let site = Site::new("before-stops");
    let commands = "before_build: test ! -f styles/stop\n\
                    before_build_watches: [styles/]\n";
    site.write("statac.yaml", &format!("{SETTINGS}{commands}"));
    site.write("styles/site.css", "body { margin: 1rem; }\n");
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    site.write("styles/stop", "");
    let round = session.rebuild_for(&["styles/stop".to_owned()]);
    assert!(round.outcome.stopped());
    let said = site.said(&round.outcome);
    assert!(
        said.contains("Hook stopped: `before_build` ended with exit code 1"),
        "{said}"
    );
    assert!(session.folder().join("index.html").is_file());

    site.remove("styles/stop");
    let round = session.rebuild_for(&["styles/stop".to_owned()]);
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
}

#[cfg(unix)]
#[test]
fn the_command_after_a_build_runs_once_and_what_it_made_stays() {
    let site = Site::new("after");
    let ran = site.top.join("after.txt");
    let commands = format!(
        "after_build: test \"$STATAC_SERVE\" = yes && echo made >> \"$STATAC_OUT/made.txt\" \
         && mkdir \"$STATAC_OUT/search\" && echo index > \"$STATAC_OUT/search/index.txt\" \
         && echo ran >> \"{}\"\n\
         after_build_makes: [search/, made.txt]\n",
        ran.display()
    );
    site.write("statac.yaml", &format!("{SETTINGS}{commands}"));
    site.write(
        "content/post-5.md",
        &post(
            "Post 5",
            "2026-01-05",
            "one",
            "See [what was made](/made.txt).",
        ),
    );
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    let said = site.said(&round.outcome);
    assert!(!round.outcome.stopped(), "{said}");
    assert!(!said.contains("Broken link"), "{said}");
    assert!(session.sets_after() && session.ran_after());
    let changed = round.outcome.hooks.after.unwrap();
    assert_eq!((changed.added, changed.removed), (2, 0));
    let mine = |file: &String| file == "made.txt" || file.starts_with("search/");
    assert!(!round.written.iter().any(mine), "{:?}", round.written);

    for n in 1..=10 {
        site.write(
            "content/post-4.md",
            &post("Post 4", "2026-01-04", "one", &format!("Edit {n}.")),
        );
        let round = session.rebuild();
        let said = site.said(&round.outcome);
        assert!(!round.outcome.stopped(), "round {n}: {said}");
        assert!(!said.contains("Broken link"), "round {n}: {said}");
        assert!(!said.contains("Nothing made"), "round {n}: {said}");
        assert!(round.outcome.hooks.after.is_none(), "round {n}");
        assert!(round.written.contains(&"post-4/index.html".to_owned()));
        assert!(!round.written.iter().any(mine), "{:?}", round.written);
        assert!(!round.removed.iter().any(mine), "{:?}", round.removed);
    }
    assert_eq!(lines(&ran), 1);
    let folder = session.folder().to_path_buf();
    let made = |path: &str| fs::read_to_string(folder.join(path)).ok();
    assert_eq!(made("made.txt").as_deref(), Some("made\n"));
    assert_eq!(made("search/index.txt").as_deref(), Some("index\n"));
    // A post removed and a stylesheet changed leave it too.
    site.remove("content/post-3.md");
    site.write("assets/site.css", "body { margin: 4rem; }\n");
    session.rebuild();
    assert_eq!(made("search/index.txt").as_deref(), Some("index\n"));
    assert_eq!(lines(&ran), 1);
}

#[cfg(unix)]
#[test]
fn the_command_after_a_build_waits_for_a_build_with_no_errors() {
    let site = Site::new("after-waits");
    let ran = site.top.join("after.txt");
    let commands = format!("after_build: echo ran >> \"{}\"\n", ran.display());
    site.write("statac.yaml", &format!("{SETTINGS}{commands}"));
    let broken = "---\ntitle: Post 5\ndate: 5/1/2026\n---\nPost 5.\n";
    let mended = post("Post 5", "2026-01-05", "one", "Post 5 mended.");
    site.write("content/post-5.md", broken);
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(round.outcome.stopped());
    assert!(session.sets_after() && !session.ran_after());
    assert_eq!(lines(&ran), 0);

    site.write("content/post-5.md", &mended);
    let round = session.rebuild();
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    assert!(round.outcome.hooks.after.is_some() && session.ran_after());
    assert_eq!(lines(&ran), 1);

    site.write("content/post-5.md", broken);
    session.rebuild();
    site.write("content/post-5.md", &mended);
    let round = session.rebuild();
    assert!(!round.outcome.stopped());
    assert_eq!(lines(&ran), 1, "not again after a later error");

    // Nor after settings that couldn't be read, where no command is known.
    let settings = fs::read_to_string(site.root().join("statac.yaml")).unwrap();
    site.write("statac.yaml", &format!("{settings}title: Site again\n"));
    assert!(session.rebuild().outcome.stopped());
    site.write("statac.yaml", &settings);
    let round = session.rebuild();
    assert!(!round.outcome.stopped());
    assert_eq!(lines(&ran), 1, "not again after an error in the settings");
}

/// A site with a draft, a post waiting for its day and a list both would
/// stand in.
fn with_drafts(name: &str) -> Site {
    let site = Site::new(name);
    site.write(
        "content/post-7.md",
        "---\ntitle: Post 7\ndate: 2026-01-07\ntags: [one, two]\ndraft: true\n---\nA draft.\n",
    );
    site.write(
        "content/post-8.md",
        &post("Post 8", "2026-12-25", "three", "Waiting for its day."),
    );
    site
}

#[test]
fn a_session_showing_drafts_leaves_what_a_build_with_drafts_leaves() {
    let site = with_drafts("drafts");
    let drafts = Options {
        drafts: true,
        ..site.options()
    };
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..drafts.clone()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    let included = round.outcome.included;
    assert_eq!((included.drafts, included.waiting), (1, 1));
    assert!(session.folder().join("post-7/index.html").is_file());
    assert!(session.folder().join("post-8/index.html").is_file());
    site.same_as(&drafts, &session, &round, "the first build");
    // The pages a build leaves out are noted for whoever looks at them.
    let noted: Vec<(&str, bool, Option<&str>)> = (round.noted.iter())
        .filter(|noted| noted.draft || noted.waiting.is_some())
        .map(|noted| (noted.file.as_str(), noted.draft, noted.waiting.as_deref()))
        .collect();
    assert_eq!(
        noted,
        [
            ("post-7/index.html", true, None),
            ("post-8/index.html", false, Some("2026-12-25"))
        ]
    );

    let edits: [Edit; 4] = [
        ("a draft edited", &|site| {
            site.write(
                "content/post-7.md",
                "---\ntitle: Post 7\ndate: 2026-01-07\ntags: [two]\ndraft: true\n---\nEdited.\n",
            )
        }),
        ("a draft published", &|site| {
            site.write(
                "content/post-7.md",
                &post("Post 7", "2026-01-07", "two", "Published."),
            )
        }),
        ("a post made to wait", &|site| {
            site.write(
                "content/post-2.md",
                &post("Post 2", "2027-01-02", "one", "Waiting now."),
            )
        }),
        ("a waiting post removed", &|site| {
            site.remove("content/post-8.md")
        }),
    ];
    for (after, edit) in edits {
        edit(&site);
        let round = session.rebuild();
        site.same_as(&drafts, &session, &round, after);
    }
}

#[test]
fn a_session_asked_for_no_drafts_leaves_what_a_build_leaves() {
    let site = with_drafts("no-drafts");
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    assert!(!session.folder().join("post-7").exists());
    assert!(!session.folder().join("post-8").exists());
    assert!(round.noted.iter().all(|noted| !noted.draft));
    site.same(&session, &round, "the first build");
    site.write(
        "content/post-7.md",
        &post("Post 7", "2026-01-07", "two", "Published."),
    );
    let round = session.rebuild();
    assert!(session.folder().join("post-7/index.html").is_file());
    site.same(&session, &round, "a draft published");
}

#[test]
fn a_still_fetched_and_a_still_found_absent() {
    let site = Site::new("stills");
    site.write(
        "snippets/youtube.html",
        "{# takes address #}\n{% set video = page.youtube(address) %}\n\
         {% if video.still %}<img src=\"{{ video.still.src }}\">{% endif %}\n",
    );
    site.write(
        "content/post-7.md",
        &post(
            "Post 7",
            "2026-01-07",
            "one",
            "Post 7 shows two videos.\n\n@youtube(\"abcdefghijk\")\n\n@youtube(\"bcdefghijkl\")",
        ),
    );
    let record = |lines: &str| format!("statac fetched 1\n{lines}");
    let built = through(
        &site,
        &[
            ("a still fetched", &|site| {
                site.put("fetched/stills/youtube-abcdefghijk.jpg", &jpeg(48, 32));
                site.write(
                    "fetched/record",
                    &record("stills/youtube-abcdefghijk.jpg\t0123456789abcdef\t2026-10-04\n"),
                );
            }),
            ("a still found absent", &|site| {
                site.write(
                    "fetched/record",
                    &record(
                        "stills/youtube-abcdefghijk.jpg\t0123456789abcdef\t2026-10-04\n\
                         stills/youtube-bcdefghijkl.jpg\tabsent\t2026-10-04\n",
                    ),
                );
            }),
            ("the line saying absent deleted", &|site| {
                site.write(
                    "fetched/record",
                    &record("stills/youtube-abcdefghijk.jpg\t0123456789abcdef\t2026-10-04\n"),
                );
            }),
            ("a still deleted", &|site| {
                site.remove("fetched/stills/youtube-abcdefghijk.jpg")
            }),
        ],
    );
    // Each reaches only what shows the post's writing.
    for (built, outputs) in &built[1..] {
        assert!(*built > 0 && built < outputs, "{built:?}");
    }
}

#[test]
fn a_mention_written_changed_and_removed_rebuilds_only_the_pages_that_read_theirs() {
    let site = Site::new("mentions");
    site.write("statac.yaml", &format!("{SETTINGS}webmentions: true\n"));
    site.write(
        "templates/mentioned.html",
        &format!(
            "{BASE}{PAGE}{{% for m in page.webmentions.said %}}{{{{ m.kind }}}} \
             {{% if m.words %}}{{{{ m.words }}}}{{% endif %}}\
             {{% if m.face %}}<img src=\"{{{{ m.face.src }}}}\" alt=\"\">{{% endif %}}\
             {{% endfor %}}\n"
        ),
    );
    site.write(
        "content/post-7.md",
        "---\ntitle: Post 7\ndate: 2026-01-07\nlayout: mentioned\n---\nPost 7.\n",
    );
    let mention = |target: &str, more: &str| {
        format!(
            "kind: \"replied\"\nurl: \"https://a.example/post-1/\"\n\
             target: \"https://example.org/{target}/\"\nfetched: \"2026-10-04\"\n{more}"
        )
    };
    let face = "0123456789abcdef.jpg";
    let built = through(
        &site,
        &[
            ("a mention of a page that reads its own", &|site| {
                site.write(
                    "fetched/mentions/1.yaml",
                    &mention("post-7", "words: \"Words.\"\n"),
                );
            }),
            ("a mention of a page that never asks", &|site| {
                site.write("fetched/mentions/2.yaml", &mention("post-1", ""));
            }),
            // As any picture added does, it reaches what looks for
            // pictures by name.
            ("a face fetched", &|site| {
                site.put(&format!("fetched/faces/{face}"), &jpeg(48, 48));
            }),
            ("a mention's words changed, and its face", &|site| {
                site.write(
                    "fetched/mentions/1.yaml",
                    &mention(
                        "post-7",
                        &format!("face: \"{face}\"\nwords: \"Other words.\"\n"),
                    ),
                );
            }),
            ("a mention of no page", &|site| {
                site.write("fetched/mentions/3.yaml", &mention("post-9", ""));
            }),
            ("a mention removed", &|site| {
                site.remove("fetched/mentions/1.yaml")
            }),
        ],
    );
    let (face, outputs) = built[3];
    assert!(face > 0 && face < outputs, "{built:?}");
    for (at, expected) in [(1, 1), (2, 0), (4, 1), (5, 0), (6, 1)] {
        assert_eq!(built[at].0, expected, "{at}: {built:?}");
    }
}

#[test]
fn a_new_day_builds_what_read_the_day() {
    let site = Site::new("midnight");
    site.write(
        "content/post-5.md",
        "---\ntitle: Post 5\ndate: 2026-01-05\nlayout: dated\n---\nPost 5.\n",
    );
    site.write(
        "templates/dated.html",
        "{{ site.today }} {{ page.title }}\n",
    );
    // Waiting on the first day, and published on the second.
    site.write(
        "content/post-7.md",
        &post("Post 7", "2026-10-05", "one", "Post 7."),
    );
    let day = |day: &str| Options {
        today: statac::date::read(day).ok(),
        ..site.options()
    };
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..day("2026-10-04")
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    site.same_as(&day("2026-10-04"), &session, &round, "the first day");
    let folder = session.folder().to_path_buf();
    let read = |path: &str| fs::read_to_string(folder.join(path)).unwrap();
    assert_eq!(read("post-5/index.html"), "4 October 2026 Post 5\n");
    assert!(!session.folder().join("post-7").exists());

    let round = session.rebuild();
    assert_eq!(round.built, 0, "the same day");

    session.set_today(statac::date::read("2026-10-05").ok());
    let round = session.rebuild();
    assert_eq!(read("post-5/index.html"), "5 October 2026 Post 5\n");
    assert!(session.folder().join("post-7/index.html").is_file());
    assert!(
        round.built > 0 && round.built < round.outputs,
        "{} of {}",
        round.built,
        round.outputs
    );
    site.same_as(&day("2026-10-05"), &session, &round, "the next day");
}

/// The files a round gave an `id:` line.
fn ids(round: &Round) -> Vec<&str> {
    round.stamps.ids.iter().map(String::as_str).collect()
}

#[test]
fn an_id_is_written_once_for_a_post_new_since_the_site_was_published() {
    let site = Site::new("ids");
    site.write("statac.yaml", &format!("{SETTINGS}write_feed_ids: true\n"));
    let text = |path: &str| fs::read_to_string(site.root().join(path)).unwrap();
    let first = text("content/post-1.md");
    // Nothing kept of the feeds yet: which posts are new can't be told.
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    assert!(round.stamps.unrecorded && ids(&round).is_empty());
    assert_eq!(text("content/post-1.md"), first);
    site.same(&session, &round, "the first build");

    // Published, and a post added: only the new one is given an id.
    assert!(build::run(&site.options()).published.is_some());
    site.write(
        "content/post-7.md",
        "---\r\ntitle: Post 7\r\ndate: 2026-01-07\r\n---\r\nPost 7.\r\n",
    );
    site.write(
        "content/2026-01-08-post-8.md",
        "# Post 8\n\nNo front matter.\n",
    );
    site.write(
        "content/post-9.md",
        "---\ntitle: Post 9\ndate: 2026-01-09\ndraft: true\n---\nA draft.\n",
    );
    let round = session.rebuild();
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    assert!(!round.stamps.unrecorded);
    assert_eq!(
        ids(&round),
        ["content/2026-01-08-post-8.md", "content/post-7.md"]
    );
    let seventh = text("content/post-7.md");
    let id = seventh
        .strip_prefix("---\r\ntitle: Post 7\r\ndate: 2026-01-07\r\nid: ")
        .and_then(|rest| rest.strip_suffix("\r\n---\r\nPost 7.\r\n"))
        .unwrap_or_else(|| panic!("{seventh:?}"));
    assert_eq!(id.len(), 36, "{id}");
    let eighth = text("content/2026-01-08-post-8.md");
    assert!(
        eighth.starts_with("---\nid: ")
            && eighth.ends_with("\n---\n# Post 8\n\nNo front matter.\n"),
        "{eighth:?}"
    );
    assert_eq!(text("content/post-1.md"), first);
    assert!(!text("content/post-9.md").contains("id:"));
    site.same(&session, &round, "ids written");

    // Never twice.
    for _ in 0..3 {
        let round = session.rebuild();
        assert!(ids(&round).is_empty() && !round.stamps.wrote());
    }
    assert_eq!(text("content/post-7.md"), seventh);

    // The feed publishes the post under the id written.
    assert!(build::run(&site.options()).published.is_some());
    let round = session.rebuild();
    assert!(ids(&round).is_empty());
    assert_eq!(text("content/post-7.md"), seventh);
}

#[test]
fn a_post_that_moved_is_given_the_id_it_had_and_nothing_is_said_of_it() {
    let site = Site::new("ids-moved");
    assert!(build::run(&site.options()).published.is_some());
    let moved =
        "---\ntitle: Post 2\ndate: 2026-01-02\ntags: [one]\npermalink: /second/\n---\nPost 2.\n";
    site.write("content/post-2.md", moved);
    // Left off, the move is told and nothing is written.
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    let said = site.said(&round.outcome);
    assert!(said.contains("Feed id changed"), "{said}");
    assert!(ids(&round).is_empty());

    site.write("statac.yaml", &format!("{SETTINGS}write_feed_ids: true\n"));
    let round = session.rebuild();
    let said = site.said(&round.outcome);
    assert!(!round.outcome.stopped(), "{said}");
    assert_eq!(ids(&round), ["content/post-2.md"]);
    assert!(!said.contains("Feed id changed"), "{said}");
    let text = fs::read_to_string(site.root().join("content/post-2.md")).unwrap();
    let id = text
        .strip_prefix(
            "---\ntitle: Post 2\ndate: 2026-01-02\ntags: [one]\npermalink: /second/\nid: ",
        )
        .and_then(|rest| rest.strip_suffix("\n---\nPost 2.\n"))
        .unwrap_or_else(|| panic!("{text:?}"));
    let feed = fs::read_to_string(site.top.join("fresh/feed.xml")).unwrap();
    assert!(!feed.contains("/second/"), "the feed as last published");
    site.same(&session, &round, "an id for a post that moved");
    let _ = id;
    assert!(ids(&session.rebuild()).is_empty());
}

/// What a round stamped: each file, its field and the date written.
fn stamped(round: &Round) -> Vec<(&str, &str, &str)> {
    let stamped = round.stamps.stamped.iter();
    stamped
        .map(|one| (one.file.as_str(), one.field.as_str(), one.written.as_str()))
        .collect()
}

#[test]
fn a_stamp_is_written_before_the_site_is_built() {
    let site = Site::new("stamp");
    let text = |path: &str| fs::read_to_string(site.root().join(path)).unwrap();
    // One waiting as the session starts. The site's day is Sunday 4
    // October 2026.
    site.write(
        "content/post-7.md",
        "---\ntitle: Post 7\ndate: =yesterday\ntags: [one]\n---\nPost 7 says this.\n",
    );
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    assert_eq!(
        stamped(&round),
        [("content/post-7.md", "date", "2026-10-03")]
    );
    assert_eq!(
        text("content/post-7.md"),
        "---\ntitle: Post 7\ndate: 2026-10-03\ntags: [one]\n---\nPost 7 says this.\n"
    );
    site.same(&session, &round, "a stamp as the session starts");

    // Two saved in one file, one of them the present minute, which a day
    // fixed for the run puts at its midnight.
    site.write(
        "content/post-8.md",
        "---\r\ntitle: Post 8\r\ndate: \"=today 09:30\"\r\nupdated: =now\r\n---\r\nPost 8.\r\n",
    );
    let round = session.rebuild();
    assert_eq!(
        stamped(&round),
        [
            ("content/post-8.md", "date", "2026-10-04 09:30"),
            ("content/post-8.md", "updated", "2026-10-04 00:00")
        ]
    );
    assert_eq!(
        text("content/post-8.md"),
        "---\r\ntitle: Post 8\r\ndate: 2026-10-04 09:30\r\nupdated: 2026-10-04 00:00\r\n---\r\n\
         Post 8.\r\n"
    );
    assert!(round.written.contains(&"post-8/index.html".to_owned()));
    site.same(&session, &round, "two stamps saved");

    // With none waiting, nothing is written and nothing built.
    let round = session.rebuild();
    assert!(!round.stamps.wrote() && stamped(&round).is_empty());
    assert_eq!(round.built, 0);
    site.same(&session, &round, "no stamp");

    // A post stamped with a day to come waits for it, as one dated by
    // hand does.
    site.write(
        "content/post-9.md",
        "---\ntitle: Post 9\ndate: =monday\n---\nPost 9.\n",
    );
    let round = session.rebuild();
    assert_eq!(
        stamped(&round),
        [("content/post-9.md", "date", "2026-10-05")]
    );
    assert!(!session.folder().join("post-9").exists());
    site.same(&session, &round, "a stamp for a day to come");

    // A value meant as a stamp and not written as one stops the round,
    // and is left as written.
    let misspelt = "---\ntitle: Post 9\ndate: =Monday\n---\nPost 9.\n";
    site.write("content/post-9.md", misspelt);
    let round = session.rebuild();
    assert!(round.outcome.stopped() && stamped(&round).is_empty());
    assert_eq!(text("content/post-9.md"), misspelt);
    site.same(&session, &round, "a stamp misspelt");
}

#[test]
fn a_stamp_waits_while_the_settings_have_an_error() {
    let site = Site::new("unsettled");
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    site.same(&session, &round, "the first build");
    let waiting = "---\ntitle: Post 7\ndate: =today\n---\nPost 7.\n";
    site.write("content/post-7.md", waiting);
    site.write("statac.yaml", &format!("{SETTINGS}title: Site again\n"));
    let round = session.rebuild();
    assert!(round.outcome.stopped());
    assert!(round.stamps.unsettled && !round.stamps.wrote());
    let text = fs::read_to_string(site.root().join("content/post-7.md")).unwrap();
    assert_eq!(text, waiting);
    site.same(&session, &round, "a stamp and an error in the settings");

    site.write("statac.yaml", SETTINGS);
    let round = session.rebuild();
    assert!(!round.outcome.stopped(), "{}", site.said(&round.outcome));
    assert!(!round.stamps.unsettled);
    assert_eq!(
        stamped(&round),
        [("content/post-7.md", "date", "2026-10-04")]
    );
    site.same(&session, &round, "the settings mended");
}

#[cfg(unix)]
#[test]
fn a_stamp_in_a_page_kept_as_a_link_is_told_and_left() {
    let site = Site::new("stamp-link");
    let (mut session, round) = Session::start(&Options {
        out: None,
        ..site.options()
    });
    site.same(&session, &round, "the first build");
    let waiting = "---\ntitle: Post 7\ndate: =today\n---\nPost 7.\n";
    site.write("content/.kept/post-7.md", waiting);
    std::os::unix::fs::symlink(
        site.root().join("content/.kept/post-7.md"),
        site.root().join("content/post-7.md"),
    )
    .unwrap();
    let round = session.rebuild();
    assert!(round.outcome.stopped() && !round.stamps.wrote());
    let refused: Vec<String> = (round.stamps.refused.iter())
        .map(|problem| problem.headline())
        .collect();
    assert_eq!(refused, ["Can't write `content/post-7.md`: it is a link"]);
    let text = fs::read_to_string(site.root().join("content/.kept/post-7.md")).unwrap();
    assert_eq!(text, waiting);
    site.same(&session, &round, "a stamp behind a link");
}

#[test]
fn pages_chosen_from_every_page_by_a_field_follow_the_field() {
    let site = Site::new("chosen");
    // Each page lists the pages that ask to be in the menu, those of its
    // own language, counts those that don't and those of another layout
    // than the home page's, and lists those ranked past the first.
    let menu = "<menu>{% for entry in site.pages | selectattr('menu') \
                | selectattr('language', 'eq', page.language) %}\
                <a href=\"{{ entry.url }}\">{{ entry.title }}</a>{% endfor %}\
                {{ site.pages | rejectattr('menu') | list | length }} \
                {{ site.pages | rejectattr('layout', 'eq', 'home') | list | length }} \
                {% for entry in site.pages | selectattr('rank', '>', 1) %}{{ entry.url }} {% endfor %}\
                </menu>\n";
    site.write("templates/page.html", &format!("{BASE}{menu}{PAGE}"));
    site.write("templates/home.html", &format!("{BASE}{menu}{HOME}"));
    let page = |more: &str| format!("---\ntitle: Page 1\n{more}---\nA page.\n");
    let built = through(
        &site,
        &[
            ("a page", &|site| site.write("content/page-1.md", &page(""))),
            ("into the menu", &|site| {
                site.write("content/page-1.md", &page("menu: true\nrank: 2\n"))
            }),
            ("another in another language", &|site| {
                site.write("content/de/page-2.md", &page("menu: true\n"))
            }),
            ("out of the menu", &|site| {
                site.write("content/page-1.md", &page("menu: false\nrank: 1\n"))
            }),
            ("a layout of its own", &|site| {
                site.write("content/page-1.md", &page("layout: home\n"))
            }),
            ("the page deleted", &|site| {
                site.remove("content/de/page-2.md")
            }),
        ],
    );
    // A page joining the menu is shown on every page.
    let outputs = built[2].1;
    assert!(built[2].0 * 2 > outputs, "{built:?}");
}

#[test]
fn pages_chosen_and_put_in_order_once_for_the_site_follow_what_they_turn_on() {
    // Each asked on every page of a site of its own, and shown so that
    // nothing else the page reads turns on the pages left out: by a tag
    // and a folder where the page's language has them, by a field, a
    // title and an address, and in three orders.
    let asked = [
        "{% if page.language != 'de' %}{{ site.posts | tagged('one') | length }}{% endif %}",
        "{% if page.language != 'de' %}{{ site.pages | in('blog') | length }}{% endif %}",
        "{{ site.pages | where('kind', 'long') | length }}",
        "{{ site.posts | selectattr('title', 'eq', 'Page 2') | length }}",
        "{{ site.pages | selectattr('url', 'startingwith', '/blog/') | length }}",
        "{{ (site.posts | sort(attribute='title') | first).url }}",
        "{{ (site.pages | newest_first | first).url }}",
        "{{ site.posts | selectattr('kind') | sort(attribute='kind', reverse=true) | length }}",
    ];
    let page = |title: &str, more: &str| format!("---\ntitle: {title}\n{more}---\nA page.\n");
    for (at, asked) in asked.iter().enumerate() {
        let site = Site::new(&format!("chosen-once-{at}"));
        let asked = format!("<p>{asked}</p>\n");
        site.write("templates/page.html", &format!("{BASE}{asked}{PAGE}"));
        site.write("templates/post.html", &format!("{BASE}{asked}{PAGE}"));
        site.write("templates/home.html", &format!("{BASE}{asked}{HOME}"));
        let more = "date: 2026-07-01\nkind: brief\n";
        site.write("content/blog/post-7.md", &page("Page 7", more));
        through(
            &site,
            &[
                ("a post", &|site| {
                    site.write(
                        "content/blog/post-8.md",
                        &page("Page 1", "date: 2026-10-01\n"),
                    )
                }),
                ("a tag", &|site| {
                    let more = "date: 2026-10-01\ntags: [one]\n";
                    site.write("content/blog/post-8.md", &page("Page 1", more))
                }),
                ("a field", &|site| {
                    let more = "date: 2026-10-01\ntags: [one]\nkind: long\n";
                    site.write("content/blog/post-8.md", &page("Page 1", more))
                }),
                ("another title", &|site| {
                    let more = "date: 2026-10-01\ntags: [one]\nkind: long\n";
                    site.write("content/blog/post-8.md", &page("Page 2", more))
                }),
                ("another, earlier", &|site| {
                    let more = "date: 2026-09-01\ntags: [one]\nkind: brief\n";
                    site.write("content/blog/post-9.md", &page("Page 0", more))
                }),
                ("another day", &|site| {
                    let more = "date: 2026-08-01\nkind: long\n";
                    site.write("content/blog/post-8.md", &page("Page 2", more))
                }),
                // A post shown nowhere so far comes first in each order.
                ("an earlier post named the first", &|site| {
                    let more = "date: 2026-07-01\nkind: wide\n";
                    site.write("content/blog/post-7.md", &page("A page", more))
                }),
                ("and made the newest", &|site| {
                    let more = "date: 2026-10-02\nkind: wide\n";
                    site.write("content/blog/post-7.md", &page("A page", more))
                }),
                ("a post deleted", &|site| {
                    site.remove("content/blog/post-9.md")
                }),
            ],
        );
    }
}

#[test]
fn a_page_that_reads_fields_past_counting_follows_every_one() {
    let site = Site::new("many-fields");
    // More reads of fields than an output holds one by one, then a field
    // of every page.
    let many = "{% for n in range(1100) %}{% if page.title %}{% endif %}{% endfor %}\
                <ul>{% for entry in site.pages if entry.shown is defined %}\
                <li>{{ entry.url }} {{ entry.shown }}</li>{% endfor %}</ul>\n";
    site.write("templates/page.html", &format!("{BASE}{many}{PAGE}"));
    site.write("templates/home.html", &format!("{BASE}{many}{HOME}"));
    let page = |more: &str| format!("---\ntitle: Page 1\n{more}---\nA page.\n");
    through(
        &site,
        &[
            ("a page", &|site| site.write("content/page-1.md", &page(""))),
            ("a field", &|site| {
                site.write("content/page-1.md", &page("shown: one\n"))
            }),
            ("the field changed", &|site| {
                site.write("content/page-1.md", &page("shown: two\n"))
            }),
            ("another page", &|site| {
                site.write("content/page-2.md", &page("shown: three\n"))
            }),
            ("the page deleted", &|site| site.remove("content/page-1.md")),
        ],
    );
}
