//! The id a feed gives each entry, and the feed itself.
//!
//! An id is a `urn:uuid` made from the site's `feed_key` and the page's
//! identity, its `id:` where it writes one and its address otherwise, so
//! it is the same on every build and never stored. The making never
//! changes: a feed reader shown a new id shows the post again as new.
//!
//! A site moving from another tool keeps that tool's ids for the posts
//! dated before `feed_ids_before`: each post's whole address, or with
//! `feed_ids_from: jekyll`, the id Jekyll's feed gave it. On such a site a
//! list's own page, asked for the feed's id, is known by the feed's whole
//! address, as those tools knew their feeds.

use std::fmt::Write as _;
use std::path::Path;

use crate::config::Settings;
use crate::content::address::{on_the_wire, whole};
use crate::content::words::PageWord;
use crate::dates::Dates;
use crate::site::{Page, PageId, PagePlace, Site};

/// The id of `page` in a feed, or of the feed a list's page speaks for.
/// None where the site has no `feed_key`.
pub fn entry_id(site: &Site, settings: &Settings, dates: &Dates, page: PageId) -> Option<String> {
    let key = settings.text("feed_key")?.value;
    if let Some(kept) = kept_id(site, settings, dates, page) {
        return Some(kept);
    }
    let p = site.pages.get(page);
    let identity = p.words.text(PageWord::Id).map(|id| id.value);
    if let Some(identity) = identity
        && is_a_feed_id(identity)
    {
        return Some(format!("urn:uuid:{identity}"));
    }
    let subject = identity.unwrap_or(p.address.as_str());
    let mut hasher = blake3::Hasher::new();
    hasher.update(key.as_bytes());
    hasher.update(&[0]);
    hasher.update(subject.as_bytes());
    Some(format!(
        "urn:uuid:{}",
        uuid_shaped(hasher.finalize().as_bytes())
    ))
}

/// The id a site moving from another tool keeps for `page`, whatever
/// `id:` the page writes: the feed's whole address for a list's page, and
/// for a post dated before `feed_ids_before`, both days read in the site's
/// zone, its whole address, or Jekyll's id for it.
pub(super) fn kept_id(
    site: &Site,
    settings: &Settings,
    dates: &Dates,
    page: PageId,
) -> Option<String> {
    // An id kept is the one feed readers know, whatever address the run
    // was given.
    let url = settings.home()?;
    let before = settings.get("feed_ids_before")?;
    let p = site.pages.get(page);
    if let Some(feed) = p
        .speaks_for
        .and_then(|list| super::address(site, settings, list))
    {
        return Some(whole(feed.as_str(), url));
    }
    let before = before.value.as_date()?;
    let date = &p.date.as_ref()?.value;
    if dates.instant(date) >= dates.instant(before) {
        return None;
    }
    let old = match settings.text("feed_ids_from").map(|from| from.value) {
        Some("jekyll") => jekyll_id(p.address.as_str(), &jekyll_name(p)),
        _ => p.address.as_str().to_owned(),
    };
    Some(whole(&old, url))
}

/// Whether an id is written as a feed's entry id is: lower-case hex in runs
/// of 8, 4, 4, 4 and 12, kept as the post's identity so its entry keeps
/// the id it was published under.
pub(super) fn is_a_feed_id(id: &str) -> bool {
    let runs: Vec<&str> = id.split('-').collect();
    runs.len() == 5
        && runs.iter().zip([8, 4, 4, 4, 12]).all(|(run, length)| {
            run.len() == length
                && run
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

/// Sixteen bytes worn as a UUID of the kind made from other data (version
/// 8), with the standard variant bits.
fn uuid_shaped(bytes: &[u8]) -> String {
    let mut uuid = [0u8; 16];
    uuid.copy_from_slice(&bytes[..16]);
    uuid[6] = (uuid[6] & 0x0f) | 0x80;
    uuid[8] = (uuid[8] & 0x3f) | 0x80;
    let mut hex = String::with_capacity(32);
    for byte in uuid {
        write!(hex, "{byte:02x}").expect("writing to a string can't fail");
    }
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// A fresh key for a site that has none, offered in the problem that asks
/// for one. Made from the moment and the process, so no two runs agree.
pub fn mint_key() -> String {
    let mut hasher = blake3::Hasher::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_nanos())
        .unwrap_or_default();
    hasher.update(&now.to_le_bytes());
    hasher.update(&std::process::id().to_le_bytes());
    uuid_shaped(hasher.finalize().as_bytes())
}

/// A post's name as Jekyll read it from its file: the file's name less a
/// leading day, its extension and any dots left at its end, its capitals
/// kept. A post that is a folder is named by the folder; a page with no
/// file of its own, by the last part of its address.
fn jekyll_name(page: &Page) -> String {
    match page.place {
        PagePlace::Made => {
            let address = page.address.as_str().trim_end_matches('/');
            address.rsplit('/').next().unwrap_or_default().to_owned()
        }
        _ => jekyll_file_name(Path::new(&page.path)),
    }
}

fn jekyll_file_name(file: &Path) -> String {
    let name = file.file_name().unwrap_or_default().to_string_lossy();
    let name = match name.rsplit_once('.') {
        Some(("index", _)) => file
            .parent()
            .and_then(Path::file_name)
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        Some((stem, extension)) if !extension.is_empty() => stem.to_owned(),
        _ => name.into_owned(),
    };
    let name = jekyll_undated(&name).unwrap_or(&name);
    name.trim_end_matches('.').to_owned()
}

/// A name less the day Jekyll reads at its front, `2013-8-1-` as much as
/// `2013-08-01-`.
fn jekyll_undated(name: &str) -> Option<&str> {
    let mut rest = name;
    for (fewest, most) in [(2, 4), (1, 2), (1, 2)] {
        let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
        if digits < fewest || digits > most {
            return None;
        }
        rest = rest[digits..].strip_prefix('-')?;
    }
    Some(rest)
}

/// Jekyll's feed's entry id, short of the site's own address: the folder
/// of the post's address, then its name. A `?` or `#` in the name starts a
/// query or a fragment, as it did for Jekyll, and one with nothing after
/// it goes.
fn jekyll_id(address: &str, name: &str) -> String {
    let trimmed = address.trim_end_matches('/');
    let folder = match trimmed.rfind('/') {
        Some(0) | None => "",
        Some(at) => &trimmed[..at],
    };
    let (name, after) = name.split_at(name.find(['?', '#']).unwrap_or(name.len()));
    let mut id = on_the_wire(&format!("{folder}/{name}"));
    if let Some((mark, rest)) = after.split_at_checked(1)
        && !rest.is_empty()
    {
        id.push_str(mark);
        id.push_str(&on_the_wire(rest));
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each fixed for good, as feeds have published it, for a site with
    /// `feed_key: key`.
    #[test]
    fn an_id_is_made_from_the_key_and_the_address_or_the_pages_own() {
        let id = |subject: &str| {
            let mut hasher = blake3::Hasher::new();
            hasher.update(b"key");
            hasher.update(&[0]);
            hasher.update(subject.as_bytes());
            uuid_shaped(hasher.finalize().as_bytes())
        };
        for (subject, made) in [
            ("/", "2415c9c8-e357-8da3-9cfe-e0f41515ba6c"),
            ("/posts/", "1cae012e-d114-86b3-a710-291c0057851c"),
            ("/posts/post-1/", "fc370633-88f0-8092-9f96-7749dbfff0eb"),
            ("/posts/café 4/", "38b188e3-d6db-8c9c-96b2-861ba5ccf39d"),
            ("abc123", "d7717683-3178-8849-9ff5-aa862293d9ef"),
        ] {
            assert_eq!(id(subject), made, "{subject}");
        }
    }

    #[test]
    fn a_feed_id_is_five_runs_of_lower_case_hex() {
        assert!(is_a_feed_id("0a1b2c3d-0000-8000-8000-000000000001"));
        assert!(!is_a_feed_id("0A1B2C3D-0000-8000-8000-000000000001"));
        assert!(!is_a_feed_id("abc123"));
        let key = mint_key();
        assert!(is_a_feed_id(&key), "{key}");
    }

    /// Each spelled by the rules Jekyll 4.4.1 and its feed plugin 0.17.0
    /// follow for a post at that address from that file, less the site's
    /// own address.
    #[test]
    fn a_jekyll_id_is_the_folder_and_the_file_name_as_jekyll_spelled_it() {
        for (address, file, id) in [
            (
                "/2026/09/01/post-1/",
                "2026-09-01-Post-1.md",
                "/2026/09/01/Post-1",
            ),
            (
                "/2026/09/02/post-2/",
                "2026-09-02-post_2.md",
                "/2026/09/02/post_2",
            ),
            (
                "/2026/09/03/cafe/",
                "2026-09-03-café.md",
                "/2026/09/03/caf%C3%A9",
            ),
            (
                "/2026/09/04/post-4/",
                "2026-09-04-post 4.md",
                "/2026/09/04/post%204",
            ),
            ("/posts/post-5/", "2026-09-06-post-5.md", "/posts/post-5"),
            ("/2026/post-6.html", "2026-09-11-post-6.md", "/2026/post-6"),
            (
                "/2026/09/08/v2.html",
                "2026-09-08-v2.0.md",
                "/2026/09/08/v2.0",
            ),
            (
                "/2026/09/12/post-7/",
                "2026-09-12-post.7..md",
                "/2026/09/12/post.7",
            ),
            ("/a/x/", "x?y.md", "/a/x?y"),
            ("/a/x/", "x#.md", "/a/x"),
            (
                "/2013/08/01/post-8/",
                "2013-8-1-post-8.md",
                "/2013/08/01/post-8",
            ),
            ("/page-1/", "page-1.md", "/page-1"),
            (
                "/2026/09/14/post-9/",
                "2026-09-14-Post-9/index.md",
                "/2026/09/14/Post-9",
            ),
        ] {
            let name = jekyll_file_name(Path::new("posts").join(file).as_path());
            assert_eq!(jekyll_id(address, &name), id, "{file} at {address}");
        }
    }
}
