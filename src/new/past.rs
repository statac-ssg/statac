//! Every version a starter file had in a release, as `starter/past/` keeps
//! it: `starter/past/<release>/<path>`, each release's folder
//! holding the files whose bytes no earlier release had. The oldest
//! release comes first. Written by `cargo run --example starter_past`,
//! run as a release is cut.

use super::starter::Past;

macro_rules! past {
    ($($release:literal $path:literal,)*) => {
        pub(super) const PAST: &[Past] = &[$(Past {
            path: $path,
            bytes: include_bytes!(concat!("../../starter/past/", $release, "/", $path)),
        },)*];

        /// The release each of [`PAST`] is kept under.
        #[cfg(test)]
        pub(super) const RELEASES: &[&str] = &[$($release,)*];
    };
}

past! {
    "0.1.0" "assets/site.css",
    "0.1.0" "content/404.md",
    "0.1.0" "content/notes/index.md",
    "0.1.0" "content/photos/index.md",
    "0.1.0" "content/photos/photo-1/photo-1.jpg",
    "0.1.0" "content/photos/photo-1/photo-1.jpg.yaml",
    "0.1.0" "content/photos/photo-2/photo-2.jpg",
    "0.1.0" "content/photos/photo-2/photo-2.jpg.yaml",
    "0.1.0" "content/photos/photo-3/photo-3.jpg",
    "0.1.0" "content/photos/photo-3/photo-3.jpg.yaml",
    "0.1.0" "content/photos/photo-4/photo-4.jpg",
    "0.1.0" "content/photos/photo-4/photo-4.jpg.yaml",
    "0.1.0" "content/photos/photo-5/photo-5.jpg",
    "0.1.0" "content/photos/photo-5/photo-5.jpg.yaml",
    "0.1.0" "content/photos/photo-6/photo-6.jpg",
    "0.1.0" "content/photos/photo-6/photo-6.jpg.yaml",
    "0.1.0" "content/posts/first-post/photo-1.jpg",
    "0.1.0" "content/posts/first-post/photo-1.jpg.yaml",
    "0.1.0" "content/posts/index.md",
    "0.1.0" "content/posts/what-a-post-can-hold/example.css",
    "0.1.0" "content/posts/what-a-post-can-hold/photo-2.jpg",
    "0.1.0" "content/posts/what-a-post-can-hold/photo-2.jpg.yaml",
    "0.1.0" "content/posts/what-a-post-can-hold/photo-3.jpg",
    "0.1.0" "content/posts/what-a-post-can-hold/photo-3.jpg.yaml",
    "0.1.0" "content/posts/what-a-post-can-hold/photo-4.jpg",
    "0.1.0" "content/posts/what-a-post-can-hold/photo-4.jpg.yaml",
    "0.1.0" "content/posts/what-a-post-can-hold/photo-5.jpg",
    "0.1.0" "content/posts/what-a-post-can-hold/photo-5.jpg.yaml",
    "0.1.0" "content/posts/what-a-post-can-hold/recording-1.m4a",
    "0.1.0" "content/posts/what-a-post-can-hold/video-1.mp4",
    "0.1.0" "content/robots.md",
    "0.1.0" "content/sitemap.md",
    "0.1.0" "snippets/audio.html",
    "0.1.0" "snippets/beside.html",
    "0.1.0" "snippets/called.html",
    "0.1.0" "snippets/entry.html",
    "0.1.0" "snippets/figure.html",
    "0.1.0" "snippets/file.html",
    "0.1.0" "snippets/filed.html",
    "0.1.0" "snippets/gallery.html",
    "0.1.0" "snippets/line.html",
    "0.1.0" "snippets/markdown/callout.html",
    "0.1.0" "snippets/markdown/image.html",
    "0.1.0" "snippets/mentions.html",
    "0.1.0" "snippets/note.html",
    "0.1.0" "snippets/pager.html",
    "0.1.0" "snippets/picture.html",
    "0.1.0" "snippets/quote.html",
    "0.1.0" "snippets/short.html",
    "0.1.0" "snippets/thumb.html",
    "0.1.0" "snippets/video.html",
    "0.1.0" "snippets/vimeo.html",
    "0.1.0" "snippets/youtube.html",
    "0.1.0" "static/favicon.ico",
    "0.1.0" "templates/404.html",
    "0.1.0" "templates/base.html",
    "0.1.0" "templates/feed.xml",
    "0.1.0" "templates/folder.html",
    "0.1.0" "templates/home.html",
    "0.1.0" "templates/note.html",
    "0.1.0" "templates/notes.html",
    "0.1.0" "templates/og.yaml",
    "0.1.0" "templates/og/NotoEmoji-OFL.txt",
    "0.1.0" "templates/og/NotoEmoji.ttf",
    "0.1.0" "templates/og/NotoSans-Regular.ttf",
    "0.1.0" "templates/og/OFL.txt",
    "0.1.0" "templates/og/background.png",
    "0.1.0" "templates/page.html",
    "0.1.0" "templates/photo.html",
    "0.1.0" "templates/photos.html",
    "0.1.0" "templates/post.html",
    "0.1.0" "templates/redirect.html",
    "0.1.0" "templates/robots.txt",
    "0.1.0" "templates/sitemap.xml",
}
