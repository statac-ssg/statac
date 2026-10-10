//! The webmentions the author turned away with `statac refuse`, and the
//! senders, kept in `fetched/refused`: text Statac alone writes, as it
//! writes the record. Its first line is `statac refused 1`, then a line an
//! address, sorted, of three fields parted by tabs: `mention` or `sender`,
//! the address as the author gave it, and the day it was refused.
//!
//! Two addresses are one where they differ only in the scheme, a leading
//! `www.`, the host's capitals and a final slash. An address is under
//! another on the same host and port where the other's path leads to it
//! at a `/`, and the other has no query.

use crate::date::Date;
use crate::problem::{Kind, Problem, RecordFault};
use crate::source::{Place, SourceId, Sources, Span};

/// The list's name in `fetched/`.
pub const NAME: &str = "refused";

const OPENING: &str = "statac refused 1";

/// How much an address turns away.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Reach {
    /// The one mention whose `url` is the address.
    Mention,
    /// Every mention whose `url` or `author_url` is at or under it.
    Sender,
}

impl Reach {
    pub fn word(self) -> &'static str {
        match self {
            Reach::Mention => "mention",
            Reach::Sender => "sender",
        }
    }
}

/// One line of the list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Entry {
    pub reach: Reach,
    /// As the author gave it, without its control characters.
    pub address: String,
    pub day: Date,
}

/// The list, an address once.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct List {
    entries: Vec<Entry>,
}

/// An address as it is compared.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Spot {
    host: String,
    port: Option<u16>,
    /// Without a final slash.
    path: String,
    /// The query and the fragment, as written.
    rest: String,
}

impl Spot {
    /// `address` as it is compared, where it is `http` or `https` and
    /// names a host.
    pub fn of(address: &str) -> Option<Spot> {
        let url = url::Url::parse(address).ok()?;
        if !matches!(url.scheme(), "http" | "https") {
            return None;
        }
        let host = url.host_str()?.to_ascii_lowercase();
        let host = host.strip_prefix("www.").map(str::to_owned).unwrap_or(host);
        let rest = format!(
            "{}{}",
            url.query()
                .map(|query| format!("?{query}"))
                .unwrap_or_default(),
            url.fragment()
                .map(|part| format!("#{part}"))
                .unwrap_or_default()
        );
        Some(Spot {
            host,
            port: url.port(),
            path: url.path().trim_end_matches('/').to_owned(),
            rest,
        })
    }

    /// Whether this is `over`, or under it.
    pub fn at_or_under(&self, over: &Spot) -> bool {
        if self == over {
            return true;
        }
        self.host == over.host
            && self.port == over.port
            && over.rest.is_empty()
            && (self.path.strip_prefix(over.path.as_str()))
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    }
}

/// Whether `address` is at or under `over`, each an address as written.
pub fn at_or_under(address: &str, over: &Spot) -> bool {
    Spot::of(address).is_some_and(|spot| spot.at_or_under(over))
}

impl List {
    /// Reads the list held in `source`, with a problem at each line that
    /// doesn't read; such a line is left out.
    pub fn read(sources: &Sources, source: SourceId) -> (List, Vec<Problem>) {
        let text = sources.text(source);
        let mut list = List::default();
        let mut problems = Vec::new();
        let malformed = |fault, at: Place| Problem::at(Kind::MalformedRecord { fault }, at);
        if text.is_empty() {
            let place = Place {
                source,
                span: Span::new(0, 0),
            };
            return (list, vec![malformed(RecordFault::RefusedOpening, place)]);
        }
        let mut start = 0;
        for (number, line) in text.split_inclusive('\n').enumerate() {
            let at = start;
            start += line.len();
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            let place = |from: usize, len: usize| Place {
                source,
                span: Span::new(at + from, at + from + len),
            };
            if number == 0 {
                if line != OPENING {
                    let fault = RecordFault::RefusedOpening;
                    problems.push(malformed(fault, place(0, line.len())));
                    return (list, problems);
                }
                continue;
            }
            let mut fields = Vec::new();
            let mut from = 0;
            for field in line.split('\t') {
                fields.push((field, place(from, field.len())));
                from += field.len() + 1;
            }
            let [(reach, reach_at), (address, address_at), (day, day_at)] = fields[..] else {
                let fault = RecordFault::RefusedFields(fields.len());
                problems.push(malformed(fault, place(0, line.len())));
                continue;
            };
            let reach = match reach {
                "mention" => Reach::Mention,
                "sender" => Reach::Sender,
                other => {
                    problems.push(malformed(RecordFault::Reach(other.into()), reach_at));
                    continue;
                }
            };
            let Some(spot) = Spot::of(address) else {
                let fault = RecordFault::Address(address.into());
                problems.push(malformed(fault, address_at));
                continue;
            };
            let day = match crate::date::read(day) {
                Ok(read) if read.time.is_none() && read.iso_day() == day => read,
                _ => {
                    problems.push(malformed(RecordFault::Day(day.into()), day_at));
                    continue;
                }
            };
            if list.get(&spot).is_some() {
                let fault = RecordFault::Twice(address.into());
                problems.push(malformed(fault, address_at));
                continue;
            }
            let address = address.to_owned();
            list.entries.push(Entry {
                reach,
                address,
                day,
            });
        }
        (list, problems)
    }

    /// The line of the address at `spot`.
    pub fn get(&self, spot: &Spot) -> Option<&Entry> {
        (self.entries.iter()).find(|entry| Spot::of(&entry.address).as_ref() == Some(spot))
    }

    /// The line turning away a mention with `url`, sent by `author_url`.
    pub fn refusing(&self, url: &str, author_url: Option<&str>) -> Option<&Entry> {
        let (url, author_url) = (Spot::of(url), author_url.and_then(Spot::of));
        self.entries.iter().find(|entry| {
            let Some(spot) = Spot::of(&entry.address) else {
                return false;
            };
            match entry.reach {
                Reach::Mention => url.as_ref() == Some(&spot),
                Reach::Sender => [&url, &author_url]
                    .into_iter()
                    .flatten()
                    .any(|sent| sent.at_or_under(&spot)),
            }
        })
    }

    /// Adds `entry`, in place of the line of the same address.
    pub fn set(&mut self, entry: Entry) {
        let spot = Spot::of(&entry.address);
        (self.entries).retain(|held| spot.is_none() || Spot::of(&held.address) != spot);
        self.entries.push(entry);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The list as written, sorted by address.
    pub fn text(&self) -> String {
        let mut entries: Vec<&Entry> = self.entries.iter().collect();
        entries.sort_by(|a, b| (&a.address, a.reach).cmp(&(&b.address, b.reach)));
        let mut text = format!("{OPENING}\n");
        for entry in entries {
            text.push_str(&format!(
                "{}\t{}\t{}\n",
                entry.reach.word(),
                entry.address,
                entry.day.iso_day()
            ));
        }
        text
    }
}

/// The list as a problem names it, from the site's folder.
pub fn shown() -> String {
    format!("{}/{NAME}", super::FOLDER)
}

#[cfg(test)]
mod tests;
