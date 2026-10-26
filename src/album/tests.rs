use super::*;
use crate::disk::KEPT;

fn plain(root: Root, path: &str, address: Option<&str>) -> Picture {
    Picture {
        root,
        path: path.to_owned(),
        at: String::new(),
        address: address.map(str::to_owned),
        seen: Seen::Plain {
            kind: Kind::Gif,
            size: None,
        },
        hash: blake3::hash(path.as_bytes()),
        located: false,
        sidecar: None,
        copies: Vec::new(),
        full: None,
        travelling: None,
    }
}

/// Pictures as a site with these files would hold them.
fn pictures(files: &[(Root, &str, Option<&str>)]) -> Pictures {
    let mut pictures = Pictures::default();
    for &(root, path, address) in files {
        let id = pictures.all.push(plain(root, path, address));
        let name = path.rsplit('/').next().unwrap();
        pictures
            .by_name
            .entry(name.to_owned())
            .or_default()
            .push(id);
        pictures.by_source.insert((root, path.to_owned()), id);
        if let Some(address) = address {
            pictures.by_address.insert(address.to_owned(), id);
        }
    }
    pictures
}

fn site() -> Pictures {
    pictures(&[
        (
            Root::Content,
            "posts/post-1/photo-1.jpg",
            Some("/posts/post-1/photo-1.jpg"),
        ),
        (
            Root::Content,
            "posts/photo-2.jpg",
            Some("/posts/photo-2.jpg"),
        ),
        (
            Root::Content,
            "Folder 1/photo 3.jpg",
            Some("/folder-1/photo 3.jpg"),
        ),
        (Root::Content, "img/photo-4.jpg", Some("/img/photo-4.jpg")),
        (Root::Static, "img/photo-4.jpg", Some("/img/photo-4.jpg")),
        (Root::Static, "img/photo-5.jpg", Some("/img/photo-5.jpg")),
        (Root::Content, "posts/post-6/photo-6.jpg", None),
    ])
}

#[test]
fn a_picture_is_found_by_its_file_and_written_as_reaches_it() {
    let site = site();
    let found = |written: &str, folder: Option<&str>, page: &str, base: &str| {
        let from = From { folder, page, base };
        site.find(written, &from)
            .map(|found| (site.get(found.id).shown(), found.address))
    };
    let at = |shown: &str, address: &str| Some((shown.to_owned(), address.to_owned()));
    let cases = [
        // Beside a page in its own folder: as written.
        (
            ("photo-1.jpg", Some("posts/post-1/"), "/posts/post-1/", ""),
            at("content/posts/post-1/photo-1.jpg", "photo-1.jpg"),
        ),
        (
            (
                "./photo-1.jpg?v=2",
                Some("posts/post-1/"),
                "/posts/post-1/",
                "",
            ),
            at("content/posts/post-1/photo-1.jpg", "./photo-1.jpg?v=2"),
        ),
        // Beside a page that is one file, a folder above where it lands.
        (
            ("photo-2.jpg", Some("posts/"), "/posts/post-2/", ""),
            at("content/posts/photo-2.jpg", "/posts/photo-2.jpg"),
        ),
        (
            ("photo-2.jpg#top", Some("posts/"), "/posts/post-2/", "/blog"),
            at("content/posts/photo-2.jpg", "/blog/posts/photo-2.jpg#top"),
        ),
        // The same picture written as a browser reaches it.
        (
            ("../photo-2.jpg", Some("posts/"), "/posts/post-2/", ""),
            at("content/posts/photo-2.jpg", "../photo-2.jpg"),
        ),
        // A folder published under another name than it has.
        (
            ("photo%203.jpg", Some("Folder 1/"), "/folder-1/", ""),
            at("content/Folder 1/photo 3.jpg", "photo%203.jpg"),
        ),
        (
            ("/Folder 1/photo 3.jpg", Some(""), "/", ""),
            at("content/Folder 1/photo 3.jpg", "/folder-1/photo 3.jpg"),
        ),
        // From the top: `content/` before `static/`.
        (
            ("/img/photo-4.jpg", Some("posts/"), "/posts/post-2/", ""),
            at("content/img/photo-4.jpg", "/img/photo-4.jpg"),
        ),
        (
            ("/img/photo-5.jpg", Some("posts/"), "/posts/post-2/", ""),
            at("static/img/photo-5.jpg", "/img/photo-5.jpg"),
        ),
        // On a site under a path, with the path and without it.
        (
            ("/blog/img/photo-5.jpg", Some(""), "/blog/", "/blog"),
            at("static/img/photo-5.jpg", "/blog/img/photo-5.jpg"),
        ),
        (
            ("/img/photo-5.jpg", Some(""), "/blog/", "/blog"),
            at("static/img/photo-5.jpg", "/blog/img/photo-5.jpg"),
        ),
        // A page no file under `content/` holds finds it as a browser does.
        (
            ("img/photo-5.jpg", None, "/", ""),
            at("static/img/photo-5.jpg", "img/photo-5.jpg"),
        ),
        // Elsewhere, nowhere, or not published.
        (("https://example.org/photo-1.jpg", Some(""), "/", ""), None),
        (("//example.org/photo-1.jpg", Some(""), "/", ""), None),
        (("photo-9.jpg", Some("posts/"), "/posts/post-2/", ""), None),
        (("../../photo-1.jpg", Some("posts/"), "/posts/", ""), None),
        (("", Some(""), "/", ""), None),
        (
            ("photo-6.jpg", Some("posts/post-6/"), "/posts/post-6/", ""),
            None,
        ),
    ];
    for ((written, folder, page, base), expected) in cases {
        assert_eq!(
            found(written, folder, page, base),
            expected,
            "{written} from {page}"
        );
    }
}

#[test]
fn a_name_is_walked_from_its_folder() {
    let cases = [
        (("", "photo-1.jpg"), Some("photo-1.jpg")),
        (("posts/", "photo-1.jpg"), Some("posts/photo-1.jpg")),
        (
            ("posts/post-1/", "../photo-1.jpg"),
            Some("posts/photo-1.jpg"),
        ),
        (
            ("posts/", "./img//photo-1.jpg"),
            Some("posts/img/photo-1.jpg"),
        ),
        (("posts/", "../../photo-1.jpg"), None),
    ];
    for ((folder, name), expected) in cases {
        assert_eq!(walked(folder, name).as_deref(), expected, "{folder} {name}");
    }
}

#[test]
fn pictures_and_the_files_describing_them_are_told_by_name() {
    for name in ["photo-1.jpg", "photo-1.JPEG", "a/b.webp", "c.svg", "d.heic"] {
        assert!(is_picture(name), "{name}");
        assert!(is_sidecar(&format!("{name}.yaml")), "{name}");
    }
    for name in ["notes.txt", "photo-1", "jpg", "photo-1.jpg.yaml"] {
        assert!(!is_picture(name), "{name}");
    }
    for name in ["photo-1.yaml", "photo-1.jpg.yml", "people.yaml"] {
        assert!(!is_sidecar(name), "{name}");
    }
}

#[test]
fn writing_left_out_names_pictures_by_their_files() {
    let site = site();
    let named = |text: &str| -> Vec<String> {
        site.named_in(text)
            .into_iter()
            .map(|id| site.get(id).shown())
            .collect()
    };
    assert_eq!(
        named("Text ![one](photo-1.jpg) and @image(\"img/photo-5.jpg\", alt='x')."),
        ["content/posts/post-1/photo-1.jpg", "static/img/photo-5.jpg"]
    );
    assert_eq!(
        named("<img src=\"/Folder%201/photo%203.jpg\">"),
        ["content/Folder 1/photo 3.jpg"]
    );
    assert_eq!(named("No picture: photo-9.jpg, notes.txt"), [""; 0]);
}

thread_local! {
    static PRINTED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[test]
fn a_panic_caught_while_reading_a_picture_prints_nothing() {
    // Counts the panics on this thread that reach the hook that prints.
    let before = std::panic::take_hook();
    std::panic::set_hook(silenced(Box::new(move |info| {
        PRINTED.set(PRINTED.get() + 1);
        before(info);
    })));
    let caught = quietly(|| std::panic::catch_unwind(|| panic!("a reader's panic")));
    assert!(caught.is_err());
    assert_eq!(PRINTED.get(), 0);
    let caught = std::panic::catch_unwind(|| panic!("any other panic"));
    assert!(caught.is_err());
    assert_eq!(PRINTED.get(), 1);
}

#[cfg(unix)]
#[test]
fn a_kept_folder_swapped_for_a_link_is_neither_read_nor_written() {
    use crate::problem::Kind as Told;
    use crate::problem::Reason;
    let scratch = std::env::temp_dir().join(format!("statac-album-kept-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    let (site, outside) = (scratch.join("site"), scratch.join("outside"));
    std::fs::create_dir_all(&site).unwrap();
    std::fs::create_dir_all(outside.join("pictures")).unwrap();
    std::fs::write(outside.join("pictures/copy-1.avif"), "outside").unwrap();
    let kept = Kept::new(&site, "pictures");
    kept.make_ready(&site);
    let staging = crate::publish::begin(&site, &scratch.join("out")).unwrap();
    kept.keep("copy-1.avif", b"copy", "copy-1.avif", &staging)
        .unwrap();
    let folder = site.join(KEPT).join("pictures");
    std::fs::rename(&folder, scratch.join("moved")).unwrap();
    std::os::unix::fs::symlink(outside.join("pictures"), &folder).unwrap();
    // Kept where it can't be, it is written straight into the built site.
    kept.keep("copy-2.avif", b"copy", "copy-2.avif", &staging)
        .unwrap();
    let wanted = ["copy-1.avif", "copy-2.avif"].map(|name| (name.to_owned(), name.to_owned()));
    let (published, problems) = kept.publish(wanted.into(), &staging);
    let shown = std::path::Path::new(KEPT).join("pictures");
    assert_eq!(published, 1);
    assert_eq!(
        problems,
        [
            Problem::nowhere(Told::CantRead {
                path: shown.join("copy-1.avif"),
                reason: Reason::Link,
            }),
            Problem::nowhere(Told::CacheNotWritten {
                path: shown,
                reason: Reason::Link,
            }),
        ]
    );
    let mut names: Vec<_> = std::fs::read_dir(outside.join("pictures"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    names.sort();
    assert_eq!(names, ["copy-1.avif"]);
    drop(staging);
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn a_kept_copy_given_a_second_name_once_found_kept_is_not_published() {
    use crate::problem::Kind as Told;
    use crate::problem::Reason;
    let scratch = crate::testing::Scratch::new("album-named-twice");
    let site = scratch.join("site");
    let outside = scratch.write("outside.avif", "outside");
    std::fs::create_dir_all(&site).unwrap();
    let kept = Kept::new(&site, "pictures");
    kept.make_ready(&site);
    let staging = crate::publish::begin(&site, &scratch.join("out")).unwrap();
    for name in ["copy-1.avif", "copy-2.avif"] {
        kept.keep(name, b"copy", name, &staging).unwrap();
    }
    assert!(kept.has("copy-1.avif"));
    let copy = site.join(KEPT).join("pictures").join("copy-1.avif");
    std::fs::remove_file(&copy).unwrap();
    std::fs::hard_link(&outside, &copy).unwrap();
    let wanted = ["copy-1.avif", "copy-2.avif"].map(|name| (name.to_owned(), name.to_owned()));
    let (published, problems) = kept.publish(wanted.into(), &staging);
    assert_eq!(published, 1);
    assert_eq!(
        problems,
        [Problem::nowhere(Told::CantRead {
            path: std::path::Path::new(KEPT)
                .join("pictures")
                .join("copy-1.avif"),
            reason: Reason::NamedTwice,
        })]
    );
    drop(staging);
}
