//! Asking the webmention relay for what it received for the site since
//! the last mention kept, and keeping each, with the sender's face.
//!
//! The relay's address carries the token, so it is never said: a refusal
//! is worded with the address the token is left out of, and the token is
//! taken out of whatever else the network said.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::{Fetch, Token, keep, unwritten};
use crate::date::Date;
use crate::fetch::Refusal;
use crate::fetched::mentions::{self, Mention, Said};
use crate::fetched::refused::Refused;
use crate::fetched::{Held, Line, Record};
use crate::pictures::Bounds;
use crate::report::{Report, shown};
use crate::source::Sources;
use crate::wording;
use crate::yaml::{Node, Value};

/// The relay's address, as it may be said.
const RELAY: &str = "https://webmention.io/api/mentions.jf2";

/// The most mentions one ask brings back.
const PER_ASK: usize = 100;

/// The most asks one refresh makes.
const ASKS: usize = 50;

/// The most bytes one answer of the relay may hold.
pub(super) const LIMIT: u64 = 5_000_000;

/// The most bytes a face fetched may hold.
pub(super) const FACE_LIMIT: u64 = 10_000_000;

/// The widest a face is kept.
pub(super) const FACE_WIDTH: u32 = 96;

/// How large a face may be for its pixels to be read at all.
const FACE_BOUNDS: Bounds = Bounds {
    side: 4_096,
    bytes: 64_000_000,
};

/// What came of asking.
#[derive(Default)]
pub(super) struct Asked {
    pub fetched: usize,
    pub faces: usize,
    pub not_kept: usize,
    /// Whether the relay answered at least once.
    pub answered: bool,
    /// Whether the relay couldn't be asked, or what it sent couldn't be
    /// kept.
    pub failed: bool,
}

/// Where what is asked for is kept, and what is known of it.
pub(super) struct Keeping<'a> {
    pub folder: &'a Path,
    pub record: &'a mut Record,
    pub standing: &'a mut HashSet<String>,
    pub today: Date,
    /// The site's `url`.
    pub url: &'a str,
    pub refused: &'a Refused,
    pub report: &'a Report<'a>,
}

/// The relay's address for the mentions after `since`, with `token`.
fn relay(token: &Token, since: u64) -> String {
    format!(
        "{RELAY}?token={}&since_id={since}&per-page={PER_ASK}&sort-by=created&sort-dir=up",
        encoded(token.text())
    )
}

/// `text` as a query's value, every byte but the unreserved few written
/// as `%` and the byte.
fn encoded(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(char::from(byte));
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Why the relay couldn't be asked, in words that never hold the token.
fn refused(refusal: &Refusal, token: &Token) -> String {
    let reason = match refusal {
        Refusal::Status {
            status: 401 | 403, ..
        } => wording::TOKEN_REFUSED.to_owned(),
        refusal => wording::refusal(RELAY, refusal),
    };
    let token = token.text();
    reason.replace(token, "•••").replace(&encoded(token), "•••")
}

/// The highest id the record holds a mention's line for.
fn highest(record: &Record) -> u64 {
    let ids = record.iter().filter_map(|(path, _)| mentions::id_of(path));
    ids.max().unwrap_or(0)
}

/// Asks the relay for what is new with `token` and keeps it with `kept`,
/// saying each on `say` as it goes.
pub(super) fn ask(
    fetcher: &dyn Fetch,
    token: &Token,
    kept: &mut Keeping<'_>,
    say: &mut dyn FnMut(&str),
) -> Asked {
    let mut asked = Asked::default();
    let mut faces = Faces::default();
    let mut since = highest(kept.record);
    for ask in 0.. {
        if ask == ASKS {
            say(&format!("{}\n", wording::asked_enough(ASKS)));
            break;
        }
        let answer = match fetcher.fetch(&relay(token, since), LIMIT) {
            Ok(answer) => answer,
            Err(refusal) => {
                say(&format!(
                    "{}\n",
                    shown(&wording::cant_ask(&refused(&refusal, token)))
                ));
                asked.failed = true;
                break;
            }
        };
        let Some(children) = children(&answer.bytes) else {
            say(&format!(
                "{}\n",
                wording::cant_ask(wording::UNREADABLE_ANSWER)
            ));
            asked.failed = true;
            break;
        };
        asked.answered = true;
        let mut newest = since;
        for child in &children {
            match child.id {
                Some(id) => newest = newest.max(id),
                None => {
                    asked.not_kept += 1;
                    continue;
                }
            }
            keep_child(fetcher, child, kept, &mut faces, &mut asked, say);
        }
        if children.len() < PER_ASK || newest == since {
            break;
        }
        since = newest;
    }
    asked
}

/// What one mention the relay sent says, as sent.
#[derive(Default, Debug)]
struct Child {
    id: Option<u64>,
    property: Option<String>,
    url: Option<String>,
    source: Option<String>,
    target: Option<String>,
    published: Option<String>,
    received: Option<String>,
    name: Option<String>,
    author_url: Option<String>,
    photo: Option<String>,
    text: Option<String>,
}

/// The mentions in the relay's answer, `{"type": "feed", "children":
/// [...]}`; none where it isn't one.
fn children(bytes: &[u8]) -> Option<Vec<Child>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut sources = Sources::default();
    let source = sources.add("", text.to_owned());
    let read = crate::yaml::read_json(&sources, source).ok()?;
    if field(&read, "type").and_then(json_text) != Some("feed") {
        return None;
    }
    let Value::Sequence(items) = &field(&read, "children")?.value else {
        return None;
    };
    let text = |node: Option<&Node>, name| {
        let found = node.and_then(|node| field(node, name));
        found.and_then(json_text).map(str::to_owned)
    };
    Some(
        items
            .iter()
            .map(|item| {
                let id = field(item, "wm-id").and_then(|id| match (&id.value, id.style) {
                    (Value::Scalar(id), crate::origin::Style::Plain) => mentions::whole_id(id),
                    _ => None,
                });
                let author = field(item, "author");
                Child {
                    id,
                    property: text(Some(item), "wm-property"),
                    url: text(Some(item), "url"),
                    source: text(Some(item), "wm-source"),
                    target: text(Some(item), "wm-target"),
                    published: text(Some(item), "published"),
                    received: text(Some(item), "wm-received"),
                    name: text(author, "name"),
                    author_url: text(author, "url"),
                    photo: text(author, "photo"),
                    text: text(field(item, "content"), "text"),
                }
            })
            .collect(),
    )
}

/// The text `node` holds, where it is JSON's text.
fn json_text(node: &Node) -> Option<&str> {
    match &node.value {
        Value::Scalar(text) if node.resolved() == Some(crate::yaml::Resolved::Str) => Some(text),
        _ => None,
    }
}

/// The value of `name` in the JSON object `node`.
fn field<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    let Value::Mapping(entries) = &node.value else {
        return None;
    };
    (entries.iter())
        .find(|entry| matches!(&entry.key.value, Value::Scalar(key) if key == name))
        .map(|entry| &entry.value)
}

/// The day a relay's time falls on, as it is written: `2026-09-03` of
/// `2026-09-03T21:48:39`, with an offset or none.
fn day_of(time: &str) -> Option<Date> {
    let day = time.get(..10)?;
    let rest = &time[10..];
    if !(rest.is_empty() || rest.starts_with(['T', 't', ' '])) {
        return None;
    }
    let read = crate::date::read(day).ok()?;
    (read.iso_day() == day).then(|| Date::day_only(read.year, read.month, read.day))
}

/// Keeps `child`, whose id it has, or says why not; either way its id is
/// recorded, so it isn't asked for again.
fn keep_child(
    fetcher: &dyn Fetch,
    child: &Child,
    kept: &mut Keeping<'_>,
    faces: &mut Faces,
    asked: &mut Asked,
    say: &mut dyn FnMut(&str),
) {
    let Some(id) = child.id else {
        return;
    };
    let path = mentions::path(id);
    if kept.record.get(&path).is_some() {
        return;
    }
    let clean = |text: &Option<String>| {
        let text = mentions::clean(text.as_deref().unwrap_or_default(), false);
        (!text.is_empty()).then_some(text)
    };
    let mut line = |line: String| say(&format!("{}\n", shown(&line)));
    let property = clean(&child.property).unwrap_or_default();
    let url = clean(&child.url).or_else(|| clean(&child.source));
    let target = clean(&child.target).unwrap_or_default();
    let refused = (url.as_deref())
        .and_then(|url| (kept.refused).refusing(url, clean(&child.author_url).as_deref()));
    let not_kept = match (Said::of_property(&property), &url) {
        (None, _) if property.is_empty() => Some(wording::no_kind_said(id)),
        (None, _) => Some(wording::not_read(id, &property)),
        (_, Some(url)) if !mentions::web_address(url) => Some(wording::not_a_web_address(id, url)),
        (_, None) => Some(wording::no_address_named(id)),
        _ if let Some(line) = refused => Some(wording::refused_here(id, line)),
        _ if mentions::on_site(&target, kept.url).is_none() => {
            Some(wording::not_on_site(id, &target))
        }
        _ => None,
    };
    if let Some(said) = not_kept {
        line(said);
        let held = Held::Absent;
        kept.record.set(
            &path,
            Line {
                held,
                day: kept.today,
            },
        );
        asked.not_kept += 1;
        return;
    }
    let (Some(kind), Some(url)) = (Said::of_property(&property), url) else {
        return;
    };
    let author_url = clean(&child.author_url).filter(|address| {
        let web = mentions::web_address(address);
        if !web {
            line(wording::sender_not_kept(id, address));
        }
        web
    });
    let face = clean(&child.photo)
        .filter(|address| {
            let web = mentions::web_address(address);
            if !web {
                let reason = wording::refusal(address, &Refusal::NotAnAddress);
                line(wording::cant_fetch_face(id, &reason));
            }
            web
        })
        .and_then(
            |address| match face(fetcher, &address, kept, faces, asked) {
                Ok((name, fetched)) => {
                    if fetched {
                        line(wording::fetched_file(&format!(
                            "{}/{}/{name}",
                            crate::fetched::FOLDER,
                            mentions::FACES
                        )));
                    }
                    Some(name)
                }
                Err(reason) => {
                    line(wording::cant_fetch_face(id, &reason));
                    None
                }
            },
        );
    let mention = Mention {
        id: Some(id),
        kind,
        author: clean(&child.name),
        author_url,
        face,
        url,
        target,
        published: (child.published.as_deref().and_then(day_of))
            .or_else(|| child.received.as_deref().and_then(day_of)),
        fetched: kept.today,
        words: child.text.as_deref().and_then(mentions::words),
    };
    let text = mention.text();
    let file = format!("{}/{path}", crate::fetched::FOLDER);
    match keep(kept.folder, &path, text.as_bytes()) {
        Ok(()) => {
            let held = Held::Hash(crate::fetched::hash_of(text.as_bytes()));
            kept.record.set(
                &path,
                Line {
                    held,
                    day: kept.today,
                },
            );
            kept.standing.insert(path);
            asked.fetched += 1;
            line(wording::fetched_file(&file));
        }
        Err(error) => {
            asked.failed = true;
            say(&unwritten(kept.report, &file, &error));
        }
    }
}

/// The faces met so far: each fetched or found there by its name, and why
/// each that couldn't be fetched wasn't.
#[derive(Default)]
struct Faces {
    known: HashMap<String, Result<(), String>>,
}

/// The name of the face at `address`, kept in `fetched/faces/`, and
/// whether it was fetched just now; or why it couldn't be.
fn face(
    fetcher: &dyn Fetch,
    address: &str,
    kept: &mut Keeping<'_>,
    faces: &mut Faces,
    asked: &mut Asked,
) -> Result<(String, bool), String> {
    let name = mentions::face_name(address);
    let path = format!("{}/{name}", mentions::FACES);
    if let Some(known) = faces.known.get(&name) {
        return known.clone().map(|()| (name, false));
    }
    if kept.standing.contains(&path) {
        faces.known.insert(name.clone(), Ok(()));
        return Ok((name, false));
    }
    let made = (fetcher.fetch(address, FACE_LIMIT))
        .map_err(|refusal| wording::refusal(address, &refusal))
        .and_then(|answer| {
            crate::album::quietly(|| {
                crate::pictures::remade(
                    &answer.bytes,
                    FACE_WIDTH,
                    FACE_BOUNDS,
                    &crate::pictures::Settings::default(),
                )
            })
            .map_err(|error| match error {
                crate::pictures::Error::TooLarge(size) => {
                    wording::too_large_to_open(size.width, size.height)
                }
                crate::pictures::Error::Encode { .. }
                | crate::pictures::Error::NotOffered { .. } => wording::NOT_MADE_AGAIN.to_owned(),
                _ => wording::NOT_A_PICTURE.to_owned(),
            })
        })
        .and_then(|bytes| {
            keep(kept.folder, &path, &bytes).map_err(|error| {
                asked.failed = true;
                crate::problem::Reason::of(&error).to_string()
            })?;
            Ok(bytes)
        });
    match made {
        Ok(bytes) => {
            let held = Held::Hash(crate::fetched::hash_of(&bytes));
            kept.record.set(
                &path,
                Line {
                    held,
                    day: kept.today,
                },
            );
            kept.standing.insert(path);
            faces.known.insert(name.clone(), Ok(()));
            asked.faces += 1;
            Ok((name, true))
        }
        Err(reason) => {
            faces.known.insert(name, Err(reason.clone()));
            Err(reason)
        }
    }
}

#[cfg(test)]
mod tests;
