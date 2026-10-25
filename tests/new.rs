//! `statac new` writes a whole site that builds: in one language or two, with an address or none, in languages Statac has words
//! for and one it hasn't, left to right and right to left, with a sharing
//! card and without. Each test makes
//! its sites in the system's temporary folder, with the machine's locale,
//! zone and day given; none touches the network.

mod scratch;

use scratch::Scratch;
use std::fs;
use std::path::Path;

use statac::build::{self, Mode, Options};
use statac::date::Date;
use statac::new::{self, Input, Locale, Machine};
use statac::report::Report;

const KEY: &str = "0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20";

/// A folder of its own for each test, named `label`, with nothing there.
fn scratch(label: &str) -> Scratch {
    Scratch::new("new-sites", label)
}

fn day() -> Date {
    Date::day_only(2026, 10, 6)
}

/// Makes a site in `folder` with `typed` as the answers, and gives its
/// feed a key of its own so what is built is the same every time.
fn made(folder: &Path, typed: &str) {
    made_with(folder, typed, false);
}

/// [`made`], with the sharing card's files where `cards` asks.
fn made_with(folder: &Path, typed: &str, cards: bool) {
    let mut reader = typed.as_bytes();
    let machine = Machine {
        input: Some(Input {
            reader: &mut reader,
            terminal: true,
        }),
        locale: Locale::default(),
        timezone: Some("Europe/London".into()),
        today: day(),
    };
    let options = new::Options {
        folder: Some(folder.to_path_buf()),
        yes: false,
        update: None,
        cards,
        no_starter: false,
    };
    let mut said = Vec::new();
    let code = new::run(&options, machine, &mut said, false);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&said));
    let settings = folder.join("statac.yaml");
    let text = fs::read_to_string(&settings).unwrap();
    let fixed: Vec<String> = text
        .lines()
        .map(|line| match line.starts_with("feed_key: ") {
            true => format!("feed_key: {KEY}"),
            false => line.to_owned(),
        })
        .collect();
    fs::write(&settings, fixed.join("\n") + "\n").unwrap();
}

/// What `mode` finds: the name of each kind of problem, and its headline,
/// in order; and whether anything was published.
fn built(folder: &Path, mode: Mode) -> (Vec<(String, String)>, bool) {
    let out = folder.with_extension("built");
    let outcome = build::run(&Options {
        mode,
        site: folder.to_path_buf(),
        out: Some(out),
        today: Some(day()),
        drafts: false,
        url: None,
    });
    let report = Report::new(outcome.sources(), folder, false);
    let mut found: Vec<(String, String)> = outcome
        .problems
        .iter()
        .map(|problem| {
            let drawn = report.problem(problem);
            let headline = drawn.lines().next().unwrap_or_default().to_owned();
            (problem.kind.code().name().to_owned(), headline)
        })
        .collect();
    found.sort();
    (found, outcome.published.is_some())
}

#[test]
fn every_combination_is_written_checks_and_builds() {
    let languages = [
        ("en-GB", &[][..]),
        ("en-GB, fr", &[]),
        ("ar", &[]),
        ("ar, en", &[]),
        ("sw", &["sw"]),
        ("sw, ar", &["sw"]),
    ];
    let mut at = 0;
    for (typed_languages, without_words) in languages {
        for address in ["https://example.org", ""] {
            at += 1;
            let folder = scratch(&format!("site-{at}"));
            made(
                &folder,
                &format!("Site {at}\n{address}\n{typed_languages}\n\n"),
            );
            let mut wanted: Vec<(String, String)> = without_words
                .iter()
                .map(|tag| {
                    let line = format!(
                        "warning  Unknown language: Statac has no translations for `{tag}`"
                    );
                    ("unknown-language".to_owned(), line)
                })
                .collect();
            // `feed: true` asks for each language's feed, and the second
            // language has nothing dated.
            if let Some((_, second)) = typed_languages.split_once(", ") {
                let line = format!(
                    "warning  Feed not written: the folder `content/{second}/` has nothing dated"
                );
                wanted.push(("feed-not-written".to_owned(), line));
            }
            if address.is_empty() {
                let waiting = [
                    "the feeds of the site's own list and 2 other lists are left out until the \
                     site has a `url`",
                    "`content/robots.md` is left out until the site has a `url`",
                    "`content/sitemap.md` is left out until the site has a `url`",
                ];
                for waits in waiting {
                    let line = format!("note  No site URL: {waits}");
                    wanted.push(("no-site-url".to_owned(), line));
                }
            }
            wanted.sort();
            let combination = format!("{typed_languages} `{address}`");
            let (found, _) = built(&folder, Mode::Check);
            assert_eq!(found, wanted, "check: {combination}");
            let (found, published) = built(&folder, Mode::Build);
            assert_eq!(found, wanted, "build: {combination}");
            assert!(published, "{combination}");
        }
    }
}

#[test]
fn a_site_in_a_language_lacking_the_words_statac_shows_is_given_them_in_english() {
    for tag in ["ar", "de", "es", "fa", "fr", "fr-CA", "ja", "ko"] {
        let folder = scratch(&format!("words-{tag}"));
        made(&folder, &format!("Site 1\nhttps://example.org\n{tag}\n\n"));
        let file = folder.join(format!("languages/{tag}.yaml"));
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "# The words Statac itself shows alongside your writing, in English for you to translate: \
             see /dates-and-languages/\n\
             back_to_the_text: back to the text\n\
             callouts: [Note, Tip, Important, Warning, Caution]\n",
            "{tag}"
        );
        assert_eq!(built(&folder, Mode::Check).0, [], "{tag}");
        let (found, published) = built(&folder, Mode::Build);
        assert_eq!(found, [], "{tag}");
        assert!(published, "{tag}");
        let post = read_built(&folder, "posts/what-a-post-can-hold/index.html");
        assert!(post.contains("aria-label=\"back to the text\""), "{tag}");

        // The file is the site's own: bringing the starter up to date
        // leaves what it holds, and never writes it again.
        let nothing = "Nothing to update: every starter file here is the newest, or yours with \
                       nothing new.\n";
        let update = Some(new::update::Update { missing: true });
        fs::write(&file, "callouts: [A, B, C, D, E]\n").unwrap();
        assert_eq!(ran(&folder, false, update), nothing, "{tag}");
        let kept = fs::read_to_string(&file).unwrap();
        assert_eq!(kept, "callouts: [A, B, C, D, E]\n", "{tag}");

        // Without the file, the sample post's footnote has no words.
        fs::remove_file(&file).unwrap();
        assert_eq!(ran(&folder, false, update), nothing, "{tag}");
        assert!(!file.exists(), "{tag}");
        let line = format!(
            "note  Missing words: the footnotes of 1 page in `{tag}` have no translation for \
             the back link"
        );
        let wanted = [("missing-words".to_owned(), line)];
        assert_eq!(built(&folder, Mode::Check).0, wanted, "{tag}");
    }
}

#[test]
fn a_site_in_english_or_a_language_with_no_words_is_given_no_file_of_words() {
    for (at, typed) in ["en", "en-GB", "sw", "en, sw"].iter().enumerate() {
        let folder = scratch(&format!("no-words-file-{at}"));
        made(
            &folder,
            &format!("Site 1\nhttps://example.org\n{typed}\n\n"),
        );
        assert!(!folder.join("languages").exists(), "{typed}");
    }
    let folder = scratch("no-words-file-for-one");
    made(&folder, "Site 1\nhttps://example.org\nen, fr\n\n");
    let held: Vec<_> = fs::read_dir(folder.join("languages"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(held, ["fr.yaml"]);
}

/// The feed `statac.yaml` asks for of each language's own list is warned
/// of while that list has nothing dated: the second language's from the
/// start, and the first's once every folder of dated samples is deleted.
#[test]
fn a_folder_of_sample_pages_deleted_leaves_a_site_with_no_problem_but_its_feed() {
    let undated = |list: &str| {
        let line = format!("warning  Feed not written: {list} has nothing dated");
        vec![("feed-not-written".to_owned(), line)]
    };
    for (at, order) in [["notes", "photos", "posts"], ["posts", "photos", "notes"]]
        .iter()
        .enumerate()
    {
        let folder = scratch(&format!("without-{at}"));
        made(&folder, "Site 1\nhttps://example.org\nar, en\n\n");
        for (deleted, name) in order.iter().enumerate() {
            fs::remove_dir_all(folder.join("content").join(name)).unwrap();
            let wanted = match deleted {
                2 => undated("the site's own list"),
                _ => undated("the folder `content/en/`"),
            };
            let (found, _) = built(&folder, Mode::Check);
            assert_eq!(found, wanted, "check without {name}");
            let (found, published) = built(&folder, Mode::Build);
            assert_eq!(found, wanted, "build without {name}");
            assert!(published, "without {name}");
            let home = read_built(&folder, "index.html");
            assert!(!home.contains(&format!("\"/{name}/\"")), "{name}: {home}");
        }
    }
}

/// Each language has a page listing its tags, which each tag's page
/// links to; a page with `menu: true` is in the menu of its language.
/// Deleting any of them leaves a site with no problem and no link to it.
#[test]
fn every_tag_is_listed_on_a_page_and_a_page_asks_into_the_menu() {
    let folder = scratch("tags-and-menu");
    made(&folder, "Site 1\nhttps://example.org\nen, fr\n\n");
    fs::create_dir_all(folder.join("content/fr/posts")).unwrap();
    fs::write(
        folder.join("content/fr/posts/index.md"),
        "---\ntitle: Posts\n---\n",
    )
    .unwrap();
    fs::write(
        folder.join("content/fr/posts/post-1.md"),
        "---\ntitle: Post 1\ndate: 2026-10-01\ntags: [Tag 1]\n---\nText.\n",
    )
    .unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let tag = "<li><a href=\"/tags/examples/\" rel=\"tag\">Examples</a></li>";
    let all = read_built(&folder, "tags/index.html");
    assert!(
        all.contains(tag) && !all.contains("/fr/tags/tag-1/"),
        "{all}"
    );
    let french = read_built(&folder, "fr/tags/index.html");
    let tag = "<li><a href=\"/fr/tags/tag-1/\" rel=\"tag\">Tag 1</a></li>";
    assert!(
        french.contains(tag) && !french.contains("/tags/examples/"),
        "{french}"
    );
    let link = "<p><a href=\"/tags/\">All tags</a></p>";
    assert!(read_built(&folder, "tags/examples/index.html").contains(link));
    let link = "<p><a href=\"/fr/tags/\">All tags</a></p>";
    assert!(read_built(&folder, "fr/tags/tag-1/index.html").contains(link));
    let about = "<li><a href=\"/about/\">About</a></li>";
    assert!(read_built(&folder, "index.html").contains(about));
    assert!(!read_built(&folder, "fr/index.html").contains("/about/"));
    let here = "<li><a href=\"/about/\" aria-current=\"page\">About</a></li>";
    assert!(read_built(&folder, "about/index.html").contains(here));

    for page in ["content/tags.md", "content/fr/tags.md", "content/about.md"] {
        fs::remove_file(folder.join(page)).unwrap();
    }
    fs::remove_dir_all(folder.with_extension("built")).unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    for page in ["tags/examples/index.html", "fr/tags/tag-1/index.html"] {
        let page = read_built(&folder, page);
        assert!(
            !page.contains("All tags") && !page.contains("/about/"),
            "{page}"
        );
    }
}

/// A folder with no `index.md` has no page and no URL: the menu marks
/// the folder a post is in all the same, and a new site's lists page
/// through their posts and their tags have pages, as the starter's
/// templates show them.
#[test]
fn a_post_in_a_folder_with_no_page_is_built_and_the_starters_lists_page_through_their_posts() {
    let folder = scratch("no-folder-page");
    made(&folder, "Site 1\nhttps://example.org\nen\n\n");
    let settings = fs::read_to_string(folder.join("statac.yaml")).unwrap();
    assert!(
        settings.contains("\ntags.pages: true\nseries.pages: true\nposts_per_page: 10\n"),
        "{settings}"
    );
    for n in 1..=11 {
        fs::write(
            folder.join(format!("content/posts/post-{n}.md")),
            format!("---\ntitle: Post {n}\ndate: 2026-09-{n:02}\ntags: [Tag 1]\n---\nText.\n"),
        )
        .unwrap();
    }
    fs::create_dir_all(folder.join("content/posts/2026")).unwrap();
    fs::write(
        folder.join("content/posts/2026/post-12.md"),
        "---\ntitle: Post 12\ndate: 2026-09-12\n---\nText.\n",
    )
    .unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let post = read_built(&folder, "posts/2026/post-12/index.html");
    assert!(post.contains("Post 12"), "{post}");
    for later in ["posts/page/2/index.html", "tags/tag-1/page/2/index.html"] {
        assert!(read_built(&folder, later).contains("Post 1"), "{later}");
    }
    // The home page shows the newest of each folder itself, on one page.
    assert!(!folder.with_extension("built").join("page/2").exists());
}

#[test]
fn a_site_naming_no_language_is_in_english() {
    let folder = scratch("no-language");
    made(&folder, "Site 1\nhttps://example.org\nen\n\n");
    let settings = folder.join("statac.yaml");
    let text = fs::read_to_string(&settings).unwrap();
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with("language: "))
        .collect();
    assert_eq!(kept.len() + 1, text.lines().count());
    fs::write(&settings, kept.join("\n") + "\n").unwrap();
    fs::write(
        folder.join("templates/languages.html"),
        "{{ site.language }} {{ site.languages }} {{ site.languages.en.url }}",
    )
    .unwrap();
    fs::write(
        folder.join("content/languages.md"),
        "---\nlayout: languages\n---\n",
    )
    .unwrap();
    let note = [(
        "no-language".to_owned(),
        "note  No language set: the site is read as English".to_owned(),
    )];
    assert_eq!(built(&folder, Mode::Check).0, note);
    let (found, published) = built(&folder, Mode::Build);
    assert_eq!(found, note);
    assert!(published);
    let home = read_built(&folder, "index.html");
    assert!(home.contains("<html lang=\"en\" dir=\"ltr\">"), "{home}");
    assert!(home.contains(">Skip to the content<"), "{home}");
    assert_eq!(read_built(&folder, "languages/index.html"), "en en /");

    let folder = scratch("several-languages");
    made(&folder, "Site 1\nhttps://example.org\nar, en\n\n");
    let undated = "warning  Feed not written: the folder `content/en/` has nothing dated";
    assert_eq!(
        built(&folder, Mode::Build).0,
        [("feed-not-written".to_owned(), undated.to_owned())]
    );
    let home = read_built(&folder, "index.html");
    assert!(home.contains("<html lang=\"ar\" dir=\"rtl\">"), "{home}");
    let home = read_built(&folder, "en/index.html");
    assert!(home.contains("<html lang=\"en\" dir=\"ltr\">"), "{home}");
}

#[test]
fn the_stylesheet_written_leaves_a_pictures_height_to_the_browser() {
    let folder = scratch("proportion");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let (found, _) = built(&folder, Mode::Check);
    assert_eq!(found, []);

    // Without the line that does, the site's own picture is out of
    // proportion.
    let stylesheet = folder.join("assets/site.css");
    let css = fs::read_to_string(&stylesheet).unwrap();
    let without: String = css
        .lines()
        .filter(|line| !line.contains("block-size") && !line.contains("aspect-ratio"))
        .map(|line| format!("{line}\n"))
        .collect();
    assert_ne!(css, without);
    fs::write(&stylesheet, without).unwrap();
    let (found, _) = built(&folder, Mode::Check);
    let line = "warning  Images out of proportion: `img` has a `max-inline-size` and no \
                `block-size`";
    assert_eq!(
        found,
        [("images-out-of-proportion".to_owned(), line.to_owned())]
    );
}

#[test]
fn a_new_site_asks_for_its_code_colours_and_is_told_once_it_no_longer_does() {
    let folder = scratch("colours");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let (found, _) = built(&folder, Mode::Check);
    assert!(found.is_empty(), "{found:?}");

    let layout = folder.join("templates/base.html");
    let text = fs::read_to_string(&layout).unwrap();
    let asks = "{{ css(\"code.css\") }}\n";
    assert!(text.contains(asks), "{text}");
    fs::write(&layout, text.replace(asks, "")).unwrap();
    let told = "warning  Code colours not loaded: no template asks for `code.css`";
    let wanted = [("code-colours-not-loaded".to_owned(), told.to_owned())];
    let (found, _) = built(&folder, Mode::Check);
    assert_eq!(found, wanted);
    let (found, published) = built(&folder, Mode::Build);
    assert_eq!(found, wanted);
    assert!(published);
}

#[test]
fn a_video_kept_on_another_site_is_a_link_its_own_script_makes_a_button() {
    let folder = scratch("video");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let post = folder.join("content/posts/first-post/index.md");
    let text = fs::read_to_string(&post).unwrap();
    let calls = "\n@youtube(\"abcdefghijk\", poster=\"photo-1.jpg\", title=\"Video 1\")\n\n\
                 @vimeo(\"123456789\", poster=\"photo-1.jpg\")\n\n\
                 @vimeo(\"123456789\", poster=\"photo-1.jpg\", caption=\"Video *2*\")\n";
    fs::write(&post, text + calls).unwrap();
    let outcome = build::run(&Options {
        mode: Mode::Build,
        site: folder.clone(),
        out: Some(folder.with_extension("built")),
        today: Some(day()),
        drafts: false,
        url: None,
    });
    assert!(outcome.problems.is_empty() && outcome.published.is_some());
    let page = read_built(&folder, "posts/first-post/index.html");
    for held in [
        "<a href=\"https://www.youtube-nocookie.com/embed/abcdefghijk?autoplay=1\" \
         data-title=\"Video 1\"><picture>",
        " alt=\"Play Video 1\" ",
        "<a href=\"https://player.vimeo.com/video/123456789?autoplay=1\" \
         data-title=\"Video player\"><picture>",
        " alt=\"Play the video\" ",
        "data-title=\"Video 2\"><picture>",
        " alt=\"Play Video 2\" ",
        "</script>\n          <figcaption>Video <em>2</em></figcaption>\n        </figure>",
    ] {
        assert!(page.contains(held), "{held} isn't in:\n{page}");
    }
    // Nothing on the page asks anything of either site before the press.
    for (host, links) in [("youtube", 1), ("vimeo.com", 2)] {
        assert_eq!(page.matches(host).count(), links, "{page}");
    }
    assert!(!page.contains("<iframe") && !page.contains("preconnect"));
    // Each call's script stands straight after its own link, the element it reads.
    assert_eq!(page.matches("<script>").count(), 3, "{page}");
    assert_eq!(
        page.matches("</picture></a>\n          <script>\n").count(),
        3,
        "{page}"
    );
    // Calling a snippet is choosing what it brings, so none of it is listed.
    assert!(outcome.javascript.is_empty(), "{:?}", outcome.javascript);
}

#[test]
fn a_video_and_a_recording_kept_in_the_site_are_named_by_their_captions() {
    let folder = scratch("captions");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let post = folder.join("content/posts/first-post");
    let text = fs::read_to_string(post.join("index.md")).unwrap();
    let calls = "\n@video(\"video-1.mp4\", caption=\"Video *1*\")\n\n@video(\"video-1.mp4\")\n\n\
                 @audio(\"recording-1.mp3\", caption=\"Recording *1*\")\n\n\
                 @audio(\"recording-1.mp3\")\n";
    fs::write(post.join("index.md"), text + calls).unwrap();
    fs::write(post.join("video-1.mp4"), "").unwrap();
    fs::write(post.join("recording-1.mp3"), "").unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let page = read_built(&folder, "posts/first-post/index.html");
    for held in [
        "<figure>\n          <video controls preload=\"metadata\" aria-label=\"Video 1\">",
        "</video>\n          <figcaption>Video <em>1</em></figcaption>\n        </figure>",
        "<figure>\n          <video controls preload=\"metadata\">",
        "</video>\n        </figure>",
        "<figure>\n          <audio controls preload=\"metadata\" aria-label=\"Recording 1\">",
        "</audio>\n          <figcaption>Recording <em>1</em></figcaption>\n        </figure>",
        "<figure>\n          <audio controls preload=\"metadata\">",
        "</audio>\n        </figure>",
    ] {
        assert!(page.contains(held), "{held} isn't in:\n{page}");
    }
}

#[test]
fn a_page_with_no_card_still_says_what_a_preview_shows() {
    let folder = scratch("no-card");
    made(&folder, "Site 1\n\nen-GB\n\n");
    let (_, published) = built(&folder, Mode::Build);
    assert!(published);
    let page = read_built(&folder, "posts/first-post/index.html");
    let said: Vec<&str> = (page.lines())
        .map(str::trim_start)
        .filter(|line| line.starts_with("<meta property=\"og:"))
        .map(|line| line.split('"').nth(1).unwrap())
        .collect();
    let all = [
        "og:title",
        "og:type",
        "og:site_name",
        "og:locale",
        "og:description",
    ];
    assert_eq!(said, all);
    assert!(page.contains("<meta property=\"og:type\" content=\"article\">"));
    assert!(!page.contains("twitter:card"));

    // With an address, the page's address is said too.
    let settings = folder.join("statac.yaml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, text + "url: https://example.org\n").unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let page = read_built(&folder, "posts/first-post/index.html");
    let address = "<meta property=\"og:url\" content=\"https://example.org/posts/first-post/\">";
    assert!(page.contains(address), "{page}");
    assert!(!page.contains("og:image"));
}

#[test]
fn the_card_fonts_licence_is_beside_the_font_and_is_no_template() {
    let folder = scratch("font-licence");
    made_with(&folder, "Site 1\nhttps://example.org\nen-GB\n\n", true);
    let licence = folder.join("templates/og/OFL.txt");
    let text = fs::read_to_string(&licence).unwrap();
    assert!(text.starts_with("Copyright 2015-2021 Google LLC. All Rights Reserved.\n"));
    assert!(text.contains("SIL OPEN FONT LICENSE Version 1.1"));
    let (found, _) = built(&folder, Mode::Check);
    assert!(found.is_empty(), "{found:?}");
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    assert!(read_built(&folder, "index.html").contains("og:image"));
    let out = folder.with_extension("built");
    assert!(!out.join("OFL.txt").exists() && !out.join("og").exists());

    // Words no template could hold are no trouble there.
    fs::write(&licence, text + "{% endfor %}\n").unwrap();
    let (found, _) = built(&folder, Mode::Check);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_page_at_an_old_address_asks_search_engines_to_leave_it_out() {
    let folder = scratch("old-address");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let post = folder.join("content/posts/first-post/index.md");
    let text = fs::read_to_string(&post).unwrap();
    let moved = "cover: photo-1.jpg\nredirect_from: [/old/post-1/]\n";
    fs::write(&post, text.replace("cover: photo-1.jpg\n", moved)).unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    assert_eq!(
        read_built(&folder, "old/post-1/index.html"),
        "<!doctype html>\n\
         <html lang=\"en-GB\" dir=\"ltr\">\n\
         <meta charset=\"utf-8\">\n\
         <title>First post</title>\n\
         <meta name=\"robots\" content=\"noindex\">\n\
         <meta http-equiv=\"refresh\" content=\"0; url=https://example.org/posts/first-post/\">\n\
         <link rel=\"canonical\" href=\"https://example.org/posts/first-post/\">\n\
         <p>This page has moved to <a href=\"https://example.org/posts/first-post/\">\
         https://example.org/posts/first-post/</a></p>\n\
         </html>\n"
    );
}

#[test]
fn a_feed_says_what_the_site_and_each_post_give_it() {
    let folder = scratch("feed");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let settings = folder.join("statac.yaml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, text + "tagline: A tagline\nauthor: Name\n").unwrap();
    let posts = folder.join("content/posts/index.md");
    let text = fs::read_to_string(&posts).unwrap();
    fs::write(&posts, text.replace("feed: false\n", "feed: true\n")).unwrap();
    let post = folder.join("content/posts/first-post/index.md");
    let text = fs::read_to_string(&post).unwrap();
    let more = "tags: [Tag 1, Tag 2]\nupdated: 2026-10-07\n";
    assert!(text.contains("tags: [Examples]\n"), "{text}");
    fs::write(&post, text.replace("tags: [Examples]\n", more)).unwrap();
    let outcome = build::run(&Options {
        mode: Mode::Build,
        site: folder.clone(),
        out: Some(folder.with_extension("built")),
        today: Some(Date::day_only(2026, 10, 8)),
        drafts: false,
        url: None,
    });
    assert!(outcome.problems.is_empty() && outcome.published.is_some());
    let feed = read_built(&folder, "feed.xml");
    for held in [
        "  <title>Site 1</title>\n  <subtitle>A tagline</subtitle>\n",
        "  <author><name>Name</name></author>\n",
        "    <published>2026-10-06T00:00:00+01:00</published>\n    \
         <updated>2026-10-07T00:00:00+01:00</updated>\n    \
         <category term=\"Tag 1\"/>\n    <category term=\"Tag 2\"/>\n    <content ",
    ] {
        assert!(feed.contains(held), "{held} isn't in:\n{feed}");
    }
    let of_posts = read_built(&folder, "posts/feed.xml");
    let titled = "  <title>Posts - Site 1</title>\n  <subtitle>A tagline</subtitle>\n";
    assert!(of_posts.contains(titled), "{of_posts}");
    // A note with no title is called by its date and time, and one with a
    // title by that.
    let titles: Vec<&str> = (feed.lines())
        .filter_map(|line| line.strip_prefix("    <title>"))
        .collect();
    assert_eq!(
        titles[..4],
        [
            "6 Oct 2026, 09:12</title>",
            "First post</title>",
            "5 Oct 2026, 16:20</title>",
            "What a post can hold</title>",
        ],
        "{feed}"
    );
    assert!(titles.contains(&"A note with a title</title>"), "{feed}");
}

#[test]
fn a_mention_shown_is_marked_up_for_what_it_is_and_its_words_stay_text() {
    let folder = scratch("mentions");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let settings = folder.join("statac.yaml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, text + "webmentions: true\n").unwrap();
    let post = folder.join("content/posts/first-post/index.md");
    let text = fs::read_to_string(&post).unwrap();
    let asks = "cover: photo-1.jpg\nmentions: true\n";
    fs::write(&post, text.replace("cover: photo-1.jpg\n", asks)).unwrap();
    let kept = folder.join("fetched/mentions");
    fs::create_dir_all(&kept).unwrap();
    let mention = |kind: &str, more: &str| {
        format!(
            "kind: \"{kind}\"\nurl: \"https://a.example/post-1/\"\n\
             target: \"https://example.org/posts/first-post/\"\nfetched: \"2026-10-06\"\n{more}"
        )
    };
    let replied = "author: \"A\"\nauthor_url: \"https://a.example/\"\n\
                   published: \"2026-09-03\"\nwords: \"<b>Words</b> *here*.\"\n";
    fs::write(kept.join("1.yaml"), mention("replied", replied)).unwrap();
    fs::write(kept.join("2.yaml"), mention("mentioned", "")).unwrap();
    fs::write(kept.join("3.yaml"), mention("liked", "author: \"A\"\n")).unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let page = read_built(&folder, "posts/first-post/index.html");
    let at = page.find("<section class=\"mentions ").unwrap();
    // Straight after the post, above the links to the posts either side.
    assert!(page[..at].ends_with("</article>\n    "), "{page}");
    let beside = page.find("<nav class=\"more wide\"").unwrap();
    assert!(at < beside, "{page}");
    let shown = &page[at..page[at..].find("</section>").unwrap() + at];
    assert_eq!(
        shown,
        r#"<section class="mentions column" aria-labelledby="mentions">
      <div>
        <h2 class="label" id="mentions">From other sites</h2>
        <ul class="faces">
          <li class="h-cite"><a class="u-url" href="https://a.example/post-1/" rel="nofollow ugc"><span class="p-author h-card"><span class="p-name">A</span></span> liked</a><data class="u-like-of" value="/posts/first-post/"></data></li>
        </ul>
        <article class="h-cite">
          <p>
            <span class="p-author h-card"><a class="p-name u-url" href="https://a.example/" rel="nofollow ugc">A</a></span>
            <a class="u-url" href="https://a.example/post-1/" rel="nofollow ugc">replied</a> <time class="dt-published" datetime="2026-09-03T00:00:00+01:00">3 September 2026</time><data class="u-in-reply-to" value="/posts/first-post/"></data></p>
          <p class="p-content">&lt;b&gt;Words&lt;/b&gt; *here*.</p>
        </article>
        <article class="h-cite">
          <p>
            <a class="u-url" href="https://a.example/post-1/" rel="nofollow ugc">mentioned</a></p>
        </article>
      </div>
    "#
    );
}

#[test]
fn a_quotation_says_who_said_it_and_where() {
    let folder = scratch("quote");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let post = folder.join("content/posts/first-post/index.md");
    let text = fs::read_to_string(&post).unwrap();
    let calls = "\n@quote(\"Name\")\nQuoted *once*.\n@end\n\n\
                 @quote(by=\"*Name*\", cite=\"[Work 1](https://example.com/work-1/)\")\n\
                 Quoted twice.\n@end\n\n\
                 @quote(cite=\"Work 1\")\nQuoted a third time.\n@end\n\n\
                 @quote()\nQuoted last.\n@end\n";
    fs::write(&post, text + calls).unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let page = read_built(&folder, "posts/first-post/index.html");
    let at = page.find("<figure class=\"quote\">").unwrap();
    let shown = &page[at..page[at..].find("</div>").unwrap() + at];
    assert_eq!(
        shown,
        r#"<figure class="quote">
          <blockquote>
            <p>Quoted <em>once</em>.</p>
          </blockquote>
          <figcaption>Name</figcaption>
        </figure>
        <figure class="quote">
          <blockquote>
            <p>Quoted twice.</p>
          </blockquote>
          <figcaption><em>Name</em>, <cite><a href="https://example.com/work-1/">Work 1</a></cite></figcaption>
        </figure>
        <figure class="quote">
          <blockquote>
            <p>Quoted a third time.</p>
          </blockquote>
          <figcaption><cite>Work 1</cite></figcaption>
        </figure>
        <figure class="quote">
          <blockquote>
            <p>Quoted last.</p>
          </blockquote>
        </figure>
      "#
    );
}

#[test]
fn a_snippet_handed_false_does_as_it_does_when_handed_nothing() {
    let folder = scratch("false");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    fs::write(
        folder.join("content/page-1.md"),
        "---\ntitle: Page 1\nlayout: page-1\n---\n",
    )
    .unwrap();
    fs::write(
        folder.join("templates/page-1.html"),
        "{% set post = (site.posts | selectattr(\"cover\") | list)[0] %}\
         {{ entry(post) }}<hr>{{ entry(post, pictured=false) }}<hr>\
         {{ short(post) }}<hr>{{ short(post, nested=false) }}\n",
    )
    .unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let page = read_built(&folder, "page-1/index.html");
    let shown: Vec<&str> = page.split("<hr>").collect();
    let [entry, entry_false, short, short_false] = shown[..] else {
        panic!("{page}");
    };
    assert!(!entry.contains("<img"), "{entry}");
    assert_eq!(entry_false, entry);
    assert!(short.contains("<h2>"), "{short}");
    assert_eq!(short_false, short);
}

/// Runs `statac new` on the site in `folder` with `cards` and `update` as
/// given, and gives what it said.
fn ran(folder: &Path, cards: bool, update: Option<new::update::Update>) -> String {
    let machine = Machine {
        input: None,
        locale: Locale::default(),
        timezone: Some("Europe/London".into()),
        today: day(),
    };
    let options = new::Options {
        folder: Some(folder.to_path_buf()),
        yes: true,
        update,
        cards,
        no_starter: false,
    };
    let mut said = Vec::new();
    let code = new::run(&options, machine, &mut said, false);
    let said = String::from_utf8(said).unwrap();
    assert_eq!(code, 0, "{said}");
    said
}

/// The sharing card's files a site has, and the cards and the pages
/// naming one that it publishes with no problem found, by `check` or by
/// `build`.
fn cards_of(folder: &Path) -> (usize, usize, usize) {
    let files = [
        "templates/og.yaml",
        "templates/og/NotoEmoji-OFL.txt",
        "templates/og/NotoEmoji.ttf",
        "templates/og/NotoSans-Regular.ttf",
        "templates/og/OFL.txt",
        "templates/og/background.png",
    ];
    let here = files.iter().filter(|path| folder.join(path).is_file());
    let (found, _) = built(folder, Mode::Check);
    assert_eq!(found, []);
    let (found, published) = built(folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let (mut drawn, mut naming) = (0, 0);
    let mut folders = vec![folder.with_extension("built")];
    while let Some(at) = folders.pop() {
        for entry in fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() {
                folders.push(path);
            } else if name.starts_with("og.") {
                drawn += 1;
            } else if name.ends_with(".html") {
                let page = fs::read_to_string(&path).unwrap();
                let card = page.contains("og:image");
                assert_eq!(page.contains("twitter:card"), card, "{name}");
                naming += usize::from(card);
            }
        }
    }
    (here.count(), drawn, naming)
}

#[test]
fn a_site_draws_cards_from_when_it_asks_until_their_description_is_removed() {
    let folder = scratch("cards");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    assert_eq!(cards_of(&folder), (0, 0, 0));
    assert!(!folder.join("templates/og").exists());

    // Neither bringing the starter up to date nor writing what is missing
    // turns cards on.
    let nothing = "Nothing to update: every starter file here is the newest, or yours with \
                   nothing new.\n";
    for missing in [false, true] {
        let update = new::update::Update { missing };
        assert_eq!(ran(&folder, false, Some(update)), nothing);
        assert_eq!(cards_of(&folder), (0, 0, 0));
    }

    let wrote = "Wrote `templates/og/NotoEmoji-OFL.txt`.\n\
                 Wrote `templates/og/NotoEmoji.ttf`.\n\
                 Wrote `templates/og/NotoSans-Regular.ttf`.\n\
                 Wrote `templates/og/OFL.txt`.\n\
                 Wrote `templates/og/background.png`.\n\
                 Wrote `templates/og.yaml`.\n";
    assert_eq!(ran(&folder, true, None), wrote);
    // A card for every page but the one nothing is published at. The home
    // page's own list has one page, so no second is published or carded.
    let on = cards_of(&folder);
    assert_eq!(on, (6, 19, 19));
    assert!(!folder.with_extension("built").join("page").exists());
    let yours = "`templates/og.yaml` is yours and wasn't touched.\n";
    assert_eq!(ran(&folder, true, None), yours);
    assert_eq!(cards_of(&folder), on);

    // The files as written are the starter's, with nothing newer.
    for missing in [false, true] {
        let update = new::update::Update { missing };
        assert_eq!(ran(&folder, false, Some(update)), nothing);
        assert_eq!(cards_of(&folder), on);
    }

    // A card's file gone is written again while the site describes a card.
    fs::remove_file(folder.join("templates/og/NotoEmoji.ttf")).unwrap();
    let update = new::update::Update { missing: true };
    assert_eq!(
        ran(&folder, false, Some(update)),
        "Wrote `templates/og/NotoEmoji.ttf`.\nUpdated 1 file.\n"
    );
    assert_eq!(cards_of(&folder), on);

    fs::remove_file(folder.join("templates/og.yaml")).unwrap();
    assert_eq!(cards_of(&folder), (5, 0, 0));

    // A site made with its card is the same as one given it afterwards.
    let with = scratch("cards-from-the-start");
    made_with(&with, "Site 1\nhttps://example.org\nen-GB\n\n", true);
    assert_eq!(cards_of(&with), on);
    assert_eq!(
        read_built(&with, "index.html"),
        HOME.replace(
            "<meta property=\"og:url\" content=\"https://example.org/\">\n",
            "<meta property=\"og:url\" content=\"https://example.org/\">\n\
             \x20 <meta property=\"og:image\" content=\"https://example.org/og.6ee18fd4.png\">\n\
             \x20 <meta property=\"og:image:width\" content=\"1200\">\n\
             \x20 <meta property=\"og:image:height\" content=\"630\">\n\
             \x20 <meta property=\"og:image:alt\" content=\"Site 1. Site 1\">\n\
             \x20 <meta name=\"twitter:card\" content=\"summary_large_image\">\n"
        )
    );
}

#[test]
fn a_site_with_a_card_and_no_address_is_told_its_cards_wait() {
    let folder = scratch("cards-waiting");
    made_with(&folder, "Site 1\n\nen-GB\n\n", true);
    let (found, published) = built(&folder, Mode::Build);
    assert!(published);
    let line = "note  No site URL: social cards are left out until the site has a `url`";
    assert!(
        found.contains(&("no-site-url".to_owned(), line.to_owned())),
        "{found:?}"
    );
}

/// The built site's file at `path`.
fn read_built(folder: &Path, path: &str) -> String {
    fs::read_to_string(folder.with_extension("built").join(path)).unwrap()
}

#[test]
fn the_search_page_sends_a_search_elsewhere_without_javascript_only_where_the_site_asks() {
    let folder = scratch("search-elsewhere");
    made(&folder, "Site 1\nhttps://example.org/blog/\nen-GB\n\n");
    let (_, published) = built(&folder, Mode::Build);
    assert!(published);
    let page = read_built(&folder, "search/index.html");
    let told = "      <noscript>\n        \
                <p>Search needs JavaScript, which is turned off in this browser.</p>\n      \
                </noscript>\n";
    assert!(page.contains(told), "{page}");
    assert!(
        !page.contains("<form") && !page.contains("duckduckgo"),
        "{page}"
    );

    let settings = folder.join("statac.yaml");
    let text = fs::read_to_string(&settings).unwrap();
    fs::write(&settings, text + "search_without_javascript: duckduckgo\n").unwrap();
    let (_, published) = built(&folder, Mode::Build);
    assert!(published);
    let page = read_built(&folder, "search/index.html");
    let form = r#"      <noscript>
        <search>
          <form action="https://duckduckgo.com/" method="get">
            <label for="search-elsewhere">Search this site with DuckDuckGo</label>
            <input id="search-elsewhere" type="search" name="q">
            <input type="hidden" name="sites" value="example.org">
            <button>Search</button>
          </form>
        </search>
      </noscript>
"#;
    assert!(page.contains(form), "{page}");
    assert!(!page.contains("Search needs JavaScript"), "{page}");
}

#[test]
fn what_one_site_publishes_is_as_held_here() {
    let folder = scratch("held");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let post = folder.join("content/posts/first-post/index.md");
    let text = fs::read_to_string(&post).unwrap();
    fs::write(&post, text.replace("tags: [Examples]\n", "tags: [Tag 1]\n")).unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    for (path, held) in [
        ("index.html", HOME),
        ("posts/first-post/index.html", POST),
        ("notes/index.html", NOTES),
        ("notes/note-1/index.html", NOTE),
        ("photos/index.html", PHOTOS),
        ("photos/photo-1/index.html", PHOTO),
        ("tags/tag-1/index.html", TAG),
        ("notes/feed.xml", FEED),
        ("sitemap.xml", SITEMAP),
        ("404.html", NOT_FOUND),
    ] {
        let written = read_built(&folder, path);
        assert!(written == held, "{path} is now:\n{written}");
    }
}

#[test]
fn the_sitemap_lists_a_page_at_an_address_ending_html_and_no_file_that_is_no_page() {
    let folder = scratch("sitemap");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    fs::write(
        folder.join("content/posts/post-1.md"),
        "---\ntitle: Post 1\ndate: 2026-10-01\npermalink: /2026/post-1.html\n---\nPost 1.\n",
    )
    .unwrap();
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");
    let sitemap = read_built(&folder, "sitemap.xml");
    assert!(sitemap.contains("<loc>https://example.org/2026/post-1.html</loc>"));
    assert!(sitemap.contains("<loc>https://example.org/posts/first-post/</loc>"));
    for left_out in ["404.html", "robots.txt", "sitemap.xml"] {
        assert!(read_built(&folder, left_out).len() > 1);
        assert!(!sitemap.contains(left_out), "{left_out}");
    }
}

#[test]
fn the_menu_ends_with_notes_and_photographs_and_a_note_is_called_by_its_date_and_time() {
    let folder = scratch("menu");
    made(&folder, "Site 1\nhttps://example.org\nen-GB\n\n");
    let page = "---\ntitle: Links\n---\n";
    fs::create_dir_all(folder.join("content/links")).unwrap();
    fs::write(folder.join("content/links/index.md"), page).unwrap();
    fs::create_dir_all(folder.join("content/writing")).unwrap();
    fs::write(
        folder.join("content/writing/index.md"),
        page.replace("Links", "Writing"),
    )
    .unwrap();
    for inside in ["links", "writing"] {
        let path = folder.join("content").join(inside).join("page-1.md");
        fs::write(path, "---\ntitle: Page 1\n---\nPage 1.\n").unwrap();
    }
    let (found, published) = built(&folder, Mode::Build);
    assert!(found.is_empty() && published, "{found:?}");

    // Home, each other folder by its name, then notes and photographs,
    // then the pages that ask to be in it.
    let home = read_built(&folder, "index.html");
    let at = home.find("<nav aria-label=\"Site\">").unwrap();
    let menu = &home[at..home[at..].find("</nav>").unwrap() + at];
    let names: Vec<&str> = (menu.lines())
        .filter_map(|line| line.strip_suffix("</a></li>"))
        .filter_map(|line| line.rsplit_once('>'))
        .map(|(_, name)| name)
        .collect();
    assert_eq!(
        names,
        [
            "Home", "Links", "Posts", "Writing", "Notes", "Photos", "About", "Search"
        ]
    );
    fs::remove_dir_all(folder.join("content/links")).unwrap();
    fs::remove_dir_all(folder.join("content/writing")).unwrap();
    let (found, _) = built(&folder, Mode::Build);
    // The pages taken away were published by the build before.
    let found: Vec<_> = (found.iter())
        .filter(|(code, _)| *code != "url-gone")
        .collect();
    assert!(found.is_empty(), "{found:?}");
    let home = read_built(&folder, "index.html");
    let menu = "<li><a href=\"/\" aria-current=\"page\">Home</a></li>\n\
                \x20       <li><a href=\"/posts/\">Posts</a></li>\n\
                \x20       <li><a href=\"/notes/\">Notes</a></li>\n\
                \x20       <li><a href=\"/photos/\">Photos</a></li>\n";
    assert!(home.contains(menu), "{home}");

    // A note with no title, listed beside posts under a tag and among a
    // post's related posts, has its time as well as its day.
    let called = "<a href=\"/notes/note-1/\">6 Oct 2026, 09:12</a>";
    let tagged = read_built(&folder, "tags/examples/index.html");
    assert!(tagged.contains(&format!("<h3>{called}</h3>")), "{tagged}");
    let post = read_built(&folder, "posts/first-post/index.html");
    let at = post.find("<section class=\"related wide\"").unwrap();
    let related = &post[at..post[at..].find("</section>").unwrap() + at];
    assert!(
        related.contains(&format!("<li>{called} <time")),
        "{related}"
    );
}

const HOME: &str = r##"<!doctype html>
<html lang="en-GB" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Site 1</title>
  <meta name="description" content="This is the home page of a new site, the file content/index.md. Write here what the site is, in place of these words.">
  <link rel="stylesheet" href="/site.e1b305c9.css">
  <link rel="stylesheet" href="/code.644d7a9d.css">
  <link rel="alternate" type="application/atom+xml" href="/feed.xml" title="Site 1">
  <meta property="og:title" content="Site 1">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Site 1">
  <meta property="og:locale" content="en_GB">
  <meta property="og:description" content="This is the home page of a new site, the file content/index.md. Write here what the site is, in place of these words.">
  <meta property="og:url" content="https://example.org/">
</head>
<body>
  <a class="skip" href="#content">Skip to the content</a>
  <header class="wide">
    <p class="name"><a href="/">Site 1</a></p>
    <nav aria-label="Site">
      <ul>
        <li><a href="/" aria-current="page">Home</a></li>
        <li><a href="/posts/">Posts</a></li>
        <li><a href="/notes/">Notes</a></li>
        <li><a href="/photos/">Photos</a></li>
        <li><a href="/about/">About</a></li>
        <li><a href="/search/">Search</a></li>
      </ul>
    </nav>
  </header>
  <main id="content">
    <header class="top column">
      <h1>Hello.</h1>
    </header>
    <div class="writing column">
      <p>This is the home page of a new site, the file <code>content/index.md</code>. Write here what the site is, in place of these words.</p>
      <p>To see the site as you change it, run <code>statac serve</code> in its folder. Posts are in <code>content/posts/</code>, notes in <code>content/notes/</code> and photographs in <code>content/photos/</code>; delete a folder you don’t want and it leaves the site. How each page looks is in <code>templates/</code>, <code>snippets/</code> and <code>assets/site.css</code>, and the words the pages show around the writing are in <code>data/words.yaml</code>. Every file is yours to change.</p>
    </div>
    <section class="listing column" aria-labelledby="home-posts">
      <div class="heading">
        <h2 id="home-posts">Posts</h2>
        <a href="/posts/">See all</a>
      </div>
      <ol class="posts">
        <li>
          <time datetime="2026-10-06T00:00:00+01:00">6 Oct 2026</time>
          <div>
            <h3><a href="/posts/first-post/">First post</a></h3>
            <p>This is the site’s first post, the file content/posts/first-post/index.md. Its picture is photo-1.jpg, kept beside it and named by its cover: line, and what a…</p>
          </div>
        </li>
        <li>
          <time datetime="2026-10-05T00:00:00+01:00">5 Oct 2026</time>
          <div>
            <h3><a href="/posts/what-a-post-can-hold/">What a post can hold</a></h3>
            <p>Every kind of thing a post can show, each beside the way it is written, so the site&#x27;s look can be judged on one page.</p>
          </div>
        </li>
      </ol>
    </section>
    <section class="listing column" aria-labelledby="home-notes">
      <div class="heading">
        <h2 id="home-notes">Notes</h2>
        <a href="/notes/">See all</a>
      </div>
      <ol class="notes">
        <li>
          <article>
            <div class="writing">
              <p>This is a note, the file <code>content/notes/note-1.md</code>: a few lines with a date and no title. Its front matter holds only the date.</p>
            </div>
            <footer>
              <a href="/notes/note-1/"><time datetime="2026-10-06T09:12:00+01:00">6 Oct 2026, 09:12</time></a>
              <ul class="tags" aria-label="Tags">
                <li><a href="/tags/examples/" rel="tag">Examples</a></li>
              </ul>
            </footer>
          </article>
        </li>
        <li>
          <article>
            <div class="writing">
              <p>A note can hold anything a post can:</p>
              <ul>
                <li>a list</li>
                <li><a href="/">a link</a></li>
                <li>a picture, where the note is a folder with the picture beside it</li>
              </ul>
            </div>
            <footer>
              <a href="/notes/note-2/"><time datetime="2026-10-05T16:20:00+01:00">5 Oct 2026, 16:20</time></a>
            </footer>
          </article>
        </li>
      </ol>
    </section>
    <section class="listing column" aria-labelledby="home-photos">
      <div class="heading wide">
        <h2 id="home-photos">Photos</h2>
        <a href="/photos/">See all</a>
      </div>
      <ul class="grid strip wide">
        <li><a href="/photos/photo-1/" aria-label="Photo 1"><picture><source type="image/avif" srcset="/photos/photo-1/photo-1-400.1823ccd1.avif 400w, /photos/photo-1/photo-1-800.3ee59d3c.avif 800w, /photos/photo-1/photo-1-900.480fa34c.avif 900w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-1/photo-1-900.ae7f342f.jpg" srcset="/photos/photo-1/photo-1-400.6e778706.jpg 400w, /photos/photo-1/photo-1-800.22c32ee8.jpg 800w, /photos/photo-1/photo-1-900.ae7f342f.jpg 900w" sizes="(max-width: 58rem) 33vw, 19rem" width="900" height="600" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-2/" aria-label="Photo 2"><picture><source type="image/avif" srcset="/photos/photo-2/photo-2-400.50c0d7ea.avif 400w, /photos/photo-2/photo-2-640.17ede13d.avif 640w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-2/photo-2-640.1d7c2a47.jpg" srcset="/photos/photo-2/photo-2-400.aa332bbe.jpg 400w, /photos/photo-2/photo-2-640.1d7c2a47.jpg 640w" sizes="(max-width: 58rem) 33vw, 19rem" width="640" height="800" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-3/" aria-label="Photo 3"><picture><source type="image/avif" srcset="/photos/photo-3/photo-3-400.5b880215.avif 400w, /photos/photo-3/photo-3-800.d3636869.avif 800w, /photos/photo-3/photo-3-900.e01a5e0f.avif 900w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-3/photo-3-900.473747b1.jpg" srcset="/photos/photo-3/photo-3-400.906615a6.jpg 400w, /photos/photo-3/photo-3-800.920f5463.jpg 800w, /photos/photo-3/photo-3-900.473747b1.jpg 900w" sizes="(max-width: 58rem) 33vw, 19rem" width="900" height="600" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-4/" aria-label="Photo 4"><picture><source type="image/avif" srcset="/photos/photo-4/photo-4-400.8c25d70d.avif 400w, /photos/photo-4/photo-4-800.28ab74b9.avif 800w, /photos/photo-4/photo-4-900.60eb75d1.avif 900w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-4/photo-4-900.3df07fe2.jpg" srcset="/photos/photo-4/photo-4-400.b9df2a3f.jpg 400w, /photos/photo-4/photo-4-800.7eca1c28.jpg 800w, /photos/photo-4/photo-4-900.3df07fe2.jpg 900w" sizes="(max-width: 58rem) 33vw, 19rem" width="900" height="600" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-5/" aria-label="Photo 5"><picture><source type="image/avif" srcset="/photos/photo-5/photo-5-400.3fb83992.avif 400w, /photos/photo-5/photo-5-640.68722789.avif 640w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-5/photo-5-640.f6ecd343.jpg" srcset="/photos/photo-5/photo-5-400.d941c06e.jpg 400w, /photos/photo-5/photo-5-640.f6ecd343.jpg 640w" sizes="(max-width: 58rem) 33vw, 19rem" width="640" height="800" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-6/" aria-label="Photo 6"><picture><source type="image/avif" srcset="/photos/photo-6/photo-6-400.35cc8b05.avif 400w, /photos/photo-6/photo-6-800.b736c58b.avif 800w, /photos/photo-6/photo-6-900.2a00d969.avif 900w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-6/photo-6-900.b4ce3140.jpg" srcset="/photos/photo-6/photo-6-400.20b40536.jpg 400w, /photos/photo-6/photo-6-800.59161668.jpg 800w, /photos/photo-6/photo-6-900.b4ce3140.jpg 900w" sizes="(max-width: 58rem) 33vw, 19rem" width="900" height="600" alt="" loading="lazy" decoding="async"></picture></a></li>
      </ul>
    </section>
  </main>
  <footer class="wide">
    <p><a href="/feed.xml">RSS feed</a></p>
    <p class="rights">© 2026 Site 1</p>
  </footer>
</body>
</html>
"##;
const POST: &str = r##"<!doctype html>
<html lang="en-GB" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>First post - Site 1</title>
  <meta name="description" content="This is the site’s first post, the file content/posts/first-post/index.md. Its picture is photo-1.jpg, kept beside it and named by its cover: line, and what a reader who can’t see the picture is told in its place is in photo-1.jpg.yaml. Change them or remove them as you like.">
  <link rel="stylesheet" href="/site.e1b305c9.css">
  <link rel="stylesheet" href="/code.644d7a9d.css">
  <link rel="alternate" type="application/atom+xml" href="/feed.xml" title="Site 1">
  <meta property="og:title" content="First post">
  <meta property="og:type" content="article">
  <meta property="og:site_name" content="Site 1">
  <meta property="og:locale" content="en_GB">
  <meta property="og:description" content="This is the site’s first post, the file content/posts/first-post/index.md. Its picture is photo-1.jpg, kept beside it and named by its cover: line, and what a reader who can’t see the picture is told in its place is in photo-1.jpg.yaml. Change them or remove them as you like.">
  <meta property="og:url" content="https://example.org/posts/first-post/">
</head>
<body>
  <a class="skip" href="#content">Skip to the content</a>
  <header class="wide">
    <p class="name"><a href="/">Site 1</a></p>
    <nav aria-label="Site">
      <ul>
        <li><a href="/">Home</a></li>
        <li><a href="/posts/" aria-current="true">Posts</a></li>
        <li><a href="/notes/">Notes</a></li>
        <li><a href="/photos/">Photos</a></li>
        <li><a href="/about/">About</a></li>
        <li><a href="/search/">Search</a></li>
      </ul>
    </nav>
  </header>
  <main id="content">
    <article>
      <header class="top column">
        <h1>First post</h1>
        <ul class="about">
          <li><time datetime="2026-10-06T00:00:00+01:00">6 October 2026</time></li>
          <li>1 min read</li>
        </ul>
      </header>
      <div class="writing column">
        <figure class="wide"><picture><source type="image/avif" srcset="/posts/first-post/photo-1-400.6ebf7d7e.avif 400w, /posts/first-post/photo-1-800.0f4dfc73.avif 800w" sizes="(max-width: 46rem) 100vw, 46rem"><img src="/posts/first-post/photo-1-800.b99b7f22.jpg" srcset="/posts/first-post/photo-1-400.8d7f3a18.jpg 400w, /posts/first-post/photo-1-800.b99b7f22.jpg 800w" sizes="(max-width: 46rem) 100vw, 46rem" width="800" height="533" alt="A gradient from blue to orange" loading="eager" decoding="async"></picture></figure>
        <p>This is the site’s first post, the file <code>content/posts/first-post/index.md</code>. Its picture is <code>photo-1.jpg</code>, kept beside it and named by its <code>cover:</code> line, and what a reader who can’t see the picture is told in its place is in <code>photo-1.jpg.yaml</code>. Change them or remove them as you like.</p>
        <p>In the file, a <code>&lt;!--more--&gt;</code> line stands above this paragraph and marks where the post’s opening ends: a list of posts shows the post as far as there, with a link to the rest. A post without the line is shown as far as the end of its first paragraph.</p>
        <p>To write the next post, add a file beside this folder in <code>content/posts/</code>, such as <code>second-post.md</code>, opening with its title and its date:</p>
        <pre class="code"><code class="language-yaml">---
<span class="hl-property">title</span><span class="hl-punctuation">:</span> <span class="hl-string">Second post</span>
<span class="hl-property">date</span><span class="hl-punctuation">:</span> <span class="hl-string">=today</span>
---</code></pre>
        <p>Then write the post below that, in Markdown. <code>statac serve</code> puts the day in place of <code>=today</code> when you save. A post with a picture is a folder like this one, its writing in <code>index.md</code> and the picture beside it. A post in <code>content/posts/</code> is listed on the home page, the newest first.</p>
      </div>
      <footer class="filed column">
        <div>
          <ul class="tags" aria-label="Tags">
            <li><a href="/tags/tag-1/" rel="tag">Tag 1</a></li>
          </ul>
        </div>
      </footer>
    </article>
    <nav class="more wide" aria-label="More posts">
      <ul>
        <li><a href="/posts/what-a-post-can-hold/" rel="next"><span class="label">Next</span> <strong>What a post can hold</strong></a></li>
      </ul>
    </nav>
  </main>
  <footer class="wide">
    <p><a href="/feed.xml">RSS feed</a></p>
    <p class="rights">© 2026 Site 1</p>
  </footer>
</body>
</html>
"##;
const NOTES: &str = r##"<!doctype html>
<html lang="en-GB" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Notes - Site 1</title>
  <meta name="description" content="Short notes, with or without a title, newest first.">
  <link rel="stylesheet" href="/site.e1b305c9.css">
  <link rel="stylesheet" href="/code.644d7a9d.css">
  <link rel="alternate" type="application/atom+xml" href="/feed.xml" title="Site 1">
  <meta property="og:title" content="Notes">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Site 1">
  <meta property="og:locale" content="en_GB">
  <meta property="og:description" content="Short notes, with or without a title, newest first.">
  <meta property="og:url" content="https://example.org/notes/">
</head>
<body>
  <a class="skip" href="#content">Skip to the content</a>
  <header class="wide">
    <p class="name"><a href="/">Site 1</a></p>
    <nav aria-label="Site">
      <ul>
        <li><a href="/">Home</a></li>
        <li><a href="/posts/">Posts</a></li>
        <li><a href="/notes/" aria-current="page">Notes</a></li>
        <li><a href="/photos/">Photos</a></li>
        <li><a href="/about/">About</a></li>
        <li><a href="/search/">Search</a></li>
      </ul>
    </nav>
  </header>
  <main id="content">
    <header class="top column">
      <h1>Notes</h1>
      <p class="summary">Short notes, with or without a title, newest first.</p>
    </header>
    <div class="listing column">
      <ol class="notes">
        <li>
          <article>
            <div class="writing">
              <p>This is a note, the file <code>content/notes/note-1.md</code>: a few lines with a date and no title. Its front matter holds only the date.</p>
            </div>
            <footer>
              <a href="/notes/note-1/"><time datetime="2026-10-06T09:12:00+01:00">6 Oct 2026, 09:12</time></a>
              <ul class="tags" aria-label="Tags">
                <li><a href="/tags/examples/" rel="tag">Examples</a></li>
              </ul>
            </footer>
          </article>
        </li>
        <li>
          <article>
            <div class="writing">
              <p>A note can hold anything a post can:</p>
              <ul>
                <li>a list</li>
                <li><a href="/">a link</a></li>
                <li>a picture, where the note is a folder with the picture beside it</li>
              </ul>
            </div>
            <footer>
              <a href="/notes/note-2/"><time datetime="2026-10-05T16:20:00+01:00">5 Oct 2026, 16:20</time></a>
            </footer>
          </article>
        </li>
        <li>
          <article>
            <h2><a href="/notes/note-3/">A note with a title</a></h2>
            <div class="writing">
              <p>A note may have a title, and then the title is shown with its date. To do without notes, delete the folder <code>content/notes/</code>.</p>
            </div>
            <footer>
              <a href="/notes/note-3/"><time datetime="2026-10-04T07:40:00+01:00">4 Oct 2026, 07:40</time></a>
            </footer>
          </article>
        </li>
      </ol>
    </div>
  </main>
  <footer class="wide">
    <p><a href="/feed.xml">RSS feed</a></p>
    <p class="rights">© 2026 Site 1</p>
  </footer>
</body>
</html>
"##;
const NOTE: &str = r##"<!doctype html>
<html lang="en-GB" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>6 Oct 2026, 09:12 - Site 1</title>
  <meta name="description" content="This is a note, the file content/notes/note-1.md: a few lines with a date and no title. Its front matter holds only the date.">
  <link rel="stylesheet" href="/site.e1b305c9.css">
  <link rel="stylesheet" href="/code.644d7a9d.css">
  <link rel="alternate" type="application/atom+xml" href="/feed.xml" title="Site 1">
  <meta property="og:title" content="6 Oct 2026, 09:12">
  <meta property="og:type" content="article">
  <meta property="og:site_name" content="Site 1">
  <meta property="og:locale" content="en_GB">
  <meta property="og:description" content="This is a note, the file content/notes/note-1.md: a few lines with a date and no title. Its front matter holds only the date.">
  <meta property="og:url" content="https://example.org/notes/note-1/">
</head>
<body>
  <a class="skip" href="#content">Skip to the content</a>
  <header class="wide">
    <p class="name"><a href="/">Site 1</a></p>
    <nav aria-label="Site">
      <ul>
        <li><a href="/">Home</a></li>
        <li><a href="/posts/">Posts</a></li>
        <li><a href="/notes/" aria-current="true">Notes</a></li>
        <li><a href="/photos/">Photos</a></li>
        <li><a href="/about/">About</a></li>
        <li><a href="/search/">Search</a></li>
      </ul>
    </nav>
  </header>
  <main id="content">
    <article>
      <header class="top column">
        <h1 class="stamp"><time datetime="2026-10-06T09:12:00+01:00">6 Oct 2026, 09:12</time></h1>
      </header>
      <div class="writing column">
        <p>This is a note, the file <code>content/notes/note-1.md</code>: a few lines with a date and no title. Its front matter holds only the date.</p>
      </div>
      <footer class="filed column">
        <div>
          <ul class="tags" aria-label="Tags">
            <li><a href="/tags/examples/" rel="tag">Examples</a></li>
          </ul>
        </div>
      </footer>
    </article>
    <nav class="more wide" aria-label="More posts">
      <ul>
        <li><a href="/notes/note-2/" rel="next"><span class="label">Next</span> <strong>5 Oct 2026, 16:20</strong></a></li>
      </ul>
    </nav>
  </main>
  <footer class="wide">
    <p><a href="/feed.xml">RSS feed</a></p>
    <p class="rights">© 2026 Site 1</p>
  </footer>
</body>
</html>
"##;
const PHOTOS: &str = r##"<!doctype html>
<html lang="en-GB" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Photos - Site 1</title>
  <meta name="description" content="One photograph to a page, newest first.">
  <link rel="stylesheet" href="/site.e1b305c9.css">
  <link rel="stylesheet" href="/code.644d7a9d.css">
  <link rel="alternate" type="application/atom+xml" href="/feed.xml" title="Site 1">
  <meta property="og:title" content="Photos">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Site 1">
  <meta property="og:locale" content="en_GB">
  <meta property="og:description" content="One photograph to a page, newest first.">
  <meta property="og:url" content="https://example.org/photos/">
</head>
<body>
  <a class="skip" href="#content">Skip to the content</a>
  <header class="wide">
    <p class="name"><a href="/">Site 1</a></p>
    <nav aria-label="Site">
      <ul>
        <li><a href="/">Home</a></li>
        <li><a href="/posts/">Posts</a></li>
        <li><a href="/notes/">Notes</a></li>
        <li><a href="/photos/" aria-current="page">Photos</a></li>
        <li><a href="/about/">About</a></li>
        <li><a href="/search/">Search</a></li>
      </ul>
    </nav>
  </header>
  <main id="content">
    <header class="top column">
      <h1>Photos</h1>
      <p class="summary">One photograph to a page, newest first.</p>
    </header>
    <div class="listing column">
      <ul class="grid wide">
        <li><a href="/photos/photo-1/" aria-label="Photo 1"><picture><source type="image/avif" srcset="/photos/photo-1/photo-1-400.1823ccd1.avif 400w, /photos/photo-1/photo-1-800.3ee59d3c.avif 800w, /photos/photo-1/photo-1-900.480fa34c.avif 900w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-1/photo-1-900.ae7f342f.jpg" srcset="/photos/photo-1/photo-1-400.6e778706.jpg 400w, /photos/photo-1/photo-1-800.22c32ee8.jpg 800w, /photos/photo-1/photo-1-900.ae7f342f.jpg 900w" sizes="(max-width: 58rem) 33vw, 19rem" width="900" height="600" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-2/" aria-label="Photo 2"><picture><source type="image/avif" srcset="/photos/photo-2/photo-2-400.50c0d7ea.avif 400w, /photos/photo-2/photo-2-640.17ede13d.avif 640w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-2/photo-2-640.1d7c2a47.jpg" srcset="/photos/photo-2/photo-2-400.aa332bbe.jpg 400w, /photos/photo-2/photo-2-640.1d7c2a47.jpg 640w" sizes="(max-width: 58rem) 33vw, 19rem" width="640" height="800" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-3/" aria-label="Photo 3"><picture><source type="image/avif" srcset="/photos/photo-3/photo-3-400.5b880215.avif 400w, /photos/photo-3/photo-3-800.d3636869.avif 800w, /photos/photo-3/photo-3-900.e01a5e0f.avif 900w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-3/photo-3-900.473747b1.jpg" srcset="/photos/photo-3/photo-3-400.906615a6.jpg 400w, /photos/photo-3/photo-3-800.920f5463.jpg 800w, /photos/photo-3/photo-3-900.473747b1.jpg 900w" sizes="(max-width: 58rem) 33vw, 19rem" width="900" height="600" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-4/" aria-label="Photo 4"><picture><source type="image/avif" srcset="/photos/photo-4/photo-4-400.8c25d70d.avif 400w, /photos/photo-4/photo-4-800.28ab74b9.avif 800w, /photos/photo-4/photo-4-900.60eb75d1.avif 900w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-4/photo-4-900.3df07fe2.jpg" srcset="/photos/photo-4/photo-4-400.b9df2a3f.jpg 400w, /photos/photo-4/photo-4-800.7eca1c28.jpg 800w, /photos/photo-4/photo-4-900.3df07fe2.jpg 900w" sizes="(max-width: 58rem) 33vw, 19rem" width="900" height="600" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-5/" aria-label="Photo 5"><picture><source type="image/avif" srcset="/photos/photo-5/photo-5-400.3fb83992.avif 400w, /photos/photo-5/photo-5-640.68722789.avif 640w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-5/photo-5-640.f6ecd343.jpg" srcset="/photos/photo-5/photo-5-400.d941c06e.jpg 400w, /photos/photo-5/photo-5-640.f6ecd343.jpg 640w" sizes="(max-width: 58rem) 33vw, 19rem" width="640" height="800" alt="" loading="lazy" decoding="async"></picture></a></li>
        <li><a href="/photos/photo-6/" aria-label="Photo 6"><picture><source type="image/avif" srcset="/photos/photo-6/photo-6-400.35cc8b05.avif 400w, /photos/photo-6/photo-6-800.b736c58b.avif 800w, /photos/photo-6/photo-6-900.2a00d969.avif 900w" sizes="(max-width: 58rem) 33vw, 19rem"><img src="/photos/photo-6/photo-6-900.b4ce3140.jpg" srcset="/photos/photo-6/photo-6-400.20b40536.jpg 400w, /photos/photo-6/photo-6-800.59161668.jpg 800w, /photos/photo-6/photo-6-900.b4ce3140.jpg 900w" sizes="(max-width: 58rem) 33vw, 19rem" width="900" height="600" alt="" loading="lazy" decoding="async"></picture></a></li>
      </ul>
    </div>
  </main>
  <footer class="wide">
    <p><a href="/feed.xml">RSS feed</a></p>
    <p class="rights">© 2026 Site 1</p>
  </footer>
</body>
</html>
"##;
const PHOTO: &str = r##"<!doctype html>
<html lang="en-GB" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Photo 1 - Site 1</title>
  <meta name="description" content="This is a photograph’s page, the file content/photos/photo-1/index.md, with the picture beside it and named by its cover: line. Write about the picture here. To do without photographs, delete the folder content/photos/.">
  <link rel="stylesheet" href="/site.e1b305c9.css">
  <link rel="stylesheet" href="/code.644d7a9d.css">
  <link rel="alternate" type="application/atom+xml" href="/feed.xml" title="Site 1">
  <meta property="og:title" content="Photo 1">
  <meta property="og:type" content="article">
  <meta property="og:site_name" content="Site 1">
  <meta property="og:locale" content="en_GB">
  <meta property="og:description" content="This is a photograph’s page, the file content/photos/photo-1/index.md, with the picture beside it and named by its cover: line. Write about the picture here. To do without photographs, delete the folder content/photos/.">
  <meta property="og:url" content="https://example.org/photos/photo-1/">
</head>
<body>
  <a class="skip" href="#content">Skip to the content</a>
  <header class="wide">
    <p class="name"><a href="/">Site 1</a></p>
    <nav aria-label="Site">
      <ul>
        <li><a href="/">Home</a></li>
        <li><a href="/posts/">Posts</a></li>
        <li><a href="/notes/">Notes</a></li>
        <li><a href="/photos/" aria-current="true">Photos</a></li>
        <li><a href="/about/">About</a></li>
        <li><a href="/search/">Search</a></li>
      </ul>
    </nav>
  </header>
  <main id="content">
    <article>
      <div class="column">
        <figure class="shot wide"><picture><source type="image/avif" srcset="/photos/photo-1/photo-1-400.1823ccd1.avif 400w, /photos/photo-1/photo-1-800.3ee59d3c.avif 800w, /photos/photo-1/photo-1-900.480fa34c.avif 900w" sizes="(max-width: 46rem) 100vw, 46rem"><img src="/photos/photo-1/photo-1-900.ae7f342f.jpg" srcset="/photos/photo-1/photo-1-400.6e778706.jpg 400w, /photos/photo-1/photo-1-800.22c32ee8.jpg 800w, /photos/photo-1/photo-1-900.ae7f342f.jpg 900w" sizes="(max-width: 46rem) 100vw, 46rem" width="900" height="600" alt="A gradient from green to cream" loading="eager" decoding="async"></picture></figure>
      </div>
      <header class="top under column">
        <h1>Photo 1</h1>
        <ul class="about">
          <li><time datetime="2026-10-03T00:00:00+01:00">3 October 2026</time></li>
        </ul>
      </header>
      <div class="writing column">
        <p>This is a photograph’s page, the file <code>content/photos/photo-1/index.md</code>, with the picture beside it and named by its <code>cover:</code> line. Write about the picture here. To do without photographs, delete the folder <code>content/photos/</code>.</p>
      </div>
    </article>
    <nav class="more wide" aria-label="More posts">
      <ul>
        <li><a href="/photos/photo-2/" rel="next"><span class="label">Next</span> <strong>Photo 2</strong></a></li>
      </ul>
    </nav>
  </main>
  <footer class="wide">
    <p><a href="/feed.xml">RSS feed</a></p>
    <p class="rights">© 2026 Site 1</p>
  </footer>
</body>
</html>
"##;
const TAG: &str = r##"<!doctype html>
<html lang="en-GB" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Tag 1 - Site 1</title>
  <link rel="stylesheet" href="/site.e1b305c9.css">
  <link rel="stylesheet" href="/code.644d7a9d.css">
  <link rel="alternate" type="application/atom+xml" href="/feed.xml" title="Site 1">
  <meta property="og:title" content="Tag 1">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="Site 1">
  <meta property="og:locale" content="en_GB">
  <meta property="og:url" content="https://example.org/tags/tag-1/">
</head>
<body>
  <a class="skip" href="#content">Skip to the content</a>
  <header class="wide">
    <p class="name"><a href="/">Site 1</a></p>
    <nav aria-label="Site">
      <ul>
        <li><a href="/">Home</a></li>
        <li><a href="/posts/">Posts</a></li>
        <li><a href="/notes/">Notes</a></li>
        <li><a href="/photos/">Photos</a></li>
        <li><a href="/about/">About</a></li>
        <li><a href="/search/">Search</a></li>
      </ul>
    </nav>
  </header>
  <main id="content">
    <header class="top column">
      <h1>Posts tagged Tag 1</h1>
      <p><a href="/tags/">All tags</a></p>
    </header>
    <div class="listing column">
      <section class="year">
        <h2>2026</h2>
        <ol class="posts">
          <li class="entry">
            <time datetime="2026-10-06T00:00:00+01:00">6 Oct</time>
            <div>
              <h3><a href="/posts/first-post/">First post</a></h3>
              <picture><source type="image/avif" srcset="/posts/first-post/photo-1-400.6ebf7d7e.avif 400w, /posts/first-post/photo-1-800.0f4dfc73.avif 800w" sizes="(max-width: 46rem) 100vw, 46rem"><img src="/posts/first-post/photo-1-800.b99b7f22.jpg" srcset="/posts/first-post/photo-1-400.8d7f3a18.jpg 400w, /posts/first-post/photo-1-800.b99b7f22.jpg 800w" sizes="(max-width: 46rem) 100vw, 46rem" width="800" height="533" alt="" loading="lazy" decoding="async"></picture>
              <p>This is the site’s first post, the file <code>content/posts/first-post/index.md</code>. Its picture is <code>photo-1.jpg</code>, kept beside it and named by its <code>cover:</code> line, and what a reader who can’t see the picture is told in its place is in <code>photo-1.jpg.yaml</code>. Change them or remove them as you like.</p>
              <p><a href="/posts/first-post/">Read more</a></p>
            </div>
          </li>
        </ol>
      </section>
    </div>
  </main>
  <footer class="wide">
    <p><a href="/feed.xml">RSS feed</a></p>
    <p class="rights">© 2026 Site 1</p>
  </footer>
</body>
</html>
"##;
const FEED: &str = r##"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom" xml:lang="en-GB">
  <title>Notes - Site 1</title>
  <link rel="self" href="https://example.org/notes/feed.xml"/>
  <link rel="alternate" href="https://example.org/notes/"/>
  <id>urn:uuid:b69028dc-c9b3-8149-a1fb-4becd82b31a2</id>
  <updated>2026-10-06T09:12:00+01:00</updated>
  <author><name>Site 1</name></author>
  <entry>
    <title>6 Oct 2026, 09:12</title>
    <link rel="alternate" href="https://example.org/notes/note-1/"/>
    <id>urn:uuid:bd0031e2-b245-8056-ab49-b273bc230903</id>
    <published>2026-10-06T09:12:00+01:00</published>
    <updated>2026-10-06T09:12:00+01:00</updated>
    <category term="Examples"/>
    <content type="html">&lt;p&gt;This is a note, the file &lt;code&gt;content/notes/note-1.md&lt;/code&gt;: a few lines with a date and no title. Its front matter holds only the date.&lt;/p&gt;</content>
  </entry>
  <entry>
    <title>5 Oct 2026, 16:20</title>
    <link rel="alternate" href="https://example.org/notes/note-2/"/>
    <id>urn:uuid:a696d899-4434-8575-a568-01fb2037e613</id>
    <published>2026-10-05T16:20:00+01:00</published>
    <updated>2026-10-05T16:20:00+01:00</updated>
    <content type="html">&lt;p&gt;A note can hold anything a post can:&lt;/p&gt;
&lt;ul&gt;
  &lt;li&gt;a list&lt;/li&gt;
  &lt;li&gt;&lt;a href=&quot;https://example.org/&quot;&gt;a link&lt;/a&gt;&lt;/li&gt;
  &lt;li&gt;a picture, where the note is a folder with the picture beside it&lt;/li&gt;
&lt;/ul&gt;</content>
  </entry>
  <entry>
    <title>A note with a title</title>
    <link rel="alternate" href="https://example.org/notes/note-3/"/>
    <id>urn:uuid:b19d83e3-b832-8bf7-8e36-ce2d09598c13</id>
    <published>2026-10-04T07:40:00+01:00</published>
    <updated>2026-10-04T07:40:00+01:00</updated>
    <content type="html">&lt;p&gt;A note may have a title, and then the title is shown with its date. To do without notes, delete the folder &lt;code&gt;content/notes/&lt;/code&gt;.&lt;/p&gt;</content>
  </entry>
</feed>
"##;
const SITEMAP: &str = r##"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"
  xmlns:xhtml="http://www.w3.org/1999/xhtml">
  <url>
    <loc>https://example.org/</loc>
  </url>
  <url>
    <loc>https://example.org/about/</loc>
  </url>
  <url>
    <loc>https://example.org/notes/</loc>
  </url>
  <url>
    <loc>https://example.org/notes/note-1/</loc>
  </url>
  <url>
    <loc>https://example.org/notes/note-2/</loc>
  </url>
  <url>
    <loc>https://example.org/notes/note-3/</loc>
  </url>
  <url>
    <loc>https://example.org/photos/</loc>
  </url>
  <url>
    <loc>https://example.org/photos/photo-1/</loc>
  </url>
  <url>
    <loc>https://example.org/photos/photo-2/</loc>
  </url>
  <url>
    <loc>https://example.org/photos/photo-3/</loc>
  </url>
  <url>
    <loc>https://example.org/photos/photo-4/</loc>
  </url>
  <url>
    <loc>https://example.org/photos/photo-5/</loc>
  </url>
  <url>
    <loc>https://example.org/photos/photo-6/</loc>
  </url>
  <url>
    <loc>https://example.org/posts/</loc>
  </url>
  <url>
    <loc>https://example.org/posts/first-post/</loc>
  </url>
  <url>
    <loc>https://example.org/posts/what-a-post-can-hold/</loc>
  </url>
  <url>
    <loc>https://example.org/search/</loc>
  </url>
  <url>
    <loc>https://example.org/tags/</loc>
  </url>
  <url>
    <loc>https://example.org/tags/examples/</loc>
  </url>
  <url>
    <loc>https://example.org/tags/tag-1/</loc>
  </url>
</urlset>
"##;
const NOT_FOUND: &str = r##"<!doctype html>
<html lang="en-GB" dir="ltr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Page not found - Site 1</title>
  <link rel="stylesheet" href="/site.e1b305c9.css">
  <link rel="stylesheet" href="/code.644d7a9d.css">
  <link rel="alternate" type="application/atom+xml" href="/feed.xml" title="Site 1">
  <meta name="robots" content="noindex">
</head>
<body>
  <a class="skip" href="#content">Skip to the content</a>
  <header class="wide">
    <p class="name"><a href="/">Site 1</a></p>
    <nav aria-label="Site">
      <ul>
        <li><a href="/">Home</a></li>
        <li><a href="/posts/">Posts</a></li>
        <li><a href="/notes/">Notes</a></li>
        <li><a href="/photos/">Photos</a></li>
        <li><a href="/about/">About</a></li>
        <li><a href="/search/">Search</a></li>
      </ul>
    </nav>
  </header>
  <main id="content">
    <header class="top column">
      <h1>Page not found</h1>
      <p class="summary">Nothing is published at this address.</p>
    </header>
    <div class="writing column">
      <p><a href="/">Go to the home page</a></p>
    </div>
  </main>
  <footer class="wide">
    <p><a href="/feed.xml">RSS feed</a></p>
    <p class="rights">© 2026 Site 1</p>
  </footer>
</body>
</html>
"##;
