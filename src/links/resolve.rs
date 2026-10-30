//! Where a link leads, and what the built site holds there.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use url::Url;

use crate::address;
use crate::config::Settings;

/// What the built site holds, and what counts as the site, for following
/// links.
pub(crate) struct Site {
    /// The path the site is published under, such as `/blog`; empty at the
    /// top of its domain.
    base: String,
    /// The top of the site's host, from its `url`, or a host no link can
    /// name where it has none.
    top: Url,
    /// The other hosts the site is published at, as `other_hosts` names them,
    /// written as the URL standard writes hosts.
    hosts: Vec<String>,
    /// The host and the path the site is at beyond this run, where the
    /// run was given another address: a link written to the site there is
    /// a link into the site still.
    home: Option<(String, String)>,
    /// Every file the build publishes, from the top of the built site.
    files: HashSet<String>,
    /// Paths on the site's host that something else answers at, named by
    /// `served_elsewhere` as a link from the top of the host names them, without a
    /// `/` at their end.
    served_elsewhere: Vec<String>,
    /// What `after_build_outputs` names, made once Statac has finished.
    made_after: Vec<String>,
    /// Every address the site publishes, gathered the first time a link
    /// that lands nowhere wants the nearest of them.
    addresses: OnceLock<Addresses>,
    /// The files of the lists' own pages.
    lists: HashSet<String>,
    /// Files published only where a page links them, and their names.
    watched: HashSet<String>,
    watched_names: HashSet<String>,
    /// Whether each page checked notes what its links asked the site for.
    notes: bool,
    /// What `statac refresh` recorded of the links to other sites, where
    /// the site checks them.
    external: Option<External>,
}

/// What the check of links to other sites reads.
pub(crate) struct External {
    pub(crate) answers: crate::fetched::links::Links,
    /// The URLs `unchecked_external_links` lists.
    pub(crate) unchecked: crate::config::refused::Refused,
}

/// Where a link leads.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Leads {
    /// Off the site: another scheme or host, a path on the site's host
    /// that isn't the site's, or one something else makes.
    Away,
    Here(Arrival),
}

/// Somewhere on the site a link leads.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Arrival {
    /// From the top of the site, without the site's own path, its `%XX`
    /// read as the characters they stand for.
    pub(crate) path: String,
    /// What follows the `#`, as written, where anything does.
    pub(crate) fragment: Option<String>,
    /// The scheme and host the link named the site by, where it named one.
    pub(crate) origin: Option<String>,
    /// Whether the path is on the site's host but outside the site's own
    /// path, where the site publishes nothing: `path` is then from the top
    /// of the host.
    pub(crate) outside: bool,
    /// Whether the link named the site by the address it is at beyond
    /// this run, and `path` is from the top of the site there.
    pub(crate) home: bool,
}

impl Site {
    /// The site `settings` describe, publishing `files`, each a path from
    /// the top of the built site.
    pub(crate) fn new(settings: &Settings, files: impl IntoIterator<Item = String>) -> Site {
        let top = settings
            .text("url")
            .and_then(|url| Url::parse(url.value).ok())
            .filter(|url| url.host_str().is_some())
            .and_then(|url| url.join("/").ok())
            .unwrap_or_else(|| Url::parse(NO_HOST).expect("a URL"));
        let hosts = settings
            .texts("other_hosts")
            .iter()
            .filter_map(|also| host_named(&also.value))
            .collect();
        let served_elsewhere = settings
            .texts("served_elsewhere")
            .iter()
            .map(|path| format!("/{}", path.value.trim_matches('/')))
            .collect();
        let made_after = settings
            .texts("after_build_outputs")
            .iter()
            .map(|place| place.value.trim_start_matches('/').to_owned())
            .collect();
        let home = settings
            .elsewhere()
            .then(|| settings.home())
            .flatten()
            .and_then(|home| Some((host_named(home)?, crate::config::path_of(home))));
        Site {
            base: settings.base(),
            top,
            hosts,
            home,
            files: files.into_iter().collect(),
            served_elsewhere,
            made_after,
            addresses: OnceLock::new(),
            lists: HashSet::new(),
            watched: HashSet::new(),
            watched_names: HashSet::new(),
            notes: false,
            external: None,
        }
    }

    /// The same site, holding each link to another site to `external`.
    pub(crate) fn checking(mut self, external: External) -> Site {
        self.external = Some(external);
        self
    }

    pub(crate) fn external(&self) -> Option<&External> {
        self.external.as_ref()
    }

    /// The URL `written`, a link on the page published at `from`, leads to
    /// on another site, as `fetched/links` records it.
    pub(super) fn elsewhere(&self, written: &str, from: &str) -> Option<String> {
        let link = self
            .top
            .join(from)
            .and_then(|page| page.join(written))
            .ok()?;
        let host = link.host_str()?;
        let ours = self.top.host_str() == Some(host)
            || self.hosts.iter().any(|also| also == host)
            || self.home.as_ref().is_some_and(|(home, _)| home == host);
        if ours {
            return None;
        }
        crate::fetched::links::key(&link)
    }

    /// The same site, with `files` the lists' own pages, each a path from
    /// the top of the built site.
    pub(crate) fn listing(mut self, files: impl IntoIterator<Item = String>) -> Site {
        self.lists = files.into_iter().collect();
        self
    }

    /// The same site, noting each page that links one of `files`, each a
    /// path from the top of the built site.
    pub(crate) fn watching(mut self, files: impl IntoIterator<Item = String>) -> Site {
        self.watched = files.into_iter().collect();
        self.watched_names = self
            .watched
            .iter()
            .map(|file| file.rsplit('/').next().unwrap_or(file).to_owned())
            .collect();
        self
    }

    /// Where on the site `written`, a link on the page published at
    /// `from`, leads, where it is named as a watched file is, and the
    /// watched file it lands on there.
    pub(super) fn watched(&self, written: &str, from: &str) -> Option<(String, Option<String>)> {
        if self.watched.is_empty() {
            return None;
        }
        let path = &written[..written.find(['?', '#']).unwrap_or(written.len())];
        let name = address::decoded(path.rsplit('/').next().unwrap_or(path));
        if !self.watched_names.contains(&name) {
            return None;
        }
        let Leads::Here(arrival) = self.leads(written, from) else {
            return None;
        };
        if arrival.outside {
            return None;
        }
        let file = self.file_at(&arrival.path);
        let file = file.filter(|file| self.watched.contains(file));
        Some((arrival.path, file))
    }

    /// The same site, noting on each page checked what its links asked
    /// the site for, so the page can later be held to another building of
    /// the site.
    pub(crate) fn noting(mut self) -> Site {
        self.notes = true;
        self
    }

    pub(super) fn notes(&self) -> bool {
        self.notes
    }

    /// Whether `file` is published only where a page links it.
    pub(super) fn waits(&self, file: &str) -> bool {
        self.watched.contains(file)
    }

    pub(super) fn base(&self) -> &str {
        &self.base
    }

    /// The path the site is under where the link arriving as `arrival`
    /// named it.
    pub(super) fn base_of(&self, arrival: &Arrival) -> &str {
        match (&self.home, arrival.home) {
            (Some((_, base)), true) => base,
            _ => &self.base,
        }
    }

    /// Where `written`, a link on the page published at `from`, leads, as
    /// the URL standard reads it. `from` is the page's path from the top of
    /// its host, the site's own path included.
    pub(super) fn leads(&self, written: &str, from: &str) -> Leads {
        let Ok(link) = self.top.join(from).and_then(|page| page.join(written)) else {
            return Leads::Away;
        };
        if !matches!(link.scheme(), "http" | "https") {
            return Leads::Away;
        }
        let host = link.host_str().unwrap_or_default();
        let on_top = self.top.host_str() == Some(host);
        let also = self.hosts.iter().any(|ours| ours == host);
        // The site's path beyond this run, on each host it is at there.
        let there = (self.home.as_ref())
            .filter(|(home, _)| home == host || also)
            .map(|(_, base)| base.as_str());
        if !on_top && !also && there.is_none() {
            return Leads::Away;
        }
        let fragment = link.fragment().map(str::to_owned);
        let origin = named_host(written).map(str::to_owned);
        let path = link.path().to_owned();
        let path = address::decoded(&path);
        let served_elsewhere = self.served_elsewhere.iter().any(|named| {
            path.strip_prefix(named.as_str())
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
        });
        if served_elsewhere {
            return Leads::Away;
        }
        // A path on the site's host outside the site's own path is
        // somewhere the site publishes nothing.
        let under = |base: &str| match base.is_empty() {
            true => Some(path.clone()),
            false if path == base => Some("/".to_owned()),
            false => path
                .strip_prefix(base)
                .filter(|inside| inside.starts_with('/'))
                .map(str::to_owned),
        };
        // On a host the run's address and the other share, the run's
        // path is tried first.
        let here = (on_top || self.home.is_none()).then_some(self.base.as_str());
        let (path, outside, home) = match (here.and_then(under), there.and_then(under)) {
            (Some(inside), _) => (inside, false, false),
            (None, Some(inside)) => (inside, false, true),
            (None, None) => (path, true, false),
        };
        let inside = &path[1..];
        let made_after = self
            .made_after
            .iter()
            .any(|place| match place.strip_suffix('/') {
                Some(folder) => inside == folder || inside.starts_with(place.as_str()),
                None => inside == place,
            });
        if made_after && !outside {
            return Leads::Away;
        }
        Leads::Here(Arrival {
            path,
            fragment,
            origin,
            outside,
            home,
        })
    }

    /// The file the built site holds at `path`, a path from the top of the
    /// site: a page at a folder's address is its `index.html`, and an
    /// address without its last `/` still reaches it.
    pub(super) fn file_at(&self, path: &str) -> Option<String> {
        self.file_among(path, |_| false)
    }

    /// [`Site::file_at`], for a site that publishes besides its files
    /// whatever `also` holds: files known only once the pages naming them
    /// are built, as a page's sharing card is.
    pub(super) fn file_among(&self, path: &str, also: impl Fn(&str) -> bool) -> Option<String> {
        let holds = |file: &str| self.files.contains(file) || also(file);
        let inside = &path[1..];
        if path.ends_with('/') {
            let file = format!("{inside}index.html");
            return holds(&file).then_some(file);
        }
        if holds(inside) {
            return Some(inside.to_owned());
        }
        let file = format!("{inside}/index.html");
        holds(&file).then_some(file)
    }

    /// The address the site publishes nearest `path`, where one is near,
    /// for a link on the page published as the file `from`, which is never
    /// the one offered.
    pub(super) fn nearest(&self, path: &str, from: Option<&str>) -> Option<&str> {
        self.addresses
            .get_or_init(|| Addresses::new(&self.files, &self.lists))
            .nearest(path, from.map(address_of).as_deref())
    }
}

/// Where a site has no `url`, the top it is read from: a host no link
/// names, so every link naming a host leads away.
const NO_HOST: &str = "https://site.invalid/";

/// The host an `other_hosts` entry names, written bare, `example.net`, or as a
/// whole address, as the URL standard writes hosts.
fn host_named(written: &str) -> Option<String> {
    let whole = Url::parse(written)
        .ok()
        .filter(|url| url.host_str().is_some());
    let url = match whole {
        Some(url) => url,
        None => Url::parse(&format!("https://{written}")).ok()?,
    };
    url.host_str().map(str::to_owned)
}

/// The scheme and host `written` names a site by, as written: everything
/// before the path of `https://example.org/a/` or `//example.org/a/`.
fn named_host(written: &str) -> Option<&str> {
    let written = written.trim_matches(|c: char| c.is_ascii_whitespace());
    let after = match written.find("//") {
        Some(0) => 2,
        Some(at) if written[..at].ends_with(':') => at + 2,
        _ => return None,
    };
    let end = written[after..]
        .find(['/', '?', '#'])
        .map_or(written.len(), |at| after + at);
    Some(&written[..end])
}

/// Every address the site publishes, as a link names it, kept so the
/// nearest to one that lands nowhere is quick to find on a large site.
struct Addresses {
    all: Vec<String>,
    /// Each address's place in `all`, by the folder it is in and by its
    /// last piece in small letters.
    by_folder: HashMap<String, Vec<usize>>,
    by_name: HashMap<String, Vec<usize>>,
    /// The addresses of the lists' own pages.
    lists: HashSet<usize>,
}

/// Past this many addresses, only those sharing a folder or a last piece
/// with the link are weighed, since comparing each costs the length of
/// both.
const WEIGHED_WHOLE: usize = 2_000;

impl Addresses {
    fn new(files: &HashSet<String>, lists: &HashSet<String>) -> Addresses {
        let mut all: Vec<String> = files.iter().map(|file| address_of(file)).collect();
        all.sort_unstable();
        let mut by_folder: HashMap<String, Vec<usize>> = HashMap::new();
        let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
        for (at, address) in all.iter().enumerate() {
            let (folder, name) = parted(address);
            by_folder.entry(folder.to_owned()).or_default().push(at);
            by_name.entry(name.to_lowercase()).or_default().push(at);
        }
        let lists = lists.iter().map(|file| address_of(file));
        let lists = lists.filter_map(|address| all.binary_search(&address).ok());
        Addresses {
            lists: lists.collect(),
            all,
            by_folder,
            by_name,
        }
    }

    fn nearest(&self, path: &str, own: Option<&str>) -> Option<&str> {
        let (folder, name) = parted(path);
        let others = |found: Option<&Vec<usize>>| -> Vec<usize> {
            let found = found.into_iter().flatten().copied();
            found
                .filter(|&at| Some(self.all[at].as_str()) != own)
                .collect()
        };
        // An address with the same last piece published once, wherever it
        // is, is the one meant, as when a link names it from the wrong
        // folder: but never a list's own page, which shares a tag's or a
        // folder's name with whatever else was once there.
        let named = others(self.by_name.get(&name.to_lowercase()));
        if !name.is_empty()
            && let [only] = named[..]
            && !self.lists.contains(&only)
        {
            return Some(&self.all[only]);
        }
        if self.all.len() <= WEIGHED_WHOLE {
            let all = self.all.iter().map(String::as_str);
            return crate::suggest::nearest(path, all.filter(|address| Some(*address) != own));
        }
        let mut weighed = others(self.by_folder.get(folder));
        weighed.extend(named);
        weighed.sort_unstable();
        weighed.dedup();
        crate::suggest::nearest(path, weighed.iter().map(|&at| self.all[at].as_str()))
    }
}

/// The address a link names the published file `file` by.
fn address_of(file: &str) -> String {
    match file.strip_suffix("index.html") {
        Some(folder) if folder.is_empty() || folder.ends_with('/') => format!("/{folder}"),
        _ => format!("/{file}"),
    }
}

/// The folder an address is in, and its last piece, a folder's own name
/// for a folder.
fn parted(address: &str) -> (&str, &str) {
    let trimmed = address.strip_suffix('/').unwrap_or(address);
    let cut = trimmed.rfind('/').map_or(0, |at| at + 1);
    (&trimmed[..cut], &trimmed[cut..])
}
