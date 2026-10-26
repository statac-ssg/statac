//! Every old address kept, or sending readers on: a `pages.permalink`
//! pattern in the page of the highest folder where one puts most of the
//! pages beneath it right, a page's own `permalink` where the pattern
//! doesn't fit it, and `redirect_from` for the page's other old addresses. Where each page
//! lands is asked of Statac itself, by reading the new site as written.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::write::{Out, built_file, page_text};
use super::{Field, HomePages, Page, Value};
use crate::assets::Assets;
use crate::build::{self, Mode, Options};
use crate::content::address::{self, Address, Pieces};
use crate::content::{self, Run};
use crate::date::Date;
use crate::feeds::Feed;
use crate::problem::{Kind, Named, Problem, Severity, Unwritten, Word};
use crate::site::{ListKey, PagePlace};

/// How the old addresses were kept.
#[derive(Default, Debug)]
pub(super) struct Kept {
    /// Each folder given a pattern, by its path under `content/`, the
    /// pattern, and how many pages land at their old address by it.
    pub patterns: Vec<(String, String, usize)>,
    /// How many pages carry a `permalink` of their own.
    pub own: usize,
    /// How many pages carry `redirect_from`.
    pub redirected: usize,
    /// How many old addresses the new site answers.
    pub answered: usize,
    /// The old addresses it doesn't, each with the old file it was for.
    pub unanswered: Vec<(String, String)>,
    /// The addresses of the pages in the old built output that Statac
    /// didn't work out and the new site doesn't answer, in order.
    pub unworked: Vec<String>,
    /// The old addresses of the files given another name, each with the
    /// old file.
    pub files: Vec<(String, String)>,
    /// The old pages of tags that a page of their own now sends readers
    /// on from: each one's address, and the file written for it.
    pub sent_on: Vec<(String, String)>,
    /// The old home page's later pages that a page of their own now sends
    /// readers on from, and how many no page could.
    pub later: LaterPages,
    /// The old addresses of the photographs beside pages that the new
    /// site publishes only as copies.
    pub pictures: Vec<String>,
    /// The pages the old site made to list every tag, or every name of
    /// another list of that kind, that nothing answers: each one's
    /// address and its list's name.
    pub lists: Vec<(String, String)>,
    /// The old addresses the new site can't publish a redirect at, each
    /// with the old file of its page, where it has one, and why.
    pub unkept: Vec<(String, String, String)>,
}

impl Kept {
    /// How many old addresses nothing answers, the photographs published
    /// only as copies apart: they are told of on their own.
    pub(super) fn lost(&self) -> usize {
        self.unanswered.len() + self.unworked.len() + self.files.len() + self.lists.len()
    }
}

/// The folder under `content/` an import puts the old site's posts in.
const POSTS: &str = "posts";

/// The most patterns tried for one folder.
const TRIED: usize = 8;

/// How many of a folder's pages patterns are made from.
const SAMPLED: usize = 8;

/// Writes what keeps every page's old addresses, rewriting each page it
/// changes.
///
/// `others` are the site's languages but its main one, each of which has
/// a folder at the top of `content/`: Statac reads a `permalink` on a page
/// there from the folder's own address, so one is written without it.
pub(super) fn keep(
    out: &mut Out,
    pages: &mut [Page],
    today: Date,
    others: &[String],
) -> Result<Kept, Problem> {
    let mut kept = Kept::default();
    let wanted: HashMap<String, String> = pages
        .iter()
        .filter_map(|page| Some((page.to.clone(), settled(page.address.as_deref()?))))
        .collect();
    // A pattern goes only into a folder's page that is there already, the
    // highest first, so one line covers the most pages; a folder beneath
    // gets one of its own only where it puts more pages right. The posts'
    // folder has no page of its own, so one is written for it where a
    // pattern is kept.
    let first = landing(out.folder, today);
    let mut folders: Vec<String> = Vec::new();
    for page in pages.iter() {
        let mut above = page.to.as_str();
        while let Some((folder, _)) = above.rsplit_once('/') {
            if !folders.iter().any(|known| known == folder)
                && (out.text(&format!("content/{folder}/index.md")).is_some() || folder == POSTS)
            {
                folders.push(folder.to_owned());
            }
            above = folder;
        }
    }
    folders.sort_by_key(|folder| (folder.matches('/').count(), folder.clone()));
    let lists = folders_own(out.folder, today);
    let mut landed = first.clone();
    for folder in folders {
        let index = format!("{folder}/index.md");
        let beneath = |page: &&Page| {
            page.to.starts_with(&format!("{folder}/"))
                && page.to != index
                && !lists.contains(&page.to)
        };
        let held: Vec<(String, &String)> = pages
            .iter()
            .filter(beneath)
            .filter_map(|page| Some((page.to.clone(), wanted.get(&page.to)?)))
            .collect();
        let right = |landed: &Landed| {
            held.iter()
                .filter(|(to, old)| landed.get(to).map(|(at, _)| at) == Some(*old))
                .count()
        };
        let before_right = right(&landed);
        let before = out.text(&format!("content/{index}"));
        // A page whose old address was written on it says nothing of a
        // pattern where the rest have one, so those are left out first;
        // where the rest give none, every page is asked, since a site
        // can write on each page what one pattern would say.
        let mut asked = 0;
        'members: for with_own in [false, true] {
            let members: Vec<String> = pages
                .iter()
                .filter(beneath)
                .filter(|page| wanted.contains_key(&page.to))
                .filter(|page| with_own || !page.own_address)
                .map(|page| page.to.clone())
                .collect();
            if with_own && members.len() == asked {
                break;
            }
            asked = members.len();
            // With one page, any pattern fits by its own literal pieces,
            // and the page's own permalink says the same more plainly.
            if members.len() < 2 || before_right == members.len() {
                continue;
            }
            for (pattern, fits) in candidates(&members, &wanted, &first, &landed) {
                if fits < 2 || fits * 2 <= members.len() || fits <= before_right {
                    break;
                }
                let Some(written) = from_its_language(&index, &pattern, others) else {
                    continue;
                };
                set_pattern(out, pages, &index, before.as_deref(), &written)?;
                let after = landing(out.folder, today);
                let after_right = right(&after);
                if after_right > before_right && after_right >= 2 {
                    kept.patterns.push((folder.clone(), pattern, after_right));
                    landed = after;
                    break 'members;
                }
                unset_pattern(out, pages, &index, before.as_deref())?;
            }
        }
    }
    let landed = landing(out.folder, today);
    for page in pages.iter_mut() {
        if let Some(old) = wanted.get(&page.to)
            && landed.get(&page.to).map(|(at, _)| at) != Some(old)
        {
            let whole = page.address.clone().unwrap_or_default();
            match from_its_language(&page.to, &whole, others) {
                Some(written) => {
                    page.front
                        .push(Field::new("permalink", Value::Text(written), None));
                    kept.own += 1;
                    out.put(&format!("content/{}", page.to), page_text(page).as_bytes())?;
                }
                // An address outside its language's folder is one no
                // `permalink` there can say; the page sends readers on
                // from it.
                None => page.other_addresses.insert(0, whole),
            }
        }
    }
    // An old address the new site can't publish a redirect at would stop
    // the build, so each is looked for among what the site publishes,
    // every page at its kept address, before it is written.
    let mut taken = published(out.folder, today);
    for page in pages.iter_mut() {
        let own = page.address.as_deref().map(settled);
        let mut others: Vec<String> = Vec::new();
        let mut files: Vec<String> = Vec::new();
        for other in std::mem::take(&mut page.other_addresses) {
            if own.as_deref() == Some(settled(&other).as_str()) {
                page.other_addresses.push(other);
                continue;
            }
            let file = match Address::read(&other) {
                Ok(address) => crate::platform::folded(&address.file()),
                Err((fault, _)) => {
                    let why = crate::wording::address_fault(&fault);
                    kept.unkept.push((other, page.from.clone(), why));
                    continue;
                }
            };
            // The same address spelled another way, as with no `/` to
            // end it, is kept by the first.
            if files.contains(&file) {
                continue;
            }
            if let Some(holder) = taken.get(&file) {
                kept.unkept.push((other, page.from.clone(), holder.clone()));
                continue;
            }
            let holder = format!(
                "{} already redirects from there",
                super::report::code(&page.from)
            );
            taken.insert(file.clone(), holder);
            files.push(file);
            page.other_addresses.push(other.clone());
            others.push(other);
        }
        if !others.is_empty() {
            let list = others.into_iter().map(Value::Text).collect();
            page.front
                .push(Field::new("redirect_from", Value::List(list), None));
            kept.redirected += 1;
            out.put(&format!("content/{}", page.to), page_text(page).as_bytes())?;
        }
    }
    Ok(kept)
}

/// Each file the site in `folder` publishes, by its path in the built
/// site as a disk holds it, with why no redirect can stand there: what
/// is published there.
fn published(folder: &Path, today: Date) -> HashMap<String, String> {
    let (read, assets, feeds) = read_site(
        folder,
        &Run {
            today: Some(today),
            drafts: true,
            url: None,
            renamed: Vec::new(),
        },
    );
    assets
        .claims()
        .into_iter()
        .chain(read.claims())
        .chain(crate::feeds::claims(&feeds))
        .chain(read.site.pictures.claims())
        .map(|(file, named)| {
            let why = format!("{} is published there", crate::wording::named(&named));
            (crate::platform::folded(&file), why)
        })
        .collect()
}

/// Sends readers on from the old page of each tag and each series to the
/// page Statac makes for it, and gives each old address so answered with
/// the file written for it. `known` is each tag or series with an old
/// address a reader worked out.
///
/// The page Statac makes for a tag or a series is no file, so it can
/// carry no `redirect_from`. A page of the author's standing at the tag's address
/// could, but in a folder it brings that folder a list and a place in the
/// navigation. So each old address gets a page of its own at the top of
/// `content/`, holding its `permalink` and `redirect_to` and nothing
/// else, which is in no list.
pub(super) fn send_on_to_tags_and_series(
    out: &mut Out,
    today: Date,
    known: &[(Word, String, String)],
    others: &[String],
) -> Result<Vec<(String, String)>, Problem> {
    let mut sent_on = Vec::new();
    if known.is_empty() {
        return Ok(sent_on);
    }
    let (read, assets, feeds) = read_site(
        out.folder,
        &Run {
            today: Some(today),
            drafts: true,
            url: None,
            renamed: Vec::new(),
        },
    );
    let mut taken: HashSet<String> = HashSet::new();
    let mut list_pages: Vec<(Word, String, String)> = Vec::new();
    let claims = assets
        .claims()
        .into_iter()
        .chain(read.claims())
        .chain(crate::feeds::claims(&feeds))
        .chain(read.site.pictures.claims());
    for (file, named) in claims {
        if let Named::ListPage(word @ (Word::Tag | Word::Series), name) = &named {
            let address = match file.strip_suffix("index.html") {
                Some(above) => format!("/{above}"),
                None => format!("/{file}"),
            };
            list_pages.push((*word, name.clone(), address));
        }
        taken.insert(crate::platform::folded(&file));
    }
    for (word, name, old) in known {
        let language = others
            .iter()
            .find(|language| old.starts_with(&format!("/{language}/")));
        let in_language = |(_, _, address): &&(Word, String, String)| match language {
            Some(language) => address.starts_with(&format!("/{language}/")),
            None => !others
                .iter()
                .any(|l| address.starts_with(&format!("/{l}/"))),
        };
        let Some((_, _, to)) = list_pages
            .iter()
            .filter(|(w, n, _)| w == word && n == name)
            .find(in_language)
        else {
            continue;
        };
        let Ok(address) = Address::read(old) else {
            continue;
        };
        let file = crate::platform::folded(&address.file());
        if settled(old) == settled(to) || !taken.insert(file) {
            continue;
        }
        if let Some(written) = send_on(out, old, to, language.map(String::as_str))? {
            sent_on.push((old.clone(), written));
        }
    }
    Ok(sent_on)
}

/// Writes a page at the top of `content/`, or of `language`'s folder,
/// that stands at `old` and sends readers on to `to`, holding its
/// `permalink` and `redirect_to` and nothing else, so it is in no list.
/// Gives the file written, or none where `old` makes no file name.
fn send_on(
    out: &mut Out,
    old: &str,
    to: &str,
    language: Option<&str>,
) -> Result<Option<String>, Problem> {
    let within = match language {
        Some(language) => old.strip_prefix(&format!("/{language}")).unwrap_or(old),
        None => old,
    };
    let stem = within.trim_matches('/');
    let stem = stem.strip_suffix(".html").unwrap_or(stem).replace('/', "-");
    if stem.is_empty() || address::name_fault(&stem).is_some() {
        return Ok(None);
    }
    let folder = language.map(|l| format!("{l}/")).unwrap_or_default();
    let mut name = format!("{folder}{stem}.md");
    let mut number = 1;
    while out.text(&format!("content/{name}")).is_some() {
        number += 1;
        name = format!("{folder}{stem}-{number}.md");
    }
    let lines = [
        ("permalink", Value::Text(within.to_owned())),
        ("redirect_to", Value::Text(to.to_owned())),
    ];
    let front = super::yaml::fields(lines.iter().map(|(name, value)| (*name, value)));
    out.put(
        &format!("content/{name}"),
        format!("---\n{front}---\n").as_bytes(),
    )?;
    Ok(Some(format!("content/{name}")))
}

/// Sends readers on from each later page of the old home page to the page
/// of the posts' list with the same number, and gives each old address so
/// answered with the file written for it. An old page past the list's
/// last is left unanswered and counted.
///
/// A list's later pages are no files, so none can carry `redirect_from`;
/// each old address gets a page of its own, as [`send_on_to_tags_and_series`] writes.
pub(super) fn send_on_to_later_pages(
    out: &mut Out,
    today: Date,
    home: &HomePages,
) -> Result<LaterPages, Problem> {
    let mut sent_on = LaterPages::default();
    if home.addresses.is_empty() || home.per_page == 0 {
        return Ok(sent_on);
    }
    let (read, assets, feeds) = read_site(
        out.folder,
        &Run {
            today: Some(today),
            drafts: false,
            url: None,
            renamed: Vec::new(),
        },
    );
    let site = &read.site;
    let posts = ListKey::Folder(POSTS.to_owned());
    let Some((_, list)) = site.lists.iter().find(|(_, list)| list.key == posts) else {
        return Ok(sent_on);
    };
    if !list.page.is_some_and(|page| site.pages.get(page).lands()) {
        return Ok(sent_on);
    }
    let mut taken: HashSet<String> = assets
        .claims()
        .into_iter()
        .chain(read.claims())
        .chain(crate::feeds::claims(&feeds))
        .chain(site.pictures.claims())
        .map(|(file, _)| crate::platform::folded(&file))
        .collect();
    // The first of `addresses` is the old home page's second page.
    for (old, number) in home.addresses.iter().zip(2..) {
        let Some(to) = list.later_address(number).map(Address::as_str) else {
            sent_on.left += 1;
            continue;
        };
        let Ok(address) = Address::read(old) else {
            continue;
        };
        if settled(old) == to || !taken.insert(crate::platform::folded(&address.file())) {
            continue;
        }
        if let Some(written) = send_on(out, old, to, None)? {
            sent_on.pages.push((old.clone(), written));
        }
    }
    Ok(sent_on)
}

/// What [`send_on_to_later_pages`] did.
#[derive(Default, Debug)]
pub(super) struct LaterPages {
    /// Each old address now answered by the page of the same number, with
    /// the file written for it.
    pub pages: Vec<(String, String)>,
    /// How many old pages had a number past the last page of the posts'
    /// list.
    pub left: usize,
}

/// `address`, an address or a pattern from the top of the site, as a
/// `permalink` says it on the page at `to` under `content/`: without the
/// folder of the page's language, where the page is in one of `others`.
/// None where the address isn't beneath that folder.
pub(super) fn from_its_language(to: &str, address: &str, others: &[String]) -> Option<String> {
    let Some(language) = others
        .iter()
        .find(|language| to.starts_with(&format!("{language}/")))
    else {
        return Some(address.to_owned());
    };
    let rest = address.strip_prefix(&format!("/{language}"))?;
    rest.starts_with('/').then(|| rest.to_owned())
}

/// An address as Statac settles it, or as written where it can't.
fn settled(written: &str) -> String {
    match Address::read(written) {
        Ok(address) => address.as_str().to_owned(),
        Err(_) => written.to_owned(),
    }
}

/// The site in `folder` read as a build reads it. It has no templates yet,
/// and may have no `url`, so its feeds are those it will publish once it
/// has everything a feed needs: each a list asks for, but where a page
/// stands.
fn read_site(folder: &Path, run: &Run) -> (content::Read, Assets, Vec<Feed>) {
    let (read, assets, _) = build::read(folder, run);
    let site = &read.site;
    let landed: HashSet<String> = (site.pages.iter())
        .filter(|(_, page)| page.lands())
        .map(|(_, page)| page.address.key())
        .collect();
    let mut feeds = Vec::new();
    for (list, _) in site.lists.iter() {
        for address in crate::feeds::wanted(site, &read.settings, list) {
            let Some(template) = crate::feeds::template(&address, &|_| true) else {
                continue;
            };
            if !landed.contains(&address.key()) {
                feeds.push(Feed {
                    list,
                    address,
                    template,
                    placed: None,
                });
            }
        }
    }
    (read, assets, feeds)
}

/// Where each page of the site in `folder` lands, drafts among them, by
/// its path under `content/`, with its date.
fn landing(folder: &Path, today: Date) -> Landed {
    let read = content::read(
        folder,
        &Run {
            today: Some(today),
            drafts: true,
            url: None,
            renamed: Vec::new(),
        },
    );
    read.site
        .pages
        .iter()
        .filter(|(_, page)| page.source.is_some() && page.made.is_none())
        .map(|(_, page)| {
            let date = page.date.as_ref().map(|date| date.value);
            (page.path.clone(), (page.address.as_str().to_owned(), date))
        })
        .collect()
}

/// Where each page lands, by its path under `content/`, with its date.
type Landed = HashMap<String, (String, Option<Date>)>;

/// The path under `content/` of each folder's own page in the site in
/// `folder`: a pattern handed down never moves one.
fn folders_own(folder: &Path, today: Date) -> HashSet<String> {
    let read = content::read(
        folder,
        &Run {
            today: Some(today),
            drafts: true,
            url: None,
            renamed: Vec::new(),
        },
    );
    read.site
        .pages
        .iter()
        .filter(|(_, page)| page.place == PagePlace::Folder)
        .map(|(_, page)| page.path.clone())
        .collect()
}

/// The patterns that might make the pages in `members` land at their old
/// addresses, each with how many of them it does that for as Statac
/// fills a pattern in, the one that fits the most first. Each is made
/// from the old address of one of a few members that land elsewhere now,
/// so no one page's chance likeness, such as a day that is also its
/// month, decides for the rest.
fn candidates(
    members: &[String],
    wanted: &HashMap<String, String>,
    first: &Landed,
    landed: &Landed,
) -> Vec<(String, usize)> {
    let pieces = |to: &String| -> Option<(String, String, Option<Date>)> {
        let (at, date) = first.get(to)?;
        let slug = at.trim_end_matches('/').rsplit('/').next()?.to_owned();
        let file = to.rsplit('/').next()?;
        let name = file
            .rsplit_once('.')
            .map_or(file, |(stem, _)| stem)
            .to_owned();
        Some((slug, name, *date))
    };
    let wrong: Vec<&String> = members
        .iter()
        .filter(|to| {
            wanted.get(*to).map(String::as_str) != landed.get(*to).map(|(at, _)| at.as_str())
        })
        .collect();
    let step = wrong.len().div_ceil(SAMPLED).max(1);
    let mut patterns: Vec<String> = Vec::new();
    for to in wrong.iter().step_by(step) {
        let (Some(old), Some((slug, name, date))) = (wanted.get(*to), pieces(to)) else {
            continue;
        };
        for pattern in patterns_of(old, &slug, &name, date) {
            if !patterns.contains(&pattern) {
                patterns.push(pattern);
            }
        }
    }
    let mut scored: Vec<(String, usize)> = patterns
        .into_iter()
        .map(|pattern| {
            let fits = members
                .iter()
                .filter(|to| {
                    let (Some(old), Some((slug, name, date))) = (wanted.get(*to), pieces(to))
                    else {
                        return false;
                    };
                    let filled = address::expand(
                        &pattern,
                        &Pieces {
                            date: date.as_ref(),
                            slug: &slug,
                            folder: "",
                            name: &name,
                        },
                    );
                    filled.is_ok_and(|filled| &settled(&filled) == old)
                })
                .count();
            (pattern, fits)
        })
        .collect();
    scored.sort_by_key(|(_, fits)| std::cmp::Reverse(*fits));
    scored.truncate(TRIED);
    scored
}

/// The patterns one page's old address could be an instance of: each of
/// its pieces as written, or as the placeholder its slug, its file's name
/// or a part of its date fills alike.
fn patterns_of(old: &str, slug: &str, name: &str, date: Option<Date>) -> Vec<String> {
    let mut patterns = vec![String::new()];
    for segment in old.trim_start_matches('/').split_inclusive('/') {
        let (bare, slash) = match segment.strip_suffix('/') {
            Some(bare) => (bare, "/"),
            None => (segment, ""),
        };
        let mut options: Vec<String> = Vec::new();
        let (stem, ending) = match bare.rsplit_once('.') {
            Some((stem, ending)) => (stem, format!(".{ending}")),
            None => (bare, String::new()),
        };
        for (value, piece) in [(slug, "{slug}"), (name, "{name}")] {
            if stem == value {
                options.push(format!("{piece}{ending}"));
            }
        }
        if let Some(date) = date {
            let parts = [
                (format!("{:04}", date.year), "{year}"),
                (format!("{:02}", date.month), "{month}"),
                (format!("{:02}", date.day), "{day}"),
            ];
            for (value, piece) in parts {
                if bare == value {
                    options.push(piece.to_owned());
                }
            }
        }
        options.push(bare.to_owned());
        options.dedup();
        let mut next = Vec::new();
        for pattern in &patterns {
            for option in &options {
                if next.len() < TRIED * 4 {
                    next.push(format!("{pattern}{option}{slash}"));
                }
            }
        }
        patterns = next;
    }
    patterns
        .into_iter()
        .map(|pattern| format!("/{pattern}"))
        .filter(|pattern| pattern.contains('{'))
        .collect()
}

/// Writes `pattern` as `pages.permalink` in the folder's `index.md`: into
/// the page that came across where it is one, else into the file there.
fn set_pattern(
    out: &mut Out,
    pages: &mut [Page],
    index: &str,
    before: Option<&str>,
    pattern: &str,
) -> Result<(), Problem> {
    let line = Field::new("pages.permalink", Value::Text(pattern.to_owned()), None);
    if let Some(page) = pages.iter_mut().find(|page| page.to == index) {
        page.front.retain(|field| field.name != "pages.permalink");
        page.front.push(line);
        return out.put(&format!("content/{index}"), page_text(page).as_bytes());
    }
    let written = super::yaml::fields([(line.name.as_str(), &line.value)]);
    let text = match before.and_then(|text| {
        let rest = text.strip_prefix("---\n")?;
        let end = rest.find("\n---\n")?;
        Some(format!(
            "---\n{}\n{written}---\n{}",
            &rest[..end],
            &rest[end + 5..]
        ))
    }) {
        Some(text) => text,
        None => format!("---\n{written}---\n{}", before.unwrap_or_default()),
    };
    out.put(&format!("content/{index}"), text.as_bytes())
}

/// Takes a pattern back out, leaving the folder's `index.md` as it was.
fn unset_pattern(
    out: &mut Out,
    pages: &mut [Page],
    index: &str,
    before: Option<&str>,
) -> Result<(), Problem> {
    let path = format!("content/{index}");
    if let Some(page) = pages.iter_mut().find(|page| page.to == index) {
        page.front.retain(|field| field.name != "pages.permalink");
        return out.put(&path, page_text(page).as_bytes());
    }
    match before {
        Some(text) => out.put(&path, text.as_bytes()),
        None => out.remove(&path),
    }
}

/// Every `.html` file in `folder` and the folders in it, by its path
/// there, in order. No link is followed.
fn pages_in(folder: &Path) -> Vec<String> {
    let mut pages = Vec::new();
    let mut waiting = vec![String::new()];
    while let Some(within) = waiting.pop() {
        let Ok(entries) = std::fs::read_dir(crate::platform::join(folder, &within)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{within}{name}");
            match std::fs::symlink_metadata(entry.path()) {
                Ok(found) if found.is_dir() => waiting.push(format!("{path}/")),
                Ok(found) if found.is_file() && name.ends_with(".html") => pages.push(path),
                _ => {}
            }
        }
    }
    pages.sort();
    pages
}

/// What reading the new site as a build does found.
pub(super) struct Checked {
    pub answered: usize,
    pub unanswered: Vec<(String, String)>,
    /// Those of `old` the new site can't publish a redirect at, each with
    /// the old file it was for and why.
    pub unkept: Vec<(String, String, String)>,
    /// The old feeds' addresses the new site publishes nothing at, each
    /// with the old file it was made from.
    pub feeds: Vec<(String, String)>,
    /// Those of `lists` the new site publishes nothing at.
    pub lists: Vec<(String, String)>,
    /// The address of each photograph beside a page that the new site
    /// publishes only as copies, in order.
    pub pictures: Vec<String>,
    /// The addresses of the pages in the old built output that are none
    /// of `old` and that the new site doesn't answer, in order.
    pub unworked: Vec<String>,
    /// Each error `statac check` finds, by its file, its line and its
    /// headline.
    pub errors: Vec<(String, Option<usize>, String)>,
    /// Each template the site's pages and feeds are built with that it
    /// doesn't have, by its path, in order.
    pub templates: Vec<String>,
    /// Whether the site asks for a feed and has no `url`, which every feed
    /// needs.
    pub feeds_need_url: bool,
}

/// Reads the site in `folder` as a build of `today` does, looks for each
/// old address, each address in `feeds` and in `lists`, and each page in
/// `built`, the
/// old site's built output where it is there, among what it publishes and
/// sends on from, and checks it as `statac check` does. The site has no
/// templates yet, so a missing one is told of in [`Checked::templates`]
/// and not among the errors.
pub(super) fn check(
    folder: &Path,
    today: Date,
    old: &[(String, String)],
    feeds: &[(String, String)],
    lists: &[(String, String)],
    built: Option<&Path>,
) -> Checked {
    let (read, assets, written) = read_site(
        folder,
        &Run {
            today: Some(today),
            drafts: false,
            url: None,
            renamed: Vec::new(),
        },
    );
    let mut published: HashSet<String> = HashSet::new();
    let claims = assets
        .claims()
        .into_iter()
        .chain(read.claims())
        .chain(crate::feeds::claims(&written))
        .chain(read.site.pictures.claims());
    for (file, _) in claims {
        published.insert(crate::platform::folded(&file));
    }
    let mut checked = Checked {
        answered: 0,
        unanswered: Vec::new(),
        unkept: Vec::new(),
        feeds: Vec::new(),
        lists: Vec::new(),
        pictures: Vec::new(),
        unworked: Vec::new(),
        errors: Vec::new(),
        templates: Vec::new(),
        feeds_need_url: !written.is_empty() && read.settings.text("url").is_none(),
    };
    let answered = |address: &str| {
        Address::read(address)
            .is_ok_and(|address| published.contains(&crate::platform::folded(&address.file())))
    };
    for (address, from) in old {
        if let Err((fault, _)) = Address::read(address) {
            let why = crate::wording::address_fault(&fault);
            checked.unkept.push((address.clone(), from.clone(), why));
            continue;
        }
        match answered(address) {
            true => checked.answered += 1,
            false => checked.unanswered.push((address.clone(), from.clone())),
        }
    }
    for (address, from) in feeds {
        if !answered(address) {
            checked.feeds.push((address.clone(), from.clone()));
        }
    }
    for (address, list) in lists {
        if !answered(address) {
            checked.lists.push((address.clone(), list.clone()));
        }
    }
    // An address Statac can't publish at is still one it worked out, at
    // the file the tool wrote for it.
    let worked_out: HashSet<String> = old
        .iter()
        .chain(lists)
        .map(|(address, _)| match Address::read(address) {
            Ok(address) => crate::platform::folded(&address.file()),
            Err(_) => crate::platform::folded(&built_file(address)),
        })
        .collect();
    for page in built.map(pages_in).unwrap_or_default() {
        let file = crate::platform::folded(&page);
        if !published.contains(&file) && !worked_out.contains(&file) {
            let address = match page.strip_suffix("index.html") {
                Some(above) if above.is_empty() || above.ends_with('/') => format!("/{above}"),
                _ => format!("/{page}"),
            };
            checked.unworked.push(address);
        }
    }
    // Reading the site finds less than building its pages does: only the
    // whole of `statac check` tells whether it builds.
    let outcome = build::run(&Options {
        mode: Mode::Check,
        site: folder.to_path_buf(),
        out: None,
        today: Some(today),
        drafts: false,
        url: None,
    });
    checked.pictures = (outcome.held_back.iter())
        .map(|file| format!("/{file}"))
        .collect();
    let sources = outcome.sources();
    for problem in &outcome.problems {
        let template = match &problem.kind {
            Kind::MissingTemplate { path, .. } => path,
            // A JSON feed is built with `templates/feed.json`, and any other
            // with the last looked for, `templates/feed.xml`.
            Kind::FeedNotWritten {
                why: Unwritten::NoTemplate(paths),
                ..
            } => match (paths.iter())
                .find(|path| *path == "templates/feed.json")
                .or(paths.last())
            {
                Some(path) => path,
                None => continue,
            },
            _ => continue,
        };
        if !checked.templates.contains(template) {
            checked.templates.push(template.clone());
        }
    }
    checked.templates.sort();
    for problem in outcome
        .problems
        .iter()
        .filter(|p| p.kind.severity() == Severity::Error)
        .filter(|p| !matches!(p.kind, Kind::MissingTemplate { .. }))
    {
        let (file, line) = match problem.place {
            Some(place) => (
                sources
                    .path(place.source)
                    .to_string_lossy()
                    .replace('\\', "/"),
                Some(sources.line_number(place.source, place.span.start as usize)),
            ),
            None => (String::new(), None),
        };
        checked.errors.push((file, line, problem.headline()));
    }
    checked
}
