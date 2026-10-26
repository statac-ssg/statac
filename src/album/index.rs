//! What each picture said of itself, kept between builds against the
//! file's size and the time it was last changed, so a picture is read
//! again only when its file changes.
//!
//! The file is lines of text, one picture to a line, its parts parted by
//! tabs. A line that doesn't read is passed over and its picture read
//! again.

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{Root, Seen};
use crate::pictures::{Kind, Photograph, Size};
use crate::problem::PictureFault;

/// Raised whenever a line's meaning changes, so an older file is read as
/// knowing nothing.
const OPENING: &str = "statac pictures 9";

/// A file's size and when it was last changed, in nanoseconds.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub(super) struct Stamp {
    len: u64,
    changed: u128,
}

impl Stamp {
    /// Of a file `len` long whose bytes last changed at `modified`; none
    /// for a time before 1970.
    pub(super) fn new(len: u64, modified: SystemTime) -> Option<Stamp> {
        let changed = modified.duration_since(UNIX_EPOCH).ok()?;
        Some(Stamp {
            len,
            changed: changed.as_nanos(),
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Entry {
    pub stamp: Stamp,
    pub hash: blake3::Hash,
    /// Whether it holds where it was taken, asked only of a file under
    /// `content/`.
    pub located: bool,
    /// None for a file that begins as no kind of picture known.
    pub seen: Option<Seen>,
}

impl Entry {
    pub(super) fn read(stamp: Stamp, root: Root, bytes: &[u8]) -> Entry {
        Entry {
            stamp,
            hash: blake3::hash(bytes),
            located: root == Root::Content && crate::pictures::holds_location(bytes),
            seen: super::seen(bytes),
        }
    }
}

#[derive(Default)]
pub(super) struct Index {
    entries: HashMap<(Root, String), Entry>,
    /// Whether it differs from the file it was read from.
    pub changed: bool,
}

impl Index {
    /// What the last build kept; nothing where it kept nothing that reads.
    pub(super) fn load(site: &Path) -> Index {
        let mut index = Index::default();
        // Asked what it is through the handle it is read by, so no link
        // put in its place, or in place of a folder on the way, is read.
        let Ok(file) = crate::platform::open_file(&site.join(crate::disk::KEPT), "pictures/index")
        else {
            return index;
        };
        if !crate::platform::named_once(&file) {
            return index;
        }
        let Ok(text) = std::io::read_to_string(file) else {
            return index;
        };
        let mut lines = text.lines();
        if lines.next() != Some(OPENING) {
            return index;
        }
        for line in lines {
            if let Some((key, entry)) = entry_of(line) {
                index.entries.insert(key, entry);
            }
        }
        index
    }

    pub(super) fn get(&self, root: Root, path: &str) -> Option<&Entry> {
        // Borrowed keys would need a key type of their own; a path is short.
        self.entries.get(&(root, path.to_owned()))
    }

    pub(super) fn insert(&mut self, root: Root, path: &str, entry: Entry) {
        self.entries.insert((root, path.to_owned()), entry);
    }

    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Writes the index where it has changed, under another name first so
    /// a build stopped halfway leaves the last one whole.
    pub(super) fn save(&self, site: &Path) -> std::io::Result<()> {
        if !self.changed {
            return Ok(());
        }
        let folder = super::kept_folder(site)?;
        if self.entries.is_empty() && fs::symlink_metadata(folder.join("index")).is_err() {
            return Ok(());
        }
        let mut lines: Vec<String> = self
            .entries
            .iter()
            .map(|(key, entry)| line_of(key, entry))
            .collect();
        lines.sort_unstable();
        let mut text = format!("{OPENING}\n");
        for line in lines {
            text.push_str(&line);
            text.push('\n');
        }
        let kept = crate::disk::make_kept_in(site, "pictures")?;
        let fresh = format!("index.{}", std::process::id());
        crate::platform::keep_file(&kept, "pictures/index", &fresh, text.as_bytes())
    }
}

/// Text with the characters that part a line's fields written apart.
pub(crate) fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\t', "\\t")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

pub(crate) fn unescaped(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next()? {
            '\\' => '\\',
            't' => '\t',
            'n' => '\n',
            'r' => '\r',
            _ => return None,
        });
    }
    Some(out)
}

const KINDS: [(Kind, &str); 10] = [
    (Kind::Jpeg, "jpeg"),
    (Kind::WebP, "webp"),
    (Kind::AnimatedWebP, "animated-webp"),
    (Kind::Png, "png"),
    (Kind::AnimatedPng, "animated-png"),
    (Kind::Gif, "gif"),
    (Kind::Svg, "svg"),
    (Kind::Avif, "avif"),
    (Kind::Heic, "heic"),
    (Kind::Heif, "heif"),
];

fn kind_name(kind: Kind) -> &'static str {
    KINDS
        .iter()
        .find(|(known, _)| *known == kind)
        .map_or("", |(_, name)| name)
}

fn kind_named(name: &str) -> Option<Kind> {
    KINDS
        .iter()
        .find(|(_, known)| *known == name)
        .map(|(kind, _)| *kind)
}

fn line_of((root, path): &(Root, String), entry: &Entry) -> String {
    let mut fields = vec![
        root.folder().to_owned(),
        escaped(path),
        entry.stamp.len.to_string(),
        entry.stamp.changed.to_string(),
        entry.hash.to_hex().to_string(),
        match entry.located {
            true => "located".to_owned(),
            false => "unlocated".to_owned(),
        },
    ];
    match &entry.seen {
        None => fields.push("none".into()),
        Some(Seen::Photograph {
            photograph,
            converts,
            transparent,
        }) => {
            fields.push("photograph".into());
            fields.push(kind_name(photograph.kind).into());
            for number in [
                photograph.stored.width,
                photograph.stored.height,
                photograph.shown.width,
                photograph.shown.height,
                u32::from(photograph.orientation),
                u32::from(photograph.alpha),
                u32::from(*converts),
                u32::from(*transparent),
            ] {
                fields.push(number.to_string());
            }
            fields.push(match &photograph.profile {
                None => "-".into(),
                Some(None) => "?".into(),
                Some(Some(name)) => format!("={}", escaped(name)),
            });
        }
        Some(Seen::Plain { kind, size }) => {
            fields.push("plain".into());
            fields.push(kind_name(*kind).into());
            match size {
                Some(size) => {
                    fields.push(size.width.to_string());
                    fields.push(size.height.to_string());
                }
                None => fields.push("-".into()),
            }
        }
        Some(Seen::Unreadable(fault)) => match fault {
            PictureFault::Unsupported(what) => {
                fields.push("unsupported".into());
                fields.push(escaped(what));
            }
            // The others are found only when a photograph is opened.
            _ => fields.push("cut".into()),
        },
    }
    fields.join("\t")
}

fn entry_of(line: &str) -> Option<((Root, String), Entry)> {
    let mut fields = line.split('\t');
    let root = match fields.next()? {
        "content" => Root::Content,
        "static" => Root::Static,
        "fetched" => Root::Fetched,
        _ => return None,
    };
    let path = unescaped(fields.next()?)?;
    let stamp = Stamp {
        len: fields.next()?.parse().ok()?,
        changed: fields.next()?.parse().ok()?,
    };
    let hash = blake3::Hash::from_hex(fields.next()?).ok()?;
    let located = match fields.next()? {
        "located" => true,
        "unlocated" => false,
        _ => return None,
    };
    let number =
        |fields: &mut std::str::Split<'_, char>| -> Option<u32> { fields.next()?.parse().ok() };
    let seen = match fields.next()? {
        "none" => None,
        "photograph" => {
            let kind = kind_named(fields.next()?).filter(|kind| kind.is_photograph())?;
            let stored = Size {
                width: number(&mut fields)?,
                height: number(&mut fields)?,
            };
            let shown = Size {
                width: number(&mut fields)?,
                height: number(&mut fields)?,
            };
            let orientation = u8::try_from(number(&mut fields)?).ok()?;
            let alpha = number(&mut fields)? == 1;
            let converts = number(&mut fields)? == 1;
            let transparent = number(&mut fields)? == 1;
            let profile = match fields.next()? {
                "-" => None,
                "?" => Some(None),
                named => Some(Some(unescaped(named.strip_prefix('=')?)?)),
            };
            Some(Seen::Photograph {
                photograph: Photograph {
                    kind,
                    stored,
                    orientation,
                    shown,
                    alpha,
                    profile,
                },
                converts,
                transparent,
            })
        }
        "plain" => {
            let kind = kind_named(fields.next()?)?;
            let size = match fields.next()? {
                "-" => None,
                width => Some(Size {
                    width: width.parse().ok()?,
                    height: number(&mut fields)?,
                }),
            };
            Some(Seen::Plain { kind, size })
        }
        "unsupported" => Some(Seen::Unreadable(PictureFault::Unsupported(unescaped(
            fields.next()?,
        )?))),
        "cut" => Some(Seen::Unreadable(PictureFault::CutShort)),
        _ => return None,
    };
    if fields.next().is_some() {
        return None;
    }
    let entry = Entry {
        stamp,
        hash,
        located,
        seen,
    };
    Some(((root, path), entry))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(seen: Option<Seen>) -> Entry {
        Entry {
            stamp: Stamp {
                len: 1234,
                changed: 1_759_000_000_123_456_789,
            },
            hash: blake3::hash(b"photo-1"),
            located: false,
            seen,
        }
    }

    #[test]
    fn every_kind_of_line_reads_back_as_written() {
        let photograph = |profile, transparent| {
            Some(Seen::Photograph {
                photograph: Photograph {
                    kind: Kind::Jpeg,
                    stored: Size {
                        width: 4000,
                        height: 3000,
                    },
                    orientation: 6,
                    shown: Size {
                        width: 3000,
                        height: 4000,
                    },
                    alpha: transparent,
                    profile,
                },
                converts: true,
                transparent,
            })
        };
        let plain = |kind, size| Some(Seen::Plain { kind, size });
        let size = Size {
            width: 40,
            height: 30,
        };
        let cases = [
            None,
            photograph(None, false),
            photograph(None, true),
            photograph(Some(None), false),
            photograph(Some(Some("Display\tP3 \\ wide".into())), true),
            plain(Kind::Png, Some(size)),
            plain(Kind::Svg, None),
            plain(Kind::AnimatedWebP, Some(size)),
            Some(Seen::Unreadable(PictureFault::CutShort)),
            Some(Seen::Unreadable(PictureFault::Unsupported(
                "JPEG without a frame header".into(),
            ))),
        ];
        for (seen, located) in cases.into_iter().zip([false, true].into_iter().cycle()) {
            let key = (Root::Content, "folder 1/photo\t1.jpg".to_owned());
            let entry = Entry {
                located,
                ..entry(seen)
            };
            let line = line_of(&key, &entry);
            assert!(!line.contains('\n'), "{line}");
            assert_eq!(entry_of(&line), Some((key, entry)), "{line}");
        }
    }

    #[test]
    fn only_an_index_kept_as_a_file_itself_is_read() {
        let site = std::env::temp_dir().join(format!("statac-index-{}", std::process::id()));
        let _ = fs::remove_dir_all(&site);
        fs::create_dir_all(&site).unwrap();
        let mut index = Index::default();
        index.insert(Root::Static, "photo-1.jpg", entry(None));
        index.changed = true;
        index.save(&site).unwrap();
        assert_eq!(Index::load(&site).len(), 1);
        let kept = site.join(crate::disk::KEPT).join("pictures");
        fs::hard_link(kept.join("index"), site.join("index")).unwrap();
        assert_eq!(Index::load(&site).len(), 0);
        fs::remove_file(site.join("index")).unwrap();
        #[cfg(unix)]
        {
            fs::rename(&kept, site.join("pictures")).unwrap();
            std::os::unix::fs::symlink(site.join("pictures"), &kept).unwrap();
            assert_eq!(Index::load(&site).len(), 0);
        }
        let _ = fs::remove_dir_all(&site);
    }

    #[test]
    fn a_line_that_does_not_read_is_passed_over() {
        let key = (Root::Static, "photo-1.jpg".to_owned());
        let line = line_of(&key, &entry(None));
        for damaged in [
            line.replace("static", "assets"),
            line.replace("none", "nothing"),
            format!("{line}\textra"),
            line[..line.len() - 6].to_owned(),
            String::new(),
        ] {
            assert_eq!(entry_of(&damaged), None, "{damaged}");
        }
    }
}
