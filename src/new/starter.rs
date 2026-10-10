//! The starter: every file `statac new` writes as it is kept in
//! `starter/site/`, byte for byte, and the sharing card's files, which it
//! writes only where it is asked for them.

/// A starter file: its path in a site, and its bytes.
pub(crate) struct File {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

/// A version a starter file had in a release: its path in a site, and its
/// bytes.
pub(crate) struct Past {
    pub path: &'static str,
    pub bytes: &'static [u8],
}

macro_rules! starter {
    ($(#[$about:meta])* $name:ident: $($path:literal,)*) => {
        $(#[$about])*
        pub(super) const $name: &[File] = &[$(File {
            path: $path,
            bytes: include_bytes!(concat!("../../starter/site/", $path)),
        },)*];
    };
}

starter! {
    /// The files every new site has.
    FILES:
    "assets/site.css",
    "content/404.md",
    "content/notes/index.md",
    "content/photos/index.md",
    "content/photos/photo-1/photo-1.jpg",
    "content/photos/photo-1/photo-1.jpg.yaml",
    "content/photos/photo-2/photo-2.jpg",
    "content/photos/photo-2/photo-2.jpg.yaml",
    "content/photos/photo-3/photo-3.jpg",
    "content/photos/photo-3/photo-3.jpg.yaml",
    "content/photos/photo-4/photo-4.jpg",
    "content/photos/photo-4/photo-4.jpg.yaml",
    "content/photos/photo-5/photo-5.jpg",
    "content/photos/photo-5/photo-5.jpg.yaml",
    "content/photos/photo-6/photo-6.jpg",
    "content/photos/photo-6/photo-6.jpg.yaml",
    "content/posts/first-post/photo-1.jpg",
    "content/posts/first-post/photo-1.jpg.yaml",
    "content/posts/index.md",
    "content/posts/what-a-post-can-hold/example.css",
    "content/posts/what-a-post-can-hold/photo-2.jpg",
    "content/posts/what-a-post-can-hold/photo-2.jpg.yaml",
    "content/posts/what-a-post-can-hold/photo-3.jpg",
    "content/posts/what-a-post-can-hold/photo-3.jpg.yaml",
    "content/posts/what-a-post-can-hold/photo-4.jpg",
    "content/posts/what-a-post-can-hold/photo-4.jpg.yaml",
    "content/posts/what-a-post-can-hold/photo-5.jpg",
    "content/posts/what-a-post-can-hold/photo-5.jpg.yaml",
    "content/posts/what-a-post-can-hold/recording-1.m4a",
    "content/posts/what-a-post-can-hold/video-1.mp4",
    "content/robots.md",
    "content/sitemap.md",
    "snippets/audio.html",
    "snippets/beside.html",
    "snippets/called.html",
    "snippets/entry.html",
    "snippets/figure.html",
    "snippets/file.html",
    "snippets/filed.html",
    "snippets/gallery.html",
    "snippets/line.html",
    "snippets/markdown/callout.html",
    "snippets/markdown/image.html",
    "snippets/mentions.html",
    "snippets/note.html",
    "snippets/pager.html",
    "snippets/picture.html",
    "snippets/quote.html",
    "snippets/short.html",
    "snippets/thumb.html",
    "snippets/video.html",
    "snippets/vimeo.html",
    "snippets/youtube.html",
    "static/favicon.ico",
    "templates/404.html",
    "templates/base.html",
    "templates/feed.xml",
    "templates/folder.html",
    "templates/home.html",
    "templates/note.html",
    "templates/notes.html",
    "templates/page.html",
    "templates/photo.html",
    "templates/photos.html",
    "templates/post.html",
    "templates/redirect.html",
    "templates/robots.txt",
    "templates/sitemap.xml",
}

/// Whether the starter has a snippet a page calls by `name`.
pub(crate) fn has_snippet(name: &str) -> bool {
    FILES.iter().any(|file| {
        let called = file.path.strip_prefix("snippets/");
        called.and_then(|file| file.strip_suffix(".html")) == Some(name)
    })
}

/// The folders of sample pages, each with the `/` that ends it, and the
/// sample icon.
const SAMPLES: &[&str] = &[
    "content/notes/",
    "content/photos/",
    "content/posts/",
    "static/favicon.ico",
];

/// The posts' own page, which a site made by an import is given for the
/// old site's posts.
pub(super) const POSTS_PAGE: &str = "content/posts/index.md";

/// Whether `path` is a file of the sample pages', a folder's own page, a
/// page in it, or a file kept beside one, or is the sample icon. A site
/// whose author deleted them does without these.
pub(crate) fn is_a_samples(path: &str) -> bool {
    SAMPLES.iter().any(|folder| path.starts_with(folder))
}

starter! {
    /// The sharing card's files, which a site has once `statac new --cards`
    /// has written them.
    CARDS:
    "templates/og.yaml",
    "templates/og/NotoEmoji-OFL.txt",
    "templates/og/NotoEmoji.ttf",
    "templates/og/NotoSans-Regular.ttf",
    "templates/og/OFL.txt",
    "templates/og/background.png",
}
