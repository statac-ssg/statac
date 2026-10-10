//! The stills of videos kept on other sites: the picture shown in a
//! video's place until a reader presses play, fetched once into
//! `fetched/stills/` and named after the video.

/// The folder under `fetched/` stills are kept in.
pub const FOLDER: &str = "stills";

/// A site that keeps videos.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Host {
    YouTube,
    Vimeo,
}

impl Host {
    pub fn site(self) -> &'static str {
        match self {
            Host::YouTube => "YouTube",
            Host::Vimeo => "Vimeo",
        }
    }

    /// The name a still of the video `id` is kept under.
    pub fn still(self, id: &str) -> String {
        match self {
            Host::YouTube => format!("youtube-{id}.jpg"),
            Host::Vimeo => format!("vimeo-{id}.jpg"),
        }
    }

    /// Whether `id` is one video, which has a still: a YouTube playlist
    /// shown in one player stands as `videoseries`.
    pub fn is_one_video(self, id: &str) -> bool {
        !(self == Host::YouTube && id == "videoseries")
    }
}

/// The still of the video `id` on `host`, by its path under `fetched/`.
pub fn path(host: Host, id: &str) -> String {
    format!("{FOLDER}/{}", host.still(id))
}
