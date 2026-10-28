//! What templates and snippets show of a page's pictures, files, videos
//! and recordings, in small sites built end to end, and what is said of a
//! picture shown with no description.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use image::{Rgb, RgbImage};

struct Site {
    root: PathBuf,
}

fn pixels(width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        let band = if (x / 16 + y / 16) % 2 == 0 { 40 } else { 0 };
        Rgb([
            (x * 255 / width) as u8,
            (y * 255 / height) as u8,
            120 + band,
        ])
    })
}

fn jpeg(width: u32, height: u32) -> Vec<u8> {
    let mut out = Vec::new();
    jpeg_encoder::Encoder::new(&mut out, 90)
        .encode(
            &pixels(width, height),
            width as u16,
            height as u16,
            jpeg_encoder::ColorType::Rgb,
        )
        .unwrap();
    out
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    pixels(width, height)
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

/// A GIF as far as its size, which is as far as it is read.
fn gif(width: u16, height: u16) -> Vec<u8> {
    let mut out = b"GIF89a".to_vec();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    out.extend([0, 0, 0]);
    out
}

/// The first bytes of a HEIC file: its `ftyp` box, and nothing after.
const HEIC: &[u8] = b"\0\0\0\x18ftypheic\0\0\0\0mif1heic";

const BASE: &str = "<!doctype html>\n<title>{{ headline(page) }}</title>\n";

/// A snippet showing a picture as a site might write one, reading the
/// picture's own description only where the call gives none.
const IMAGE: &str = "{# takes photo, alt #}\n\
    {% set picture = page.image(photo, decorative=(decorative if decorative is defined else false)) %}\n\
    <img src=\"{{ picture.src }}\" alt=\"{{ alt if alt is defined else picture.alt }}\">\n";

impl Site {
    /// A site whose pages are built with `layout`.
    fn new(name: &str, layout: &str) -> Site {
        let root = std::env::temp_dir().join(format!("statac-shown-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let site = Site { root };
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
        );
        site.write("content/index.md", "---\ntitle: Home\n---\n");
        site.write("templates/home.html", BASE);
        site.write("templates/folder.html", BASE);
        site.write("templates/page.html", &format!("{BASE}{layout}\n"));
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
        assert_eq!(String::from_utf8(output.stderr).unwrap(), "", "{said}");
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

    /// The built page `file`, after the line every layout opens with, its
    /// copies' hashes written as `#`.
    fn page(&self, file: &str) -> String {
        let html = fs::read_to_string(self.root.join("_site").join(file)).unwrap();
        let body = html.split_once("</title>\n").unwrap().1;
        unhash(body.trim_end())
    }

    fn published(&self, file: &str) -> bool {
        self.root.join("_site").join(file).is_file()
    }

    /// The copies published in `folder` of the built site, their hashes
    /// written as `#`.
    fn copies(&self, folder: &str) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.root.join("_site").join(folder))
            .unwrap()
            .map(|entry| unhash(&entry.unwrap().file_name().to_string_lossy()))
            .filter(|name| name.contains(".#."))
            .collect();
        names.sort();
        names
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// `text` with each copy's eight hex digits written as `#`.
fn unhash(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(at) = rest.find('.') {
        let after = &rest[at + 1..];
        let hex = after.len() > 8
            && after.as_bytes()[8] == b'.'
            && after[..8].bytes().all(|b| b.is_ascii_hexdigit());
        out.push_str(&rest[..=at]);
        match hex {
            true => {
                out.push('#');
                rest = &after[8..];
            }
            false => rest = after,
        }
    }
    out.push_str(rest);
    out
}

/// Every name a photograph answers to, one to a line.
const EVERY_NAME: &str = "{% set p = page.image(page.shows) %}\
    src={{ p.src }}\navif={{ p.avif }}\nfallback={{ p.fallback }}\nsizes={{ p.sizes }}\n\
    full={{ p.full }}\nwidth={{ p.width }}\nheight={{ p.height }}\nalt={{ p.alt }}\n\
    caption={{ p.caption }}\nname={{ p.name }}";

#[test]
fn a_photograph_asked_for_by_a_template_answers_to_every_name_and_is_published_as_copies() {
    let site = Site::new("photograph", EVERY_NAME);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\nshows: first-photo-1.jpg\n---\n",
    );
    site.put("content/post-1/first-photo-1.jpg", &jpeg(900, 600));
    site.write(
        "content/post-1/first-photo-1.jpg.yaml",
        "alt: Photo 1\ncaption: Caption 1\n",
    );
    let said = site.build();
    assert!(said.contains("Made 6 resized copies of 1 image."), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "src=/post-1/first-photo-1-900.#.jpg\n\
         avif=/post-1/first-photo-1-400.#.avif 400w, /post-1/first-photo-1-800.#.avif 800w, \
         /post-1/first-photo-1-900.#.avif 900w\n\
         fallback=/post-1/first-photo-1-400.#.jpg 400w, /post-1/first-photo-1-800.#.jpg 800w, \
         /post-1/first-photo-1-900.#.jpg 900w\n\
         sizes=(max-width: 46rem) 100vw, 46rem\n\
         full=/post-1/first-photo-1-900.#.jpg\n\
         width=900\nheight=600\nalt=Photo 1\ncaption=Caption 1\nname=First Photo 1"
    );
    assert_eq!(site.copies("post-1").len(), 6);
    // Shown as copies and linked by nothing, the photograph stays behind.
    assert!(!site.published("post-1/first-photo-1.jpg"));
}

#[test]
fn a_plain_picture_asked_for_by_a_template_is_the_file_itself() {
    let site = Site::new("plain", EVERY_NAME);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\nshows: diagram-1.gif\n---\n",
    );
    site.put("content/post-1/diagram-1.gif", &gif(40, 30));
    site.write("content/post-1/diagram-1.gif.yaml", "alt: How it fits\n");
    let said = site.build();
    assert!(!said.contains("Made"), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "src=/post-1/diagram-1.gif\navif=\nfallback=\nsizes=(max-width: 46rem) 100vw, 46rem\n\
         full=/post-1/diagram-1.gif\nwidth=40\nheight=30\nalt=How it fits\ncaption=\n\
         name=Diagram 1"
    );
    assert!(site.published("post-1/diagram-1.gif"));
}

#[test]
fn a_picture_is_found_from_the_page_asked_wherever_it_is_shown() {
    // A list shows each post's picture, named from the post's own folder;
    // one in `static/` is named from the top, under the site's path.
    let site = Site::new("found", "");
    site.write(
        "statac.yaml",
        "title: Site\nurl: https://example.org/blog/\nlanguage: en\ntimezone: UTC\n\
         pages.feed: false\n",
    );
    site.write(
        "templates/home.html",
        &format!(
            "{BASE}{{% for post in site.pages %}}{{% if post.shows %}}\
             {{{{ post.image(post.shows).src }}}}\n{{% endif %}}{{% endfor %}}"
        ),
    );
    site.write(
        "content/posts/post-1.md",
        "---\ntitle: Post 1\nshows: photo 1.jpg\n---\n",
    );
    site.write(
        "content/posts/post-2.md",
        "---\ntitle: Post 2\nshows: /img/logo-1.gif\n---\n",
    );
    site.put("content/posts/photo 1.jpg", &jpeg(400, 200));
    site.put("static/img/logo-1.gif", &gif(40, 30));
    site.build();
    assert_eq!(
        site.page("index.html"),
        "/blog/posts/photo%201-400.#.jpg\n/blog/img/logo-1.gif"
    );
    assert_eq!(
        site.copies("posts"),
        ["photo 1-400.#.avif", "photo 1-400.#.jpg"]
    );
}

#[test]
fn a_pages_photographs_are_gathered_by_name_or_given_as_named() {
    let site = Site::new(
        "photos",
        "{% for p in page.images() %}{{ p.name }} {% endfor %}|\
         {% for p in page.images(['photo-2.jpg', 'diagram-1.png']) %}{{ p.name }} {% endfor %}|\
         {% for p in page.images(none, sizes='10rem') %}{{ p.sizes }} {% endfor %}",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    for name in ["photo-2.jpg", "photo-10.jpg", "photo-1.jpg"] {
        site.put(&format!("content/post-1/{name}"), &jpeg(30, 20));
    }
    site.put("content/post-1/diagram-1.png", &png(40, 30));
    site.put("content/post-1/drawing-1.gif", &gif(40, 30));
    // A post that is one file has no photographs of its own.
    site.write(
        "templates/gathered.html",
        &format!("{BASE}{{% for p in page.images() %}}{{{{ p.name }}}}{{% endfor %}}|\n"),
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\nlayout: gathered\n---\n",
    );
    site.put("content/photo-3.jpg", &jpeg(30, 20));
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "Diagram 1 Photo 1 Photo 10 Photo 2 |Photo 2 Diagram 1 |10rem 10rem 10rem 10rem"
    );
    assert_eq!(site.page("post-2/index.html"), "|");
}

#[test]
fn full_adds_a_copy_at_the_photographs_own_width() {
    let layout =
        "{{ page.image('photo-1.jpg').full }}\n{{ page.image('photo-1.jpg', full=true).full }}";
    let site = Site::new("full", layout);
    site.write(
        "statac.yaml",
        "title: Site\nmax_width: 800\npages.feed: false\n",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    site.put("content/post-1/photo-1.jpg", &jpeg(1000, 600));
    let said = site.build();
    assert!(said.contains("Made 5 resized copies of 1 image."), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "/post-1/photo-1-800.#.jpg\n/post-1/photo-1-1000.#.jpg"
    );
    assert_eq!(
        site.copies("post-1"),
        [
            "photo-1-1000.#.jpg",
            "photo-1-400.#.avif",
            "photo-1-400.#.jpg",
            "photo-1-800.#.avif",
            "photo-1-800.#.jpg",
        ]
    );
    // A second build makes nothing, and keeps the copy.
    let said = site.build();
    assert!(
        !said.contains("Made") && !said.contains("Removed"),
        "{said}"
    );

    // Without the asking, the copy is neither made nor kept.
    site.write(
        "templates/page.html",
        &format!("{BASE}{{{{ page.image('photo-1.jpg').full }}}}\n"),
    );
    let said = site.build();
    assert!(said.contains("Removed 1 resized copy"), "{said}");
    assert_eq!(site.copies("post-1").len(), 4);
}

#[test]
fn what_a_pictures_own_file_says_reaches_the_template_under_statacs_names() {
    let site = Site::new(
        "sidecar",
        "{% set p = page.image('photo-1.jpg') %}{{ p.credit }}|{{ p.taken.place }}|{{ p.src }}|\
         {{ p.width }}|{{ p.caption | markdown }}",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    site.put("content/post-1/photo-1.jpg", &jpeg(400, 200));
    site.write(
        "content/post-1/photo-1.jpg.yaml",
        "credit: A photographer\ntaken:\n  place: Place 1\nsrc: elsewhere.jpg\nwidth: 1\n\
         caption: Caption *1*\ndecorative: true\n",
    );
    let said = site.build();
    assert!(!said.contains("note"), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "A photographer|Place 1|/post-1/photo-1-400.#.jpg|400|Caption <em>1</em>"
    );
}

#[test]
fn a_picture_with_no_description_is_noted_once_where_it_is_shown() {
    let site = Site::new("undescribed", "{{ page.content }}");
    site.write("snippets/image.html", IMAGE);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n\
         ![](photo-1.jpg) and again ![](photo-1.jpg)\n\n\
         ![](https://example.org/photo-2.jpg)\n\n\
         <img src=\"photo-3.jpg\">\n\n\
         @image(\"photo-4.jpg\")\n\n\
         ![](/img/photo-5.jpg)\n",
    );
    for n in [1, 3, 4] {
        site.put(&format!("content/post-1/photo-{n}.jpg"), &jpeg(30, 20));
    }
    site.put("static/img/photo-5.jpg", &jpeg(30, 20));
    let said = site.check();
    let expected = "\
note  Missing alt text: `photo-1.jpg`
  content/post-1/index.md, line 4
   4 │ ![](photo-1.jpg) and again ![](photo-1.jpg)
     │     ^^^^^^^^^^^
  Describe it in the brackets, or mark it decorative.
  More: https://statac.dev/e/missing-alt-text/

note  Missing alt text: `https://example.org/photo-2.jpg`
  content/post-1/index.md, line 6
   6 │ ![](https://example.org/photo-2.jpg)
     │     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  Describe it in the brackets.
  More: https://statac.dev/e/missing-alt-text/

note  Missing alt text: `photo-3.jpg`
  content/post-1/index.md, line 8
   8 │ <img src=\"photo-3.jpg\">
     │           ^^^^^^^^^^^
  Give the `<img>` an `alt`, or `alt=\"\"` if it is decorative.
  More: https://statac.dev/e/missing-alt-text/

note  Missing alt text: `content/post-1/photo-4.jpg`
  content/post-1/index.md, line 10
  10 │ @image(\"photo-4.jpg\")
     │ ^^^^^^^^^^^^^^^^^^^^^
  Describe it with `alt:` in `content/post-1/photo-4.jpg.yaml`, or mark it decorative.
  More: https://statac.dev/e/missing-alt-text/

note  Missing alt text: `/img/photo-5.jpg`
  content/post-1/index.md, line 12
  12 │ ![](/img/photo-5.jpg)
     │     ^^^^^^^^^^^^^^^^
  Describe it in the brackets.
  More: https://statac.dev/e/missing-alt-text/
";
    assert!(said.contains(expected), "{said}");
    assert_eq!(said.matches("Missing alt text:").count(), 5);

    // `build` counts them, and a site may quieten them by name.
    let said = site.build();
    assert!(said.contains("5 notes: 5 missing-alt-text."), "{said}");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n\
         quiet: [missing-alt-text]\n",
    );
    let said = site.check();
    assert!(!said.contains("Missing alt text:"), "{said}");
}

#[test]
fn a_picture_shown_by_the_writing_and_the_layout_is_noted_once_at_the_writing() {
    let site = Site::new(
        "twice",
        "{{ page.content }}\n\
         {{ page.image('/img/photo-1.jpg').alt }}{{ page.image('/img/photo-2.jpg').alt }}",
    );
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![](/img/photo-1.jpg)\n",
    );
    site.write(
        "content/post-2/index.md",
        "---\ntitle: Post 2\n---\n![](/img/photo-1.jpg)\n",
    );
    for n in [1, 2] {
        site.put(&format!("static/img/photo-{n}.jpg"), &jpeg(30, 20));
    }
    let said = site.check();
    // The one both show is told at the writing on each page, and the one
    // only the layout shows is told for the pages building it.
    let at_writing = said
        .matches("note  Missing alt text: `/img/photo-1.jpg`\n  content/post-")
        .count();
    let by_layout = said
        .matches("note  Missing alt text: `static/img/photo-2.jpg`\n")
        .count();
    assert_eq!((at_writing, by_layout), (2, 1), "{said}");
    assert!(
        said.contains(
            "note  Missing alt text: `static/img/photo-2.jpg`\n  \
             templates/page.html, line 4\n   \
             4 │ {{ page.image('/img/photo-1.jpg').alt }}\
             {{ page.image('/img/photo-2.jpg').alt }}\n     \
             │                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n  \
             While building: content/post-1/index.md\n  \
             And while building 1 more:\n    \
             content/post-2/index.md\n"
        ),
        "{said}"
    );
    assert_eq!(said.matches("Missing alt text:").count(), 3);
}

#[test]
fn a_picture_described_or_marked_decorative_is_never_mentioned() {
    let site = Site::new("described", "{{ page.content }}");
    site.write("snippets/image.html", IMAGE);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n\
         ![In the brackets](photo-1.jpg)\n\n\
         ![](photo-2.jpg)\n\n\
         ![](photo-3.jpg)\n\n\
         <img src=\"photo-4.jpg\" alt=\"\">\n\n\
         <img src=\"photo-3.jpg\">\n\n\
         @image(\"photo-5.jpg\", alt=\"In the call\")\n\n\
         @image(\"photo-2.jpg\")\n\n\
         @image(\"photo-3.jpg\")\n\n\
         @image(\"photo-6.jpg\", decorative=true)\n",
    );
    for n in 1..=6 {
        site.put(&format!("content/post-1/photo-{n}.jpg"), &jpeg(400, 200));
    }
    site.write("content/post-1/photo-2.jpg.yaml", "alt: In its own file\n");
    site.write("content/post-1/photo-3.jpg.yaml", "decorative: true\n");
    let said = site.build();
    assert!(!said.contains("note"), "{said}");
    let page = site.page("post-1/index.html");
    // What the picture's own file says is used where the brackets are empty.
    assert!(page.contains("alt=\"In its own file\" loading"), "{page}");
    assert!(
        page.contains("width=\"400\" height=\"200\" alt=\"\" loading"),
        "{page}"
    );
    assert!(page.contains("jpg\" alt=\"In the call\">"), "{page}");
    assert!(page.contains("jpg\" alt=\"In its own file\">"), "{page}");
}

#[test]
fn a_picture_a_template_shows_is_noted_where_its_description_is_read_empty() {
    let site = Site::new(
        "template",
        "{% set p = page.image('photo-1.jpg') %}<img src=\"{{ p.src }}\">\n\
         {% set q = page.image('/img/photo-2.jpg') %}<img src=\"{{ q.src }}\" alt=\"{{ q.alt }}\">",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    site.put("content/post-1/photo-1.jpg", &jpeg(30, 20));
    site.put("static/img/photo-2.jpg", &jpeg(30, 20));
    let said = site.check();
    // Only the picture whose description the template reads is told, at
    // the call that asked for it.
    assert!(
        said.contains(
            "note  Missing alt text: `static/img/photo-2.jpg`\n  \
             templates/page.html, line 4\n   \
             4 │ {% set q = page.image('/img/photo-2.jpg') %}\
             <img src=\"{{ q.src }}\" alt=\"{{ q.alt }}\">\n     \
             │            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^\n"
        ) && said.contains("Describe it where it is shown, or mark it decorative."),
        "{said}"
    );
    assert_eq!(said.matches("Missing alt text:").count(), 1);

    // A template asking once is told at the call.
    site.write(
        "templates/page.html",
        &format!("{BASE}{{{{ page.image('photo-1.jpg').alt }}}}\n"),
    );
    let said = site.check();
    assert!(
        said.contains(
            "note  Missing alt text: `content/post-1/photo-1.jpg`\n  \
             templates/page.html, line 3\n   \
             3 │ {{ page.image('photo-1.jpg').alt }}\n     \
             │    ^^^^^^^^^^^^^^^^^^^^^^^^^\n"
        ),
        "{said}"
    );
    site.write(
        "templates/page.html",
        &format!("{BASE}{{{{ page.image('photo-1.jpg', decorative=true).alt }}}}\n"),
    );
    let said = site.check();
    assert!(!said.contains("Missing alt text:"), "{said}");
}

#[test]
fn a_picture_handed_to_a_snippet_is_noted_only_where_its_description_is_read() {
    let site = Site::new("handed", "{{ page.content }}");
    site.write(
        "snippets/shown.html",
        "{# takes photo #}\n\
         <img src=\"{{ photo.src }}\" alt=\"{{ alt if alt is defined else photo.alt }}\">\n",
    );
    site.write(
        "snippets/image.html",
        "{# takes photo, alt #}\n\
         {% set picture = page.image(photo) %}\n\
         {% if alt is defined %}{{ shown(picture, alt=alt) }}{% else %}\
         {{ shown(picture) }}{% endif %}\n",
    );
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n\
         @image(\"photo-1.jpg\", alt=\"In the call\")\n\n\
         @image(\"photo-2.jpg\")\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(30, 20));
    site.put("content/post-1/photo-2.jpg", &jpeg(30, 20));
    let said = site.check();
    assert!(
        said.contains("note  Missing alt text: `content/post-1/photo-2.jpg`\n"),
        "{said}"
    );
    assert_eq!(said.matches("Missing alt text:").count(), 1);
}

/// A picture with no alt text that a template shows inside a link or a
/// button is told at the call that asks for it, and the link or the button
/// is told as having no words: a template's `<a>` may hold another picture
/// on another page. So is a link a template writes round an image in the
/// writing.
#[test]
fn a_picture_a_template_shows_inside_a_link_is_told_and_so_is_the_link() {
    const ALT: &str = "note  Missing alt text: `content/post-1/photo-1.jpg`";
    const ALONE: &str =
        "  Describe it with `alt:` in `content/post-1/photo-1.jpg.yaml`, or mark it decorative.";
    const LINK: &str =
        "note  Missing link text: the link to `/post-1/` has nothing a screen reader can read out";
    const INSIDE: &str = "  Give the `<img>` inside it an `alt` saying where the link leads.";
    let cases: &[(&str, &str, &str, &[&str])] = &[
        (
            "linked",
            "{% set p = page.image('photo-1.jpg') %}\
             <a href=\"/post-1/\"><img src=\"{{ p.src }}\" alt=\"{{ p.alt }}\"></a>",
            "",
            &[ALT, ALONE, LINK, INSIDE],
        ),
        (
            "a button",
            "{% set p = page.image('photo-1.jpg') %}\
             <button><img src=\"{{ p.src }}\" alt=\"{{ p.alt }}\"></button>",
            "",
            &[
                ALT,
                ALONE,
                "note  Missing button text: a `<button>` has nothing a screen reader can read out",
                "  Give the `<img>` inside it an `alt` saying what the button does.",
            ],
        ),
        (
            "in a snippet the writing calls",
            "{{ page.content }}",
            "<a href=\"/post-1/\">@image(\"photo-1.jpg\")</a>\n",
            &[LINK, INSIDE, ALT, ALONE],
        ),
        (
            "linked by a snippet the writing calls",
            "{{ page.content }}",
            "@figure(\"photo-1.jpg\")\n",
            &[ALT, ALONE, LINK, INSIDE],
        ),
        (
            "the writing's image in a template's link",
            "<a href=\"/post-1/\">{{ page.content }}</a>",
            "![](photo-1.jpg)\n",
            &[
                "note  Missing alt text: `photo-1.jpg`",
                "  Describe it in the brackets, or mark it decorative.",
                LINK,
                INSIDE,
            ],
        ),
    ];
    for (name, layout, writing, expected) in cases {
        let site = Site::new("linked", layout);
        site.write("snippets/image.html", IMAGE);
        site.write(
            "snippets/figure.html",
            "{# takes photo #}\n\
             {% set picture = page.image(photo) %}\n\
             <a href=\"/post-1/\"><img src=\"{{ picture.src }}\" alt=\"{{ picture.alt }}\"></a>\n",
        );
        site.write(
            "content/post-1/index.md",
            &format!("---\ntitle: Post 1\n---\n{writing}"),
        );
        site.put("content/post-1/photo-1.jpg", &jpeg(30, 20));
        let said = site.check();
        let told: Vec<&str> = (said.lines())
            .filter(|line| {
                ["note  ", "  Say ", "  Give ", "  Describe "]
                    .iter()
                    .any(|opens| line.starts_with(opens))
            })
            .collect();
        assert_eq!(told, *expected, "{name}: {said}");
    }
}

#[test]
fn a_piece_writing_pictures_is_handed_the_picture() {
    let site = Site::new("piece", "{{ page.content }}");
    site.write(
        "snippets/markdown/image.html",
        "{% if image.avif %}<picture><source srcset=\"{{ image.avif }}\">\
         <img src=\"{{ image.src }}\" alt=\"{{ alt }}\"></picture>\
         {% elif image %}\
         <img src=\"{{ image.src }}\" width=\"{{ image.width }}\" alt=\"{{ alt }}\">\
         {% else %}<img src=\"{{ url }}\" alt=\"{{ alt }}\" class=\"elsewhere\">{% endif %}",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n\n![A diagram](diagram-1.gif)\n\n\
         ![Elsewhere](https://example.org/photo-2.jpg)\n\n![](photo-1.jpg)\n",
    );
    site.put("content/photo-1.jpg", &jpeg(400, 200));
    site.put("content/diagram-1.gif", &gif(40, 30));
    site.write("content/photo-1.jpg.yaml", "alt: In its own file\n");
    let said = site.build();
    assert!(said.contains("Made 2 resized copies of 1 image."), "{said}");
    assert!(!said.contains("note"), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "<p><picture><source srcset=\"/photo-1-400.#.avif 400w\">\
         <img src=\"/photo-1-400.#.jpg\" alt=\"A photograph\"></picture></p>\n\
         <p><img src=\"/diagram-1.gif\" width=\"40\" alt=\"A diagram\"></p>\n\
         <p><img src=\"https://example.org/photo-2.jpg\" alt=\"Elsewhere\" class=\"elsewhere\"></p>\n\
         <p><picture><source srcset=\"/photo-1-400.#.avif 400w\">\
         <img src=\"/photo-1-400.#.jpg\" alt=\"In its own file\"></picture></p>"
    );
}

#[test]
fn a_piece_that_names_no_copy_has_none_made() {
    let site = Site::new("piece-plain", "{{ page.content }}");
    site.write(
        "snippets/markdown/image.html",
        "<img src=\"{{ url }}\" alt=\"{{ alt }}\">",
    );
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(400, 200));
    let said = site.build();
    assert!(!said.contains("Made"), "{said}");
    assert!(site.published("post-1/photo-1.jpg"));
}

#[test]
fn a_heic_picture_a_template_shows_is_told_at_the_call_that_asks_for_it() {
    let site = Site::new(
        "heic-template",
        "{{ page.image('/img/photo-1.jpg').src }}\n{{ page.image('/img/photo-2.heic').src }}",
    );
    for n in [1, 2] {
        site.write(
            &format!("content/post-{n}/index.md"),
            &format!("---\ntitle: Post {n}\n---\n"),
        );
    }
    site.put("static/img/photo-1.jpg", &jpeg(30, 20));
    site.put("static/img/photo-2.heic", HEIC);
    let said = site.check();
    let expected = "\
warning  HEIC image: Statac publishes `static/img/photo-2.heic` unchanged, with no resized copies
  templates/page.html, line 4
   4 │ {{ page.image('/img/photo-2.heic').src }}
     │    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  While building: content/post-1/index.md
  And while building 1 more:
    content/post-2/index.md
  Export it as a JPEG: most browsers can't show a HEIC image.
  More: https://statac.dev/e/heic-image/
";
    assert!(said.contains(expected), "{said}");
    assert_eq!(said.matches("HEIC image:").count(), 1, "{said}");
}

#[test]
fn a_heic_picture_is_published_as_it_is_and_told_where_it_is_shown() {
    let site = Site::new("heic", "{{ page.content }}");
    site.write("snippets/image.html", IMAGE);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.heic)\n\n\
         @image(\"photo-2.heic\", alt=\"Another\")\n",
    );
    site.put("content/post-1/photo-1.heic", HEIC);
    site.put("content/post-1/photo-2.heic", HEIC);
    let said = site.check();
    let expected = "\
warning  HEIC image: Statac publishes `content/post-1/photo-1.heic` unchanged, with no resized copies
  content/post-1/index.md, line 4
   4 │ ![A photograph](photo-1.heic)
     │                 ^^^^^^^^^^^^
  Export it as a JPEG: most browsers can't show a HEIC image.
  More: https://statac.dev/e/heic-image/

warning  HEIC image: Statac publishes `content/post-1/photo-2.heic` unchanged, with no resized copies
  content/post-1/index.md, line 6
   6 │ @image(\"photo-2.heic\", alt=\"Another\")
     │ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  Export it as a JPEG: most browsers can't show a HEIC image.
  More: https://statac.dev/e/heic-image/
";
    assert!(said.contains(expected), "{said}");
    let said = site.build();
    assert!(said.contains("2 warnings: 2 heic-image."), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "<p><img src=\"photo-1.heic\" alt=\"A photograph\" loading=\"lazy\" decoding=\"async\"></p>\n\
         <img src=\"/post-1/photo-2.heic\" alt=\"Another\">"
    );
    assert!(site.published("post-1/photo-1.heic") && site.published("post-1/photo-2.heic"));
}

#[test]
fn a_file_beside_the_page_gives_its_address_its_size_and_its_text() {
    let site = Site::new(
        "file",
        "{% set f = page.file('drawing-1.svg') %}{{ f.url }}|{{ f.name }}|{{ f.size }}|\
         {{ f.text | safe }}\n{{ page.file('/files/notes 1.txt').url }}",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    site.write(
        "content/post-1/drawing-1.svg",
        "<svg><title>A drawing</title></svg>",
    );
    site.write("static/files/notes 1.txt", "Notes.\n");
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "/post-1/drawing-1.svg|drawing-1.svg|35|<svg><title>A drawing</title></svg>\n\
         /files/notes%201.txt"
    );
}

/// A snippet showing a file kept beside the writing as code, where it is
/// text, and as a link where it isn't.
const FILE: &str = "{% set file = page.file(name) %}\n\
{% set code = file.code %}\n\
{% if code %}\n\
<figure class=\"code\"><figcaption><a href=\"{{ file.url }}\" download>{{ file.name }}</a>\
</figcaption><pre><code{% if file.language %} class=\"language-{{ file.language }}\"{% endif %}>\
{{ code }}</code></pre></figure>\n\
{% else %}\n\
<p><a href=\"{{ file.url }}\" download>{{ file.name }}</a></p>\n\
{% endif %}\n";

#[test]
fn a_file_beside_the_page_is_shown_as_code_coloured_by_its_name() {
    let site = Site::new("file-code", "{{ page.content }}");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\ncode_colours: true\n",
    );
    site.write("snippets/file.html", FILE);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n@file(\"code-1.rs\")\n\n@file(\"notes-1.txt\")\n\n\
         @file(\"data-1.bin\")\n\n@file(\"code-2.rs\")\n",
    );
    site.write("content/post-1/code-1.rs", "fn main() {}\n");
    site.write("content/post-1/notes-1.txt", "a < b\n");
    site.put("content/post-1/data-1.bin", &[0xff, 0xfe, 0x00]);
    site.write("content/post-1/code-2.rs", &"//".repeat(100_001));
    assert_eq!(site.check(), "No problems.\n");
    site.build();
    let html = site.page("post-1/index.html");
    let (code, rest) = html.split_once('\n').unwrap();
    assert!(
        code.starts_with(
            "<figure class=\"code\"><figcaption><a href=\"/post-1/code-1.rs\" download>\
             code-1.rs</a></figcaption><pre><code class=\"language-rs\"><span class=\"hl-"
        ) && code.ends_with("</code></pre></figure>"),
        "{code}"
    );
    assert_eq!(
        rest,
        "<figure class=\"code\"><figcaption><a href=\"/post-1/notes-1.txt\" download>\
         notes-1.txt</a></figcaption><pre><code>a &lt; b</code></pre></figure>\n\
         <p><a href=\"/post-1/data-1.bin\" download>data-1.bin</a></p>\n\
         <p><a href=\"/post-1/code-2.rs\" download>code-2.rs</a></p>"
    );
}

#[test]
fn an_unknown_language_is_told_at_its_own_highlight_where_two_colour_worked_out_text() {
    let site = Site::new(
        "highlight-two",
        "<code>{{ (\"let x\" ~ \" = 1;\") | highlight(\"pyhton\") }}</code>\n\
         <code>{{ (\"let x\" ~ \" = 1;\") | highlight(\"rbuy\") }}</code>",
    );
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\ncode_colours: true\n",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    let said = site.check();
    for (word, line, carets) in [("pyhton", 3, 41), ("rbuy", 4, 39)] {
        assert!(
            said.contains(&format!(
                "Statac has no colours for `{word}`\n  templates/page.html, line {line}\n   \
                 {line} │ \
                 <code>{{{{ (\"let x\" ~ \" = 1;\") | highlight(\"{word}\") }}}}</code>\n     \
                 │          {}\n",
                "^".repeat(carets)
            )),
            "{said}"
        );
    }
}

#[test]
fn html_left_open_is_told_at_its_own_markdown_where_two_show_worked_out_values() {
    let site = Site::new(
        "markdown-two",
        "{{ (\"<div>a\" ~ \"\") | markdown }}\n{{ (\"<section>b\" ~ \"\") | markdown }}",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    let said = site.check();
    for (tag, line, value) in [("div", 3, "<div>a"), ("section", 4, "<section>b")] {
        let call = format!("(\"{value}\" ~ \"\") | markdown");
        assert!(
            said.contains(&format!(
                "`<{tag}>` is never closed\n  templates/page.html, line {line}\n   \
                 {line} │ {{{{ {call} }}}}\n     \
                 │    {}\n",
                "^".repeat(call.len())
            )),
            "{said}"
        );
    }
}

#[test]
fn text_a_template_colours_is_told_of_a_language_unknown_and_a_line_too_long() {
    let site = Site::new(
        "highlight",
        "<code>{{ \"let x = 1;\" | highlight(\"rust\") }}</code>\n\
         <code>{{ \"a < b\" | highlight(\"text\") }}</code>\n\
         <code>{{ \"let x = 1;\" | highlight(\"pyhton\") }}</code>\n\
         <code>{{ page.file(\"code-1.rs\").code | length }}</code>",
    );
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\ncode_colours: true\n",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    site.write("content/post-1/code-1.rs", &"x".repeat(5_001));
    let said = site.check();
    assert_eq!(
        said,
        "\
warning  Unknown code language: Statac has no colours for `pyhton`
  templates/page.html, line 5
   5 │ <code>{{ \"let x = 1;\" | highlight(\"pyhton\") }}</code>
     │          ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  Did you mean `python`?
  More: https://statac.dev/e/unknown-code-language/

warning  Code not coloured: a line in this block is 5,001 characters long
  templates/page.html, line 6
   6 │ <code>{{ page.file(\"code-1.rs\").code | length }}</code>
     │          ^^^^^^^^^^^^^^^^^^^^^^
  More: https://statac.dev/e/code-not-coloured/

2 warnings: 1 code-not-coloured, 1 unknown-code-language.
"
    );
    site.build();
    let html = site.page("post-1/index.html");
    let lines: Vec<&str> = html.lines().collect();
    assert!(lines[0].starts_with("<code><span class=\"hl-"), "{html}");
    assert_eq!(
        lines[1..],
        [
            "<code>a &lt; b</code>",
            "<code>let x = 1;</code>",
            "<code>5001</code>"
        ]
    );
}

#[test]
fn a_video_and_a_recording_beside_the_page_give_their_address_and_type() {
    let site = Site::new(
        "players",
        "{% set v = page.video('video-1.mp4') %}{{ v.src }} {{ v.type }}\n\
         {% set a = page.audio('recording-1.MP3') %}{{ a.src }} {{ a.type }}",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    site.put("content/post-1/video-1.mp4", b"video");
    site.put("content/post-1/recording-1.MP3", b"audio");
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "/post-1/video-1.mp4 video/mp4\n/post-1/recording-1.MP3 audio/mpeg"
    );
}

#[test]
fn a_video_kept_elsewhere_gives_its_player_and_the_still_fetched_for_it() {
    let site = Site::new(
        "videos",
        "{% set y = page.youtube('https://youtu.be/abcdefghijk?t=1m30s') %}\
         {{ y.id }} {{ y.start }} {{ y.embed }} {% if y.thumbnail %}{{ y.thumbnail.src }}{% else %}no still{% endif %} | \
         {% set v = page.vimeo('https://vimeo.com/123456789') %}\
         {{ v.id }} {{ v.start | default('from the start') }} {{ v.embed }} \
         {% if v.thumbnail %}{{ v.thumbnail.src }}{% else %}{{ v.thumbnail | default('no still') }}{% endif %}",
    );
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    site.put("fetched/stills/youtube-abcdefghijk.jpg", &jpeg(400, 200));
    // A picture beside the page is the author's own, and no still.
    site.put("content/post-1/vimeo-123456789.jpg", &jpeg(400, 200));
    let said = site.build();
    assert!(said.contains("Made 2 resized copies of 1 image."), "{said}");
    assert!(said.contains("1 warning: 1 no-thumbnail."), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "abcdefghijk 90 https://www.youtube-nocookie.com/embed/abcdefghijk?autoplay=1&amp;start=90 \
         /stills/youtube-abcdefghijk-400.#.jpg | \
         123456789 from the start https://player.vimeo.com/video/123456789?autoplay=1 no still"
    );
    assert_eq!(
        site.copies("stills"),
        [
            "youtube-abcdefghijk-400.#.avif",
            "youtube-abcdefghijk-400.#.jpg"
        ]
    );
    // Nothing fetched is published as it stands.
    assert!(!site.published("stills/youtube-abcdefghijk.jpg"));
    assert!(!site.published("fetched/stills/youtube-abcdefghijk.jpg"));
    assert!(!site.published("fetched/record"));
}

/// A start the address doesn't give is no value, as a field a page
/// doesn't set is none: printed, it stops the build, and `default` gives
/// something in its place, as the message says.
#[test]
fn a_start_the_address_does_not_give_is_no_value() {
    for host in ["youtube('abcdefghijk')", "vimeo('123456789')"] {
        let site = Site::new("no-start", &format!("{{{{ page.{host}.start }}}}"));
        site.write("content/post-1.md", "---\ntitle: Post 1\n---\n");
        let (said, code) = site.run(&["build", "--today=2026-10-04"]);
        assert_eq!(code, 1, "{said}");
        let line = format!("{{{{ page.{host}.start }}}}");
        let under = format!("{}{}", " ".repeat(3), "^".repeat(line.len() - 6));
        assert!(
            said.starts_with(&format!(
                "\
error  No value: nothing is set for `page.{host}.start`
  templates/page.html, line 3
   3 │ {line}
     │ {under}
  While building: content/post-1.md
  Write `page.{host}.start | default(\"\")` to show nothing if it isn't set.
  More: https://statac.dev/e/no-value/
"
            )),
            "{said}"
        );
        site.write(
            "templates/page.html",
            &format!("{BASE}{{{{ page.{host}.start | default(\"\") }}}}|\n"),
        );
        site.build();
        assert_eq!(site.page("post-1/index.html"), "|");
    }
}

/// A video shown whole is its id, and handed whole to a snippet, it is
/// read only for what the snippet reads: neither asks for its `start`.
#[test]
fn a_video_shown_or_handed_whole_needs_no_start() {
    let site = Site::new(
        "video-whole",
        "{{ page.vimeo('123456789') }}|{{ page.youtube('abcdefghijk') }}|\
         {{ player(page.youtube('abcdefghijk')) }}",
    );
    site.write("snippets/player.html", "{# takes video #}{{ video.id }}");
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\n");
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "123456789|abcdefghijk|abcdefghijk"
    );
}

#[test]
fn a_still_is_read_by_its_bytes_and_shown_as_copies_whatever_the_settings() {
    let site = Site::new(
        "still-png",
        "{% set y = page.youtube('abcdefghijk') %}{{ y.thumbnail.src }} {{ y.thumbnail.width }}",
    );
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\noptimise_images: false\n",
    );
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\n");
    // A PNG, kept under the name every still has.
    site.put("fetched/stills/youtube-abcdefghijk.jpg", &png(48, 32));
    let said = site.build();
    assert!(said.contains("Made 2 resized copies of 1 image."), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "/stills/youtube-abcdefghijk-48.#.jpg 48"
    );
    assert_eq!(
        site.copies("stills"),
        [
            "youtube-abcdefghijk-48.#.avif",
            "youtube-abcdefghijk-48.#.jpg"
        ]
    );
    assert!(!site.published("stills/youtube-abcdefghijk.jpg"));

    // A file there that is no photograph is never shown as it is.
    site.put("fetched/stills/youtube-abcdefghijk.jpg", &gif(48, 32));
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains(
            "error  Unreadable image: `fetched/stills/youtube-abcdefghijk.jpg` isn't a JPEG, \
             a PNG or a still WebP\n"
        ) && said.contains("  Delete it, then run `statac refresh` to fetch one.\n"),
        "{said}"
    );
}

/// A snippet showing a video kept on YouTube in place of its still.
const YOUTUBE: &str = "{# takes address #}\n\
    {% set video = page.youtube(address) %}\n\
    {% if video.thumbnail %}<img src=\"{{ video.thumbnail.src }}\">{% endif %}\n";

#[test]
fn a_video_with_no_still_is_told_once_at_the_first_call_in_the_writing() {
    let site = Site::new("no-thumbnail", "{{ page.content }}");
    site.write("snippets/youtube.html", YOUTUBE);
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\nA video.\n\n@youtube(\"https://youtu.be/abcdefghijk\")\n\n\
         @youtube(\"https://youtu.be/bcdefghijkl\")\n",
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\n---\nThe same video.\n\n@youtube(\"abcdefghijk\")\n",
    );
    let said = site.check();
    assert_eq!(
        said,
        "\
warning  No thumbnail: `statac refresh` hasn't fetched an image for this video yet
  content/post-1.md, line 6
   6 │ @youtube(\"https://youtu.be/abcdefghijk\")
     │           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  Run `statac refresh` to fetch one.
  More: https://statac.dev/e/no-thumbnail/

warning  No thumbnail: `statac refresh` hasn't fetched an image for this video yet
  content/post-1.md, line 8
   8 │ @youtube(\"https://youtu.be/bcdefghijkl\")
     │           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  Run `statac refresh` to fetch one.
  More: https://statac.dev/e/no-thumbnail/

2 warnings: 2 no-thumbnail.
"
    );
    let said = site.build();
    assert!(said.contains("2 warnings: 2 no-thumbnail."), "{said}");
    assert!(!site.root.join("fetched").exists());

    // Quietened by name, and counted still.
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\nquiet: [no-thumbnail]\n",
    );
    let said = site.check();
    assert_eq!(said, "2 warnings, all quietened.\n");
}

#[test]
fn a_video_a_template_shows_with_no_still_is_told_at_the_templates_call() {
    let site = Site::new(
        "no-still-template",
        "{% set video = page.vimeo('https://vimeo.com/123456789') %}\n\
         {% if video.thumbnail %}{{ video.thumbnail.src }}{% endif %}",
    );
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\n");
    site.write("content/post-2.md", "---\ntitle: Post 2\n---\n");
    site.write(
        "fetched/record",
        "statac fetched 1\nstills/vimeo-123456789.jpg\tabsent\t2026-10-01\n",
    );
    let said = site.check();
    assert_eq!(
        said,
        "\
warning  No thumbnail: Vimeo has no image for this video
  templates/page.html, line 3
   3 │ {% set video = page.vimeo('https://vimeo.com/123456789') %}
     │                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^
  Show an image of your own in its place.
  More: https://statac.dev/e/no-thumbnail/

1 warning: 1 no-thumbnail.
"
    );
}

#[test]
fn a_video_whose_still_is_never_read_or_has_none_is_never_told() {
    let site = Site::new(
        "no-still-unread",
        "{{ page.youtube('abcdefghijk').embed }} \
         {% if page.youtube('https://www.youtube.com/embed/videoseries?list=PL0123').thumbnail %}\
         still{% else %}none{% endif %}",
    );
    site.write("content/post-1.md", "---\ntitle: Post 1\n---\n");
    assert_eq!(site.check(), "No problems.\n");
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "https://www.youtube-nocookie.com/embed/abcdefghijk?autoplay=1 none"
    );
}

/// What a page says of each of two pictures a template asks for, then
/// of those `page.photos()` gathers.
const BOTH: &str = "{{ page.content }}\n\
    {% for name in ['diagram-1.png', 'drawing-2.png'] %}\
    {% set p = page.image(name) %}\
    {{ p.src }}|{{ p.avif }}|{{ p.fallback }}|{{ p.full }}|{{ p.width }}x{{ p.height }}\n\
    {% endfor %}\
    {% for p in page.images() %}{{ p.name }}:{{ p.avif }} {% endfor %}";

/// A PNG that says it moves: a PNG with the chunk announcing its frames
/// before its pixels.
fn moving_png(width: u32, height: u32) -> Vec<u8> {
    let still = png(width, height);
    let mut out = still[..33].to_vec();
    out.extend(b"\0\0\0\x08acTL\0\0\0\x01\0\0\0\0\xb4\x2d\xe9\xa0");
    out.extend(&still[33..]);
    out
}

#[test]
fn a_comma_in_a_photographs_name_never_parts_the_addresses_of_its_copies() {
    let site = Site::new("comma", "{{ page.content }}");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![One](photo,1.jpg)\n",
    );
    site.put("content/post-1/photo,1.jpg", &jpeg(400, 200));
    site.build();
    let page = site.page("post-1/index.html");
    assert!(
        page.contains("srcset=\"photo%2C1-400.#.avif 400w\""),
        "{page}"
    );
    assert!(
        page.contains("srcset=\"photo%2C1-400.#.jpg 400w\""),
        "{page}"
    );
    assert_eq!(site.copies("post-1").len(), 2);
}

#[test]
fn a_png_is_published_as_copies_however_it_is_shown_and_one_that_moves_as_it_is() {
    let site = Site::new("png", BOTH);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![One](diagram-1.png) ![Two](drawing-2.png)\n",
    );
    site.put("content/post-1/diagram-1.png", &png(500, 200));
    site.put("content/post-1/drawing-2.png", &moving_png(500, 200));
    let said = site.build();
    assert!(said.contains("Made 4 resized copies of 1 image."), "{said}");
    let sizes = "(max-width: 46rem) 100vw, 46rem";
    assert_eq!(
        site.page("post-1/index.html"),
        format!(
            "<p><picture><source type=\"image/avif\" srcset=\"diagram-1-400.#.avif 400w, \
             diagram-1-500.#.avif 500w\" sizes=\"{sizes}\"><img src=\"diagram-1-500.#.jpg\" \
             srcset=\"diagram-1-400.#.jpg 400w, diagram-1-500.#.jpg 500w\" sizes=\"{sizes}\" \
             width=\"500\" height=\"200\" alt=\"One\" loading=\"lazy\" decoding=\"async\">\
             </picture> <img src=\"drawing-2.png\" alt=\"Two\" width=\"500\" height=\"200\" \
             loading=\"lazy\" decoding=\"async\"></p>\n\
             /post-1/diagram-1-500.#.jpg|/post-1/diagram-1-400.#.avif 400w, \
             /post-1/diagram-1-500.#.avif 500w|/post-1/diagram-1-400.#.jpg 400w, \
             /post-1/diagram-1-500.#.jpg 500w|/post-1/diagram-1-500.#.jpg|500x200\n\
             /post-1/drawing-2.png|||/post-1/drawing-2.png|500x200\n\
             Diagram 1:/post-1/diagram-1-400.#.avif 400w, /post-1/diagram-1-500.#.avif 500w"
        )
    );
    assert_eq!(
        site.copies("post-1"),
        [
            "diagram-1-400.#.avif",
            "diagram-1-400.#.jpg",
            "diagram-1-500.#.avif",
            "diagram-1-500.#.jpg",
        ]
    );
    // Shown as copies and linked by nothing, the PNG stays behind.
    assert!(!site.published("post-1/diagram-1.png"));
    assert!(site.published("post-1/drawing-2.png"));

    // A piece writing the pictures is handed them the same.
    site.write(
        "snippets/markdown/image.html",
        "[{{ image.src }}|{{ image.fallback }}|{{ image.width }}]",
    );
    site.build();
    assert!(
        site.page("post-1/index.html").starts_with(
            "<p>[/post-1/diagram-1-500.#.jpg|/post-1/diagram-1-400.#.jpg 400w, \
             /post-1/diagram-1-500.#.jpg 500w|500] [/post-1/drawing-2.png||500]</p>"
        ),
        "{}",
        site.page("post-1/index.html")
    );
}

#[test]
fn a_small_photograph_is_published_as_copies_at_its_own_width() {
    let site = Site::new("small", "{{ page.content }}");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![Small](photo-1.jpg) ![Small too](diagram-2.png)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(120, 80));
    site.put("content/post-1/diagram-2.png", &png(120, 80));
    let said = site.build();
    assert!(
        said.contains("Made 4 resized copies of 2 images."),
        "{said}"
    );
    assert_eq!(
        site.copies("post-1"),
        [
            "diagram-2-120.#.avif",
            "diagram-2-120.#.jpg",
            "photo-1-120.#.avif",
            "photo-1-120.#.jpg",
        ]
    );
    // Neither file is published as it is, with whatever it carries.
    assert!(!site.published("post-1/photo-1.jpg"));
    assert!(!site.published("post-1/diagram-2.png"));
}

/// What `check` says of a site whose one post's layout is `layout`: the
/// first problem's headline, where it is, and its suggestion.
fn told(name: &str, layout: &str) -> String {
    let site = Site::new(name, layout);
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    site.put("content/post-1/photo-1.jpg", &jpeg(30, 20));
    site.write("content/post-1/notes-1.txt", "Notes.\n");
    site.put("content/post-1/notes-2.bin", &[0xff, 0xfe, 0x00]);
    let said = site.check();
    let start = said.find("error  ").unwrap_or_else(|| panic!("{said}"));
    let told = &said[start..];
    let end = told.find("\n  More:").unwrap_or(told.len());
    told[..end].to_owned()
}

#[test]
fn each_mistake_asking_for_a_picture_or_a_file_is_told_at_the_call() {
    let at = |call: &str| {
        let carets = "^".repeat(call.len());
        format!(
            "\n  templates/page.html, line 3\n   3 │ {{{{ {call} }}}}\n     │    {carets}\n  \
             While building: content/post-1/index.md"
        )
    };
    for (n, (call, headline, suggestion)) in [
        (
            "page.image('photo-2.jpg')",
            "Missing file: there is no `content/post-1/photo-2.jpg`",
            Some("Did you mean `photo-1.jpg`?"),
        ),
        (
            "page.image('/img/photo-1.jpg')",
            "Missing file: there is no `/img/photo-1.jpg`",
            None,
        ),
        (
            "page.image()",
            "Wrong argument: `page.image` needs the name of one image",
            Some("Write it as `page.image(\"photo-1.jpg\")`."),
        ),
        (
            "page.image('')",
            "Wrong argument: `page.image` needs the name of one image",
            Some("Write it as `page.image(\"photo-1.jpg\")`."),
        ),
        (
            "page.image('photo-1.jpg', 'photo-2.jpg')",
            "Wrong argument: `page.image` needs the name of one image",
            Some("Write it as `page.image(\"photo-1.jpg\")`."),
        ),
        (
            "page.image('https://example.org/photo-1.jpg')",
            "Wrong argument: `page.image` needs a file of the site, not \
             `https://example.org/photo-1.jpg`",
            None,
        ),
        (
            "page.image('photo-1.jpg', size='10rem')",
            "Wrong argument: `page.image` takes no `size`",
            Some("Did you mean `sizes`?"),
        ),
        (
            "page.image('photo-1.jpg', sizes=10)",
            "Wrong argument: `page.image` needs `sizes` as text",
            Some(
                "Write it the way a browser reads it: `sizes=\"(max-width: 40rem) 100vw, 40rem\"`.",
            ),
        ),
        (
            "page.image('photo-1.jpg', full='yes')",
            "Wrong argument: `page.image` needs `full` to be `true` or `false`",
            None,
        ),
        (
            "page.images('photo-1.jpg')",
            "Wrong argument: `page.images` needs a list of names",
            Some(
                "Write it as `page.images([\"photo-1.jpg\", \"photo-2.jpg\"])`, or \
                 `page.images()` for every JPEG, PNG or WebP image beside the page.",
            ),
        ),
        (
            "page.file('notes-2.txt')",
            "Missing file: there is no `content/post-1/notes-2.txt`",
            Some("Did you mean `notes-1.txt`?"),
        ),
        (
            "page.file('notes-1.txt', as='text')",
            "Wrong argument: `page.file` takes no `as`",
            None,
        ),
        (
            "page.file('notes-2.bin').text",
            "Can't read `content/post-1/notes-2.bin`: it isn't UTF-8 text",
            None,
        ),
        (
            "page.video('notes-1.txt')",
            "Wrong argument: `page.video` needs a video a browser plays, not `notes-1.txt`",
            Some("Save it as `.mp4`, `.webm` or `.ogv`."),
        ),
        (
            "page.audio('notes-1.txt')",
            "Wrong argument: `page.audio` needs a recording a browser plays, not `notes-1.txt`",
            Some("Save it as `.mp3`, `.m4a`, `.ogg`, `.wav` or `.flac`."),
        ),
        (
            "page.audio()",
            "Wrong argument: `page.audio` needs the name of one recording",
            Some("Write it as `page.audio(\"recording-1.mp3\")`."),
        ),
        (
            "page.youtube('video-1')",
            "Wrong argument: `page.youtube` needs a YouTube URL or id, not `video-1`",
            Some(
                "Write the URL YouTube gives for the video, such as \
                 `https://www.youtube.com/watch?v=abcdefghijk`.",
            ),
        ),
        (
            "page.vimeo()",
            "Wrong argument: `page.vimeo` needs a Vimeo URL or id",
            Some(
                "Write the URL Vimeo gives for the video, such as \
                 `https://vimeo.com/123456789`.",
            ),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let said = told(&format!("mistake-{n}"), &format!("{{{{ {call} }}}}"));
        let suggestion = suggestion.map_or_else(String::new, |s| format!("\n  {s}"));
        assert_eq!(
            said,
            format!("error  {headline}{}{suggestion}", at(call)),
            "{call}"
        );
    }
}

#[test]
fn a_name_never_given_is_told_as_a_missing_value() {
    let said = told("never-given", "{{ page.image(page.shows).src }}");
    assert!(
        said.starts_with("error  No value: nothing is set for `page.shows`"),
        "{said}"
    );
}

#[test]
fn a_photograph_that_cannot_be_read_stops_the_template_asking_for_it() {
    let site = Site::new("cut-short", "{{ page.image('photo-1.jpg').src }}");
    site.write("content/post-1/index.md", "---\ntitle: Post 1\n---\n");
    let whole = jpeg(30, 20);
    site.put("content/post-1/photo-1.jpg", &whole[..20]);
    let said = site.check();
    assert!(
        said.contains(
            "error  Unreadable image: `content/post-1/photo-1.jpg` is cut short or damaged\n  \
             templates/page.html, line 3\n"
        ),
        "{said}"
    );
}

/// A snippet a site might keep of its own, by its path in
/// `tests/snippets/`.
macro_rules! snippet {
    ($path:literal) => {
        include_str!(concat!("snippets/", $path))
    };
}

#[test]
fn snippets_of_a_sites_own_show_its_pictures_and_videos_as_they_are_written() {
    let site = Site::new("own-snippets", "{{ page.content }}");
    site.write("snippets/image.html", snippet!("image.html"));
    site.write("snippets/picture.html", snippet!("picture.html"));
    site.write("snippets/gallery.html", snippet!("gallery.html"));
    site.write("snippets/video.html", snippet!("video.html"));
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n\
         @image(\"photo-1.jpg\", alt=\"In the call\", caption=\"A *caption*\")\n\n\
         @image(\"photo-2.jpg\")\n\n\
         @image(\"diagram-1.gif\")\n\n\
         @image(\"flourish-1.gif\", decorative=true)\n\n\
         @gallery(\"photo-2.jpg\", \"photo-1.jpg\", \"flourish-1.gif\")\n\n\
         @gallery()\n\n\
         @video(\"video-1.mp4\", poster=\"photo-1.jpg\")\n\n\
         @video(\"video-1.mp4\")\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(1000, 600));
    site.put("content/post-1/photo-2.jpg", &jpeg(400, 200));
    site.put("content/post-1/diagram-1.gif", &gif(40, 30));
    site.put("content/post-1/flourish-1.gif", &gif(40, 30));
    site.put("content/post-1/video-1.mp4", b"video");
    site.write(
        "content/post-1/photo-2.jpg.yaml",
        "alt: In its own file\ncaption: Its own caption\n",
    );
    let said = site.check();
    // The first call describes its picture; a gallery reads each
    // picture's own description, and shows one that has no copies as
    // it is.
    for (picture, line) in [
        ("diagram-1.gif", 8),
        ("photo-1.jpg", 12),
        ("flourish-1.gif", 12),
    ] {
        assert!(
            said.contains(&format!(
                "note  Missing alt text: `content/post-1/{picture}`\n  \
                 content/post-1/index.md, line {line}\n"
            )),
            "{said}"
        );
    }
    assert!(said.contains("3 notes: 3 missing-alt-text."), "{said}");
    site.build();
    let page = site.page("post-1/index.html");
    for shown in [
        "alt=\"In the call\" loading=\"lazy\" decoding=\"async\"></picture>\n\
         <figcaption>A <em>caption</em></figcaption>",
        "alt=\"In its own file\" loading=\"lazy\" decoding=\"async\"></picture>\n\
         <figcaption>Its own caption</figcaption>",
        "<img src=\"/post-1/diagram-1.gif\" width=\"40\" height=\"30\" alt=\"\" loading=\"lazy\" \
         decoding=\"async\">\n</figure>",
        "<a href=\"/post-1/photo-1-1000.#.jpg\" aria-label=\"Photo 1\">",
        "<a href=\"/post-1/photo-2-400.#.jpg\">",
        "sizes=\"(max-width: 30rem) 100vw, 15rem\"",
        "<a href=\"/post-1/flourish-1.gif\" aria-label=\"Flourish 1\">\n    \
         <img src=\"/post-1/flourish-1.gif\" width=\"40\" height=\"30\" alt=\"\" loading=\"lazy\" \
         decoding=\"async\">\n  </a>",
        "<video controls preload=\"metadata\" poster=\"/post-1/photo-1-1000.#.jpg\">\n  \
         <source src=\"/post-1/video-1.mp4\" type=\"video/mp4\">",
        "<video controls preload=\"metadata\">",
    ] {
        assert!(page.contains(shown), "{shown}\n{page}");
    }
    // Each gallery shows both photographs.
    assert_eq!(page.matches("<a href=\"/post-1/photo-").count(), 4);

    let site = Site::new("own-piece", "{{ page.content }}");
    site.write(
        "snippets/markdown/image.html",
        snippet!("markdown/image.html"),
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n\n![A diagram](diagram-1.gif)\n\n\
         ![Elsewhere](https://example.org/photo-2.jpg)\n",
    );
    site.put("content/photo-1.jpg", &jpeg(400, 200));
    site.put("content/diagram-1.gif", &gif(40, 30));
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "<p><picture><source type=\"image/avif\" srcset=\"/photo-1-400.#.avif 400w\" \
         sizes=\"(max-width: 46rem) 100vw, 46rem\"><img src=\"/photo-1-400.#.jpg\" \
         srcset=\"/photo-1-400.#.jpg 400w\" sizes=\"(max-width: 46rem) 100vw, 46rem\" \
         width=\"400\" height=\"200\" alt=\"A photograph\"></picture></p>\n\
         <p><img src=\"/diagram-1.gif\" alt=\"A diagram\"></p>\n\
         <p><img src=\"https://example.org/photo-2.jpg\" alt=\"Elsewhere\"></p>"
    );
}

#[test]
fn a_snippet_shows_a_picture_of_the_sites_own_in_place_of_the_still_fetched() {
    let site = Site::new("own-poster", "{{ page.content }}");
    site.write("snippets/youtube.html", snippet!("youtube.html"));
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n@youtube(\"abcdefghijk\", poster=\"still-1.jpg\")\n",
    );
    site.put("content/post-1/still-1.jpg", &jpeg(400, 200));
    assert_eq!(site.check(), "No problems.\n");
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "<a href=\"https://www.youtube-nocookie.com/embed/abcdefghijk?autoplay=1\">\n  \
         <img src=\"/post-1/still-1-400.#.jpg\" alt=\"Play the video\">\n</a>"
    );

    // Handed none, it shows the still fetched, and says where there is none.
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n@youtube(\"abcdefghijk\")\n",
    );
    let said = site.check();
    assert!(said.contains("1 warning: 1 no-thumbnail."), "{said}");
}

#[test]
fn a_snippet_for_code_numbers_its_lines_and_names_those_picked_out() {
    let site = Site::new("own-code", "{{ page.content }}");
    site.write(
        "snippets/markdown/code.html",
        snippet!("markdown/code.html"),
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\n```rust 2,4\nlet x = 1 < 2;\n```\n\n```\nas it is\n```\n",
    );
    assert_eq!(site.check(), "No problems.\n");
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "<pre class=\"numbered\" data-lines=\"2,4\"><code class=\"language-rust\">\
         let x = 1 &lt; 2;</code></pre>\n\
         <pre class=\"numbered\"><code>as it is</code></pre>"
    );
}

#[test]
fn a_snippet_for_code_is_handed_it_coloured_however_it_comes_to_read_it() {
    let site = Site::new("own-code-coloured", "{{ page.content }}");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\ncode_colours: true\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\n```yaml\na: 1\n```\n",
    );
    // The second reads what it was handed under a name it also sets,
    // where the line setting it is never reached.
    let snippets = [
        "<pre><code>{{ body }}</code></pre>\n",
        "{% if not code %}{% set body = \"\" %}{% endif %}<pre><code>{{ body }}</code></pre>\n",
    ];
    for snippet in snippets {
        site.write("snippets/markdown/code.html", snippet);
        site.build();
        let page = site.page("post-1/index.html");
        assert!(
            page.starts_with("<pre><code><span class=\"hl-"),
            "{snippet}: {page}"
        );
    }
}

const MENTIONED: &str = "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n\
                         url: https://example.org/\nwebmentions: true\n";

/// The face kept for the sender `a.example`, by its name.
const FACE: &str = "0123456789abcdef.jpg";

/// A mention's file, of `kind`, to the address `target`, with `more`
/// lines after.
fn mention(kind: &str, target: &str, more: &str) -> String {
    format!(
        "kind: \"{kind}\"\nurl: \"https://a.example/post-1/\"\ntarget: \"{target}\"\n\
         fetched: \"2026-10-06\"\n{more}"
    )
}

impl Site {
    /// A site using webmentions, built with `layout`, with one post.
    fn mentioned(name: &str, layout: &str) -> Site {
        let site = Site::new(name, layout);
        site.write("statac.yaml", MENTIONED);
        site.write(
            "templates/redirect.html",
            "<a href=\"{{ to }}\">{{ title }}</a>\n",
        );
        site.write("content/post-1.md", "---\ntitle: Post 1\n---\n");
        site
    }
}

/// Every name a mention answers to, a mention to a line.
const EVERY_MENTION_NAME: &str = "{% for m in page.webmentions.comments %}\
    said {{ m.kind }} {% if m.author %}{{ m.author }}{% endif %} \
    {% if m.author_url %}{{ m.author_url }}{% endif %} {{ m.url }} \
    {% if m.published %}{{ m.published }} {{ m.published.year }}{% endif %} \
    {% if m.content %}{{ m.content }}{% endif %} \
    {% if m.avatar %}{{ m.avatar.src }} {{ m.avatar.width }} {{ m.avatar.height }} [{{ m.avatar.alt }}]{% endif %}\n\
    {% endfor %}{% for m in page.webmentions.reactions %}faces {{ m.kind }} {{ m.url }}\n{% endfor %}";

#[test]
fn a_pages_mentions_are_said_or_faces_oldest_first_with_every_field() {
    let site = Site::mentioned("mentions", EVERY_MENTION_NAME);
    let to = "https://example.org/post-1/";
    site.write(
        "fetched/mentions/3.yaml",
        &mention(
            "replied",
            to,
            "published: \"2026-09-05\"\nwords: \"Three.\"\n",
        ),
    );
    site.write(
        "fetched/mentions/1.yaml",
        &mention(
            "mentioned",
            to,
            &format!(
                "author: \"A\"\nauthor_url: \"https://a.example/\"\nface: \"{FACE}\"\n\
                 published: \"2026-09-03\"\nwords: \"Words.\"\n"
            ),
        ),
    );
    site.write("fetched/mentions/2.yaml", &mention("bookmarked", to, ""));
    site.write(
        "fetched/mentions/10.yaml",
        &mention("replied", to, "published: \"2026-09-05\"\n"),
    );
    site.write(
        "fetched/mentions/5.yaml",
        &mention("liked", to, "published: \"2026-09-01\"\n"),
    );
    site.write("fetched/mentions/4.yaml", &mention("reposted", to, ""));
    site.put(&format!("fetched/faces/{FACE}"), &jpeg(96, 96));
    assert_eq!(site.check(), "No problems.\n");
    let said = site.build();
    assert!(said.contains("Made 2 resized copies of 1 image."), "{said}");
    assert_eq!(
        site.page("post-1/index.html"),
        "said mentioned A https://a.example/ https://a.example/post-1/ September 3, 2026 2026 \
         Words. /faces/0123456789abcdef-96.#.jpg 96 96 []\n\
         said replied   https://a.example/post-1/ September 5, 2026 2026 Three. \n\
         said replied   https://a.example/post-1/ September 5, 2026 2026  \n\
         said bookmarked   https://a.example/post-1/   \n\
         faces liked https://a.example/post-1/\n\
         faces reposted https://a.example/post-1/"
    );
    assert_eq!(
        site.copies("faces"),
        ["0123456789abcdef-96.#.avif", "0123456789abcdef-96.#.jpg"]
    );
    assert!(!site.published(&format!("faces/{FACE}")));
    assert!(!site.published("fetched/mentions/1.yaml"));
}

#[test]
fn a_site_built_at_another_address_keeps_the_mentions_of_its_own() {
    let site = Site::mentioned(
        "mentions-elsewhere",
        "{{ page.webmentions.comments | length }} {{ page.url | absolute }}\n{{ webmention() }}",
    );
    site.write(
        "fetched/mentions/1.yaml",
        &mention("replied", "https://example.org/post-1/", ""),
    );
    let ours = "<link rel=\"webmention\" href=\"https://webmention.io/example.org/webmention\">";
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        format!("1 https://example.org/post-1/\n{ours}")
    );
    for address in [
        "https://preview.example.org/",
        "https://example.org/preview/42/",
    ] {
        let at = format!("--url={address}");
        let (said, code) = site.run(&["check", "--today=2026-10-04", &at]);
        assert_eq!((said.as_str(), code), ("No problems.\n", 0), "{address}");
        let (said, code) = site.run(&["build", "--today=2026-10-04", &at]);
        assert_eq!(code, 0, "{address}: {said}");
        assert_eq!(
            site.page("post-1/index.html"),
            format!("1 {address}post-1/\n{ours}"),
            "{address}"
        );
    }
}

#[test]
fn every_page_answers_its_mentions_whatever_mentions_says() {
    let site = Site::mentioned(
        "mentions-everywhere",
        "{{ page.webmentions.comments | length }} {{ page.webmentions.reactions | length }}\
         {% if page.mentions %} shown{% endif %}",
    );
    site.write(
        "templates/home.html",
        &format!(
            "{BASE}{{% for post in site.pages %}}{{{{ post.url }}}} \
             {{{{ post.webmentions.comments | length }}}};{{% endfor %}}"
        ),
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\nmentions: true\n---\n",
    );
    site.write(
        "fetched/mentions/1.yaml",
        &mention("replied", "https://example.org/post-2/", ""),
    );
    site.build();
    assert_eq!(site.page("post-1/index.html"), "0 0");
    assert_eq!(site.page("post-2/index.html"), "1 0 shown");
    let home = fs::read_to_string(site.root.join("_site/index.html")).unwrap();
    assert!(
        home.contains("/post-1/ 0;") && home.contains("/post-2/ 1;"),
        "{home}"
    );
}

#[test]
fn a_mention_reaches_its_page_however_its_address_is_written() {
    let layout = "{{ page.webmentions.comments | length }}";
    let site = Site::mentioned("mention-addresses", layout);
    site.write(
        "statac.yaml",
        &MENTIONED.replace("https://example.org/", "https://www.example.org/blog/"),
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\nredirect_from: [/old-2/]\n---\n",
    );
    site.write("content/привет.md", "---\ntitle: Привет\n---\n");
    for (id, target) in [
        (1, "https://www.example.org/blog/post-1/"),
        (2, "http://example.org/blog/post-1"),
        (3, "https://EXAMPLE.org/blog/post-1/#mentions"),
        (4, "https://example.org/blog/old-2/"),
        (
            5,
            "https://example.org/blog/%D0%BF%D1%80%D0%B8%D0%B2%D0%B5%D1%82/",
        ),
        (6, "https://example.org/blog/привет"),
    ] {
        site.write(
            &format!("fetched/mentions/{id}.yaml"),
            &mention("replied", target, ""),
        );
    }
    assert_eq!(site.check(), "No problems.\n");
    site.build();
    assert_eq!(site.page("post-1/index.html"), "3");
    assert_eq!(site.page("post-2/index.html"), "1");
    assert_eq!(site.page("привет/index.html"), "2");
}

#[test]
fn a_mention_of_no_page_is_told_at_its_target_and_kept() {
    let site = Site::mentioned(
        "mention-of-no-page",
        "{{ page.webmentions.comments | length }}",
    );
    let target = "https://example.org/post-9/";
    site.write("fetched/mentions/1.yaml", &mention("replied", target, ""));
    // On the site's host, under no path of the site, it is still no page.
    site.write(
        "fetched/mentions/2.yaml",
        &mention("liked", "https://example.com/post-1/", ""),
    );
    assert_eq!(
        site.check(),
        "\
warning  Mention of no page: nothing is published at `https://example.org/post-9/`
  fetched/mentions/1.yaml, line 3
   3 │ target: \"https://example.org/post-9/\"
     │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  If the page moved, add the old URL to its `redirect_from`.
  More: https://statac.dev/e/mention-of-no-page/

warning  Mention of no page: nothing is published at `https://example.com/post-1/`
  fetched/mentions/2.yaml, line 3
   3 │ target: \"https://example.com/post-1/\"
     │         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  If the page moved, add the old URL to its `redirect_from`.
  More: https://statac.dev/e/mention-of-no-page/

2 warnings: 2 mention-of-no-page.
"
    );
    let said = site.build();
    assert!(said.contains("2 warnings: 2 mention-of-no-page."), "{said}");
    assert!(site.root.join("fetched/mentions/1.yaml").is_file());
    site.write(
        "statac.yaml",
        &format!("{MENTIONED}quiet: [mention-of-no-page]\n"),
    );
    assert_eq!(site.check(), "2 warnings, all quietened.\n");

    // Its old address in the page's `redirect_from`, it is the page's.
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\nredirect_from: [/post-9/]\n---\n",
    );
    site.build();
    assert_eq!(site.page("post-1/index.html"), "1");
}

#[test]
fn a_mention_of_a_page_left_out_waits_with_it() {
    let site = Site::mentioned(
        "mention-of-a-draft",
        "{{ page.webmentions.comments | length }}",
    );
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndraft: true\nredirect_from: [/old-2/]\n---\n",
    );
    site.write(
        "content/post-3.md",
        "---\ntitle: Post 3\ndate: 2026-12-01\n---\n",
    );
    for (id, target) in [
        (1, "https://example.org/post-2/"),
        (2, "https://example.org/old-2/"),
        (3, "https://example.org/post-3/"),
    ] {
        site.write(
            &format!("fetched/mentions/{id}.yaml"),
            &mention("replied", target, ""),
        );
    }
    let said = site.check();
    assert!(!said.contains("mention-of-no-page"), "{said}");
    let (said, code) = site.run(&["build", "--today=2026-10-04", "--drafts"]);
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("mention-of-no-page"), "{said}");
    assert_eq!(site.page("post-2/index.html"), "2");
    assert_eq!(site.page("post-3/index.html"), "1");
}

#[test]
fn a_mention_file_that_isnt_as_statac_writes_it_is_told_at_its_value() {
    let site = Site::mentioned("mention-files", "{{ page.webmentions.comments | length }}");
    let to = "https://example.org/post-1/";
    let cases: [(&str, String, &str, &str); 12] = [
        (
            "1",
            "kind: \"replied\nurl: x\n".into(),
            "Malformed YAML",
            "line 1",
        ),
        (
            "2",
            "- replied\n".into(),
            "Wrong kind of value: `fetched/mentions/2.yaml` is `name: value` lines, not a list",
            "line 1",
        ),
        (
            "3",
            mention("replied", to, "kinds: \"liked\"\n"),
            "Unknown field: `kinds`",
            "line 5",
        ),
        (
            "4",
            mention("loved", to, ""),
            "Wrong kind of value: `kind` is `replied`, `liked`, `reposted`, `bookmarked` or `mentioned`, not `\"loved\"`",
            "line 1",
        ),
        (
            "5",
            mention("replied", to, "").replace("https://a.example/post-1/", "javascript:alert(1)"),
            "Wrong kind of value: `url` is a full URL, such as `https://example.org`, not `\"javascript:alert(1)\"`",
            "line 2",
        ),
        (
            "6",
            mention("replied", to, "author_url: \"ftp://a.example/\"\n"),
            "Wrong kind of value: `author_url` is a full URL, such as `https://example.org`, not `\"ftp://a.example/\"`",
            "line 5",
        ),
        (
            "7",
            mention("replied", to, "face: \"../record\"\n"),
            "Wrong kind of value: `face` is an avatar's file name, such as `0123456789abcdef.jpg`, not `\"../record\"`",
            "line 5",
        ),
        (
            "8",
            mention("replied", to, &format!("face: \"{FACE}\"\n")),
            "Missing file: there is no `fetched/faces/0123456789abcdef.jpg`",
            "line 5",
        ),
        (
            "9",
            mention("replied", to, "published: \"yesterday\"\n"),
            "Wrong kind of value: `published` is a date, not `\"yesterday\"`",
            "line 5",
        ),
        (
            "10",
            mention("replied", to, "").replace(&format!("target: \"{to}\"\n"), ""),
            "Missing field: `target`",
            "line 1",
        ),
        (
            "11",
            mention("replied", to, "words: [Words.]\n"),
            "Wrong kind of value: `words` is text, not a list",
            "line 5",
        ),
        (
            "12",
            mention("replied", to, "url: \"https://a.example/\"\n"),
            "Duplicate key: `url`",
            "line 5",
        ),
    ];
    for (id, text, headline, line) in &cases {
        site.write(&format!("fetched/mentions/{id}.yaml"), text);
        let (said, code) = site.run(&["check", "--today=2026-10-04"]);
        assert_eq!(code, 1, "{id}: {said}");
        assert!(
            said.contains(&format!("error  {headline}"))
                && said.contains(&format!("  fetched/mentions/{id}.yaml, {line}\n")),
            "{id}: {said}"
        );
        fs::remove_file(site.root.join(format!("fetched/mentions/{id}.yaml"))).unwrap();
    }
    let said = site.check();
    assert_eq!(said, "No problems.\n");
}

#[test]
fn a_misspelt_name_of_a_mention_is_told_with_the_nearest() {
    let site = Site::mentioned(
        "mention-names",
        "{{ page.webmentions.coments | length }}\n\
         {% for m in page.webmentions.reactions %}{{ m.contnt }}{% endfor %}",
    );
    let said = site.check();
    assert!(
        said.contains("Unknown field: `page.webmentions.coments`")
            && said.contains("Did you mean `page.webmentions.comments`?"),
        "{said}"
    );
    assert!(
        said.contains("Unknown field: `m.contnt`") && said.contains("Did you mean `m.content`?"),
        "{said}"
    );
}

#[test]
fn webmention_called_twice_on_a_site_without_webmentions_is_told_at_a_call() {
    let site = Site::mentioned("webmention-twice", "{{ webmention() }}\n{{ webmention() }}");
    site.write("statac.yaml", &MENTIONED.replace("webmentions: true\n", ""));
    let said = site.check();
    assert!(
        said.contains(
            "warning  Webmentions off: `webmention()` writes nothing here\n  \
             templates/page.html, line 3\n"
        ),
        "{said}"
    );
}

#[test]
fn webmention_names_the_relay_only_on_a_site_using_webmentions() {
    let site = Site::mentioned("webmention", "{{ webmention() }}");
    site.write("content/post-2.md", "---\ntitle: Post 2\n---\n");
    assert_eq!(site.check(), "No problems.\n");
    site.build();
    assert_eq!(
        site.page("post-1/index.html"),
        "<link rel=\"webmention\" href=\"https://webmention.io/example.org/webmention\">"
    );

    // Off, it writes nothing, told once at the call.
    site.write("statac.yaml", &MENTIONED.replace("webmentions: true\n", ""));
    assert_eq!(
        site.check(),
        "\
warning  Webmentions off: `webmention()` writes nothing here
  templates/page.html, line 3
   3 │ {{ webmention() }}
     │    ^^^^^^^^^^^^
  Add `webmentions: true` to `statac.yaml`, or remove the call.
  More: https://statac.dev/e/webmentions-off/

1 warning: 1 webmentions-off.
"
    );
    site.build();
    assert_eq!(site.page("post-1/index.html"), "");

    // Without `url`, it writes nothing, and the site is told why.
    site.write(
        "statac.yaml",
        &MENTIONED.replace("url: https://example.org/\n", ""),
    );
    let said = site.check();
    assert!(
        said.starts_with(
            "note  No site URL: Statac leaves out webmentions until the site has a `url`\n  \
             statac.yaml, line 5\n"
        ),
        "{said}"
    );
    site.build();
    assert_eq!(site.page("post-1/index.html"), "");
}

#[test]
fn a_token_in_the_settings_stops_the_build_unshown() {
    let site = Site::mentioned("token-in-settings", "");
    site.write(
        "statac.yaml",
        &format!("{MENTIONED}webmention_token: s3cr3t-t0k3n\n"),
    );
    for command in ["check", "build"] {
        let (said, code) = site.run(&[command, "--today=2026-10-04"]);
        assert_eq!(code, 1, "{said}");
        assert!(!said.contains("s3cr3t"), "{said}");
        assert!(
            said.contains(
                "error  Token in settings: `webmention_token` doesn't belong in a file kept \
                 with the site\n  statac.yaml, line 7\n   7 │ webmention_token: •••\n"
            ),
            "{said}"
        );
        assert!(
            said.contains(
                "  Remove the line, add `webmentions: true`, and put the token in \
                 `STATAC_WEBMENTION_TOKEN` or in the site's `.statac/token`.\n"
            ),
            "{said}"
        );
    }
}

#[test]
fn a_layout_shows_a_pages_webmentions_by_their_kind() {
    let site = Site::mentioned(
        "own-mentions",
        &format!("{{{{ page.content }}}}\n{}", snippet!("mentions.html")),
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\nmentions: true\n---\n",
    );
    site.write("content/post-2.md", "---\ntitle: Post 2\n---\n");
    let to = "https://example.org/post-1/";
    site.write(
        "fetched/mentions/1.yaml",
        &mention(
            "replied",
            to,
            &format!(
                "author: \"A\"\nauthor_url: \"https://a.example/\"\nface: \"{FACE}\"\n\
                 published: \"2026-09-03\"\nwords: \"<b>Words.</b>\"\n"
            ),
        ),
    );
    site.write(
        "fetched/mentions/2.yaml",
        &mention("liked", to, &format!("face: \"{FACE}\"\n")),
    );
    site.write("fetched/mentions/3.yaml", &mention("mentioned", to, ""));
    site.put(&format!("fetched/faces/{FACE}"), &jpeg(96, 96));
    assert_eq!(site.check(), "No problems.\n");
    site.build();
    let page = site.page("post-1/index.html");
    for shown in [
        "<a href=\"https://a.example/post-1/\" rel=\"nofollow ugc\"><img \
         src=\"/faces/0123456789abcdef-96.#.jpg\" width=\"96\" height=\"96\" alt=\"\"> liked</a>",
        "<a href=\"https://a.example/\" rel=\"nofollow ugc\">A</a>",
        "<a href=\"https://a.example/post-1/\" rel=\"nofollow ugc\">replied</a> on September 3, 2026",
        "<p>&lt;b&gt;Words.&lt;/b&gt;</p>",
        "<a href=\"https://a.example/post-1/\" rel=\"nofollow ugc\">mentioned</a>",
    ] {
        assert!(page.contains(shown), "{shown}\n{page}");
    }
    assert_eq!(page.matches("<article>").count(), 2, "{page}");
    // A page that doesn't turn them on shows none.
    assert!(!site.page("post-2/index.html").contains("mentions"));
}
