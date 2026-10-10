//! The spare: a build into the output kept from the build before last
//! publishes, byte for byte and name for name, what a build with nothing
//! kept publishes, whatever happened to the site, the output or the spare
//! in between.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::{Rgb, RgbImage};

const FONT: &[u8] = include_bytes!("../src/cards/fonts/test/test-latin.ttf");

const LAYOUT: &str = "<!doctype html>
<title>{{ page.title }}</title>
{{ css('site.css') }}
<meta property=\"og:image\" content=\"{{ page.card.url | absolute }}\">
{{ page.content }}
";

const CARD: &str = "background: background.png
font: text.ttf

title.text: \"{{ page.og_title }}\"
title.place: centre
title.size: up to 75
title.colour: \"#F2E9D3\"
";

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

fn background() -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    RgbImage::from_pixel(1200, 630, Rgb([20, 30, 40]))
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

/// Everything under `folder` as the disk spells it: each file with its
/// bytes, each folder with none, each link by where it leads.
fn tree(folder: &Path) -> Vec<(String, Option<Vec<u8>>)> {
    fn inside(folder: &Path, rel: &str, found: &mut Vec<(String, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(folder).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            let at = match rel {
                "" => name,
                rel => format!("{rel}/{name}"),
            };
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                inside(&entry.path(), &at, found);
                found.push((format!("{at}/"), None));
            } else if kind.is_file() {
                found.push((at, Some(fs::read(entry.path()).unwrap())));
            } else {
                let leads = fs::read_link(entry.path()).unwrap();
                found.push((format!("{at} -> {}", leads.display()), None));
            }
        }
    }
    let mut found = Vec::new();
    inside(folder, "", &mut found);
    found.sort();
    found
}

/// Two copies of one site: one keeps its spare and all else Statac keeps
/// from build to build, the other has it all removed before each build.
struct Pair {
    root: PathBuf,
}

impl Pair {
    fn new(name: &str) -> Pair {
        let root = std::env::temp_dir().join(format!("statac-spare-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let pair = Pair { root };
        pair.write(
            "statac.yaml",
            "title: Site\nurl: https://example.org\nlanguage: en\ntimezone: UTC\n\
             pages.feed: false\n",
        );
        pair.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        pair.write(
            "content/post-1/index.md",
            "---\ntitle: Post 1\n---\n![A photograph](photo-1.jpg)\n",
        );
        pair.put("content/post-1/photo-1.jpg", &jpeg(500, 300));
        pair.write("content/post-2.md", "---\ntitle: Post 2\n---\nA post.\n");
        pair.write(
            "content/post-3.md",
            "---\ntitle: Post 3\ndraft: true\n---\nA draft.\n",
        );
        for layout in ["home", "page"] {
            pair.write(&format!("templates/{layout}.html"), LAYOUT);
        }
        pair.write("templates/og.yaml", CARD);
        pair.put("templates/og/background.png", &background());
        pair.put("templates/og/text.ttf", FONT);
        pair.write("assets/site.css", "body { margin: 0; }\n");
        pair.write("static/Files/notes.txt", "Notes.\n");
        pair
    }

    fn kept(&self) -> PathBuf {
        self.root.join("kept")
    }

    fn fresh(&self) -> PathBuf {
        self.root.join("fresh")
    }

    /// The site built with everything kept.
    fn out(&self) -> PathBuf {
        self.kept().join("_site")
    }

    fn spare(&self) -> PathBuf {
        self.kept().join(".statac").join("spare")
    }

    fn write(&self, path: &str, text: &str) {
        self.put(path, text.as_bytes());
    }

    fn put(&self, path: &str, bytes: &[u8]) {
        for site in [self.kept(), self.fresh()] {
            let path = site.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
    }

    fn remove(&self, path: &str) {
        for site in [self.kept(), self.fresh()] {
            fs::remove_file(site.join(path)).unwrap();
        }
    }

    fn remove_folder(&self, path: &str) {
        for site in [self.kept(), self.fresh()] {
            fs::remove_dir_all(site.join(path)).unwrap();
        }
    }

    fn rename(&self, from: &str, to: &str) {
        for site in [self.kept(), self.fresh()] {
            fs::rename(site.join(from), site.join(to)).unwrap();
        }
    }

    fn build(site: &Path) -> (String, i32) {
        Pair::build_with(site, &[])
    }

    fn build_with(site: &Path, options: &[&str]) -> (String, i32) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(["build", "--today=2026-10-04"])
            .args(options)
            .arg(site)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert!(output.stderr.is_empty(), "{output:?}");
        let said = String::from_utf8(output.stdout).unwrap();
        (said, output.status.code().unwrap())
    }

    /// Builds the site with everything kept for one run at `address`, into
    /// the same output, and holds every page to naming its card there.
    fn build_at(&self, address: &str) {
        let (said, code) = Pair::build_with(&self.kept(), &[&format!("--url={address}")]);
        assert_eq!(code, 0, "{said}");
        let image = format!(
            "<meta property=\"og:image\" content=\"{}/",
            address.trim_end_matches('/')
        );
        let pages: Vec<String> = tree(&self.out())
            .into_iter()
            .filter(|(name, _)| name.ends_with(".html"))
            .map(|(_, bytes)| String::from_utf8(bytes.unwrap()).unwrap())
            .collect();
        assert!(!pages.is_empty());
        for page in pages {
            assert!(page.contains(&image), "{address}: {page}");
        }
    }

    /// Builds both, twice so that each of the two folders the kept site
    /// goes between is built into, and holds the one to the other; gives
    /// what the last build with everything kept said.
    fn same(&self, after: &str) -> String {
        let mut last = String::new();
        for nth in 1..3 {
            let (said, code) = Pair::build(&self.kept());
            assert_eq!(code, 0, "{after}, build {nth}: {said}");
            let fresh = self.fresh();
            for made in [".statac", "_site"] {
                let _ = fs::remove_dir_all(fresh.join(made));
            }
            let (said_fresh, code) = Pair::build(&fresh);
            assert_eq!(code, 0, "{after}, build {nth}: {said_fresh}");
            let (kept, fresh) = (tree(&self.out()), tree(&fresh.join("_site")));
            let names = |tree: &[(String, Option<Vec<u8>>)]| -> Vec<String> {
                tree.iter().map(|(name, _)| name.clone()).collect()
            };
            assert_eq!(names(&kept), names(&fresh), "{after}, build {nth}");
            for ((name, kept), (_, fresh)) in kept.iter().zip(&fresh) {
                assert!(kept == fresh, "{after}, build {nth}: {name} differs");
            }
            last = said;
        }
        last
    }
}

impl Drop for Pair {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_build_into_the_spare_publishes_what_a_build_with_nothing_kept_does() {
    let pair = Pair::new("same");
    pair.same("a first build and a second");
    assert!(pair.spare().join(".statac-built").is_file());
    pair.same("nothing changed");

    pair.write(
        "content/post-2.md",
        "---\ntitle: Post 2\n---\nA post, changed.\n",
    );
    pair.same("a page changed");

    pair.remove("content/post-2.md");
    pair.same("a page removed");

    pair.rename("content/post-1/photo-1.jpg", "content/post-1/photo-2.jpg");
    pair.write(
        "content/post-1/index.md",
        "---\ntitle: Post 1\n---\n![A photograph](photo-2.jpg)\n",
    );
    pair.same("a picture renamed");

    pair.write(
        "templates/page.html",
        &LAYOUT.replace("<title>", "<title>Site: "),
    );
    pair.same("a template changed");

    pair.write(
        "content/post-3.md",
        "---\ntitle: Post 3\n---\nA draft no more.\n",
    );
    pair.same("a draft published");

    pair.write("static/Files/more/notes.txt", "More notes.\n");
    pair.same("a folder added");
    pair.remove_folder("static/Files/more");
    pair.write("static/Files/more", "A file now.\n");
    pair.same("a folder become a file");
    pair.remove("static/Files/more");
    pair.write("static/Files/more/notes.txt", "A folder again.\n");
    pair.same("a file become a folder");
}

#[test]
fn a_build_at_another_address_costs_the_next_nothing() {
    let pair = Pair::new("another-address");
    // A card drawn with the site's host on it is another card at another
    // address.
    pair.write(
        "templates/og.yaml",
        &CARD.replace("{{ page.og_title }}", "{{ page.og_title }} {{ site.host }}"),
    );
    pair.same("a first build and a second");
    for address in [
        "https://preview.example.org/",
        "https://example.org/preview/42/",
    ] {
        pair.build_at(address);
        pair.same("a build at another address");
        // Into each of the two folders the output goes between.
        pair.build_at(address);
        pair.build_at(address);
        pair.same("two builds at another address");
    }

    // The first build of all, and one of a site changed since.
    let pair = Pair::new("another-address-first");
    pair.build_at("https://preview.example.org/42/");
    pair.same("a first build at another address");
    pair.write("content/post-4.md", "---\ntitle: Post 4\n---\nA post.\n");
    pair.remove("content/post-2.md");
    pair.build_at("https://preview.example.org/42/");
    pair.write("content/post-5.md", "---\ntitle: Post 5\n---\nA post.\n");
    pair.same("a site changed, built at another address and changed again");
}

#[test]
fn a_failed_build_between_two_good_ones_costs_the_next_nothing() {
    let pair = Pair::new("failed");
    pair.same("a first build and a second");
    let before = tree(&pair.out());

    pair.write("content/index.md", "---\ntitle: Home\n---\nChanged.\n");
    pair.write("content/post-4.md", "---\ntitle: Post 4\n---\nNew.\n");
    pair.write(
        "templates/page.html",
        &LAYOUT.replace("site.css", "missing.css"),
    );
    let (said, code) = Pair::build(&pair.kept());
    assert_eq!(code, 1, "{said}");
    assert!(said.contains("Wrote nothing"), "{said}");
    assert!(tree(&pair.out()) == before);
    // The failed build got as far as writing into the spare.
    assert!(tree(&pair.spare()) != before);

    pair.write("templates/page.html", LAYOUT);
    pair.same("a failed build");
}

#[test]
fn what_is_done_to_the_output_by_hand_is_undone_by_the_next_build() {
    let pair = Pair::new("by-hand");
    pair.same("a first build and a second");
    let out = pair.out();
    let copy = tree(&out)
        .into_iter()
        .find_map(|(name, _)| name.ends_with(".avif").then_some(name))
        .unwrap();

    fs::write(out.join("index.html"), "Edited by hand.\n").unwrap();
    fs::write(out.join(&copy), "Not a picture.\n").unwrap();
    fs::remove_file(out.join("Files/notes.txt")).unwrap();
    pair.same("files edited and removed by hand");

    fs::write(out.join("post-1/added.html"), "Added by hand.\n").unwrap();
    fs::create_dir_all(out.join("added/empty")).unwrap();
    fs::write(out.join("added/notes.txt"), "Added by hand.\n").unwrap();
    pair.same("files and folders added by hand");
    pair.same("nothing changed");
}

#[test]
fn a_spare_deleted_is_made_again() {
    let pair = Pair::new("deleted");
    pair.same("a first build and a second");
    fs::remove_dir_all(pair.spare()).unwrap();
    pair.same("the spare deleted");
    assert!(pair.spare().join("index.html").is_file());
    fs::remove_dir_all(pair.kept().join(".statac")).unwrap();
    pair.same("everything kept deleted");
    fs::remove_dir_all(pair.out()).unwrap();
    pair.same("the output deleted");
}

#[test]
fn an_address_respelt_in_its_capitals_is_published_as_respelt() {
    let pair = Pair::new("capitals");
    pair.same("a first build and a second");
    // By way of another name, for a disk that holds the two as one.
    pair.rename("static/Files", "static/moved");
    pair.rename("static/moved", "static/files");
    pair.rename("static/files/notes.txt", "static/files/moved.txt");
    pair.rename("static/files/moved.txt", "static/files/Notes.txt");
    pair.same("a folder and a file respelt");
    let names: Vec<String> = tree(&pair.out())
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert!(names.contains(&"files/Notes.txt".to_owned()), "{names:?}");
}

#[cfg(unix)]
#[test]
fn a_spare_replaced_by_a_link_is_never_used() {
    use std::os::unix::fs::symlink;
    let pair = Pair::new("linked");
    pair.same("a first build and a second");
    let outside = pair.root.join("outside");
    fs::create_dir_all(outside.join("post-1")).unwrap();
    fs::write(outside.join(".statac-built"), "").unwrap();
    fs::write(outside.join("index.html"), "Mine.\n").unwrap();
    fs::write(outside.join("post-1/notes.txt"), "Mine.\n").unwrap();
    let before = tree(&outside);
    fs::remove_dir_all(pair.spare()).unwrap();
    symlink(&outside, pair.spare()).unwrap();

    let said = pair.same("the spare replaced by a link");
    assert!(said.contains("1 warning: 1 cache-not-written."), "{said}");
    assert!(tree(&outside) == before);
    assert!(pair.spare().is_symlink());

    fs::remove_file(pair.spare()).unwrap();
    let said = pair.same("the link removed");
    assert!(!said.contains("cache-not-written"), "{said}");
    assert!(pair.spare().join("index.html").is_file());
}
