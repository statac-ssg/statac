//! The webmentions of the site's pages: what another site said of one,
//! fetched from the relay by `statac refresh` and kept a file a mention in
//! `fetched/mentions/`, named by the relay's id, with the sender's face in
//! `fetched/faces/`.
//!
//! Everything in a mention was written by a stranger. Each text is kept
//! without its control characters, and written in double quotes with every
//! character that needs it escaped, so it reads back as it was sent; an
//! address is kept only where it is `http` or `https`.

use std::collections::HashMap;

use crate::config::Settings;
use crate::content::address::{Address, decoded};
use crate::content::words::key_name;
use crate::date::Date;
use crate::problem::{Expected, Kind, Problem, Requirer, Waiting, Word};
use crate::site::{PageId, Site};
use crate::source::{Place, SourceId, Sources};
use crate::value::Reader;
use crate::yaml::{self, Value as Yaml};

/// The folder under `fetched/` mentions are kept in.
pub const FOLDER: &str = "mentions";

/// The folder under `fetched/` the senders' faces are kept in.
pub const FACES: &str = "faces";

/// The most characters of a mention's words kept, its ellipsis counted.
pub const LONGEST_WORDS: usize = 2_000;

/// What another site's page did with one of the site's pages.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Said {
    Replied,
    Liked,
    Reposted,
    Bookmarked,
    Mentioned,
}

impl Said {
    pub const ALL: [Said; 5] = [
        Said::Replied,
        Said::Liked,
        Said::Reposted,
        Said::Bookmarked,
        Said::Mentioned,
    ];

    /// The word a mention's file and a template give it.
    pub fn word(self) -> &'static str {
        match self {
            Said::Replied => "replied",
            Said::Liked => "liked",
            Said::Reposted => "reposted",
            Said::Bookmarked => "bookmarked",
            Said::Mentioned => "mentioned",
        }
    }

    /// The kind a relay's `wm-property` names.
    pub fn of_property(property: &str) -> Option<Said> {
        Some(match property {
            "in-reply-to" => Said::Replied,
            "like-of" => Said::Liked,
            "repost-of" => Said::Reposted,
            "bookmark-of" => Said::Bookmarked,
            "mention-of" => Said::Mentioned,
            _ => return None,
        })
    }

    /// Whether a page shows it as a face alone, with no words.
    pub fn is_a_face(self) -> bool {
        matches!(self, Said::Liked | Said::Reposted)
    }
}

/// One mention, as its file holds it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Mention {
    /// The relay's id, from the file's name; none for a file named
    /// otherwise.
    pub id: Option<u64>,
    pub kind: Said,
    pub author: Option<String>,
    pub author_url: Option<String>,
    /// The sender's face, by its name in `fetched/faces/`.
    pub face: Option<String>,
    /// The sender's page.
    pub url: String,
    /// The address on the site it mentions.
    pub target: String,
    pub published: Option<Date>,
    pub fetched: Date,
    pub words: Option<String>,
}

const FIELDS: &[&str] = &[
    "kind",
    "author",
    "author_url",
    "face",
    "url",
    "target",
    "published",
    "fetched",
    "words",
];

impl Mention {
    /// The file Statac writes for it.
    pub fn text(&self) -> String {
        let fields = [
            ("kind", Some(self.kind.word().to_owned())),
            ("author", self.author.clone()),
            ("author_url", self.author_url.clone()),
            ("face", self.face.clone()),
            ("url", Some(self.url.clone())),
            ("target", Some(self.target.clone())),
            ("published", self.published.map(|day| day.iso_day())),
            ("fetched", Some(self.fetched.iso_day())),
            ("words", self.words.clone()),
        ];
        let mut text = String::new();
        for (name, value) in fields {
            if let Some(value) = value {
                text.push_str(&format!("{name}: {}\n", quoted(&value)));
            }
        }
        text
    }
}

/// The mention `id`, by its path under `fetched/`.
pub fn path(id: u64) -> String {
    format!("{FOLDER}/{id}.yaml")
}

/// The id of the mention at `path` under `fetched/`, where it is one.
pub fn id_of(path: &str) -> Option<u64> {
    let name = path.strip_prefix(FOLDER)?.strip_prefix('/')?;
    whole_id(name.strip_suffix(".yaml")?)
}

/// `text` as a relay's id: a whole number above nought, in digits alone.
pub fn whole_id(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().filter(|id| *id > 0)
}

/// The name a face fetched from `address` is kept under.
pub fn face_name(address: &str) -> String {
    format!("{}.jpg", &blake3::hash(address.as_bytes()).to_hex()[..16])
}

/// Whether `name` is one [`face_name`] gives.
fn a_face_name(name: &str) -> bool {
    name.strip_suffix(".jpg").is_some_and(|hex| {
        hex.len() == 16 && hex.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    })
}

/// A stranger's text as it is kept: without its control characters, its
/// line breaks spaces where `breaks` says so, and trimmed.
pub fn clean(text: &str, breaks: bool) -> String {
    let text = text.replace("\r\n", "\n");
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' | '\r' | '\u{2028}' | '\u{2029}' if breaks => out.push(' '),
            c if c.is_control() || bidirectional(c) => {}
            c => out.push(c),
        }
    }
    out.trim().to_owned()
}

/// Unicode's controls of the direction text runs in, which reorder what a
/// reader sees.
fn bidirectional(c: char) -> bool {
    matches!(
        c,
        '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
    )
}

/// A mention's words as they are kept: cleaned, at most [`LONGEST_WORDS`]
/// characters, cut with an ellipsis; none where nothing is left.
pub fn words(text: &str) -> Option<String> {
    let cleaned = clean(text, true);
    if cleaned.chars().count() <= LONGEST_WORDS {
        return (!cleaned.is_empty()).then_some(cleaned);
    }
    let kept: String = cleaned.chars().take(LONGEST_WORDS - 1).collect();
    Some(format!("{}…", kept.trim_end()))
}

/// Whether `text` is an address on the web: `http` or `https`, and a host.
pub fn web_address(text: &str) -> bool {
    url::Url::parse(text)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
}

/// Where `target` is on the site at `site`, its `url`, as a path from the
/// top of the site: the scheme and a leading `www.` aside, the host in any
/// capitals, and the site's path at a boundary. None for an address
/// elsewhere.
pub fn on_site(target: &str, site: &str) -> Option<String> {
    let (target, site) = (url::Url::parse(target).ok()?, url::Url::parse(site).ok()?);
    let host = |url: &url::Url| {
        let host = url.host_str()?.to_ascii_lowercase();
        Some(host.strip_prefix("www.").map(str::to_owned).unwrap_or(host))
    };
    if !matches!(target.scheme(), "http" | "https")
        || host(&target)? != host(&site)?
        || target.port() != site.port()
    {
        return None;
    }
    let base = site.path().trim_end_matches('/');
    let rest = target.path().strip_prefix(base)?;
    match rest {
        "" => Some("/".into()),
        rest if rest.starts_with('/') => Some(rest.to_owned()),
        _ => None,
    }
}

/// One spelling of an address on the site for every way of writing it:
/// percent-decoded, composed, a final slash aside, in the capitals a disk
/// folds.
fn spelt(address: &Address) -> String {
    crate::platform::folded(address.as_str().trim_end_matches('/'))
}

/// `text` in YAML's double quotes, every character a reader would take
/// as other than itself escaped.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if printable(c) => out.push(c),
            c if u32::from(c) <= 0xFFFF => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push_str(&format!("\\U{:08X}", u32::from(c))),
        }
    }
    out.push('"');
    out
}

/// Whether YAML reads `c` as itself in double quotes.
fn printable(c: char) -> bool {
    matches!(
        u32::from(c),
        0x20..=0x7E | 0xA0..=0x2027 | 0x202A..=0xD7FF | 0xE000..=0xFEFE | 0xFF00..=0xFFFD | 0x10000..
    )
}

/// The mentions of the site's pages, each page's in the order a page
/// shows them: by the day each was published, then by id, the undated
/// last.
#[derive(Default, Debug)]
pub struct Mentions {
    by_page: HashMap<PageId, Vec<Mention>>,
}

impl Mentions {
    /// The mentions of `page`.
    pub fn of(&self, page: PageId) -> &[Mention] {
        self.by_page.get(&page).map_or(&[], Vec::as_slice)
    }
}

/// Whether the site reads mentions at all: it says it uses them, or it
/// holds some.
fn read_at_all(site: &Site, settings: &Settings) -> bool {
    settings.truth("webmentions") == Some(true)
        || (site.fetched.files.iter()).any(|(path, _)| mention_file(path))
}

/// Whether `path` under `fetched/` is a file in `fetched/mentions/` that
/// is read as a mention.
pub(crate) fn mention_file(path: &str) -> bool {
    path.strip_prefix(FOLDER)
        .and_then(|rest| rest.strip_prefix('/'))
        .is_some_and(|name| !name.contains('/') && name.ends_with(".yaml"))
}

/// Reads every mention in `fetched/mentions/` into its page, telling each
/// value that doesn't read as Statac writes it, and each mention of an
/// address no page stands at. A site without a `url` reads none.
pub fn read(site: &Site, settings: &Settings, sources: &mut Sources) -> (Mentions, Vec<Problem>) {
    let mut problems = Vec::new();
    let mut mentions = Mentions::default();
    if !read_at_all(site, settings) {
        return (mentions, problems);
    }
    // A mention names the page where its sender found it, whatever
    // address the run was given.
    let Some(url) = settings.home().map(str::to_owned) else {
        let kind = Kind::NoSiteAddress {
            waiting: Waiting::Webmentions,
        };
        let at = settings.get("webmentions").and_then(|on| on.origin.place);
        problems.push(Problem::placed(kind, at));
        return (mentions, problems);
    };
    let pages = pages_by_address(site);
    for (_, mention, target_at) in kept(&site.fetched, sources, &mut problems) {
        let found = on_site(&mention.target, &url)
            .and_then(|path| Address::read(&decoded(&path)).ok())
            .and_then(|address| pages.get(&spelt(&address)).copied());
        match found {
            Some(Some(page)) => mentions.by_page.entry(page).or_default().push(mention),
            // A page left out of this build: its mentions wait with it.
            Some(None) => {}
            None => problems.push(Problem::at(
                Kind::MentionOfNoPage {
                    target: mention.target.clone(),
                },
                target_at,
            )),
        }
    }
    for held in mentions.by_page.values_mut() {
        held.sort_by_key(|mention| {
            let day = mention.published.map(|day| (day.year, day.month, day.day));
            (day.is_none(), day, mention.id)
        });
    }
    (mentions, problems)
}

/// Every mention in `fetched/mentions/` that reads, by its path under
/// `fetched/`, in order of id, with where its `target` is written; each
/// value that doesn't read as Statac writes it is told.
pub fn kept(
    fetched: &super::Fetched,
    sources: &mut Sources,
    problems: &mut Vec<Problem>,
) -> Vec<(String, Mention, Place)> {
    let faces: Vec<&str> = (fetched.files.iter())
        .filter_map(|(path, _)| path.strip_prefix(FACES)?.strip_prefix('/'))
        .collect();
    let mut files: Vec<_> = (fetched.files.iter())
        .filter(|(path, _)| mention_file(path))
        .collect();
    files.sort_by_key(|(path, _)| (id_of(path), path.clone()));
    let mut kept = Vec::new();
    for (path, file) in files {
        let shown = format!("{}/{path}", super::FOLDER);
        let text = match std::fs::read_to_string(file) {
            Ok(text) => text,
            Err(error) => {
                problems.push(Problem::nowhere(Kind::CantRead {
                    path: shown.into(),
                    reason: crate::problem::system_reason(&error),
                }));
                continue;
            }
        };
        let source = sources.add(shown, text);
        if let Some((mention, target_at)) = mention(sources, source, id_of(path), &faces, problems)
        {
            kept.push((path.clone(), mention, target_at));
        }
    }
    kept
}

/// Each address a mention may name a page by, to the page there: its own,
/// and each in its `redirect_from`. A page left out of the build stands at
/// its addresses as none.
fn pages_by_address(site: &Site) -> HashMap<String, Option<PageId>> {
    let mut pages = HashMap::new();
    for held in &site.held_back {
        for address in std::iter::once(&held.address).chain(&held.redirect_from) {
            pages.insert(spelt(address), None);
        }
    }
    for redirect in &site.redirects {
        pages.insert(spelt(&redirect.from), Some(redirect.page));
    }
    for (id, page) in site.pages.iter() {
        pages.insert(spelt(&page.address), Some(id));
    }
    pages
}

/// The mention in `source`, and where its `target` is written; none where
/// it doesn't read, which is told.
fn mention(
    sources: &Sources,
    source: SourceId,
    id: Option<u64>,
    faces: &[&str],
    problems: &mut Vec<Problem>,
) -> Option<(Mention, Place)> {
    let node = match yaml::read(sources, source) {
        Ok(node) => node,
        Err(problem) => {
            problems.push(problem);
            return None;
        }
    };
    let Yaml::Mapping(entries) = &node.value else {
        let name = sources.path(source).to_string_lossy().into_owned();
        problems.push(Reader::new(sources, &name).wrong(&node, Expected::Fields, None));
        return None;
    };
    let told = problems.len();
    let mut refused = Vec::new();
    let mut texts: HashMap<&str, (String, Place)> = HashMap::new();
    let (mut kind, mut published, mut fetched) = (None, None, None);
    for entry in entries {
        let name = key_name(&entry.key);
        let reader = Reader::new(sources, &name);
        let value = &entry.value;
        let Some(&field) = FIELDS.iter().find(|field| **field == name) else {
            problems.push(Problem::at(
                Kind::UnknownWord {
                    word: Word::Field,
                    written: name.clone(),
                    nearest: crate::suggest::nearest(&name, FIELDS.iter().copied())
                        .map(str::to_owned),
                },
                entry.key.place,
            ));
            continue;
        };
        let read = match field {
            "kind" => {
                let words: Vec<&str> = Said::ALL.iter().map(|said| said.word()).collect();
                reader.one_of(value, &words).map(|word| {
                    kind = word.and_then(|word| {
                        Said::ALL.into_iter().find(|said| said.word() == word.value)
                    });
                })
            }
            "published" => reader.date(value).map(|day| published = day),
            "fetched" => reader.date(value).map(|day| fetched = day),
            _ => reader.text(value).and_then(|text| {
                let Some(text) = text else {
                    return Ok(());
                };
                let wrong = |expected| Err(reader.wrong(value, expected, None));
                match field {
                    "url" | "author_url" if !web_address(&text.value) => {
                        return wrong(Expected::SiteAddress);
                    }
                    "face" if !a_face_name(&text.value) => return wrong(Expected::FaceName),
                    "face" if !faces.contains(&text.value.as_str()) => {
                        return Err(Problem::at(
                            Kind::MissingFile {
                                path: format!("{}/{FACES}/{}", super::FOLDER, text.value),
                                hint: None,
                            },
                            value.place,
                        ));
                    }
                    _ => {}
                }
                texts.insert(field, (text.value, value.place));
                Ok(())
            }),
        };
        if let Err(problem) = read {
            refused.push(field);
            problems.push(problem);
        }
    }
    let head = crate::content::words::head(sources, source);
    for (field, there) in [
        ("kind", kind.is_some()),
        ("url", texts.contains_key("url")),
        ("target", texts.contains_key("target")),
        ("fetched", fetched.is_some()),
    ] {
        if !there && !refused.contains(&field) {
            problems.push(Problem::at(
                Kind::MissingField {
                    field: field.to_owned(),
                    required_by: Requirer::Mention,
                },
                head,
            ));
        }
    }
    if problems.len() > told {
        return None;
    }
    let mut take = |field| texts.remove(field);
    let (target, target_at) = take("target")?;
    Some((
        Mention {
            id,
            kind: kind?,
            author: take("author").map(|(text, _)| text),
            author_url: take("author_url").map(|(text, _)| text),
            face: take("face").map(|(text, _)| text),
            url: take("url")?.0,
            target,
            published: published.map(|day| day.value),
            fetched: fetched?.value,
            words: take("words").map(|(text, _)| text),
        },
        target_at,
    ))
}

#[cfg(test)]
mod tests;
