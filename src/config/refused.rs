//! The webmentions the author refuses, a URL a line in
//! `refuse_webmentions_from:`. A line refuses every mention sent from a
//! page at or under its URL, and every mention whose sender's own URL is
//! at or under it.
//!
//! Two URLs are one where they differ only in the scheme, a leading
//! `www.`, the host's capitals and a final slash. A URL is under another
//! on the same host and port where the other's path leads to it at a
//! `/`, and the other has no query.
//!
//! Statac once kept the refused in `fetched/refused`, which it no longer
//! reads: `check` tells a site that still has one the lines that carry its
//! URLs over.

use crate::config::Settings;

/// The retired list's name in `fetched/`.
pub(crate) const NAME: &str = "refused";

/// The first line Statac wrote in the retired list.
const OPENING: &str = "statac refused 1";

/// The setting that lists the refused.
pub(crate) const SETTING: &str = "refuse_webmentions_from";

/// An address as it is compared.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) struct Spot {
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
    pub(crate) fn of(address: &str) -> Option<Spot> {
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
    pub(crate) fn at_or_under(&self, over: &Spot) -> bool {
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

/// The lines of `refuse_webmentions_from:`, each as written with where it
/// is compared.
#[derive(Clone, Default, Debug)]
pub(crate) struct Refused {
    lines: Vec<(String, Spot)>,
}

impl Refused {
    /// The lines `settings` write. A line that is no URL was told as the
    /// settings were read, and is left out.
    pub(crate) fn of(settings: &Settings) -> Refused {
        let lines = (settings.texts(SETTING).iter())
            .filter_map(|line| Some((line.value.clone(), Spot::of(&line.value)?)))
            .collect();
        Refused { lines }
    }

    /// The line, as written, that refuses a mention sent from `url` by
    /// the sender at `author_url`.
    pub(crate) fn refusing(&self, url: &str, author_url: Option<&str>) -> Option<&str> {
        let sent: Vec<Spot> = std::iter::once(url)
            .chain(author_url)
            .filter_map(Spot::of)
            .collect();
        (self.lines.iter())
            .find(|(_, over)| sent.iter().any(|spot| spot.at_or_under(over)))
            .map(|(line, _)| line.as_str())
    }
}

/// The URLs the retired list in `text` holds, each as written, in order,
/// and how many of its lines don't read as Statac wrote them: its first
/// line `statac refused 1`, then a line a URL, of three fields parted by
/// tabs, the URL the second.
pub(crate) fn carried(text: &str) -> (Vec<String>, usize) {
    let (mut urls, mut unread) = (Vec::new(), 0);
    for (number, line) in text.lines().enumerate() {
        if (number == 0 && line == OPENING) || line.trim().is_empty() {
            continue;
        }
        match line.split('\t').collect::<Vec<_>>()[..] {
            [_, url, _] if Spot::of(url).is_some() => urls.push(url.to_owned()),
            _ => unread += 1,
        }
    }
    (urls, unread)
}

#[cfg(test)]
mod tests;
