//! Pictures in small sites built end to end: found from the writing,
//! photographs published as copies, and the copies kept between builds.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::{Rgb, RgbImage};
use statac::album::Seen;
use statac::content::Run;

struct Site {
    root: PathBuf,
}

/// A photograph's colours: bands across a gradient, so its copies differ
/// from one another and compress as a photograph does.
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

/// A GIF as far as its size, which is as far as it is read.
fn gif(width: u16, height: u16) -> Vec<u8> {
    let mut out = b"GIF89a".to_vec();
    out.extend(width.to_le_bytes());
    out.extend(height.to_le_bytes());
    out.extend([0, 0, 0]);
    out
}

/// `jpeg` carrying `profile` in an APP2 segment after its start of image.
fn with_profile(jpeg: &[u8], profile: &[u8]) -> Vec<u8> {
    let mut body = b"ICC_PROFILE\0\x01\x01".to_vec();
    body.extend(profile);
    let length = u16::try_from(body.len() + 2).unwrap().to_be_bytes();
    let mut out = jpeg[..2].to_vec();
    out.extend([0xff, 0xe2, length[0], length[1]]);
    out.extend(body);
    out.extend(&jpeg[2..]);
    out
}

const TEMPLATE: &str = "<!doctype html>\n<title>{{ page.title }}</title>\n{{ page.content }}\n";

impl Site {
    /// A site with a home page and a template for every page.
    fn new(name: &str) -> Site {
        let root =
            std::env::temp_dir().join(format!("statac-pictures-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let site = Site { root };
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
        );
        site.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        site.write("templates/home.html", TEMPLATE);
        site.write("templates/page.html", TEMPLATE);
        site.write(
            "templates/folder.html",
            "<!doctype html>\n<title>{{ headline(page) }}</title>\n",
        );
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
    /// said, what it said aside, and its exit code.
    fn run_all(&self, args: &[&str]) -> (String, String, i32) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(args)
            .arg(&self.root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        let shown = self.root.display().to_string();
        let said = String::from_utf8(output.stdout).unwrap();
        let aside = String::from_utf8(output.stderr).unwrap();
        (
            said.replace(&shown, "SITE"),
            aside,
            output.status.code().unwrap(),
        )
    }

    /// Runs statac, which must say nothing aside.
    fn run(&self, args: &[&str]) -> (String, i32) {
        let (said, aside, code) = self.run_all(args);
        assert_eq!(aside, "", "{said}");
        (said, code)
    }

    /// Builds the site, which must build, and gives what it said.
    fn build(&self) -> String {
        let (said, code) = self.run(&["build", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{said}");
        said
    }

    fn html(&self, file: &str) -> String {
        fs::read_to_string(self.out().join(file)).unwrap()
    }

    /// The files of the built site, but for its marker.
    fn built(&self) -> Vec<String> {
        names(&self.out())
            .into_iter()
            .filter(|name| name != ".statac-built")
            .collect()
    }

    /// The copies kept between builds.
    fn kept(&self) -> Vec<String> {
        names(&self.root.join(".statac/pictures"))
            .into_iter()
            .filter(|name| name != "index")
            .collect()
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

/// `names` with each copy's eight hex digits written as `#`.
fn unhashed(names: &[String]) -> Vec<String> {
    let mut names: Vec<String> = names.iter().map(|name| unhash(name)).collect();
    names.sort();
    names
}

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

/// The line of `html` holding `needle`, its copies' hashes written as `#`.
fn line_with(html: &str, needle: &str) -> String {
    let line = html
        .lines()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no {needle} in {html}"));
    unhash(line)
}

#[test]
fn two_files_differing_in_capitals_are_told_once_photographs_or_not() {
    let site = Site::new("capitals");
    site.put("static/Photo-1.jpg", &jpeg(1200, 800));
    site.put("content/photo-1.jpg", &jpeg(1200, 800));
    site.write("static/File-1.txt", "one\n");
    site.write("content/file-1.txt", "one\n");
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    let told: Vec<&str> = said.lines().filter(|l| l.starts_with("error")).collect();
    assert_eq!(
        told,
        [
            "error  Same address: `static/File-1.txt` and `content/file-1.txt` differ only in capitals",
            "error  Same address: `static/Photo-1.jpg` and `content/photo-1.jpg` differ only in capitals",
        ]
    );
}

#[test]
fn build_times_the_copies_it_makes_of_each_photograph() {
    let site = Site::new("timed");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(900, 600));
    let timed = || {
        let (said, code) = site.run(&["build", "--timings", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{said}");
        said
    };
    let said = timed();
    let stage = said
        .lines()
        .find(|line| line.starts_with("  Making photograph copies"));
    assert!(stage.is_some(), "{said}");
    let (_, slowest) = said
        .split_once("\nSlowest photographs (copies made this build only):\n")
        .expect(&said);
    let row = slowest.lines().next().unwrap();
    assert!(row.starts_with("  content/post-1/photo-1.jpg  "), "{said}");
    assert!(
        row.ends_with(" ms  6 copies") || row.ends_with(" s  6 copies"),
        "{said}"
    );
    // Copies kept from the build before took nothing to make.
    let said = timed();
    assert!(!said.contains("Slowest photographs"), "{said}");
}

#[test]
fn a_photograph_beside_a_page_in_its_own_folder_is_published_as_copies() {
    let site = Site::new("own-folder");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\nSee ![A photograph](photo-1.jpg \"Its title\") here, \
         and ![A plain picture](picture-1.gif).\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(900, 600));
    site.put("content/post-1/picture-1.gif", &gif(40, 30));
    let said = site.build();
    assert!(
        said.contains("Made 6 copies of 1 photograph.\nWrote 2 pages and 8 other files"),
        "{said}"
    );
    let sizes = "(max-width: 46rem) 100vw, 46rem";
    assert_eq!(
        line_with(&site.html("post-1/index.html"), "<picture>"),
        format!(
            "<p>See <picture><source type=\"image/avif\" srcset=\"photo-1-400.#.avif 400w, \
             photo-1-800.#.avif 800w, photo-1-900.#.avif 900w\" sizes=\"{sizes}\">\
             <img src=\"photo-1-900.#.jpg\" srcset=\"photo-1-400.#.jpg 400w, \
             photo-1-800.#.jpg 800w, photo-1-900.#.jpg 900w\" sizes=\"{sizes}\" width=\"900\" \
             height=\"600\" alt=\"A photograph\" title=\"Its title\" loading=\"lazy\" \
             decoding=\"async\"></picture> here, and <img src=\"picture-1.gif\" \
             alt=\"A plain picture\" width=\"40\" height=\"30\" loading=\"lazy\" \
             decoding=\"async\">.</p>"
        )
    );
    // The photograph itself is not published: nothing links it.
    assert_eq!(
        unhashed(&site.built()),
        [
            "code.#.css",
            "index.html",
            "post-1/index.html",
            "post-1/photo-1-400.#.avif",
            "post-1/photo-1-400.#.jpg",
            "post-1/photo-1-800.#.avif",
            "post-1/photo-1-800.#.jpg",
            "post-1/photo-1-900.#.avif",
            "post-1/photo-1-900.#.jpg",
            "post-1/picture-1.gif",
        ]
    );
    let copy = site
        .built()
        .into_iter()
        .find(|name| name.starts_with("post-1/photo-1-400.") && name.ends_with(".jpg"))
        .unwrap();
    let copy = image::load_from_memory(&fs::read(site.out().join(copy)).unwrap()).unwrap();
    assert_eq!((copy.width(), copy.height()), (400, 267));
}

#[test]
fn a_picture_beside_a_post_that_is_one_file_reaches_its_page() {
    let site = Site::new("one-file");
    site.write(
        "content/posts/post-1.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n\n\
         ![A plain picture](picture-1.gif)\n",
    );
    site.put("content/posts/photo-1.jpg", &jpeg(400, 200));
    site.put("content/posts/picture-1.gif", &gif(40, 30));
    let (said, code) = site.run(&["check"]);
    assert_eq!((said.as_str(), code), ("No problems.\n", 0));
    site.build();
    let html = site.html("posts/post-1/index.html");
    assert_eq!(
        line_with(&html, "<picture>"),
        "<p><picture><source type=\"image/avif\" srcset=\"/posts/photo-1-400.#.avif 400w\" \
         sizes=\"(max-width: 46rem) 100vw, 46rem\"><img src=\"/posts/photo-1-400.#.jpg\" \
         srcset=\"/posts/photo-1-400.#.jpg 400w\" sizes=\"(max-width: 46rem) 100vw, 46rem\" \
         width=\"400\" height=\"200\" alt=\"A photograph\" loading=\"lazy\" \
         decoding=\"async\"></picture></p>"
    );
    assert_eq!(
        line_with(&html, "picture-1.gif"),
        "<p><img src=\"/posts/picture-1.gif\" alt=\"A plain picture\" width=\"40\" height=\"30\" \
         loading=\"lazy\" decoding=\"async\"></p>"
    );
    let built = unhashed(&site.built());
    assert!(
        built.contains(&"posts/photo-1-400.#.avif".to_owned()),
        "{built:?}"
    );
    assert!(
        built.contains(&"posts/picture-1.gif".to_owned()),
        "{built:?}"
    );
    assert!(
        !built.contains(&"posts/photo-1.jpg".to_owned()),
        "{built:?}"
    );
}

/// Where the site writes pictures itself, the page carries what its file
/// made of each address, and that is what the links are held to.
#[test]
fn a_picture_the_sites_own_file_writes_is_checked_as_the_page_carries_it() {
    let site = Site::new("own-file");
    site.write(
        "snippets/markdown/image.html",
        "{% if photo %}<img src=\"{{ photo.src }}\" alt=\"{{ alt }}\">\
         {% else %}<img src=\"{{ address }}\" alt=\"{{ alt }}\">{% endif %}",
    );
    site.write(
        "content/posts/post-1.md",
        "---\ntitle: Post 1\npermalink: /post-1/\n---\n![A photograph](photo-1.jpg)\n",
    );
    site.put("content/posts/photo-1.jpg", &jpeg(400, 200));
    let (said, code) = site.run(&["check"]);
    assert_eq!((said.as_str(), code), ("No problems.\n", 0));
    let said = site.build();
    assert!(!said.contains("warning"), "{said}");
    let html = site.html("post-1/index.html");
    assert_eq!(
        line_with(&html, "<img"),
        "<p><img src=\"/posts/photo-1-400.#.jpg\" alt=\"A photograph\"></p>"
    );

    // One the file writes as it was written leads nowhere from the page,
    // and so does one the site has no file for.
    site.write(
        "snippets/markdown/image.html",
        "<img src=\"{{ address }}\" alt=\"{{ alt }}\">",
    );
    site.write(
        "content/posts/post-1.md",
        "---\ntitle: Post 1\npermalink: /post-1/\n---\n![A photograph](photo-1.jpg)\n\n\
         ![Another](photo-9.jpg)\n",
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(
            "warning  Broken link: nothing is published at `photo-1.jpg`\n  \
             content/posts/post-1.md, line 5\n"
        ),
        "{said}"
    );
    assert!(
        said.contains(
            "warning  Broken link: nothing is published at `photo-9.jpg`\n  \
             content/posts/post-1.md, line 7\n"
        ),
        "{said}"
    );
    assert!(said.ends_with("2 warnings: 2 broken-link.\n"), "{said}");
    let said = site.build();
    assert!(said.contains("2 warnings: 2 broken-link."), "{said}");
}

#[test]
fn a_picture_is_found_in_static_and_from_the_top_under_the_sites_path() {
    let site = Site::new("static");
    site.write(
        "statac.yaml",
        "title: Site\nurl: https://example.org/blog/\nlanguage: en\ntimezone: UTC\n\
         pages.feed: false\nmax_width: 400\nimage_sizes: 20rem\n",
    );
    site.write(
        "content/post-1.md",
        "---\ntitle: Post 1\n---\n![In static](/img/photo-1.jpg)\n\n\
         ![Among the writing](/blog/shared/photo-2.jpg)\n\n\
         ![Elsewhere](https://example.com/a.jpg)\n\n\
         ![Nowhere](photo-9.jpg)\n",
    );
    site.put("static/img/photo-1.jpg", &jpeg(900, 600));
    site.put("content/shared/photo-2.jpg", &jpeg(500, 100));
    let (said, code) = site.run(&["build", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("1 warning: 1 broken-link"), "{said}");
    let html = site.html("post-1/index.html");
    // Capped at 400 wide, and reached under the site's own path.
    assert_eq!(
        line_with(&html, "In static"),
        "<p><picture><source type=\"image/avif\" srcset=\"/blog/img/photo-1-400.#.avif 400w\" \
         sizes=\"20rem\"><img src=\"/blog/img/photo-1-400.#.jpg\" \
         srcset=\"/blog/img/photo-1-400.#.jpg 400w\" sizes=\"20rem\" width=\"900\" \
         height=\"600\" alt=\"In static\" loading=\"lazy\" decoding=\"async\"></picture></p>"
    );
    assert!(
        line_with(&html, "Among the writing").contains("src=\"/blog/shared/photo-2-400.#.jpg\""),
        "{html}"
    );
    assert!(html.contains("<img src=\"https://example.com/a.jpg\" alt=\"Elsewhere\" loading"));
    assert!(html.contains("<img src=\"photo-9.jpg\" alt=\"Nowhere\" loading"));
    let built = unhashed(&site.built());
    for file in [
        "img/photo-1.jpg",
        "img/photo-1-400.#.avif",
        "img/photo-1-400.#.jpg",
        "shared/photo-2-400.#.avif",
        "shared/photo-2-400.#.jpg",
    ] {
        assert!(built.contains(&file.to_owned()), "{file} in {built:?}");
    }
    assert!(
        !built.contains(&"shared/photo-2.jpg".to_owned()),
        "{built:?}"
    );
}

#[test]
fn a_photograph_two_pages_show_has_one_set_of_copies() {
    let site = Site::new("two-pages");
    for post in ["post-1", "post-2"] {
        site.write(
            &format!("content/{post}.md"),
            "---\ntitle: Post\n---\n![A photograph](/shared/photo-1.jpg)\n",
        );
    }
    site.put("content/shared/photo-1.jpg", &jpeg(400, 200));
    let said = site.build();
    assert!(said.contains("Made 2 copies of 1 photograph."), "{said}");
    assert_eq!(site.kept().len(), 2);
    assert_eq!(
        unhashed(&site.built()),
        [
            "code.#.css",
            "index.html",
            "post-1/index.html",
            "post-2/index.html",
            "shared/photo-1-400.#.avif",
            "shared/photo-1-400.#.jpg",
        ]
    );
    assert_eq!(
        line_with(&site.html("post-1/index.html"), "<picture>"),
        line_with(&site.html("post-2/index.html"), "<picture>")
    );
}

#[test]
fn a_second_build_makes_nothing_and_publishes_the_same_bytes() {
    let site = Site::new("warm");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(900, 600));
    let said = site.build();
    assert!(said.contains("Made 6 copies of 1 photograph."), "{said}");
    let first: Vec<(String, Vec<u8>)> = site
        .built()
        .into_iter()
        .map(|name| (name.clone(), fs::read(site.out().join(name)).unwrap()))
        .collect();
    let kept = site.kept();
    assert_eq!(kept.len(), 6);

    let said = site.build();
    assert!(
        !said.contains("Made") && !said.contains("Removed"),
        "{said}"
    );
    let second: Vec<(String, Vec<u8>)> = site
        .built()
        .into_iter()
        .map(|name| (name.clone(), fs::read(site.out().join(name)).unwrap()))
        .collect();
    assert_eq!(first.len(), 9);
    assert!(first == second);
    assert_eq!(site.kept(), kept);
}

#[test]
fn a_site_read_again_opens_no_picture_it_has_read() {
    let site = Site::new("index");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(400, 200));
    site.put("content/post-1/picture-1.gif", &gif(40, 30));
    site.put("static/photo-2.jpg", &jpeg(200, 100));
    site.put("static/notes.jpg", b"not a picture at all");
    let read = || statac::build::read(&site.root, &Run::default()).0;
    // `check` keeps nothing, so a site only checked is read whole again.
    assert_eq!(read().site.pictures.opened, 4);
    site.run(&["check"]);
    assert_eq!(read().site.pictures.opened, 4);
    site.build();

    #[cfg(unix)]
    let locked = {
        use std::os::unix::fs::PermissionsExt;
        let locked = ["content/post-1/photo-1.jpg", "static/photo-2.jpg"];
        for file in locked {
            let none = fs::Permissions::from_mode(0o000);
            fs::set_permissions(site.root.join(file), none).unwrap();
            assert!(fs::read(site.root.join(file)).is_err());
        }
        locked
    };
    let again = read();
    assert_eq!(again.site.pictures.opened, 0);
    assert_eq!(again.site.pictures.len(), 3);
    let known: Vec<String> = again
        .site
        .pictures
        .iter()
        .map(|(_, picture)| {
            let size = picture.size().unwrap();
            let copies = picture.copies.len();
            format!(
                "{} {} by {}, {copies} copies",
                picture.shown(),
                size.width,
                size.height
            )
        })
        .collect();
    assert_eq!(
        known,
        [
            "content/post-1/photo-1.jpg 400 by 200, 2 copies",
            "content/post-1/picture-1.gif 40 by 30, 0 copies",
            "static/photo-2.jpg 200 by 100, 2 copies",
        ]
    );
    #[cfg(unix)]
    for file in locked {
        use std::os::unix::fs::PermissionsExt;
        let mine = fs::Permissions::from_mode(0o644);
        fs::set_permissions(site.root.join(file), mine).unwrap();
    }

    // A file changed is read again, and only that file.
    site.put("static/photo-2.jpg", &jpeg(240, 100));
    let changed = read();
    assert_eq!(changed.site.pictures.opened, 1);
    let photo = changed
        .site
        .pictures
        .iter()
        .find(|(_, picture)| picture.shown() == "static/photo-2.jpg")
        .unwrap()
        .1;
    assert!(matches!(&photo.seen, Seen::Photograph { photograph, .. }
        if photograph.shown.width == 240));
}

/// A PNG of [`pixels`]: without alpha where `last` is none, and else with
/// every pixel opaque but the last of the last row, whose alpha is `last`.
fn png(width: u32, height: u32, last: Option<u8>) -> Vec<u8> {
    use image::{ExtendedColorType, ImageEncoder};
    let rgb = pixels(width, height);
    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut out);
    match last {
        None => encoder.write_image(&rgb, width, height, ExtendedColorType::Rgb8),
        Some(last) => {
            let mut rgba = image::DynamicImage::ImageRgb8(rgb).into_rgba8();
            rgba.get_pixel_mut(width - 1, height - 1)[3] = last;
            encoder.write_image(&rgba, width, height, ExtendedColorType::Rgba8)
        }
    }
    .unwrap();
    out
}

#[test]
fn a_second_copy_is_a_png_only_where_the_photograph_is_transparent() {
    let site = Site::new("transparent");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![No alpha](picture-1.png)\n\n![Alpha unused](picture-2.png)\n\n\
         ![Alpha used](picture-3.png)\n",
    );
    site.put("content/post-1/picture-1.png", &png(400, 200, None));
    site.put("content/post-1/picture-2.png", &png(400, 200, Some(255)));
    site.put("content/post-1/picture-3.png", &png(400, 200, Some(254)));
    let read = || statac::build::read(&site.root, &Run::default()).0;
    // The copies each picture is to have, and whether it is transparent.
    let named = |read: &statac::content::Read| -> Vec<(String, bool)> {
        let mut named = Vec::new();
        for (_, picture) in read.site.pictures.iter() {
            let Seen::Photograph { transparent, .. } = picture.seen else {
                panic!("{} is no photograph", picture.shown());
            };
            for copy in &picture.copies {
                named.push((format!("post-1/{}", copy.name), transparent));
            }
        }
        named.sort();
        named
    };

    // With nothing kept, as `check` finds a site never built.
    let unbuilt = read();
    assert_eq!(unbuilt.site.pictures.opened, 3);
    let before = named(&unbuilt);
    let (said, code) = site.run(&["check"]);
    assert_eq!((said.as_str(), code), ("No problems.\n", 0));
    assert!(!site.root.join(".statac").exists());

    let said = site.build();
    assert!(said.contains("Made 6 copies of 3 photographs."), "{said}");
    let copies: Vec<String> = site
        .built()
        .into_iter()
        .filter(|name| name.starts_with("post-1/picture-"))
        .collect();
    assert_eq!(
        unhashed(&copies),
        [
            "post-1/picture-1-400.#.avif",
            "post-1/picture-1-400.#.jpg",
            "post-1/picture-2-400.#.avif",
            "post-1/picture-2-400.#.jpg",
            "post-1/picture-3-400.#.avif",
            "post-1/picture-3-400.#.png",
        ]
    );
    let names: Vec<String> = before.iter().map(|(name, _)| name.clone()).collect();
    assert_eq!(names, copies);
    let transparent: Vec<bool> = before.iter().map(|&(_, transparent)| transparent).collect();
    assert_eq!(transparent, [false, false, false, false, true, true]);
    let html = site.html("post-1/index.html");
    assert!(
        line_with(&html, "Alpha unused").contains("<img src=\"picture-2-400.#.jpg\""),
        "{html}"
    );
    assert!(
        line_with(&html, "Alpha used").contains("<img src=\"picture-3-400.#.png\""),
        "{html}"
    );
    let published = |name: &str| {
        let copy = copies.iter().find(|copy| unhash(copy) == name).unwrap();
        image::load_from_memory(&fs::read(site.out().join(copy)).unwrap()).unwrap()
    };
    assert!(!published("post-1/picture-2-400.#.jpg").color().has_alpha());
    let png_copy = published("post-1/picture-3-400.#.png").into_rgba8();
    assert_eq!(png_copy.get_pixel(399, 199)[3], 254);
    assert_eq!(png_copy.get_pixel(0, 0)[3], 255);

    // With the index kept, the same is known of files that can't be opened.
    #[cfg(unix)]
    let locked = {
        use std::os::unix::fs::PermissionsExt;
        let locked = ["picture-1.png", "picture-2.png", "picture-3.png"];
        for file in locked {
            let file = site.root.join("content/post-1").join(file);
            fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
            assert!(fs::read(&file).is_err());
        }
        locked
    };
    let built = read();
    assert_eq!(built.site.pictures.opened, 0);
    assert_eq!(named(&built), before);
    #[cfg(unix)]
    for file in locked {
        use std::os::unix::fs::PermissionsExt;
        let file = site.root.join("content/post-1").join(file);
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    }
    let (said, code) = site.run(&["check"]);
    assert_eq!((said.as_str(), code), ("No problems.\n", 0));

    // A file changed is looked at again, and only that file.
    site.put("content/post-1/picture-2.png", &png(400, 200, Some(0)));
    let changed = read();
    assert_eq!(changed.site.pictures.opened, 1);
    let seconds: Vec<String> = named(&changed)
        .into_iter()
        .filter(|(name, _)| !name.ends_with(".avif"))
        .map(|(name, transparent)| format!("{} {transparent}", unhash(&name)))
        .collect();
    assert_eq!(
        seconds,
        [
            "post-1/picture-1-400.#.jpg false",
            "post-1/picture-2-400.#.png true",
            "post-1/picture-3-400.#.png true",
        ]
    );
}

#[test]
fn a_photograph_left_alone_is_published_as_it_is() {
    let site = Site::new("left-alone");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![Left alone](photo-1.jpg) ![Developed](photo-2.jpg)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(400, 200));
    site.put("content/post-1/photo-2.jpg", &jpeg(400, 200));
    site.write(
        "content/post-1/photo-1.jpg.yaml",
        "optimise: false\nalt: A photograph\ncredit: Person 1\n",
    );
    let said = site.build();
    assert!(said.contains("Made 2 copies of 1 photograph."), "{said}");
    let html = site.html("post-1/index.html");
    assert!(
        html.contains(
            "<img src=\"photo-1.jpg\" alt=\"Left alone\" width=\"400\" height=\"200\" \
             loading=\"lazy\" decoding=\"async\"> <picture>"
        ),
        "{html}"
    );
    // The file describing a picture is never published.
    assert_eq!(
        unhashed(&site.built()),
        [
            "code.#.css",
            "index.html",
            "post-1/index.html",
            "post-1/photo-1.jpg",
            "post-1/photo-2-400.#.avif",
            "post-1/photo-2-400.#.jpg",
        ]
    );

    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\noptimise_images: false\n",
    );
    let said = site.build();
    assert!(
        said.contains("Removed 2 copies no page shows any more"),
        "{said}"
    );
    assert!(!site.html("post-1/index.html").contains("<picture>"));
    assert_eq!(
        site.built(),
        [
            site.built()[0].as_str(),
            "index.html",
            "post-1/index.html",
            "post-1/photo-1.jpg",
            "post-1/photo-2.jpg",
        ]
    );
}

#[test]
fn a_file_describing_a_picture_is_read_and_one_describing_none_is_told() {
    let site = Site::new("sidecar");
    site.put("content/photo-1.jpeg", &jpeg(30, 20));
    site.write("content/photo-1.jpg.yaml", "alt: A photograph\n");
    site.write("content/photo-1.jpeg.yaml", "optimise: perhaps\n");
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains(
            "warning  Describes no picture: there is no `content/photo-1.jpg`\n  \
             content/photo-1.jpg.yaml, line 1\n"
        ) && said.contains("Did you mean `photo-1.jpeg`?"),
        "{said}"
    );
    assert!(
        said.contains("error  Wrong kind of value: `optimise` is `true` or `false`, not `perhaps`"),
        "{said}"
    );
}

#[test]
fn a_photograph_cut_short_stops_the_build_where_the_writing_shows_it() {
    let site = Site::new("cut-short");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\nA line.\n\n![A photograph](photo-1.jpg)\n",
    );
    let whole = jpeg(400, 200);
    site.put("content/post-1/photo-1.jpg", &whole[..whole.len() / 2]);
    let (said, aside, code) = site.run_all(&["build"]);
    assert_eq!(aside, "");
    assert_eq!(code, 1, "{said}");
    assert!(
        said.starts_with(
            "error  Unreadable picture: `content/post-1/photo-1.jpg` is cut short or damaged\n  \
             content/post-1/index.md, line 6\n"
        ),
        "{said}"
    );
    assert!(said.contains("![A photograph](photo-1.jpg)\n"), "{said}");
    assert!(
        said.contains("  Copy or export the file again.\n"),
        "{said}"
    );
    assert!(
        said.ends_with("Wrote nothing: 1 error stopped the build.\n"),
        "{said}"
    );
    assert!(!site.out().exists());

    // Cut within its header, it is found without opening it.
    site.put("content/post-1/photo-1.jpg", &whole[..24]);
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("error  Unreadable picture: `content/post-1/photo-1.jpg`"),
        "{said}"
    );

    // Left alone, it is published as the file it is.
    site.write("content/post-1/photo-1.jpg.yaml", "optimise: false\n");
    let (said, code) = site.run(&["build"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        site.html("post-1/index.html")
            .contains("<img src=\"photo-1.jpg\" alt=")
    );
}

#[test]
fn a_photograph_nothing_shows_travels_and_one_linked_plainly_is_published() {
    let site = Site::new("unshown");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![Shown](photo-1.jpg) ![Shown and linked](photo-2.jpg)\n\n\
         [The whole photograph](photo-2.jpg)\n",
    );
    for photo in ["photo-1.jpg", "photo-2.jpg", "photo-3.jpg"] {
        site.put(&format!("content/post-1/{photo}"), &jpeg(400, 200));
    }
    let said = site.build();
    assert!(said.contains("Made 4 copies of 2 photographs."), "{said}");
    assert!(said.contains("Wrote 2 pages and 7 other files"), "{said}");
    assert_eq!(
        unhashed(&site.built()),
        [
            "code.#.css",
            "index.html",
            "post-1/index.html",
            "post-1/photo-1-400.#.avif",
            "post-1/photo-1-400.#.jpg",
            "post-1/photo-2-400.#.avif",
            "post-1/photo-2-400.#.jpg",
            "post-1/photo-2.jpg",
            "post-1/photo-3.jpg",
        ]
    );
    assert_eq!(
        fs::read(site.out().join("post-1/photo-2.jpg")).unwrap(),
        jpeg(400, 200)
    );
}

/// `jpeg` holding where it was taken, a latitude and a longitude of
/// nought, as a camera writes them.
fn located(jpeg: &[u8]) -> Vec<u8> {
    let entry = |tag: u16, kind: u16, count: u32, value: u32| {
        let mut out = tag.to_le_bytes().to_vec();
        out.extend(kind.to_le_bytes());
        out.extend(count.to_le_bytes());
        out.extend(value.to_le_bytes());
        out
    };
    let mut body = b"Exif\0\0II*\0".to_vec();
    body.extend(8u32.to_le_bytes());
    body.extend(1u16.to_le_bytes());
    body.extend(entry(0x8825, 4, 1, 26));
    body.extend(0u32.to_le_bytes());
    body.extend(4u16.to_le_bytes());
    body.extend(entry(0x0001, 2, 2, u32::from(b'N')));
    body.extend(entry(0x0002, 5, 3, 80));
    body.extend(entry(0x0003, 2, 2, u32::from(b'E')));
    body.extend(entry(0x0004, 5, 3, 80));
    body.extend(0u32.to_le_bytes());
    body.extend([0, 0, 0, 0, 1, 0, 0, 0].repeat(3));
    let length = u16::try_from(body.len() + 2).unwrap().to_be_bytes();
    let mut out = jpeg[..2].to_vec();
    out.extend([0xff, 0xe1, length[0], length[1]]);
    out.extend(body);
    out.extend(&jpeg[2..]);
    out
}

#[test]
fn a_picture_published_as_it_is_holding_a_location_is_noted_by_check_alone() {
    let site = Site::new("located");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![Shown](photo-1.jpg) ![Shown and linked](photo-2.jpg)\n\n\
         [The whole photograph](photo-2.jpg) ![Left alone](photo-4.jpg)\n",
    );
    let taken = located(&jpeg(400, 200));
    for photo in ["photo-1.jpg", "photo-2.jpg", "photo-3.jpg", "photo-4.jpg"] {
        site.put(&format!("content/post-1/{photo}"), &taken);
    }
    site.write("content/post-1/photo-4.jpg.yaml", "optimise: false\n");
    // One that holds none, and the same file where nothing is read for one.
    site.put("content/post-1/photo-5.jpg", &jpeg(400, 200));
    site.put("static/photo-6.jpg", &taken);

    // The photograph a page links, the one nothing shows and the one left
    // alone are published as the files they are; of the one a page shows
    // and nothing links, only copies are.
    let note = |photo: &str| {
        format!(
            "note  Location published: `content/post-1/{photo}` holds where it was taken in \
             its metadata\n  Publish a copy without it, or add `location-published` to \
             `quiet:` in `statac.yaml`.\n  More: https://statac.dev/e/location-published/\n"
        )
    };
    let noted = ["photo-2.jpg", "photo-3.jpg", "photo-4.jpg"].map(note);
    for _ in 0..2 {
        let (said, code) = site.run(&["check", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{said}");
        assert!(said.starts_with(&noted.join("\n")), "{said}");
        assert_eq!(said.matches("Location published").count(), 3, "{said}");
        assert!(
            said.contains("\n3 notes: 3 location-published.\n"),
            "{said}"
        );

        let said = site.build();
        assert!(!said.to_lowercase().contains("location"), "{said}");
        assert!(!said.contains("note"), "{said}");
        for photo in ["photo-2.jpg", "photo-3.jpg", "photo-4.jpg"] {
            let published = fs::read(site.out().join("post-1").join(photo)).unwrap();
            assert!(published == taken, "{photo}");
        }
        assert!(fs::read(site.out().join("photo-6.jpg")).unwrap() == taken);
        assert!(!site.out().join("post-1/photo-1.jpg").exists());
    }

    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n\
         quiet: [location-published]\n",
    );
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains("Location published"), "{said}");
    assert!(said.contains("3 notes, all quietened"), "{said}");
}

#[test]
fn a_drafts_photograph_is_kept_and_what_no_page_shows_is_removed() {
    let site = Site::new("sweep");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
    );
    site.write(
        "content/post-2/index.md",
        "---\ntitle: Post 2\ndraft: true\n---\n![A photograph](photo-2.jpg)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(400, 200));
    site.put("content/post-2/photo-2.jpg", &jpeg(400, 300));
    let (said, code) = site.run(&["build", "--drafts"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("Made 4 copies of 2 photographs."), "{said}");
    assert_eq!(site.kept().len(), 4);

    // The draft left out, its photograph is not published and its copies
    // are kept for when it is.
    let said = site.build();
    assert!(
        !said.contains("Made") && !said.contains("Removed"),
        "{said}"
    );
    assert_eq!(site.kept().len(), 4);
    assert_eq!(
        unhashed(&site.built()),
        [
            "code.#.css",
            "index.html",
            "post-1/index.html",
            "post-1/photo-1-400.#.avif",
            "post-1/photo-1-400.#.jpg",
        ]
    );

    // A photograph changed has new copies, and the old ones go.
    site.put("content/post-1/photo-1.jpg", &jpeg(420, 200));
    let said = site.build();
    assert!(
        said.contains("Made 2 copies of 1 photograph.\nRemoved 2 copies no page shows any more"),
        "{said}"
    );
    assert_eq!(site.kept().len(), 4);

    // No longer shown, its copies go; a build that stops removes nothing.
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n{{ no }}\n@missing()\n",
    );
    let (said, code) = site.run(&["build"]);
    assert_eq!(code, 1, "{said}");
    assert_eq!(site.kept().len(), 4);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\nNo picture.\n",
    );
    let said = site.build();
    assert!(
        said.contains("Removed 2 copies no page shows any more"),
        "{said}"
    );
    assert_eq!(site.kept().len(), 2);
    assert!(site.built().contains(&"post-1/photo-1.jpg".to_owned()));
}

#[test]
fn a_photograph_in_another_colour_space_is_converted_and_said() {
    let site = Site::new("colour");
    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\n",
    );
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
    );
    let profile = moxcms::ColorProfile::new_display_p3().encode().unwrap();
    site.put(
        "content/post-1/photo-1.jpg",
        &with_profile(&jpeg(400, 200), &profile),
    );
    site.put(
        "content/post-1/photo-2.jpg",
        &with_profile(&jpeg(30, 20), &profile),
    );
    let (said, code) = site.run(&["check"]);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with(
            "note  Colour converted: the copies of `content/post-1/photo-1.jpg` are in sRGB, \
             converted from `"
        ),
        "{said}"
    );
    assert!(said.contains("\n1 note: 1 colour-converted.\n"), "{said}");
    let said = site.build();
    assert!(said.contains("1 note: 1 colour-converted."), "{said}");
    // Said of every build, not only the one that made the copies.
    let said = site.build();
    assert!(said.contains("1 note: 1 colour-converted."), "{said}");

    site.write(
        "statac.yaml",
        "title: Site\nlanguage: en\ntimezone: UTC\npages.feed: false\nquiet: [colour-converted]\n",
    );
    let said = site.build();
    assert!(said.contains("1 note, quietened."), "{said}");
}

#[cfg(unix)]
#[test]
fn copies_that_cannot_be_kept_are_still_published() {
    use std::os::unix::fs::PermissionsExt;
    let site = Site::new("unkept");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(900, 600));
    // A file where the folder of copies would be.
    site.write(".statac/pictures", "");
    fs::set_permissions(site.root.join(".statac"), fs::Permissions::from_mode(0o555)).unwrap();
    let (said, code) = site.run(&["build"]);
    fs::set_permissions(site.root.join(".statac"), fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("1 warning: 1 cache-not-written."), "{said}");
    assert!(said.contains("Made 6 copies of 1 photograph."), "{said}");
    assert_eq!(site.built().len(), 9);
    let (said, _) = site.run(&["check"]);
    assert_eq!(said, "No problems.\n");
}

/// A site showing one photograph, and a folder outside it holding a file
/// no build may read into the site, write to or remove.
#[cfg(unix)]
fn site_and_outside(name: &str) -> (Site, PathBuf) {
    let site = Site::new(name);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
    );
    site.put("content/post-1/photo-1.jpg", &jpeg(400, 200));
    let outside = site.root.with_extension("outside");
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("private.avif"), "private").unwrap();
    (site, outside)
}

/// Whether the built site's copies are the pictures they are named as.
#[cfg(unix)]
fn copies_are_pictures(site: &Site) -> bool {
    let copies: Vec<String> = site
        .built()
        .into_iter()
        .filter(|name| name.starts_with("post-1/photo-1-400."))
        .collect();
    copies.len() == 2
        && copies.iter().all(|name| {
            let bytes = fs::read(site.out().join(name)).unwrap();
            bytes.len() > 100 && !bytes.starts_with(b"private")
        })
}

#[cfg(unix)]
#[test]
fn a_link_where_copies_are_kept_is_never_followed() {
    use std::os::unix::fs::symlink;
    for (name, linked) in [
        ("linked-kept", ".statac"),
        ("linked-pictures", ".statac/pictures"),
    ] {
        let (site, outside) = site_and_outside(name);
        fs::create_dir_all(site.root.join(linked).parent().unwrap()).unwrap();
        symlink(&outside, site.root.join(linked)).unwrap();
        for _ in 0..2 {
            let (said, code) = site.run(&["build"]);
            assert_eq!(code, 0, "{linked}: {said}");
            assert!(
                said.contains("1 warning: 1 cache-not-written."),
                "{linked}: {said}"
            );
            assert!(
                said.contains("Made 2 copies of 1 photograph."),
                "{linked}: {said}"
            );
            assert!(copies_are_pictures(&site), "{linked}");
            // Nothing was written through the link, and nothing removed.
            assert_eq!(names(&outside), ["private.avif"], "{linked}");
            assert_eq!(fs::read(outside.join("private.avif")).unwrap(), b"private");
        }
        fs::remove_dir_all(&outside).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn a_link_standing_for_a_copy_kept_is_neither_published_nor_removed() {
    use std::os::unix::fs::symlink;
    let (site, outside) = site_and_outside("linked-copy");
    site.build();
    let kept = site.root.join(".statac/pictures");
    // Each copy kept gives way to a link to the file outside; beside them
    // stand a link and a folder that are no copy of anything shown.
    let copies = site.kept();
    assert_eq!(copies.len(), 2);
    for name in &copies {
        fs::remove_file(kept.join(name)).unwrap();
        symlink(outside.join("private.avif"), kept.join(name)).unwrap();
    }
    symlink(outside.join("private.avif"), kept.join("stale.avif")).unwrap();
    fs::create_dir(kept.join("folder.avif")).unwrap();
    fs::write(kept.join("folder.avif/inside.avif"), "inside").unwrap();
    fs::write(kept.join("0000.avif"), "a copy of nothing shown").unwrap();

    let (said, code) = site.run(&["build"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("1 warning: 1 cache-not-written."), "{said}");
    assert!(said.contains("Made 2 copies of 1 photograph."), "{said}");
    assert!(copies_are_pictures(&site));
    assert_eq!(fs::read(outside.join("private.avif")).unwrap(), b"private");
    // Only the file that is itself a stale copy went.
    assert!(
        said.contains("Removed 1 copy no page shows any more"),
        "{said}"
    );
    let left = names(&kept);
    assert!(!left.contains(&"0000.avif".to_owned()), "{left:?}");
    assert!(
        left.contains(&"folder.avif/inside.avif".to_owned()),
        "{left:?}"
    );
    for name in copies.iter().map(String::as_str).chain(["stale.avif"]) {
        let found = fs::symlink_metadata(kept.join(name)).unwrap();
        assert!(found.is_symlink(), "{name}");
    }
    fs::remove_dir_all(&outside).unwrap();
}

#[cfg(unix)]
#[test]
fn a_link_standing_for_the_index_is_neither_read_nor_written() {
    use std::os::unix::fs::symlink;
    let (site, outside) = site_and_outside("linked-index");
    site.build();
    let index = site.root.join(".statac/pictures/index");
    // The index as kept, but for a size no header gave.
    let kept = fs::read_to_string(&index).unwrap();
    let altered = kept.replace("\t400\t200\t400\t200\t", "\t40\t20\t40\t20\t");
    assert_ne!(kept, altered);
    fs::write(outside.join("index"), &altered).unwrap();
    fs::remove_file(&index).unwrap();
    symlink(outside.join("index"), &index).unwrap();

    let read = statac::build::read(&site.root, &Run::default()).0;
    assert_eq!(read.site.pictures.opened, 1);
    let (_, picture) = read.site.pictures.iter().next().unwrap();
    assert_eq!(picture.size().map(|size| size.width), Some(400));

    let (said, code) = site.run(&["build"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("1 warning: 1 cache-not-written."), "{said}");
    assert!(copies_are_pictures(&site));
    assert_eq!(fs::read_to_string(outside.join("index")).unwrap(), altered);
    assert!(fs::symlink_metadata(&index).unwrap().is_symlink());
    fs::remove_dir_all(&outside).unwrap();
}

#[cfg(unix)]
#[test]
fn a_copy_kept_that_is_another_name_for_a_file_outside_is_not_published() {
    let (site, outside) = site_and_outside("hard-linked-copy");
    site.build();
    let kept = site.root.join(".statac/pictures");
    let copies = site.kept();
    assert_eq!(copies.len(), 2);
    for name in &copies {
        fs::remove_file(kept.join(name)).unwrap();
        fs::hard_link(outside.join("private.avif"), kept.join(name)).unwrap();
    }
    fs::remove_file(kept.join("index")).unwrap();
    fs::write(outside.join("index"), "statac pictures 3\n").unwrap();
    fs::hard_link(outside.join("index"), kept.join("index")).unwrap();

    let (said, code) = site.run(&["build"]);
    assert_eq!(code, 0, "{said}");
    assert!(said.contains("1 warning: 1 cache-not-written."), "{said}");
    assert!(said.contains("Made 2 copies of 1 photograph."), "{said}");
    assert!(copies_are_pictures(&site));
    assert_eq!(fs::read(outside.join("private.avif")).unwrap(), b"private");
    assert_eq!(
        fs::read_to_string(outside.join("index")).unwrap(),
        "statac pictures 3\n"
    );
    fs::remove_dir_all(&outside).unwrap();
}

/// A site whose one post shows a picture published with its width and
/// height.
fn with_a_sized_picture(name: &str) -> Site {
    let site = Site::new(name);
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A drawing](drawing-1.gif)\n",
    );
    site.put("content/post-1/drawing-1.gif", &gif(640, 400));
    site
}

const OUT_OF_PROPORTION: &str = "warning  Pictures out of proportion: ";

#[test]
fn a_rule_narrowing_pictures_and_keeping_their_height_is_told_once_at_its_property() {
    let site = with_a_sized_picture("narrowed");
    site.write(
        "content/post-2/index.md",
        "---\ntitle: Post 2\n---\n![A drawing](drawing-2.gif)\n",
    );
    site.put("content/post-2/drawing-2.gif", &gif(640, 400));
    site.write("assets/first.css", "body { margin: 0; }\n");
    site.write(
        "assets/site.css",
        "/* img { height: auto; } */\n@media (min-width: 40em) {\n  .entry img,\n  video {\n    \
         max-width: 100%;\n  }\n}\nimg { width: 50%; }\n",
    );
    site.write("assets/third.css", "img { inline-size: 100%; }\n");
    let (said, _) = site.run(&["check"]);
    assert!(
        said.contains(
            "warning  Pictures out of proportion: `img` has a `max-width` and no `height`\n  \
             assets/site.css, line 5\n   \
             5 │     max-width: 100%;\n     \
             │     ^^^^^^^^^\n  \
             Add `height: auto` to the rule.\n  \
             More: "
        ),
        "{said}"
    );
    assert_eq!(said.matches(OUT_OF_PROPORTION).count(), 1, "{said}");
    let said = site.build();
    assert!(
        said.contains("1 warning: 1 pictures-out-of-proportion."),
        "{said}"
    );

    site.write("assets/site.css", "body { margin: 0; }\n");
    let (said, _) = site.run(&["check"]);
    assert!(
        said.contains(
            "warning  Pictures out of proportion: `img` has an `inline-size` and no \
             `block-size`\n  assets/third.css, line 1\n"
        ) && said.contains("Add `block-size: auto` to the rule."),
        "{said}"
    );
}

#[test]
fn a_height_in_any_stylesheet_or_no_picture_given_its_size_says_nothing() {
    let site = with_a_sized_picture("not-narrowed");
    site.write("assets/site.css", "img { max-width: 100%; }\n");
    let (said, _) = site.run(&["check"]);
    assert!(said.contains(OUT_OF_PROPORTION), "{said}");

    for height in [
        "img { height: auto; }\n",
        ".entry img { block-size: 10rem; }\n",
        ":where(img, video) { aspect-ratio: 4 / 3; }\n",
    ] {
        site.write("assets/more/sizes.css", height);
        let (said, _) = site.run(&["check"]);
        assert!(!said.contains(OUT_OF_PROPORTION), "{height}: {said}");
    }

    // A picture written with no size is drawn at the height its width gives.
    site.write("assets/more/sizes.css", "body { margin: 0; }\n");
    site.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n<img src=\"drawing-1.gif\" alt=\"A drawing\">\n",
    );
    let (said, _) = site.run(&["check"]);
    assert!(!said.contains(OUT_OF_PROPORTION), "{said}");
}

#[test]
fn a_rule_in_sass_is_told_where_its_author_wrote_it() {
    let site = with_a_sized_picture("narrowed-in-sass");
    site.write("assets/site.scss", "@use 'parts';\nbody { margin: 0; }\n");
    site.write(
        "assets/_parts.scss",
        "// img { height: auto; }\n@mixin unused {\n  video { width: 10%; }\n}\n\
         .entry {\n  img {\n    max-width: 100%;\n  }\n}\n",
    );
    let (said, _) = site.run(&["check"]);
    assert!(
        said.contains(
            "  assets/_parts.scss, line 7\n   \
             7 │     max-width: 100%;\n     \
             │     ^^^^^^^^^\n"
        ),
        "{said}"
    );
    assert_eq!(said.matches(OUT_OF_PROPORTION).count(), 1, "{said}");

    // A rule Sass never writes out narrows nothing.
    site.write(
        "assets/_parts.scss",
        "@mixin unused {\n  img { width: 10%; }\n}\n",
    );
    let (said, _) = site.run(&["check"]);
    assert!(!said.contains(OUT_OF_PROPORTION), "{said}");

    // A height a mixin gives is in the stylesheet Sass makes.
    site.write(
        "assets/_parts.scss",
        "@mixin fluid {\n  height: auto;\n}\nimg {\n  max-width: 100%;\n  @include fluid;\n}\n",
    );
    let (said, _) = site.run(&["check"]);
    assert!(!said.contains(OUT_OF_PROPORTION), "{said}");
}
