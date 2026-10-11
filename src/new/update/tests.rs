use super::*;

use std::path::PathBuf;

use crate::album::Progress;
use crate::build::{self, Mode, Options};
use crate::report::Report;

const BASE_1: &[u8] =
    b"<!doctype html>\n<title>{{ page.title }}</title>\n<main>\n{{ body }}\n</main>\n";
const BASE_2: &[u8] =
    b"<!doctype html>\n<meta charset=\"utf-8\">\n<title>{{ page.title }}</title>\n\
<main>\n{{ body }}\n</main>\n";
const BASE_3: &[u8] =
    b"<!doctype html>\n<meta charset=\"utf-8\">\n<title>{{ page.title }}</title>\n\
<main id=\"content\">\n{{ body }}\n</main>\n";
const NOTE_1: &[u8] = b"<aside>\n{{ body }}\n</aside>\n";
const NOTE_2: &[u8] = b"<aside class=\"note\">\n{{ body }}\n</aside>\n";
const VIDEO_1: &[u8] = b"<video src=\"{{ src }}\" controls></video>\n";
const PNG_1: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x01";
const PNG_2: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x02";
const HOME_W1: &[u8] = b"{% extends \"base.html\" %}\n<ul class=\"posts\">\n</ul>\n";
const HOME_W2: &[u8] =
    b"{% extends \"base.html\" %}\n<ul class=\"posts\">\n<li>{{ post.title }}</li>\n</ul>\n";
const JPG: &[u8] = b"\xff\xd8\xff\xe0\x00\x10JFIF\x00";
const OLD: &[u8] = b"<p>Only a past starter had this file.</p>\n";
const CARD_1: &[u8] = b"background: background.png\n";
const CARD_2: &[u8] = b"background: background.png\npadding: 80\n";

const CARD: &str = "templates/og.yaml";

const PHOTO: &str = "content/posts/first-post/photo-1.jpg";

const FILES: &[File] = &[
    File {
        path: "snippets/note.html",
        bytes: NOTE_2,
    },
    File {
        path: "snippets/video.html",
        bytes: VIDEO_1,
    },
    File {
        path: "templates/base.html",
        bytes: BASE_3,
    },
    File {
        path: "templates/og/background.png",
        bytes: PNG_2,
    },
    File {
        path: PHOTO,
        bytes: JPG,
    },
    File {
        path: "templates/home.html",
        bytes: HOME_W2,
    },
];

const PAST: &[Past] = &[
    Past {
        path: "templates/base.html",
        bytes: BASE_1,
    },
    Past {
        path: "snippets/note.html",
        bytes: NOTE_1,
    },
    Past {
        path: "templates/og/background.png",
        bytes: PNG_1,
    },
    Past {
        path: "templates/old.html",
        bytes: OLD,
    },
    Past {
        path: CARD,
        bytes: CARD_1,
    },
    Past {
        path: "templates/home.html",
        bytes: HOME_W1,
    },
    Past {
        path: "templates/base.html",
        bytes: BASE_2,
    },
];

const WORDS: &[(&str, &str)] = &[
    ("home", "Home"),
    ("newer_posts", "Newer posts"),
    ("older_posts", "Older posts"),
];

const WORDS_EN: &[u8] =
    b"en:\n  home: Home\n  newer_posts: Newer posts\n  older_posts: Older posts\n";

const STARTER: Starter = Starter {
    files: FILES,
    cards: &[File {
        path: CARD,
        bytes: CARD_2,
    }],
    past: PAST,
    words: WORDS,
};

/// A starter of words alone.
const WORDS_ONLY: Starter = Starter {
    files: &[],
    cards: &[],
    past: &[],
    words: WORDS,
};

/// A folder of its own for each test, named `label`, in the system's
/// temporary folder, with nothing there yet.
fn scratch(label: &str) -> PathBuf {
    let tests = std::env::temp_dir().join(format!("statac-update-{}", std::process::id()));
    let path = tests.join(label);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn put(folder: &Path, path: &str, bytes: &[u8]) {
    let path = platform::join(folder, path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn read(folder: &Path, path: &str) -> Vec<u8> {
    std::fs::read(platform::join(folder, path)).unwrap()
}

/// A site in English holding `files`, by their paths.
fn site(label: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let folder = scratch(label);
    put(&folder, "statac.yaml", b"language: en\n");
    for (path, bytes) in files {
        put(&folder, path, bytes);
    }
    folder
}

/// A site made by [`STARTER`]'s newest files, with every word, and with
/// `files` in place of the newest where they say.
fn made_site(label: &str, files: &[(&str, &[u8])]) -> PathBuf {
    let folder = site(
        label,
        &[
            (WORDS_FILE, WORDS_EN),
            (PHOTO, JPG),
            ("snippets/note.html", NOTE_2),
            ("snippets/video.html", VIDEO_1),
            ("templates/base.html", BASE_3),
            ("templates/og/background.png", PNG_2),
            ("templates/home.html", HOME_W2),
        ],
    );
    for (path, bytes) in files {
        put(&folder, path, bytes);
    }
    folder
}

fn concat(a: &[u8], b: &[u8]) -> Vec<u8> {
    [a, b].concat()
}

/// What `statac new --update` says of the site in `folder`, held to
/// `starter`, and its exit code.
fn updated_by(folder: &Path, update: Update, starter: &Starter) -> (String, u8) {
    let mut out = Vec::new();
    let code = run_holding(folder, update, starter, &mut out, false);
    (String::from_utf8(out).unwrap(), code)
}

fn updated(folder: &Path, update: Update) -> (String, u8) {
    updated_by(folder, update, &STARTER)
}

fn surveyed(folder: &Path) -> Vec<(&'static str, Is)> {
    let root = std::fs::canonicalize(folder).unwrap();
    let found = survey(&root, &STARTER);
    found
        .into_iter()
        .map(|found| (found.path, found.is))
        .collect()
}

#[test]
fn a_starter_file_or_a_folder_on_the_way_to_one_is_reached() {
    for path in [
        "templates/base.html",
        "templates",
        "templates/og",
        PHOTO,
        CARD,
    ] {
        assert!(STARTER.reaches(path), "{path}");
    }
    for path in [
        "templates/old.html",
        "templ",
        "templates/base",
        "content/index.md",
    ] {
        assert!(!STARTER.reaches(path), "{path}");
    }
}

#[test]
fn every_state_of_a_file_is_told() {
    let mine = concat(NOTE_2, b"<p>Mine.</p>\n");
    let folder = made_site(
        "states",
        &[
            ("snippets/note.html", &mine),
            ("templates/base.html", BASE_2),
            ("templates/old.html", OLD),
        ],
    );
    std::fs::remove_file(folder.join("snippets/video.html")).unwrap();
    assert_eq!(
        surveyed(&folder),
        [
            (PHOTO, Is::Newest),
            ("snippets/note.html", Is::Yours { new: None }),
            ("snippets/video.html", Is::NotHere { current: VIDEO_1 }),
            (
                "templates/base.html",
                Is::Older {
                    read: BASE_2.to_vec(),
                    current: BASE_3
                }
            ),
            ("templates/home.html", Is::Newest),
            (CARD, Is::NotHere { current: CARD_2 }),
            ("templates/og/background.png", Is::Newest),
        ]
    );
}

#[test]
fn an_older_file_is_replaced_and_nothing_else_is_written() {
    let mine = concat(NOTE_2, b"<p>Mine.</p>\n");
    let folder = made_site(
        "older",
        &[
            ("snippets/note.html", &mine),
            ("templates/base.html", BASE_1),
            ("templates/old.html", OLD),
        ],
    );
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, "Updated `templates/base.html`.\nUpdated 1 file.\n");
    assert_eq!(read(&folder, "templates/base.html"), BASE_3);
    assert_eq!(read(&folder, "snippets/note.html"), mine);
    assert_eq!(read(&folder, "templates/old.html"), OLD);
    assert_eq!(read(&folder, WORDS_FILE), WORDS_EN);
}

#[test]
fn what_is_new_since_the_version_nearest_a_file_of_the_authors_is_shown() {
    let base = concat(BASE_2, b"<footer>{{ site.title }}</footer>\n");
    let note = concat(NOTE_1, b"<p>Mine.</p>\n");
    let folder = made_site(
        "yours",
        &[
            ("templates/base.html", &base),
            ("snippets/note.html", &note),
        ],
    );
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "`snippets/note.html` is yours and wasn't touched. The starter's file has \
         changed since the version nearest to yours:\n\
         \x20 @@ line 1\n\
         \x20 -<aside>\n\
         \x20 +<aside class=\"note\">\n\
         \x20  {{ body }}\n\
         \x20  </aside>\n\
         `templates/base.html` is yours and wasn't touched. The starter's file has \
         changed since the version nearest to yours:\n\
         \x20 @@ line 1\n\
         \x20  <!doctype html>\n\
         \x20  <meta charset=\"utf-8\">\n\
         \x20  <title>{{ page.title }}</title>\n\
         \x20 -<main>\n\
         \x20 +<main id=\"content\">\n\
         \x20  {{ body }}\n\
         \x20  </main>\n\
         2 of yours have something new, shown above.\n"
    );
    assert_eq!(read(&folder, "templates/base.html"), base);
    assert_eq!(read(&folder, "snippets/note.html"), note);
}

#[test]
fn a_file_nearest_the_newest_or_as_near_it_as_a_past_one_has_nothing_new() {
    let nearest_newest = concat(BASE_3, b"<footer>{{ site.title }}</footer>\n");
    let folder = made_site(
        "nearest-newest",
        &[("templates/base.html", &nearest_newest)],
    );
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, format!("{}\n", wording::NOTHING_TO_UPDATE));

    // Two lines from the second version and from the third.
    let between = String::from_utf8(BASE_2.to_vec())
        .unwrap()
        .replace("<main>", "<main class=\"wide\">");
    let folder = made_site("as-near", &[("templates/base.html", between.as_bytes())]);
    assert_eq!(
        surveyed(&folder)[3],
        ("templates/base.html", Is::Yours { new: None })
    );
    assert_eq!(read(&folder, "templates/base.html"), between.as_bytes());
}

#[test]
fn missing_writes_the_files_the_site_doesnt_have() {
    let folder = site("missing", &[(WORDS_FILE, WORDS_EN)]);
    let (said, code) = updated(&folder, Update { missing: true });
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "Wrote `snippets/note.html`.\n\
         Wrote `snippets/video.html`.\n\
         Wrote `templates/base.html`.\n\
         Wrote `templates/home.html`.\n\
         Wrote `templates/og/background.png`.\n\
         Updated 5 files.\n"
    );
    assert_eq!(read(&folder, "templates/home.html"), HOME_W2);
    assert!(!platform::join(&folder, PHOTO).exists());
    assert!(!platform::join(&folder, CARD).exists());
}

#[test]
fn a_sample_pages_file_is_never_missed_where_it_isnt_here() {
    for path in [
        "content/posts/index.md",
        "content/posts/first-post/photo-1.jpg",
        "content/posts/what-a-post-can-hold/example.css",
        "content/notes/index.md",
        "content/photos/index.md",
        "content/photos/photo-1/photo-1.jpg.yaml",
        "static/favicon.ico",
    ] {
        assert!(starter::is_a_samples(path), "{path}");
    }
    for path in [
        "content/404.md",
        "content/robots.md",
        "content/sitemap.md",
        "content/index.md",
        "templates/photos.html",
        "snippets/short.html",
    ] {
        assert!(!starter::is_a_samples(path), "{path}");
    }
    for file in starter::FILES {
        let in_a_folder = ["content/posts/", "content/notes/", "content/photos/"]
            .iter()
            .any(|folder| file.path.starts_with(folder))
            || file.path == "static/favicon.ico";
        assert_eq!(
            starter::is_a_samples(file.path),
            in_a_folder,
            "{}",
            file.path
        );
    }

    let folder = scratch("no-photos").join("site-1");
    let answers = super::super::Answers {
        title: "Site 1".into(),
        address: Some("https://example.org".into()),
        languages: vec!["en".into()],
        date_format: "2025-12-31".into(),
    };
    let today = crate::date::Date::day_only(2026, 10, 6);
    super::super::make(
        crate::problem::Maker::New,
        &folder,
        &answers,
        None,
        today,
        false,
    )
    .unwrap();
    std::fs::remove_dir_all(platform::join(&folder, "content/photos")).unwrap();
    let before = held(&folder);
    for update in [Update::default(), Update { missing: true }] {
        let (said, code) = updated_by(&folder, update, &Starter::OWN);
        assert_eq!(code, 0, "{said}");
        assert_eq!(said, format!("{}\n", wording::NOTHING_TO_UPDATE));
        assert!(!platform::join(&folder, "content/photos").exists());
        assert!(held(&folder) == before);
    }

    // A sample page's file that is here is held to the starter as any
    // other is, and one that isn't a sample page's is still missed.
    let page = "content/notes/index.md";
    assert!(surveyed_own(&folder).contains(&(page, Is::Newest)));
    std::fs::remove_file(platform::join(&folder, "snippets/thumb.html")).unwrap();
    let (said, _) = updated_by(&folder, Update::default(), &Starter::OWN);
    assert_eq!(
        said,
        "1 starter file isn't here: `snippets/thumb.html`. To write it, run \
         `statac new --update --missing`.\n"
    );
}

/// What a site holds at each of this Statac's starter files.
fn surveyed_own(folder: &Path) -> Vec<(&'static str, Is)> {
    let root = std::fs::canonicalize(folder).unwrap();
    (survey(&root, &Starter::OWN).into_iter())
        .map(|found| (found.path, found.is))
        .collect()
}

#[test]
fn a_cards_file_is_brought_up_to_date_where_it_is_here_and_never_missed_where_it_isnt() {
    let folder = made_site("card-not-here", &[]);
    assert_eq!(newer_starter(&folder, &STARTER), None);
    for update in [Update::default(), Update { missing: true }] {
        let (said, code) = updated(&folder, update);
        assert_eq!(code, 0, "{said}");
        assert_eq!(said, format!("{}\n", wording::NOTHING_TO_UPDATE));
        assert!(!platform::join(&folder, CARD).exists());
    }

    let folder = made_site("card-older", &[(CARD, CARD_1)]);
    let told = newer_starter(&folder, &STARTER);
    assert_eq!(
        told.map(|problem| problem.kind),
        Some(Kind::NewerStarter { files: 1 })
    );
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, format!("Updated `{CARD}`.\nUpdated 1 file.\n"));
    assert_eq!(read(&folder, CARD), CARD_2);
}

#[test]
fn a_cards_file_not_here_is_missed_only_where_the_site_describes_its_card() {
    const FONT: &str = "templates/og/text.ttf";
    let starter = Starter {
        files: &[],
        cards: &[
            File {
                path: CARD,
                bytes: CARD_2,
            },
            File {
                path: FONT,
                bytes: b"font",
            },
        ],
        past: &[],
        words: &[],
    };
    let folder = site("card-file-undescribed", &[]);
    for update in [Update::default(), Update { missing: true }] {
        let (said, code) = updated_by(&folder, update, &starter);
        assert_eq!(code, 0, "{said}");
        assert_eq!(said, format!("{}\n", wording::NOTHING_TO_UPDATE));
        assert!(!platform::join(&folder, FONT).exists());
    }

    let folder = site("card-file-not-here", &[(CARD, b"font: mine.ttf\n")]);
    let (said, code) = updated_by(&folder, Update::default(), &starter);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "1 starter file isn't here: `templates/og/text.ttf`. To write it, run \
         `statac new --update --missing`.\n"
    );
    let (said, code) = updated_by(&folder, Update { missing: true }, &starter);
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, "Wrote `templates/og/text.ttf`.\nUpdated 1 file.\n");
    assert_eq!(read(&folder, FONT), b"font");
    assert_eq!(read(&folder, CARD), b"font: mine.ttf\n");
}

#[test]
fn a_font_to_bring_is_one_of_the_cards_not_here_with_every_character() {
    let bring = |folder: &Path, characters: &[char]| {
        let root = std::fs::canonicalize(folder).unwrap();
        font_to_bring(&root, &Starter::OWN, characters)
    };
    let folder = site("font-undescribed", &[]);
    assert_eq!(bring(&folder, &['\u{1f600}']), None);

    let folder = site("font-to-bring", &[(CARD, b"font: mine.ttf\n")]);
    assert_eq!(bring(&folder, &['\u{1f600}']), Some("NotoEmoji.ttf"));
    assert_eq!(bring(&folder, &['a', '1']), Some("NotoSans-Regular.ttf"));
    assert_eq!(bring(&folder, &['\u{1f600}', 'a']), None);
    assert_eq!(bring(&folder, &['\u{4e2d}']), None);
    put(&folder, "templates/og/NotoEmoji.ttf", b"mine");
    assert_eq!(bring(&folder, &['\u{1f600}']), None);
}

#[test]
fn starter_files_not_here_are_named_once_six_at_most() {
    let folder = made_site("not-here", &[]);
    for path in ["snippets/note.html", "snippets/video.html"] {
        std::fs::remove_file(platform::join(&folder, path)).unwrap();
    }
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "2 starter files aren't here: `snippets/note.html`, `snippets/video.html`. To write \
         them, run `statac new --update --missing`.\n"
    );

    const MANY: &[File] = &[
        File {
            path: "snippets/a.html",
            bytes: b"a\n",
        },
        File {
            path: "snippets/b.html",
            bytes: b"b\n",
        },
        File {
            path: "snippets/c.html",
            bytes: b"c\n",
        },
        File {
            path: "snippets/d.html",
            bytes: b"d\n",
        },
        File {
            path: "snippets/e.html",
            bytes: b"e\n",
        },
        File {
            path: "snippets/f.html",
            bytes: b"f\n",
        },
        File {
            path: "snippets/g.html",
            bytes: b"g\n",
        },
    ];
    let many = Starter {
        files: MANY,
        cards: &[],
        past: &[],
        words: &[],
    };
    let folder = site("seven-not-here", &[]);
    let (said, _) = updated_by(&folder, Update::default(), &many);
    assert_eq!(
        said,
        "7 starter files aren't here: `snippets/a.html`, `snippets/b.html`, `snippets/c.html`, \
         `snippets/d.html`, `snippets/e.html`, `snippets/f.html` and 1 more. To write them, run \
         `statac new --update --missing`.\n"
    );
    let folder = site("one-not-here", &[("snippets/a.html", b"a\n")]);
    let few = Starter {
        files: &MANY[..2],
        ..many
    };
    let (said, _) = updated_by(&folder, Update::default(), &few);
    assert_eq!(
        said,
        "1 starter file isn't here: `snippets/b.html`. To write it, run \
         `statac new --update --missing`.\n"
    );
}

#[test]
fn a_file_that_isnt_text_is_replaced_where_older_and_named_where_yours_and_the_starters_changed() {
    let folder = made_site("picture-older", &[("templates/og/background.png", PNG_1)]);
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "Updated `templates/og/background.png`.\nUpdated 1 file.\n"
    );
    assert_eq!(read(&folder, "templates/og/background.png"), PNG_2);

    // The starter's background has changed in a release, and its
    // photograph never has.
    let mine = b"\x89PNG\r\n\x1a\n\x00\x00\x00\x09";
    let my_photo = b"\xff\xd8\xff\xe0\x00\x10JFIF\x09";
    let folder = made_site(
        "picture-yours",
        &[("templates/og/background.png", mine), (PHOTO, my_photo)],
    );
    let states = surveyed(&folder);
    assert_eq!(states[0], (PHOTO, Is::NotText { changed: false }));
    assert_eq!(
        states[6],
        ("templates/og/background.png", Is::NotText { changed: true })
    );
    // Said on every run, since nothing is kept in the site.
    for _ in 0..2 {
        let (said, code) = updated(&folder, Update::default());
        assert_eq!(code, 0, "{said}");
        assert_eq!(
            said,
            "`templates/og/background.png` is yours; the starter has a newer one.\n"
        );
    }
    assert_eq!(read(&folder, "templates/og/background.png"), mine);
    assert_eq!(read(&folder, PHOTO), my_photo);
    assert_eq!(newer_starter(&folder, &STARTER), None);
}

#[test]
fn update_run_twice_does_nothing_the_second_time() {
    let folder = made_site("twice", &[("templates/base.html", BASE_2)]);
    std::fs::remove_file(folder.join("snippets/video.html")).unwrap();
    put(&folder, WORDS_FILE, b"en:\n  home: Home\n");
    let all = Update { missing: true };
    let (said, code) = updated(&folder, all);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "Updated `templates/base.html`.\n\
         Added 2 words to `data/words.yaml` under `en`: `newer_posts`, `older_posts`.\n\
         Wrote `snippets/video.html`.\n\
         Updated 3 files.\n"
    );
    let (said, code) = updated(&folder, all);
    assert_eq!(code, 0, "{said}");
    assert_eq!(said, format!("{}\n", wording::NOTHING_TO_UPDATE));
}

#[test]
fn a_folder_that_isnt_a_site_is_refused() {
    let folder = scratch("not-a-site");
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 1);
    let shown = folder.display();
    assert_eq!(
        said,
        format!(
            "error  Can't read `{shown}`: it has no `statac.yaml`, so it isn't a site\n\
             \x20 More: https://statac.dev/e/cant-read/\n"
        )
    );
    let (said, code) = updated(&folder.join("site-1"), Update::default());
    assert_eq!(code, 1);
    assert!(said.contains(": it doesn't exist\n"), "{said}");
}

#[cfg(unix)]
#[test]
fn a_file_with_a_second_name_isnt_replaced_and_the_run_exits_with_1() {
    let folder = made_site("second-name", &[("templates/base.html", BASE_2)]);
    let other = scratch("second-name-other").join("base.html");
    std::fs::hard_link(folder.join("templates/base.html"), &other).unwrap();
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 1, "{said}");
    assert!(
        said.starts_with("error  Can't write `templates/base.html`: "),
        "{said}"
    );
    assert_eq!(read(&folder, "templates/base.html"), BASE_2);
    assert_eq!(std::fs::read(&other).unwrap(), BASE_2);
}

#[cfg(unix)]
#[test]
fn a_link_in_place_of_a_file_or_a_folder_on_the_way_is_never_gone_through() {
    use std::os::unix::fs::symlink;
    let outside = scratch("link-outside");
    put(&outside, "base.html", BASE_2);
    put(&outside, "templates/base.html", BASE_1);
    put(&outside, "templates/home.html", HOME_W1);

    let folder = made_site("link-file", &[]);
    std::fs::remove_file(folder.join("templates/base.html")).unwrap();
    symlink(
        outside.join("base.html"),
        folder.join("templates/base.html"),
    )
    .unwrap();
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        said,
        "`templates/base.html` was left alone: it is a link, which Statac never goes through.\n"
    );
    assert_eq!(read(&outside, "base.html"), BASE_2);

    let folder = made_site("link-folder", &[]);
    std::fs::remove_dir_all(folder.join("templates")).unwrap();
    symlink(outside.join("templates"), folder.join("templates")).unwrap();
    let (said, code) = updated(&folder, Update::default());
    assert_eq!(code, 0, "{said}");
    for path in [
        "templates/base.html",
        "templates/home.html",
        "templates/og/background.png",
    ] {
        let line = format!(
            "`{path}` was left alone: `templates` is a link, which Statac never goes through.\n"
        );
        assert!(said.contains(&line), "{said}");
    }
    assert!(!said.contains("Updated"), "{said}");
    assert!(!said.contains("aren't here"), "{said}");
    assert_eq!(read(&outside, "templates/base.html"), BASE_1);
    assert_eq!(read(&outside, "templates/home.html"), HOME_W1);
    let missing = Update { missing: true };
    let (said, _) = updated(&folder, missing);
    assert!(!said.contains("Wrote"), "{said}");
    assert!(!outside.join("templates/og").exists());
}

/// What adding [`WORDS`] to `words` says and makes of it, on a site in
/// `languages`.
fn words_added_to(label: &str, languages: &str, words: &[u8]) -> (String, Vec<u8>) {
    let folder = site(label, &[(WORDS_FILE, words)]);
    put(
        &folder,
        "statac.yaml",
        format!("language: {languages}\n").as_bytes(),
    );
    let (said, code) = updated_by(&folder, Update::default(), &WORDS_ONLY);
    assert_eq!(code, 0, "{said}");
    (said, read(&folder, WORDS_FILE))
}

#[test]
fn a_word_missing_is_added_at_the_end_of_its_block() {
    let (said, written) = words_added_to(
        "one-word",
        "en",
        b"en:\n  home: Home\n  newer_posts: Newer posts\n",
    );
    assert_eq!(
        said,
        "Added 1 word to `data/words.yaml` under `en`: `older_posts`.\nUpdated 1 file.\n"
    );
    assert_eq!(written, WORDS_EN);
    let (said, written) = words_added_to("no-words-missing", "en", WORDS_EN);
    assert_eq!(said, format!("{}\n", wording::NOTHING_TO_UPDATE));
    assert_eq!(written, WORDS_EN);
}

#[test]
fn several_words_missing_in_two_languages_keep_every_other_byte() {
    let words = "# The words the pages show.\n\
                 \"en\":\n\
                 \x20 # In an order of the author's own.\n\
                 \x20 older_posts: Older posts\n\
                 \x20 home: Home   # the home page\n\
                 \n\
                 fr:\n\
                 \x20   home: Accueil\n";
    let (said, written) = words_added_to("two-languages", "[en, fr]", words.as_bytes());
    assert_eq!(
        said,
        "Added 1 word to `data/words.yaml` under `en`: `newer_posts`.\n\
         Added 2 words to `data/words.yaml` under `fr`: `newer_posts`, `older_posts`.\n\
         Updated 1 file.\n"
    );
    assert_eq!(
        String::from_utf8(written).unwrap(),
        "# The words the pages show.\n\
         \"en\":\n\
         \x20 # In an order of the author's own.\n\
         \x20 older_posts: Older posts\n\
         \x20 home: Home   # the home page\n\
         \x20 newer_posts: Newer posts\n\
         \n\
         fr:\n\
         \x20   home: Accueil\n\
         \x20   newer_posts: Newer posts\n\
         \x20   older_posts: Older posts\n"
    );
}

#[test]
fn words_are_added_in_the_files_own_line_breaks() {
    let (_, written) = words_added_to("crlf", "en", b"en:\r\n  home: Home\r\n");
    assert_eq!(
        written,
        b"en:\r\n  home: Home\r\n  newer_posts: Newer posts\r\n  older_posts: Older posts\r\n"
    );
    let (_, written) = words_added_to("no-last-break", "en", b"en:\n  home: Home");
    assert_eq!(
        written,
        b"en:\n  home: Home\n  newer_posts: Newer posts\n  older_posts: Older posts"
    );
}

#[test]
fn a_block_or_a_file_of_words_not_there_is_said_and_left() {
    let (said, written) = words_added_to("block-missing", "[en, fr]", WORDS_EN);
    assert_eq!(
        said,
        "No words were added under `fr`: `data/words.yaml` has no words under it.\n"
    );
    assert_eq!(written, WORDS_EN);
    let (said, written) = words_added_to("flow", "en", b"en: {home: Home}\n");
    assert_eq!(
        said,
        "No words were added under `en`: its words in `data/words.yaml` aren't one to a line.\n"
    );
    assert_eq!(written, b"en: {home: Home}\n");
    let (said, written) = words_added_to("unreadable", "en", b"en: [\n");
    assert_eq!(said, format!("{}\n", wording::WORDS_UNREADABLE));
    assert_eq!(written, b"en: [\n");

    let folder = site("words-missing", &[]);
    let (said, code) = updated_by(&folder, Update::default(), &WORDS_ONLY);
    assert_eq!(code, 0);
    assert_eq!(said, "No words were added: `data/words.yaml` isn't here.\n");
    assert!(!platform::join(&folder, WORDS_FILE).exists());
}

#[test]
fn the_note_counts_older_files_and_the_authors_with_something_new() {
    let note = concat(NOTE_1, b"<p>Mine.</p>\n");
    let near_newest = concat(BASE_3, b"<p>Mine.</p>\n");
    let folder = made_site(
        "note-counted",
        &[
            ("snippets/note.html", &note),
            ("templates/home.html", HOME_W1),
            ("templates/base.html", &near_newest),
        ],
    );
    std::fs::remove_file(folder.join("snippets/video.html")).unwrap();
    let told = newer_starter(&folder, &STARTER);
    assert_eq!(
        told.map(|problem| problem.kind),
        Some(Kind::NewerStarter { files: 2 })
    );
    let newest = made_site("note-newest", &[]);
    assert_eq!(newer_starter(&newest, &STARTER), None);
    let no_past = Starter {
        past: &[],
        ..STARTER
    };
    assert_eq!(newer_starter(&folder, &no_past), None);
    assert_eq!(newer_starter(&folder, &Starter::OWN), None);
}

const CSS_1: &[u8] = b"body { margin: 0; }\n";
const CSS_2: &[u8] = b"body { margin: 0 auto; }\n";

/// A starter of one stylesheet in two versions.
const STYLED: Starter = Starter {
    files: &[File {
        path: "assets/site.css",
        bytes: CSS_2,
    }],
    cards: &[],
    past: &[Past {
        path: "assets/site.css",
        bytes: CSS_1,
    }],
    words: &[],
};

/// A site that builds with no problem, its stylesheet the first version.
fn styled_site(label: &str, settings: &str) -> PathBuf {
    site(
        label,
        &[
            ("statac.yaml", settings.as_bytes()),
            ("content/index.md", b"Home.\n"),
            (
                "templates/home.html",
                b"<p>{{ site.title }}</p>{{ page.content }}\n",
            ),
            ("assets/site.css", CSS_1),
        ],
    )
}

fn options(folder: &Path, mode: Mode) -> Options {
    Options {
        mode,
        site: folder.to_path_buf(),
        out: Some(folder.with_extension("built")),
        today: Some(crate::date::Date::day_only(2026, 10, 6)),
        drafts: false,
        url: None,
    }
}

#[test]
fn check_tells_the_note_once_and_build_counts_it() {
    let folder = styled_site("note-built", "title: Site 1\nlanguage: en\n");
    let checked = build::run_holding(&options(&folder, Mode::Check), Progress::Silent, &STYLED);
    let report = Report::new(checked.sources(), &folder, false);
    assert_eq!(
        report.check(&checked.problems, &checked.quiet),
        "note  Newer starter files: 1 of the files `statac new` wrote has a newer version\n\
         \x20 Run `statac new --update`: it replaces those you never changed and shows what is \
         new in the rest.\n\
         \x20 More: https://statac.dev/e/newer-starter/\n\
         \n\
         1 note: 1 newer-starter.\n"
    );
    let built = build::run_holding(&options(&folder, Mode::Build), Progress::Silent, &STYLED);
    let report = Report::new(built.sources(), &folder, false);
    let said = report.build(&built.problems, &built.quiet);
    assert!(said.starts_with("1 note: 1 newer-starter. "), "{said}");
    assert!(built.published.is_some());
    assert_eq!(read(&folder, "assets/site.css"), CSS_1);
}

#[test]
fn the_note_is_quietened_by_name() {
    let settings = "title: Site 1\nlanguage: en\nquiet: [newer-starter]\n";
    let folder = styled_site("note-quiet", settings);
    let checked = build::run_holding(&options(&folder, Mode::Check), Progress::Silent, &STYLED);
    let report = Report::new(checked.sources(), &folder, false);
    let said = report.check(&checked.problems, &checked.quiet);
    assert!(!said.contains("Newer starter files"), "{said}");
    assert_eq!(checked.problems.len(), 1);
}

/// Every file under `folder` with its bytes, by its path.
fn held(folder: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(at) = folders.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            match path.is_dir() {
                true => folders.push(path),
                false => files.push((
                    path.strip_prefix(folder).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                )),
            }
        }
    }
    files.sort();
    files
}

#[test]
fn with_no_past_versions_a_new_site_builds_and_says_what_it_did_before() {
    let made = scratch("made").join("site-1");
    let answers = super::super::Answers {
        title: "Site 1".into(),
        address: Some("https://example.org".into()),
        languages: vec!["en".into(), "fr".into()],
        date_format: "2025-12-31".into(),
    };
    super::super::make(
        crate::problem::Maker::New,
        &made,
        &answers,
        None,
        crate::date::Date::day_only(2026, 10, 6),
        true,
    )
    .unwrap();
    let sites = [made];
    let none = Starter {
        files: &[],
        cards: &[],
        past: &[],
        words: &[],
    };
    let no_past = Starter {
        past: &[],
        ..Starter::OWN
    };
    for site in sites {
        let out = scratch("before-and-after");
        let mut said = Vec::new();
        let mut written = Vec::new();
        for (at, starter) in [none, no_past, Starter::OWN].iter().enumerate() {
            let into = out.join(at.to_string());
            let options = Options {
                mode: Mode::Build,
                site: site.clone(),
                out: Some(into.clone()),
                today: Some(crate::date::Date::day_only(2026, 10, 6)),
                drafts: false,
                url: None,
            };
            let outcome = build::run_holding(&options, Progress::Silent, starter);
            let report = Report::new(outcome.sources(), &site, false);
            said.push((
                report.check(&outcome.problems, &outcome.quiet),
                report.build(&outcome.problems, &outcome.quiet),
            ));
            written.push(held(&into));
        }
        assert_eq!(said[0], said[1], "{}", site.display());
        assert_eq!(said[0], said[2], "{}", site.display());
        assert!(written[0] == written[1], "{}", site.display());
        assert!(written[0] == written[2], "{}", site.display());
    }
}
