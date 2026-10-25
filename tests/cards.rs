//! Sharing cards in small sites built end to end: which page gets which
//! card, what a layout reads of it, and the cards kept between builds.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::{Rgb, RgbImage};

const LATIN: &[u8] = include_bytes!("../src/cards/fonts/test/test-latin.ttf");
const LETTERS: &[u8] = include_bytes!("../src/cards/fonts/test/test-latin-letters-only.ttf");
const EMOJI: &[u8] = include_bytes!("../src/cards/fonts/test/test-emoji.ttf");

/// A layout writing what a link preview reads of a page.
const LAYOUT: &str = "<!doctype html>
<title>{{ page.og_title }}</title>
{% if page.card is defined %}
<meta property=\"og:image\" content=\"{{ page.card.url | absolute }}\">
{% if page.card.width %}
<meta property=\"og:image:width\" content=\"{{ page.card.width }}\">
<meta property=\"og:image:height\" content=\"{{ page.card.height }}\">
{% endif %}
<meta property=\"og:image:alt\" content=\"{{ page.card.alt }}\">
{% endif %}
{{ page.content }}
";

/// A card with the page's title in the middle and the site's at the
/// bottom.
const CARD: &str = "background: background.png
font: text.ttf

title.text: \"{{ page.og_title }}\"
title.place: centre
title.size: up to 75
title.colour: \"#F2E9D3\"

footer.text: \"{{ site.title }}\"
footer.place: bottom
footer.size: 36
footer.colour: \"#B9A88F\"
";

fn png_of(image: &RgbImage) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}

/// A background of one colour, `width` by `height`.
fn flat(width: u32, height: u32) -> Vec<u8> {
    png_of(&RgbImage::from_pixel(width, height, Rgb([20, 30, 40])))
}

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let pixels = RgbImage::from_fn(width, height, |x, y| {
        Rgb([(x * 255 / width) as u8, (y * 255 / height) as u8, 120])
    });
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 90)
        .encode(
            &pixels,
            width as u16,
            height as u16,
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();
    out
}

struct Site {
    root: PathBuf,
}

impl Site {
    /// A site with an address, a home page, a post, one layout for every
    /// page and a card described.
    fn new(name: &str) -> Site {
        let site = Site::bare(name);
        site.write("templates/og.yaml", CARD);
        site.put("templates/og/background.png", &flat(1200, 630));
        site.put("templates/og/text.ttf", LATIN);
        site
    }

    /// The same site with no card described.
    fn bare(name: &str) -> Site {
        let root = std::env::temp_dir().join(format!("statac-cards-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let site = Site { root };
        site.write(
            "statac.yaml",
            "title: Site\nurl: https://example.org\nlanguage: en\ntimezone: UTC\n\
             pages.feed: false\n",
        );
        site.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        site.write("content/post-1.md", "---\ntitle: Post 1\n---\nA post.\n");
        for layout in ["home", "page", "folder"] {
            site.write(&format!("templates/{layout}.html"), LAYOUT);
        }
        site
    }

    fn write(&self, path: &str, text: &str) {
        self.put(path, text.as_bytes());
    }

    fn put(&self, path: &str, bytes: &[u8]) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn out(&self) -> PathBuf {
        self.root.join("_site")
    }

    /// Runs statac with `args` and the site's folder, and gives what it
    /// said and its exit code. It must say nothing aside.
    fn run(&self, args: &[&str]) -> (String, i32) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(args)
            .arg(&self.root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        let shown = self.root.display().to_string();
        let said = String::from_utf8(output.stdout).unwrap();
        let aside = String::from_utf8(output.stderr).unwrap();
        assert_eq!(aside, "", "{said}");
        (said.replace(&shown, "SITE"), output.status.code().unwrap())
    }

    /// Builds the site, which must build, and gives what it said.
    fn build(&self) -> String {
        let (said, code) = self.run(&["build", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{said}");
        said
    }

    /// What `check` says of the site.
    fn check(&self) -> String {
        self.run(&["check", "--today=2026-10-04"]).0
    }

    fn html(&self, file: &str) -> String {
        fs::read_to_string(self.out().join(file)).unwrap()
    }

    /// The cards published, each by its file in the built site.
    fn cards(&self) -> Vec<String> {
        names(&self.out())
            .into_iter()
            .filter(|name| name.rsplit('/').next().unwrap().starts_with("og."))
            .collect()
    }

    /// The cards kept between builds.
    fn kept(&self) -> Vec<String> {
        names(&self.root.join(".statac/cards"))
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Every file under `folder`, by its path with forward slashes.
fn names(folder: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut folders = vec![folder.to_path_buf()];
    while let Some(next) = folders.pop() {
        let Ok(entries) = fs::read_dir(&next) else {
            continue;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else {
                let rel = path.strip_prefix(folder).unwrap();
                found.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    found.sort();
    found
}

/// What the `<meta>` line of `html` naming `property` holds.
fn meta(html: &str, property: &str) -> Option<String> {
    let opening = format!("<meta property=\"{property}\" content=\"");
    let line = html.lines().find(|line| line.starts_with(&opening))?;
    let content = line.strip_prefix(&opening)?.strip_suffix("\">")?;
    Some(content.to_owned())
}

/// The problems `said` draws, each as its headline, where it is, the text
/// under its caret and what it suggests.
fn told(said: &str) -> Vec<String> {
    said.lines()
        .filter(|line| {
            ["error  ", "warning  ", "note  "]
                .iter()
                .any(|severity| line.starts_with(severity))
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_page_whose_layout_asks_gets_a_card_beside_it() {
    let site = Site::new("drawn");
    let said = site.build();
    assert!(said.contains("Drew 2 cards."), "{said}");
    assert!(said.contains("2 pages and 2 other files"), "{said}");

    let cards = site.cards();
    assert_eq!(cards.len(), 2, "{cards:?}");
    let post = site.html("post-1/index.html");
    let address = meta(&post, "og:image").unwrap();
    let file = address.strip_prefix("https://example.org/").unwrap();
    assert!(
        file.starts_with("post-1/og.") && file.ends_with(".png"),
        "{address}"
    );
    assert!(cards.contains(&file.to_owned()), "{cards:?}");
    assert_eq!(meta(&post, "og:image:width").unwrap(), "1200");
    assert_eq!(meta(&post, "og:image:height").unwrap(), "630");
    assert_eq!(meta(&post, "og:image:alt").unwrap(), "Post 1. Site");

    let bytes = fs::read(site.out().join(file)).unwrap();
    let drawn = image::load_from_memory(&bytes).unwrap().into_rgb8();
    assert_eq!(drawn.dimensions(), (1200, 630));
    // The title is drawn across the middle, over the background.
    let ground = Rgb([20, 30, 40]);
    assert_eq!(*drawn.get_pixel(10, 10), ground);
    assert!((500..700).any(|x| (280..350).any(|y| *drawn.get_pixel(x, y) != ground)));

    let home = site.html("index.html");
    let address = meta(&home, "og:image").unwrap();
    assert!(address.starts_with("https://example.org/og."), "{address}");
    assert_eq!(meta(&home, "og:image:alt").unwrap(), "Home. Site");
    assert_eq!(site.check(), "No problems.\n");
}

#[test]
fn a_site_published_under_a_path_names_its_cards_there() {
    let site = Site::new("path");
    site.write(
        "statac.yaml",
        "title: Site\nurl: https://example.org/blog/\nlanguage: en\ntimezone: UTC\n\
         pages.feed: false\n",
    );
    site.build();
    let post = site.html("post-1/index.html");
    let address = meta(&post, "og:image").unwrap();
    let file = address
        .strip_prefix("https://example.org/blog/")
        .unwrap_or_else(|| panic!("{address}"));
    assert!(site.cards().contains(&file.to_owned()), "{address}");
}

#[test]
fn a_site_with_no_address_draws_nothing_and_says_so() {
    let site = Site::new("no-address");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    let layout = LAYOUT.replace(" | absolute", "");
    for name in ["home", "page", "folder"] {
        site.write(&format!("templates/{name}.html"), &layout);
    }
    let said = site.build();
    assert!(!said.contains("Drew"), "{said}");
    assert!(said.contains("1 note: 1 no-site-url."), "{said}");
    assert_eq!(site.cards(), [""; 0]);
    assert_eq!(meta(&site.html("post-1/index.html"), "og:image"), None);
    assert_eq!(
        told(&site.check()),
        ["note  No site URL: social cards are left out until the site has a `url`"]
    );
}

#[test]
fn a_draft_has_no_card_until_it_is_built() {
    let site = Site::new("draft");
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndraft: true\n---\nA draft.\n",
    );
    let said = site.build();
    assert!(said.contains("Drew 2 cards."), "{said}");
    assert_eq!(site.cards().len(), 2);
    let (said, code) = site.run(&["build", "--drafts"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("Drew 1 card."), "{said}");
    let draft = site.html("post-2/index.html");
    assert_eq!(meta(&draft, "og:image:alt").unwrap(), "Post 2. Site");
    assert_eq!(site.cards().len(), 3);
}

#[test]
fn a_page_picks_another_card_and_a_folder_hands_one_down() {
    let site = Site::new("picked");
    site.write(
        "templates/og.plain.yaml",
        &CARD.replace("footer.size: 36", "footer.size: 40"),
    );
    site.write(
        "templates/og.notes.yaml",
        &CARD.replace("{{ site.title }}", "Notes"),
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ncard: plain\n---\nA post.\n",
    );
    site.write(
        "content/notes-1/index.md",
        "---\ntitle: Notes\npages.card: notes\n---\n",
    );
    site.write(
        "content/notes-1/note-1.md",
        "---\ntitle: Note 1\n---\nA note.\n",
    );
    let said = site.build();
    assert!(said.contains("Drew 4 cards."), "{said}");
    assert_eq!(site.check(), "No problems.\n");
    let alt = |file: &str| meta(&site.html(file), "og:image:alt").unwrap();
    assert_eq!(alt("notes-1/note-1/index.html"), "Note 1. Notes");
    assert_eq!(alt("notes-1/index.html"), "Notes. Site");
    assert_eq!(alt("post-1/index.html"), "Post 1. Site");

    // The same words on another card are another picture.
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\nA post.\n");
    let before = site.cards();
    let said = site.build();
    assert!(said.contains("Drew 1 card."), "{said}");
    assert!(
        said.contains("Removed 1 card no page is shared with any more from `.statac`."),
        "{said}"
    );
    assert_ne!(site.cards(), before);
    assert_eq!(
        told(&site.check()),
        ["warning  Unused card: no page picks `plain`"]
    );
}

#[test]
fn a_page_is_called_by_its_og_title_where_it_has_one() {
    let site = Site::new("og-title");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\nog_title: The first post\n---\nA post.\n",
    );
    site.build();
    let post = site.html("post-1/index.html");
    assert!(post.contains("<title>The first post</title>"), "{post}");
    assert_eq!(meta(&post, "og:image:alt").unwrap(), "The first post. Site");
    let home = site.html("index.html");
    assert!(home.contains("<title>Home</title>"), "{home}");
}

#[test]
fn a_page_with_no_title_is_noted_once_for_its_layout_and_its_card() {
    let site = Site::new("untitled");
    site.write("content/post-2.md", "Its opening words.\n");
    let said = site.check();
    let notes = told(&said);
    assert_eq!(notes.len(), 1, "{said}");
    assert!(notes[0].starts_with("note  Untitled page:"), "{said}");
    site.build();
    let post = site.html("post-2/index.html");
    assert_eq!(
        meta(&post, "og:image:alt").unwrap(),
        "Its opening words. Site"
    );
}

#[test]
fn a_page_naming_its_own_picture_is_shared_with_that() {
    let site = Site::new("own");
    // Shown by the page too, so published as copies; named by `og_image`,
    // so published as it is as well.
    site.write(
        "content/post-2/index.md",
        "---\ntitle: Post 2\nog_image: cover.jpg\n---\n![The cover](cover.jpg)\n",
    );
    site.put("content/post-2/cover.jpg", &jpeg(900, 600));
    site.write(
        "content/post-3.md",
        "---\ntitle: Post 3\nog_image: /shared/cover.png\n---\nA post.\n",
    );
    site.put("static/shared/cover.png", &flat(640, 320));
    site.write(
        "content/post-4/index.md",
        "---\ntitle: Post 4\nog_image: cover.png\n---\nA post.\n",
    );
    site.put("content/post-4/cover.png", &flat(320, 240));
    site.write(
        "content/post-4/cover.png.yaml",
        "alt: A flat colour, 320 by 240\n",
    );
    let said = site.build();
    // The home page and the first post alone have a card drawn.
    assert!(said.contains("Drew 2 cards."), "{said}");
    // Nothing is said of a JPEG or a PNG.
    assert_eq!(site.check(), "No problems.\n");

    let second = site.html("post-2/index.html");
    assert_eq!(
        meta(&second, "og:image").unwrap(),
        "https://example.org/post-2/cover.jpg"
    );
    assert_eq!(meta(&second, "og:image:width").unwrap(), "900");
    assert_eq!(meta(&second, "og:image:height").unwrap(), "600");
    assert_eq!(meta(&second, "og:image:alt").unwrap(), "Post 2");
    assert_eq!(
        fs::read(site.out().join("post-2/cover.jpg")).unwrap(),
        jpeg(900, 600)
    );
    assert!(second.contains("<picture>"), "{second}");

    let third = site.html("post-3/index.html");
    assert_eq!(
        meta(&third, "og:image").unwrap(),
        "https://example.org/shared/cover.png"
    );
    assert_eq!(meta(&third, "og:image:width").unwrap(), "640");

    let fourth = site.html("post-4/index.html");
    assert_eq!(
        meta(&fourth, "og:image").unwrap(),
        "https://example.org/post-4/cover.png"
    );
    assert_eq!(meta(&fourth, "og:image:width").unwrap(), "320");
    assert_eq!(meta(&fourth, "og:image:height").unwrap(), "240");
    assert_eq!(
        meta(&fourth, "og:image:alt").unwrap(),
        "A flat colour, 320 by 240"
    );
}

#[test]
fn a_picture_a_page_cannot_be_shared_with_stops_the_build() {
    let site = Site::new("own-refused");
    let post = |n: u32, named: &str| {
        site.write(
            &format!("content/post-{n}.md"),
            &format!("---\ntitle: Post {n}\nog_image: {named}\n---\nA post.\n"),
        );
    };
    post(1, "photo-1.jpg");
    post(2, "https://example.com/photo-2.jpg");
    post(3, "photo-3.jpg");
    site.put("content/photo-10.jpg", &jpeg(40, 30));
    site.put("content/photo-3.jpg", &jpeg(40, 30)[..12]);
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        told(&said),
        [
            "error  Missing file: there is no `content/photo-1.jpg`",
            "error  Wrong kind of value: `og_image` is an image of the site's, not \
             `https://example.com/photo-2.jpg`",
            "error  Unreadable image: `content/photo-3.jpg` is cut short or damaged",
        ]
    );
    assert!(said.contains("  Did you mean `photo-10.jpg`?"), "{said}");
    assert!(
        said.contains("  content/post-1.md, line 3\n   3 │ og_image: photo-1.jpg\n"),
        "{said}"
    );
    let (said, code) = site.run(&["build"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("Wrote nothing: 3 errors stopped the build."),
        "{said}"
    );
}

/// A WebP of one pixel.
const WEBP: &[u8] = b"RIFF\x1a\x00\x00\x00WEBPVP8L\x0d\x00\x00\x00\
                      \x2f\x00\x00\x00\x10\x07\x10\x11\x11\x88\x88\xfe\x07\x00";

/// A picture a page names as its own: its name, its bytes, and the width
/// and height its file gives.
type Own = (
    &'static str,
    &'static [u8],
    Option<(&'static str, &'static str)>,
);

#[test]
fn a_picture_of_any_kind_is_used_and_one_some_previews_may_not_show_is_warned_of() {
    let site = Site::new("own-any-kind");
    let measured = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"10\" height=\"20\"></svg>";
    let boundless = b"<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 10 20\"></svg>";
    let gif = b"GIF89a\x40\x01\xf0\x00\x00\x00\x00;";
    // The opening of a kind of picture Statac has no name for.
    let unnamed = b"BM\x46\x00\x00\x00\x00\x00\x00\x00\x36\x00\x00\x00";
    // A WebP named as a PNG is: the file's bytes say what it is.
    let pictures: [Own; 5] = [
        ("photo-1.png", WEBP, Some(("1", "1"))),
        ("drawing-2.svg", measured, Some(("10", "20"))),
        ("drawing-3.svg", boundless, None),
        ("moving-4.gif", gif, Some(("320", "240"))),
        ("scan-5.bmp", unnamed, None),
    ];
    for (n, (name, bytes, _)) in pictures.iter().enumerate() {
        let n = n + 1;
        site.put(&format!("content/{name}"), bytes);
        site.write(
            &format!("content/post-{n}.md"),
            &format!("---\ntitle: Post {n}\nog_image: {name}\n---\nA post.\n"),
        );
    }
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert_eq!(
        told(&said),
        [
            "warning  Social image may not show: `content/photo-1.png` is a WebP",
            "warning  Social image may not show: `content/drawing-2.svg` is an SVG",
            "warning  Social image may not show: `content/drawing-3.svg` is an SVG",
            "warning  Social image may not show: `content/moving-4.gif` is a GIF",
            "warning  Social image may not show: `content/scan-5.bmp` is in a format some link \
             previews don't show",
        ]
    );
    assert!(
        said.contains(
            "  content/post-1.md, line 3\n   3 │ og_image: photo-1.png\n     \
             │           ^^^^^^^^^^^\n  \
             For an image more link previews show, save it as a PNG.\n  \
             More: https://statac.dev/e/image-may-not-show/\n"
        ),
        "{said}"
    );

    // Each page is shared with the picture it names, and the home page
    // alone has a card drawn.
    let said = site.build();
    assert!(said.contains("Drew 1 card."), "{said}");
    assert!(
        said.contains("5 warnings: 5 image-may-not-show. Run `statac check` to see them."),
        "{said}"
    );
    assert_eq!(site.cards().len(), 1, "{:?}", site.cards());
    for (n, (name, bytes, size)) in pictures.iter().enumerate() {
        let post = site.html(&format!("post-{}/index.html", n + 1));
        assert_eq!(
            meta(&post, "og:image").unwrap(),
            format!("https://example.org/{name}")
        );
        assert_eq!(fs::read(site.out().join(name)).unwrap(), *bytes, "{name}");
        let written = (
            meta(&post, "og:image:width"),
            meta(&post, "og:image:height"),
        );
        let sides = size.map(|(width, height)| (width.to_owned(), height.to_owned()));
        assert_eq!(written, sides.unzip(), "{name}");
        assert_eq!(
            meta(&post, "og:image:alt").unwrap(),
            format!("Post {}", n + 1)
        );
    }
}

#[test]
fn a_picture_several_pages_name_is_warned_of_once_with_how_many() {
    let site = Site::new("own-counted");
    site.put("static/shared/cover.webp", WEBP);
    for n in 1..4 {
        site.write(
            &format!("content/post-{n}.md"),
            &format!("---\ntitle: Post {n}\nog_image: /shared/cover.webp\n---\nA post.\n"),
        );
    }
    let said = site.check();
    assert_eq!(
        told(&said),
        ["warning  Social image may not show: the `static/shared/cover.webp` of 3 pages is a WebP"]
    );
    assert!(
        said.contains("  content/post-1.md, line 3\n   3 │ og_image: /shared/cover.webp\n"),
        "{said}"
    );
    let said = site.build();
    assert!(
        said.contains("1 warning: 1 image-may-not-show. Run `statac check` to see it."),
        "{said}"
    );
    for n in 1..4 {
        assert_eq!(
            meta(&site.html(&format!("post-{n}/index.html")), "og:image").unwrap(),
            "https://example.org/shared/cover.webp"
        );
    }
}

#[test]
fn the_warning_of_a_picture_some_previews_may_not_show_is_quietened_by_name() {
    let site = Site::new("own-quietened");
    site.write(
        "statac.yaml",
        "title: Site\nurl: https://example.org\nlanguage: en\ntimezone: UTC\n\
         pages.feed: false\nquiet: [image-may-not-show]\n",
    );
    site.put("content/photo-1.webp", WEBP);
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\nog_image: photo-1.webp\n---\nA post.\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!((said.as_str(), code), ("1 warning, quietened.\n", 0));
    let said = site.build();
    assert!(said.starts_with("1 warning, quietened.\n"), "{said}");
    assert_eq!(
        meta(&site.html("post-1/index.html"), "og:image").unwrap(),
        "https://example.org/photo-1.webp"
    );
}

#[test]
fn a_card_picked_beside_a_picture_of_the_pages_own_is_set_aside() {
    let site = Site::new("both");
    site.write("templates/og.plain.yaml", CARD);
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\nog_image: photo-1.jpg\ncard: plain\n---\nA post.\n",
    );
    // Handed down, it isn't the page's own line to be told of.
    site.write(
        "content/notes-1/index.md",
        "---\ntitle: Notes\npages.card: plain\n---\n",
    );
    site.write(
        "content/notes-1/note-1.md",
        "---\ntitle: Note 1\nog_image: /photo-1.jpg\n---\nA note.\n",
    );
    site.put("content/photo-1.jpg", &jpeg(300, 200));
    let said = site.check();
    assert_eq!(
        told(&said),
        ["warning  Field set aside: `og_image` names this page's social image"]
    );
    assert!(
        said.contains("  content/post-1.md, line 4\n   4 │ card: plain\n"),
        "{said}"
    );
    site.build();
    let note = site.html("notes-1/note-1/index.html");
    assert_eq!(
        meta(&note, "og:image").unwrap(),
        "https://example.org/photo-1.jpg"
    );
}

#[test]
fn a_card_named_that_is_not_described_stops_the_build() {
    let site = Site::new("named");
    site.write("templates/og.plain.yaml", CARD);
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\ncard: plan\n---\nA post.\n",
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ncard: plain\n---\nA post.\n",
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(told(&said), ["error  Unknown card: `plan`"]);
    assert!(
        said.contains(
            "  content/post-1.md, line 3\n   3 │ card: plan\n     │       ^^^^\n  \
             Did you mean `plain`?\n"
        ),
        "{said}"
    );
}

#[test]
fn files_no_card_can_be_drawn_with_stop_the_build_once() {
    let unusable = |name: &str, change: &dyn Fn(&Site)| {
        let site = Site::new(name);
        // More pages asking than one, and the file is told once.
        site.write("content/post-2.md", "---\ntitle: Post 2\n---\nA post.\n");
        change(&site);
        let (said, code) = site.run(&["check"]);
        assert_eq!(code, 1, "{said}");
        let (built, code) = site.run(&["build"]);
        assert_eq!(code, 1, "{built}");
        assert_eq!(told(&built), told(&said));
        said
    };
    let said = unusable("size", &|site| {
        site.put("templates/og/background.png", &flat(1600, 900));
    });
    assert_eq!(
        told(&said),
        [
            "error  Unusable card file: `templates/og/background.png` is 1,600 by 900 pixels, \
          not 1,200 by 630"
        ]
    );
    assert!(
        said.contains(
            "  templates/og.yaml, line 1\n   1 │ background: background.png\n     \
             │             ^^^^^^^^^^^^^^\n  Save the background 1,200 pixels wide and 630 tall.\n"
        ),
        "{said}"
    );
    let said = unusable("cut", &|site| {
        site.put("templates/og/background.png", &flat(1200, 630)[..200]);
    });
    assert_eq!(
        told(&said),
        ["error  Unreadable image: `templates/og/background.png` is cut short or damaged"]
    );
    let said = unusable("moving", &|site| {
        site.put(
            "templates/og/background.png",
            b"GIF89a\xb0\x04\x76\x02\x00\x00\x00;",
        );
    });
    assert_eq!(
        told(&said),
        [
            "error  Unusable card file: `templates/og/background.png` isn't a JPEG, a PNG or \
          a still WebP"
        ]
    );
    let said = unusable("font", &|site| {
        site.write(
            "templates/og.yaml",
            &CARD.replace("font: text.ttf", "font: [text.ttf, more.ttf]"),
        );
        site.write("templates/og/more.ttf", "not a font");
    });
    assert_eq!(
        told(&said),
        ["error  Unusable card file: `templates/og/more.ttf` doesn't read as a font"]
    );
    assert!(
        said.contains("   2 │ font: [text.ttf, more.ttf]\n     │                  ^^^^^^^^\n"),
        "{said}"
    );
    let said = unusable("own-font", &|site| {
        site.write(
            "templates/og.yaml",
            &format!("{CARD}title.font: [more.ttf, text.ttf]\n"),
        );
        site.write("templates/og/more.ttf", "not a font");
    });
    assert_eq!(
        told(&said),
        ["error  Unusable card file: `templates/og/more.ttf` doesn't read as a font"]
    );
    assert!(
        said.contains("  13 │ title.font: [more.ttf, text.ttf]\n     │              ^^^^^^^^\n"),
        "{said}"
    );
}

#[test]
fn pages_left_without_a_card_are_told_once_for_each_cause() {
    let site = Site::new("causes");
    let long = "word ".repeat(120);
    for n in 2..5 {
        site.write(
            &format!("content/long-{n}.md"),
            &format!("---\ntitle: {long}\n---\nA post.\n"),
        );
    }
    site.write(
        "content/sign-1.md",
        "---\ntitle: Two → three\n---\nA post.\n",
    );
    site.write(
        "content/sign-2.md",
        "---\ntitle: Four → five\n---\nA post.\n",
    );
    site.write(
        "content/sign-3.md",
        "---\ntitle: Six ✓ seven\n---\nA post.\n",
    );
    // Needing a character two others need, it is left without for their
    // cause, whatever else it needs.
    site.write(
        "content/sign-4.md",
        "---\ntitle: Eight Ω nine → ten\n---\nA post.\n",
    );
    let said = site.check();
    assert_eq!(
        told(&said),
        [
            "warning  Card not drawn: 3 pages need `→` and `Ω`, which no font has, starting \
             with `content/sign-1.md`",
            "warning  Card not drawn: `content/sign-3.md` needs `✓`, which no font has",
            "warning  Card not drawn: the `title` of 3 pages doesn't fit, even at 45 pixels, \
             starting with `content/long-2.md`",
        ]
    );
    assert!(
        said.contains(
            "  templates/og.yaml, line 6\n   6 │ title.size: up to 75\n     \
             │             ^^^^^^^^\n"
        ),
        "{said}"
    );
    assert!(
        said.contains(
            "  templates/og.yaml, line 4\n   4 │ title.text: \"{{ page.og_title }}\"\n     \
             │             ^^^^^^^^^^^^^^^^^^^^^\n  \
             Add a font that has them to the card's `font`.\n"
        ),
        "{said}"
    );

    // Each such page is built with no card, and the rest have theirs.
    let said = site.build();
    assert!(
        said.contains("3 warnings: 3 card-not-drawn. Run `statac check` to see them."),
        "{said}"
    );
    assert!(said.contains("Drew 2 cards."), "{said}");
    assert_eq!(meta(&site.html("long-2/index.html"), "og:image"), None);
    assert_eq!(meta(&site.html("sign-3/index.html"), "og:image"), None);
    assert!(meta(&site.html("post-1/index.html"), "og:image").is_some());
    // Nothing is kept of them, and a second build says the same.
    let again = site.build();
    assert!(
        again.contains("3 warnings: 3 card-not-drawn. Run `statac check` to see them."),
        "{again}"
    );
    assert!(!again.contains("Drew"), "{again}");
}

#[test]
fn texts_at_the_top_and_bottom_that_meet_leave_a_page_without_a_card() {
    let site = Site::new("no-room");
    let card = CARD
        .replace("title.place: centre", "title.place: top")
        .replace("up to 75", "300")
        .replace("footer.size: 36", "footer.size: 300");
    site.write("templates/og.yaml", &card);
    assert_eq!(
        told(&site.check()),
        [
            "warning  Card not drawn: the top and bottom texts of 2 pages leave no room \
          between them, starting with `content/index.md`"
        ]
    );
}

#[test]
fn an_emoji_is_missed_without_a_font_that_has_it_and_drawn_with_one() {
    let site = Site::new("emoji");
    site.write(
        "content/post-2.md",
        "---\ntitle: Boots \u{1f97e}\n---\nA post.\n",
    );
    let said = site.check();
    assert_eq!(
        told(&said),
        ["warning  Card not drawn: `content/post-2.md` needs `\u{1f97e}`, which no font has"]
    );
    let hint = "  Add a font that has it to the card's `font`.\n";
    assert!(said.contains(hint), "{said}");

    // Once its file is there, the font is named for the card's `font`.
    site.put("templates/og/emoji.ttf", EMOJI);
    let said = site.check();
    assert!(
        said.contains("  Add `emoji.ttf`, which has it, to the card's `font`.\n"),
        "{said}"
    );

    site.write(
        "templates/og.yaml",
        &CARD.replace("font: text.ttf", "font: [text.ttf, emoji.ttf]"),
    );
    assert_eq!(site.check(), "No problems.\n");
    let said = site.build();
    assert!(said.contains("Drew 3 cards."), "{said}");
}

#[test]
fn a_digit_is_drawn_by_a_second_text_font_and_never_by_an_emoji_font() {
    let site = Site::new("second-font");
    site.write(
        "templates/og.yaml",
        &CARD.replace("font: text.ttf", "font: [letters.ttf, emoji.ttf, text.ttf]"),
    );
    site.put("templates/og/letters.ttf", LETTERS);
    site.put("templates/og/emoji.ttf", EMOJI);
    site.write("content/post-1.md", "---\ntitle: Post 2024\n---\nA post.\n");
    assert_eq!(site.check(), "No problems.\n");
    site.write(
        "templates/og.yaml",
        &CARD.replace("font: text.ttf", "font: [letters.ttf, emoji.ttf]"),
    );
    assert_eq!(
        told(&site.check()),
        ["warning  Card not drawn: `content/post-1.md` needs `2`, `0` and `4`, which no font has"]
    );
}

#[test]
fn a_text_drawn_in_fonts_of_its_own_is_told_of_what_they_lack() {
    let site = Site::new("own-fonts");
    site.put("templates/og/letters.ttf", LETTERS);
    site.put("templates/og/emoji.ttf", EMOJI);
    site.write(
        "templates/og.yaml",
        &format!("{CARD}title.font: [letters.ttf, emoji.ttf]\n"),
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Boots \u{1f97e}\n---\nA post.\n",
    );
    let said = site.check();
    assert_eq!(
        told(&said),
        [
            "warning  Card not drawn: `content/post-1.md` needs `1`, which no font in `title.font` has"
        ]
    );
    assert!(
        said.contains(
            "   4 │ title.text: \"{{ page.og_title }}\"\n     \
             │             ^^^^^^^^^^^^^^^^^^^^^\n  \
             Add `text.ttf`, which has it, to `title.font`.\n"
        ),
        "{said}"
    );

    // The site's title is in the card's fonts, which lack what the
    // title's have.
    site.write(
        "templates/og.yaml",
        &format!("{CARD}title.font: [text.ttf, emoji.ttf]\n"),
    );
    site.write(
        "statac.yaml",
        "title: Site \u{1f97e}\nurl: https://example.org\nlanguage: en\ntimezone: UTC\n\
         pages.feed: false\n",
    );
    let said = site.check();
    assert_eq!(
        told(&said),
        [
            "warning  Card not drawn: 3 pages need `\u{1f97e}`, which no font has, starting \
             with `content/index.md`"
        ]
    );
    assert!(
        said.contains("  Add `emoji.ttf`, which has it, to the card's `font`.\n"),
        "{said}"
    );
}

#[test]
fn a_card_no_page_is_shared_with_is_warned_of() {
    let site = Site::new("unused");
    site.write("templates/og.plain.yaml", CARD);
    let said = site.check();
    assert_eq!(told(&said), ["warning  Unused card: no page picks `plain`"]);
    // The card's file is named alone, with no line of it marked.
    assert!(
        said.contains("`plain`\n  templates/og.plain.yaml\n  Write `card: plain`"),
        "{said}"
    );
    // A draft picking it is a page picking it.
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndraft: true\ncard: plain\n---\nA draft.\n",
    );
    assert_eq!(
        told(&site.check()),
        ["note  Draft left out: `content/post-2.md`"]
    );

    for name in ["home", "page", "folder"] {
        site.write(
            &format!("templates/{name}.html"),
            "<!doctype html>\n<title>{{ page.og_title }}</title>\n{{ page.content }}\n",
        );
    }
    let said = site.check();
    assert_eq!(
        told(&said),
        [
            "note  Draft left out: `content/post-2.md`",
            "warning  Unused card: no layout reads `page.card`",
        ]
    );
    assert!(
        said.contains("`page.card`\n  templates/og.yaml\n  Write the card's"),
        "{said}"
    );
    let said = site.build();
    assert!(!said.contains("Drew"), "{said}");
    assert_eq!(site.cards(), [""; 0]);
}

#[test]
fn a_mistake_in_a_text_is_told_where_the_description_writes_it() {
    let site = Site::new("text");
    site.write(
        "templates/og.yaml",
        &CARD.replace("page.og_title", "page.og_titel"),
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(told(&said), ["error  Unknown field: `page.og_titel`"]);
    assert!(
        said.contains(
            "  templates/og.yaml, line 4\n   4 │ title.text: \"{{ page.og_titel }}\"\n     \
             │                 ^^^^^^^^^^^^^\n  Did you mean `page.og_title`?\n"
        ),
        "{said}"
    );

    // A value some page lacks is told for that page, at the same place.
    site.write(
        "templates/og.yaml",
        &CARD.replace("page.og_title", "page.summary"),
    );
    site.write(
        "content/index.md",
        "---\ntitle: Home\nsummary: The home page\n---\nThe home page.\n",
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        told(&said),
        ["error  No value: `page.summary` isn't set on `content/post-1.md`"]
    );
    assert!(
        said.contains(
            "  templates/og.yaml, line 4\n   4 │ title.text: \"{{ page.summary }}\"\n     \
             │                 ^^^^^^^^^^^^\n"
        ),
        "{said}"
    );
}

#[test]
fn a_text_is_drawn_as_written_and_not_as_markup() {
    let site = Site::new("unescaped");
    site.write(
        "content/post-1.md",
        "---\ntitle: Salt & pepper\n---\nA post.\n",
    );
    site.build();
    let post = fs::read_to_string(site.out().join("post-1/index.html")).unwrap();
    // The page escapes what the card's words hold; the card held them bare.
    assert_eq!(
        meta(&post, "og:image:alt").unwrap(),
        "Salt &amp; pepper. Site"
    );
}

#[test]
fn a_colour_built_for_each_page_is_drawn_and_one_that_is_none_is_told() {
    let site = Site::new("colour");
    site.write(
        "templates/og.yaml",
        &CARD.replace("\"#F2E9D3\"", "\"{{ page.accent | default('#F2E9D3') }}\""),
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\naccent: \"#FF0000\"\n---\nA post.\n",
    );
    site.build();
    let address = meta(&site.html("post-1/index.html"), "og:image").unwrap();
    let file = address.strip_prefix("https://example.org/").unwrap();
    let bytes = fs::read(site.out().join(file)).unwrap();
    let drawn = image::load_from_memory(&bytes).unwrap().into_rgb8();
    let reddest = (0..1200)
        .flat_map(|x| (200..400).map(move |y| (x, y)))
        .map(|(x, y)| drawn.get_pixel(x, y).0)
        .max_by_key(|pixel| pixel[0])
        .unwrap();
    assert_eq!(reddest, [255, 0, 0]);

    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\naccent: red\n---\nA post.\n",
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\naccent: \"\"\n---\nA post.\n",
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        told(&said),
        [
            "error  Wrong kind of value: `title.colour` is a colour such as `#F2E9D3` or \
             `#FFF`, not `red`",
            "error  Wrong kind of value: `title.colour` is a colour such as `#F2E9D3` or \
             `#FFF`, not nothing",
        ]
    );
    assert!(
        said.contains(
            "  templates/og.yaml, line 7\n   7 │ title.colour: \"{{ page.accent | \
             default('#F2E9D3') }}\"\n"
        ),
        "{said}"
    );
    assert!(
        said.contains("  While building: content/post-1.md\n"),
        "{said}"
    );
}

#[test]
fn a_card_is_answered_in_a_layout_and_nowhere_else() {
    let site = Site::new("asked");
    site.write("snippets/shared.html", "{{ page.card.url }}");
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\nA post.\n\n@shared()\n",
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        told(&said),
        ["error  Template stopped: `page.card` isn't answered in a page's content"]
    );
    assert!(
        said.contains("  snippets/shared.html, line 1\n   1 │ {{ page.card.url }}\n"),
        "{said}"
    );
    assert!(
        said.contains("  Called here: content/post-1.md, line 6\n"),
        "{said}"
    );

    // The same snippet called by a layout is answered.
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\nA post.\n");
    site.write(
        "templates/page.html",
        "<!doctype html>\n<title>{{ page.og_title }}</title>\n<p>{{ shared() }}</p>\n",
    );
    site.build();
    let post = site.html("post-1/index.html");
    assert!(post.contains("<p>/post-1/og."), "{post}");

    let site = Site::new("asked-by-card");
    site.write(
        "templates/og.yaml",
        &CARD.replace("page.og_title", "page.card.alt"),
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        told(&said),
        ["error  Template stopped: `page.card` isn't answered in a card's own text"]
    );
    assert!(
        said.contains("   4 │ title.text: \"{{ page.card.alt }}\"\n"),
        "{said}"
    );
}

#[test]
fn a_problem_raised_in_a_text_or_a_snippet_it_calls_is_told_where_raised() {
    let site = Site::new("raised");
    site.write("snippets/where.html", "{{ url(to) }}");
    site.write(
        "templates/og.yaml",
        &CARD
            .replace("page.og_title", "where(to=\\\"nowhere\\\")")
            .replace("site.title", "page.card.alt"),
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(
        told(&said),
        [
            "error  Template stopped: `url` needs a URL starting with `/`",
            "error  Template stopped: `page.card` isn't answered in a card's own text",
        ]
    );
    assert!(
        said.contains(
            "  snippets/where.html, line 1\n   1 │ {{ url(to) }}\n     │    ^^^^^^^\n  \
             Called here: templates/og.yaml, line 4\n   4 │ title.text: \"{{ where(to=\\\"nowhere\\\") \
             }}\"\n     │             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n  \
             While building: content/index.md\n  Write it as `url(\"/about/\")`.\n"
        ),
        "{said}"
    );
    assert!(
        said.contains(
            "  templates/og.yaml, line 9\n   9 │ footer.text: \"{{ page.card.alt }}\"\n     \
             │                  ^^^^^^^^^\n  While building: content/index.md\n"
        ),
        "{said}"
    );
}

#[test]
fn a_link_to_a_card_lands_on_it() {
    let site = Site::new("linked");
    site.write(
        "templates/page.html",
        "<!doctype html>\n<title>{{ page.og_title }}</title>\n\
         <a href=\"{{ page.card.url }}\">The card</a>\n\
         <a href=\"/post-1/og.00000000.png\">No card</a>\n\
         {{ page.content }}\n",
    );
    assert_eq!(
        told(&site.check()),
        ["warning  Broken link: nothing is published at `/post-1/og.00000000.png`"]
    );
}

#[test]
fn a_second_build_draws_nothing_and_publishes_the_same_bytes() {
    let site = Site::new("warm");
    let said = site.build();
    assert!(said.contains("Drew 2 cards."), "{said}");
    let published = |site: &Site| -> Vec<(String, Vec<u8>)> {
        site.cards()
            .into_iter()
            .map(|name| (name.clone(), fs::read(site.out().join(name)).unwrap()))
            .collect()
    };
    let first = published(&site);
    let kept = site.kept();
    assert_eq!(kept.len(), 2, "{kept:?}");
    assert!(kept.iter().all(|name| name.ends_with(".png")), "{kept:?}");

    let said = site.build();
    assert!(
        !said.contains("Drew") && !said.contains("Removed"),
        "{said}"
    );
    assert!(first == published(&site));
    assert_eq!(site.kept(), kept);

    // A page's words changed, its card is drawn again and the old one goes.
    site.write("content/post-1.md", "---\ntitle: Post one\n---\nA post.\n");
    let said = site.build();
    assert!(
        said.contains(
            "Drew 1 card.\nRemoved 1 card no page is shared with any more from `.statac`."
        ),
        "{said}"
    );
    assert_eq!(site.kept().len(), 2);
    assert_ne!(site.kept(), kept);
}

#[test]
fn a_change_to_a_texts_own_fonts_draws_its_cards_again() {
    let site = Site::new("own-fonts-kept");
    site.write(
        "templates/og.yaml",
        &format!("{CARD}title.font: [title.ttf, text.ttf]\n"),
    );
    site.put("templates/og/title.ttf", LATIN);
    let said = site.build();
    assert!(said.contains("Drew 2 cards."), "{said}");
    let first = site.cards();

    // A font file no list names is no part of a card.
    site.put("templates/og/other.ttf", LETTERS);
    let said = site.build();
    assert!(!said.contains("Drew"), "{said}");
    assert_eq!(site.cards(), first);

    site.put("templates/og/title.ttf", LETTERS);
    let said = site.build();
    assert!(said.contains("Drew 2 cards."), "{said}");
    assert!(site.cards().iter().all(|card| !first.contains(card)));
}

#[test]
fn pages_with_the_same_words_share_a_card_drawn_once() {
    let site = Site::new("shared");
    site.write(
        "templates/og.yaml",
        &CARD.replace("{{ page.og_title }}", "{{ site.title }}"),
    );
    let said = site.build();
    assert!(said.contains("Drew 1 card."), "{said}");
    // Published beside each page all the same.
    let cards = site.cards();
    assert_eq!(cards.len(), 2, "{cards:?}");
    assert_eq!(site.kept().len(), 1);
    let bytes = |name: &String| fs::read(site.out().join(name)).unwrap();
    assert!(bytes(&cards[0]) == bytes(&cards[1]));
}

#[test]
fn a_background_too_large_as_a_png_gives_cards_as_jpegs() {
    let site = Site::new("jpeg");
    // Grain over every pixel, which no PNG compresses far.
    let mut state = 0x5eed_u32;
    let noisy = RgbImage::from_fn(1200, 630, |_, _| {
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 24) as u8
        };
        Rgb([next(), next(), next()])
    });
    site.put("templates/og/background.png", &png_of(&noisy));
    site.build();
    let cards = site.cards();
    assert_eq!(cards.len(), 2);
    assert!(cards.iter().all(|name| name.ends_with(".jpg")), "{cards:?}");
    let address = meta(&site.html("post-1/index.html"), "og:image").unwrap();
    assert!(address.ends_with(".jpg"), "{address}");
}

#[cfg(unix)]
#[test]
fn cards_that_cannot_be_kept_are_still_published() {
    use std::os::unix::fs::PermissionsExt;
    let site = Site::new("unkept");
    // A file where the folder of cards would be.
    site.write(".statac/cards", "");
    fs::set_permissions(site.root.join(".statac"), fs::Permissions::from_mode(0o555)).unwrap();
    let (said, code) = site.run(&["build"]);
    fs::set_permissions(site.root.join(".statac"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("1 warning: 1 cache-not-written."), "{said}");
    assert!(said.contains("Drew 2 cards."), "{said}");
    assert_eq!(site.cards().len(), 2);
}

/// Whether each card published is a picture of a card's size.
#[cfg(unix)]
fn cards_are_cards(site: &Site) -> bool {
    let cards = site.cards();
    cards.len() == 2
        && cards.iter().all(|name| {
            let bytes = fs::read(site.out().join(name)).unwrap();
            image::load_from_memory(&bytes)
                .is_ok_and(|card| (card.width(), card.height()) == (1200, 630))
        })
}

#[cfg(unix)]
#[test]
fn a_link_where_cards_are_kept_is_never_followed() {
    use std::os::unix::fs::symlink;
    for (name, linked) in [
        ("linked-kept", ".statac"),
        ("linked-cards", ".statac/cards"),
    ] {
        let site = Site::new(name);
        let outside = site.root.with_extension("outside");
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("private.png"), "private").unwrap();
        fs::create_dir_all(site.root.join(linked).parent().unwrap()).unwrap();
        symlink(&outside, site.root.join(linked)).unwrap();
        for _ in 0..2 {
            let (said, code) = site.run(&["build"]);
            assert_eq!(code, 0, "{linked}: {said}");
            assert!(
                said.contains("1 warning: 1 cache-not-written."),
                "{linked}: {said}"
            );
            assert!(said.contains("Drew 2 cards."), "{linked}: {said}");
            assert!(cards_are_cards(&site), "{linked}");
            // Nothing was written through the link, and nothing removed.
            assert_eq!(names(&outside), ["private.png"], "{linked}");
            assert_eq!(fs::read(outside.join("private.png")).unwrap(), b"private");
        }
        fs::remove_dir_all(&outside).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn a_link_standing_for_a_card_kept_is_neither_published_nor_removed() {
    use std::os::unix::fs::symlink;
    let site = Site::new("linked-card");
    let outside = site.root.with_extension("outside");
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("private.png"), "private").unwrap();
    site.build();
    let kept = site.root.join(".statac/cards");
    // Each card kept gives way to a link to the file outside, or to
    // another name for it; beside them stand a link that is no card of any
    // page, and a card of none.
    let cards = site.kept();
    assert_eq!(cards.len(), 2);
    fs::remove_file(kept.join(&cards[0])).unwrap();
    symlink(outside.join("private.png"), kept.join(&cards[0])).unwrap();
    fs::remove_file(kept.join(&cards[1])).unwrap();
    fs::hard_link(outside.join("private.png"), kept.join(&cards[1])).unwrap();
    symlink(outside.join("private.png"), kept.join("stale.png")).unwrap();
    fs::write(kept.join("0000.png"), "a card of no page").unwrap();

    let (said, code) = site.run(&["build"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("1 warning: 1 cache-not-written."), "{said}");
    assert!(said.contains("Drew 2 cards."), "{said}");
    assert!(cards_are_cards(&site));
    assert_eq!(fs::read(outside.join("private.png")).unwrap(), b"private");
    // Only the file that is itself a stale card went.
    assert!(
        said.contains("Removed 1 card no page is shared with any more"),
        "{said}"
    );
    let left = names(&kept);
    assert!(!left.contains(&"0000.png".to_owned()), "{left:?}");
    for name in [cards[0].as_str(), "stale.png"] {
        let found = fs::symlink_metadata(kept.join(name)).unwrap();
        assert!(found.is_symlink(), "{name}");
    }
    fs::remove_dir_all(&outside).unwrap();
}
