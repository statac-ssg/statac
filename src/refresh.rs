//! `statac refresh`: fetching into `fetched/` what the site shows from
//! elsewhere, which `build` never fetches. The site is built in memory,
//! drafts and posts waiting for their day with it, to learn what it asks
//! for and lacks: the still of each video a page reads one for. Where the
//! site stops only with them, it is built as `build` publishes it. Where
//! it stops only on values it has none of, as a layout reading a missing
//! still unguarded does, the stills are fetched first and the site read
//! again.
//!
//! Each file is written as it arrives, and the record once every fetch is
//! done, whatever became of each. A fetch the other site answered it has
//! nothing for is recorded `absent` and not made again.
//!
//! On a site with `webmentions: true` and a `url`, the webmention relay is
//! then asked for what it received since, with the token the command line
//! takes from the environment or from the site's `.statac/token`.
//!
//! First, each mention `refuse_webmentions_from` refuses is removed and
//! recorded `absent`, so it is never fetched again, with each face no
//! mention kept names.

mod mentions;

use std::collections::{BTreeSet, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::build::{self, Mode};
use crate::config::refused::Refused;
use crate::date::Date;
use crate::fetch::{Fetched, Refusal};
use crate::fetched::stills;
use crate::fetched::{self as kept, Held, Line, Record};
use crate::problem::Host;
use crate::problem::{Kind, Problem, Reason, Severity};
use crate::reads::Key;
use crate::report::{Report, shown};
use crate::wording;

/// What `statac refresh` is asked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    /// The site's folder.
    pub site: PathBuf,
    /// The day what is fetched is recorded as fetched on; today in the
    /// site's zone where none is given.
    pub today: Option<Date>,
    /// What the webmention relay gave the author to read the site's
    /// mentions with, where it was given.
    pub token: Option<Token>,
}

/// The environment variable the command line reads the relay's token from.
pub(crate) const TOKEN_VARIABLE: &str = "STATAC_WEBMENTION_TOKEN";

/// The file in a site the relay's token is read from where the variable
/// isn't set: kept out of the site's history by its `.gitignore`.
pub(crate) const TOKEN_FILE: &str = ".statac/token";

/// The relay's token: `variable`, the value of [`TOKEN_VARIABLE`], where
/// it is set, and else the one line of [`TOKEN_FILE`] in the site at
/// `site`, spaces and line breaks around it taken off. With it, what to
/// say of a file that is there and isn't used: one that is empty, holds
/// more than one line, is a link or can't be read.
pub(crate) fn token(site: &Path, variable: Option<String>) -> (Option<Token>, Option<String>) {
    if let Some(token) = variable.and_then(Token::new) {
        return (Some(token), None);
    }
    let unused = |reason: &str| (None, Some(wording::token_unused(reason)));
    let Ok(root) = std::fs::canonicalize(site) else {
        return (None, None);
    };
    let bytes = match crate::platform::read_file(&root, TOKEN_FILE) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return (None, None),
        Err(error) => {
            let linked = [".statac", TOKEN_FILE].into_iter().find(|rel| {
                let found = std::fs::symlink_metadata(crate::platform::join(&root, rel));
                found.is_ok_and(|found| found.is_symlink())
            });
            return match linked {
                Some(rel) => unused(&wording::token_file_linked(rel)),
                None => unused(&Reason::of(&error).to_string()),
            };
        }
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return unused(&Reason::NotUtf8.to_string());
    };
    let text = text.trim();
    if text.is_empty() {
        return unused(wording::TOKEN_FILE_EMPTY);
    }
    if text.contains(['\n', '\r']) {
        return unused(wording::TOKEN_FILE_LINES);
    }
    (Token::new(text.to_owned()), None)
}

/// The relay's token, which is never shown: drawn for a programmer, it is
/// dots.
#[derive(Clone, PartialEq, Eq)]
pub struct Token(String);

impl Token {
    /// The token `text`, where there is one.
    pub fn new(text: String) -> Option<Token> {
        (!text.is_empty()).then_some(Token(text))
    }

    fn text(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Token(•••)")
    }
}

/// The most bytes one answer may hold.
pub(crate) const LIMIT: u64 = 20_000_000;

/// What fetches an address within a limit: [`crate::fetch::fetch`], or
/// whatever a test stands in with.
pub trait Fetch {
    fn fetch(&self, address: &str, limit: u64) -> Result<Fetched, Refusal>;
}

impl<F: Fn(&str, u64) -> Result<Fetched, Refusal>> Fetch for F {
    fn fetch(&self, address: &str, limit: u64) -> Result<Fetched, Refusal> {
        self(address, limit)
    }
}

/// Why a still wasn't brought back.
enum Unfetched {
    /// The video site answered it has none.
    Absent,
    /// Anything else, as the reason is worded.
    Failed(String),
}

/// Fetches into the site `options` names, with `fetcher`, each still a
/// page reads that it lacks, and the mentions the relay has that it
/// lacks, and writes them and the record, saying each on `out` as it
/// goes. Gives the exit code: 1 where the site has an error, where a still
/// couldn't be fetched for any reason but the video site having none,
/// where the relay couldn't be asked, where what was fetched couldn't be
/// kept, or where what is refused couldn't be removed.
pub fn run(options: &Options, fetcher: &dyn Fetch, out: &mut dyn Write, colour: bool) -> u8 {
    let mut say = |line: &str| {
        let _ = out.write_all(line.as_bytes());
        let _ = out.flush();
    };
    refresh(options, fetcher, &mut say, colour, true)
}

/// [`run`], as `serve` fetches: each line is handed to `say` as it is
/// said, and nothing is said where there is nothing to fetch.
pub(crate) fn fetch_new(
    options: &Options,
    fetcher: &dyn Fetch,
    say: &mut dyn FnMut(&str),
    colour: bool,
) -> u8 {
    refresh(options, fetcher, say, colour, false)
}

/// Whether the site built as `outcome` has something to fetch: the still
/// of a video a page reads one for, or webmentions.
pub(crate) fn wanted(outcome: &build::Outcome) -> bool {
    let lacks = |problem: &Problem| matches!(problem.kind, Kind::NoThumbnail { absent: false, .. });
    let webmentions = outcome
        .settings()
        .and_then(|settings| settings.truth("webmentions"));
    outcome.problems.iter().any(lacks) || webmentions == Some(true)
}

/// [`run`], saying where there is nothing to fetch only where
/// `nothing_said`.
fn refresh(
    options: &Options,
    fetcher: &dyn Fetch,
    say: &mut dyn FnMut(&str),
    colour: bool,
    nothing_said: bool,
) -> u8 {
    let read = |drafts| {
        build::run(&build::Options {
            mode: Mode::Check,
            site: options.site.clone(),
            out: None,
            today: options.today,
            drafts,
            url: None,
        })
    };
    // Where only what isn't published yet stops the site, what is
    // published still gets its stills.
    let settle = || {
        let outcome = read(true);
        if outcome.stopped() {
            let published = read(false);
            if !published.stopped() {
                let left_out = (outcome.problems.iter())
                    .filter(|problem| problem.kind.severity() == Severity::Error)
                    .count();
                return (published, left_out);
            }
        }
        (outcome, 0)
    };
    let (mut outcome, mut left_out) = settle();
    // A template reading a still the site lacks stops it on the value it
    // has none of, so where only such values stop the site, the stills it
    // asks for are fetched first and the site read again.
    let mut tried: Vec<String> = Vec::new();
    let mut code = 0;
    let errors =
        || (outcome.problems.iter()).filter(|problem| problem.kind.severity() == Severity::Error);
    if outcome.stopped()
        && errors().all(|problem| problem.kind.code() == crate::problem::Code::NoValue)
        && let (Some(site), Some(today)) = (outcome.site(), outcome.today())
        && let missing = missing_stills(&outcome.problems)
        && !missing.is_empty()
    {
        let report = Report::new(outcome.sources(), &options.site, colour);
        let folder = options.site.join(kept::FOLDER);
        let mut record = site.fetched.record.clone();
        let mut standing = standing_of(site);
        let here = stills_here(&outcome, site);
        let keeping = (folder.as_path(), &mut record, &mut standing, today);
        let (fetched, failed) = fetch_stills(&missing, fetcher, keeping, &report, say);
        if let Err(error) = keep_record(&folder, &site.fetched.record, &record, &standing) {
            say(&unwritten(&report, &kept::record_shown(), &error));
            return 1;
        }
        say(&format!("{}\n", wording::refreshed(fetched, here, failed)));
        code = u8::from(failed > 0);
        tried = missing.into_iter().map(|(path, ..)| path).collect();
        (outcome, left_out) = settle();
    }
    let report = Report::new(outcome.sources(), &options.site, colour);
    let (Some(site), Some(today), false) = (outcome.site(), outcome.today(), outcome.stopped())
    else {
        say(&report.check(&outcome.problems, &outcome.quiet));
        return 1;
    };
    let mut missing = missing_stills(&outcome.problems);
    missing.retain(|(path, ..)| !tried.contains(path));
    let nothing_said = nothing_said && tried.is_empty();
    let drafts_left_out = |say: &mut dyn FnMut(&str)| {
        if left_out > 0 {
            say(&format!(
                "{}\n",
                shown(&wording::drafts_not_refreshed(left_out))
            ));
        }
    };
    let settings = outcome.settings().expect("read with the site");
    let url = (settings.text("url")).map(|url| url.value.to_owned());
    let webmentions = settings.truth("webmentions") == Some(true);
    let waiting = |problem: &&Problem| {
        problem.kind
            == Kind::NoSiteUrl {
                waiting: crate::problem::Waiting::Webmentions,
            }
    };
    if webmentions && let Some(problem) = outcome.problems.iter().find(waiting) {
        say(&report.problem(problem));
    }
    let url = url.filter(|_| webmentions);
    let folder = options.site.join(kept::FOLDER);
    let mut record = site.fetched.record.clone();
    let mut standing = standing_of(site);
    code |= remove_refused(
        &folder,
        &site.mentions,
        (&mut record, &mut standing, today),
        &report,
        say,
    );
    if missing.is_empty() && url.is_none() {
        if let Err(error) = keep_record(&folder, &site.fetched.record, &record, &standing) {
            say(&unwritten(&report, &kept::record_shown(), &error));
            code = 1;
        }
        if nothing_said {
            say(&format!("{}\n", wording::NOTHING_TO_FETCH));
            drafts_left_out(say);
        }
        return code;
    }
    let stills_missing = !missing.is_empty();
    let here = stills_here(&outcome, site);
    let keeping = (folder.as_path(), &mut record, &mut standing, today);
    let (fetched, failed) = fetch_stills(&missing, fetcher, keeping, &report, say);
    code |= u8::from(failed > 0);
    if stills_missing {
        say(&format!("{}\n", wording::refreshed(fetched, here, failed)));
    }
    let mut asked = None;
    if let Some(url) = &url {
        match &options.token {
            None => say(&format!("{}\n", wording::NO_TOKEN)),
            Some(token) => {
                let mut keeping = mentions::Keeping {
                    folder: &folder,
                    record: &mut record,
                    standing: &mut standing,
                    today,
                    url,
                    refused: &Refused::of(settings),
                    report: &report,
                };
                let answer = mentions::ask(fetcher, token, &mut keeping, say);
                code |= u8::from(answer.failed);
                asked = Some(answer);
            }
        }
    }
    if let Err(error) = keep_record(&folder, &site.fetched.record, &record, &standing) {
        say(&unwritten(&report, &kept::record_shown(), &error));
        code = 1;
    }
    if let Some(asked) = asked.filter(|asked| asked.answered) {
        say(&format!(
            "{}\n",
            wording::mentions_refreshed(asked.fetched, asked.faces, asked.not_kept)
        ));
    }
    drafts_left_out(say);
    code
}

/// The stills the site's pages read and the site lacks, each by its path
/// in `fetched/`, and not yet answered as absent.
fn missing_stills(problems: &[Problem]) -> Vec<(String, Host, String)> {
    let mut missing: Vec<(String, Host, String)> = (problems.iter())
        .filter_map(|problem| match &problem.kind {
            Kind::NoThumbnail {
                host,
                id,
                absent: false,
            } => Some((host.still(id), *host, id.clone())),
            _ => None,
        })
        .collect();
    missing.sort_unstable();
    missing.dedup();
    missing
}

/// The paths of the files in `fetched/` that are there.
fn standing_of(site: &crate::site::Site) -> HashSet<String> {
    (site.fetched.files.iter())
        .map(|(path, _)| path.clone())
        .collect()
}

/// How many stills the site built as `outcome` read that it already has.
fn stills_here(outcome: &build::Outcome, site: &crate::site::Site) -> usize {
    outcome
        .record
        .keys()
        .filter(|key| match key {
            Key::Picture(id) => {
                let picture = site.pictures.get(*id);
                picture.root == crate::album::Root::Fetched
                    && picture.path.starts_with(&format!("{}/", stills::FOLDER))
            }
            _ => false,
        })
        .count()
}

/// Fetches each of `missing` into `folder`, recording what became of it,
/// and says each: how many were fetched, and how many couldn't be.
fn fetch_stills(
    missing: &[(String, Host, String)],
    fetcher: &dyn Fetch,
    (folder, record, standing, today): (&Path, &mut Record, &mut HashSet<String>, Date),
    report: &Report,
    say: &mut dyn FnMut(&str),
) -> (usize, usize) {
    let (mut fetched, mut failed) = (0, 0);
    for (_, host, id) in missing {
        let (host, id) = (*host, id.as_str());
        let path = stills::path(host, id);
        let file = format!("{}/{path}", kept::FOLDER);
        match still(fetcher, host, id) {
            Ok(bytes) => match keep(folder, &path, &bytes) {
                Ok(()) => {
                    let held = Held::Hash(kept::hash_of(&bytes));
                    record.set(&path, Line { held, day: today });
                    standing.insert(path);
                    fetched += 1;
                    say(&format!("{}\n", shown(&wording::fetched_file(&file))));
                }
                Err(error) => {
                    failed += 1;
                    say(&unwritten(report, &file, &error));
                }
            },
            Err(Unfetched::Absent) => {
                let held = Held::Absent;
                record.set(&path, Line { held, day: today });
                say(&format!("{}\n", wording::no_still_there(host.site(), id)));
            }
            Err(Unfetched::Failed(reason)) => {
                failed += 1;
                let line = wording::cant_fetch_still(id, &reason);
                say(&format!("{}\n", shown(&line)));
            }
        }
    }
    (fetched, failed)
}

/// Removes the file of each mention `refuse_webmentions_from` refuses,
/// recording it `absent` so it is never fetched again, and each face no
/// mention kept names, saying each on `say`. Each file is asked what it is
/// before any is removed, so a link in a mention's place leaves every one
/// where it is. Gives 1 where something couldn't be removed.
fn remove_refused(
    folder: &Path,
    mentions: &crate::fetched::mentions::Mentions,
    (record, standing, today): (&mut Record, &mut HashSet<String>, Date),
    report: &Report,
    say: &mut dyn FnMut(&str),
) -> u8 {
    if mentions.refused.is_empty() {
        return 0;
    }
    let faces: BTreeSet<String> = (mentions.refused.iter())
        .filter_map(|(_, mention)| mention.face.as_deref())
        .map(|face| format!("{}/{face}", crate::fetched::mentions::FACES))
        .filter(|face| !mentions.faces_named.contains(face) && standing.contains(face))
        .collect();
    let paths = (mentions.refused.iter()).map(|(path, _)| path);
    let files: Vec<&String> = paths.chain(&faces).collect();
    let unremoved = |path: &str, error: &std::io::Error| {
        report.problem(&Problem::nowhere(Kind::CantRemove {
            path: format!("{}/{path}", kept::FOLDER).into(),
            reason: Reason::of(error),
        }))
    };
    let links: Vec<String> = (files.iter())
        .filter_map(|path| {
            let error = crate::platform::no_link_to(folder, path).err()?;
            Some(unremoved(path, &error))
        })
        .collect();
    if !links.is_empty() {
        links.iter().for_each(|told| say(told));
        return 1;
    }
    let mut code = 0;
    let mut remove =
        |path: &str, say: &mut dyn FnMut(&str)| match crate::platform::remove_file(folder, path) {
            Ok(()) => {
                let file = format!("{}/{path}", kept::FOLDER);
                say(&format!("{}\n", shown(&wording::removed_file(&file))));
                true
            }
            Err(error) => {
                say(&unremoved(path, &error));
                code = 1;
                false
            }
        };
    let mut removed = 0;
    for (path, _) in &mentions.refused {
        if remove(path, say) {
            standing.remove(path);
            let held = Held::Absent;
            record.set(path, Line { held, day: today });
            removed += 1;
        }
    }
    // A mention left where it was still names its face.
    let left: BTreeSet<String> = (mentions.refused.iter())
        .filter(|(path, _)| standing.contains(path))
        .filter_map(|(_, mention)| mention.face.as_deref())
        .map(|face| format!("{}/{face}", crate::fetched::mentions::FACES))
        .collect();
    let mut faces_removed = 0;
    for face in faces.difference(&left) {
        if remove(face, say) {
            standing.remove(face);
            faces_removed += 1;
        }
    }
    if removed + faces_removed > 0 {
        say(&format!(
            "{}\n",
            wording::removed_refused(removed, faces_removed)
        ));
    }
    code
}

/// The problem with a file of `fetched/` that couldn't be written, drawn.
pub(crate) fn unwritten(report: &Report, file: &str, error: &std::io::Error) -> String {
    report.problem(&Problem::nowhere(Kind::CantWrite {
        path: file.into(),
        reason: Reason::of(error),
    }))
}

/// The still of the video `id` on `host`, as the video site gives it.
fn still(fetcher: &dyn Fetch, host: Host, id: &str) -> Result<Vec<u8>, Unfetched> {
    match host {
        Host::YouTube => {
            let largest = format!("https://i.ytimg.com/vi/{id}/maxresdefault.jpg");
            match fetcher.fetch(&largest, LIMIT) {
                Err(Refusal::Missing { .. }) => {
                    let smaller = format!("https://i.ytimg.com/vi/{id}/hqdefault.jpg");
                    picture(fetcher.fetch(&smaller, LIMIT), &smaller)
                }
                answer => picture(answer, &largest),
            }
        }
        Host::Vimeo => {
            let asked = format!("https://vimeo.com/api/v2/video/{id}.json");
            let answer = fetcher
                .fetch(&asked, LIMIT)
                .map_err(|refusal| unfetched(&asked, refusal))?;
            let Some(named) = named_still(&answer.bytes) else {
                return Err(Unfetched::Failed(wording::NO_PICTURE_NAMED.into()));
            };
            picture(fetcher.fetch(&named, LIMIT), &named)
        }
    }
}

fn unfetched(address: &str, refusal: Refusal) -> Unfetched {
    match refusal {
        Refusal::Missing { .. } => Unfetched::Absent,
        refusal => Unfetched::Failed(wording::refusal(address, &refusal)),
    }
}

/// The bytes of `answer` to `address` where the picture reader takes them
/// as a photograph, a JPEG, a PNG or a still WebP, within its limits.
fn picture(answer: Result<Fetched, Refusal>, address: &str) -> Result<Vec<u8>, Unfetched> {
    let answer = answer.map_err(|refusal| unfetched(address, refusal))?;
    match crate::album::quietly(|| crate::pictures::open(&answer.bytes)) {
        Ok(_) => Ok(answer.bytes),
        Err(_) => Err(Unfetched::Failed(wording::NOT_A_PICTURE.into())),
    }
}

/// The address of the picture Vimeo's answer about a video names: a JSON
/// list of one object, whose `thumbnail_large` is the address.
fn named_still(bytes: &[u8]) -> Option<String> {
    use crate::yaml::Value;
    let text = std::str::from_utf8(bytes).ok()?;
    let mut sources = crate::source::Sources::default();
    let source = sources.add("", text.to_owned());
    let read = crate::yaml::read_json(&sources, source).ok()?;
    let Value::Sequence(items) = &read.value else {
        return None;
    };
    let [item] = &items[..] else {
        return None;
    };
    let Value::Mapping(entries) = &item.value else {
        return None;
    };
    entries
        .iter()
        .find_map(|entry| match (&entry.key.value, &entry.value) {
            (Value::Scalar(key), value) if key == "thumbnail_large" => match &value.value {
                // A string, which JSON quotes; `null` is none.
                Value::Scalar(address)
                    if value.style == crate::origin::Style::DoubleQuoted && !address.is_empty() =>
                {
                    Some(address.clone())
                }
                _ => None,
            },
            _ => None,
        })
}

/// Writes `bytes` as `path` under the folder `fetched`, made where it
/// isn't there yet. Nothing is written through a link, nor over anything
/// but a file.
pub(crate) fn keep(fetched: &Path, path: &str, bytes: &[u8]) -> std::io::Result<()> {
    let file = crate::platform::join(fetched, path);
    let (Some(folder), Some(name)) = (file.parent(), file.file_name()) else {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    };
    let name = name.to_string_lossy();
    crate::platform::no_link_to(fetched, path)?;
    std::fs::create_dir_all(folder)?;
    // Made just now, a folder on the way may have been put there as a
    // link since.
    crate::platform::no_link_to(fetched, path)?;
    crate::platform::keep_file(fetched, path, &crate::platform::beside(&name), bytes)
}

/// Writes `record`, without the lines of files not `standing`, where it
/// says other than `was` did.
fn keep_record(
    fetched: &Path,
    was: &Record,
    record: &Record,
    standing: &HashSet<String>,
) -> std::io::Result<()> {
    let text = record.text(|path| standing.contains(path));
    if text == was.text(|_| true) {
        return Ok(());
    }
    keep(fetched, kept::RECORD, text.as_bytes())
}

#[cfg(test)]
mod tests;
