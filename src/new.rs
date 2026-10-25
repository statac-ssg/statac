//! `statac new`: five questions, and a new site's settings written from
//! the answers into an empty folder or a new one.
//!
//! A new site is `statac.yaml`, `.gitignore`, an empty `content/` and, for
//! a language Statac has only some of its words for, those words in
//! English in `languages/`. Statac writes no templates into it.
//!
//! Everything `new` learns from the machine comes in a [`Machine`], so
//! that a run can be repeated exactly. [`make`] writes a site from
//! [`Answers`] and says nothing.

#[cfg(test)]
mod tests;

use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::config;
use crate::date::Date;
use crate::dates::format::DateWords;
use crate::language::Languages;
use crate::origin::{Origin, Sourced};
use crate::platform;
use crate::problem::{Kind, Maker, Problem, Reason, SiteMark, WordsFrom};
use crate::report::{self, Report, shown};
use crate::source::Sources;
use crate::wording;
use crate::yaml;

/// What `statac new` is asked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    /// The folder the site is made in; the folder Statac is run in where
    /// none is given.
    pub folder: Option<PathBuf>,
    /// Whether every default is taken without asking.
    pub yes: bool,
}

/// What `statac new` learns from the machine it runs on.
pub struct Machine<'a> {
    /// Where the answers are read from; none where every default is taken.
    pub input: Option<Input<'a>>,
    pub locale: Locale,
    /// The machine's time zone, by its name in the time zone database.
    pub timezone: Option<String>,
}

/// Where answers are typed.
pub struct Input<'a> {
    pub reader: &'a mut dyn BufRead,
    /// Whether a person is there to answer. Where none is, each line read
    /// is the answer to the next question, and no question is shown.
    pub terminal: bool,
}

/// The machine's locale, as its environment gives it.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Locale {
    pub lc_all: Option<String>,
    pub lc_messages: Option<String>,
    pub lang: Option<String>,
}

impl Locale {
    /// This machine's, from its environment.
    pub fn of_this_machine() -> Locale {
        let read = |name| std::env::var(name).ok();
        Locale {
            lc_all: read("LC_ALL"),
            lc_messages: read("LC_MESSAGES"),
            lang: read("LANG"),
        }
    }

    /// The language the locale names, as a code: `en_GB.UTF-8` is
    /// `en-GB`. `C` and `POSIX` name none, and neither does a code
    /// `statac.yaml` couldn't hold.
    fn language(&self) -> Option<String> {
        [&self.lc_all, &self.lc_messages, &self.lang]
            .into_iter()
            .flatten()
            .find_map(|written| {
                let code = written.split(['.', '@']).next().unwrap_or_default();
                let code = code.replace('_', "-");
                let named = !code.is_empty() && code != "C" && code != "POSIX";
                (named && a_language_code(&code)).then_some(code)
            })
    }
}

/// This machine's time zone, by its name, where it has one.
pub fn zone_of_this_machine() -> Option<String> {
    let zone = jiff::tz::TimeZone::try_system().ok()?;
    zone.iana_name().map(str::to_owned)
}

/// Today on this machine's clock, in its zone.
pub fn today_on_this_machine() -> Date {
    let now = jiff::Zoned::now();
    Date::day_only(
        i32::from(now.year()),
        now.month().unsigned_abs(),
        now.day().unsigned_abs(),
    )
}

/// The answers a site's settings are made from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Answers {
    pub title: String,
    /// Whose site it is, where that is given.
    pub author: Option<String>,
    /// The site's address, `http` or `https` with a host, and no slash at
    /// its end.
    pub address: Option<String>,
    /// Codes `statac.yaml` takes, the main language first, each once.
    pub languages: Vec<String>,
    /// The zone the site's times are in, by its name in the time zone
    /// database.
    pub timezone: Option<String>,
}

/// What [`make`] wrote.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Made {
    /// How many files.
    pub files: usize,
    /// `statac.yaml` as written.
    pub settings: String,
}

/// Asks for a new site and makes it, saying what it did on `out`. Gives
/// the exit code: 0 where the site was made, 1 where it wasn't.
pub fn run(options: &Options, machine: Machine, out: &mut dyn Write, colour: bool) -> u8 {
    let mut say = |text: &str| {
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    };
    let folder = options.folder.clone().unwrap_or_else(|| PathBuf::from("."));
    let shown_as = match &options.folder {
        Some(folder) => folder.to_string_lossy().into_owned(),
        None => own_name(&folder),
    };
    let sources = Sources::default();
    let report = Report::new(&sources, &folder, colour);
    if let Err(problem) = refusal(&folder, &shown_as, Maker::New) {
        say(&report.problem(&problem));
        return 1;
    }
    let defaults = Defaults::of(&shown_as, &machine.locale, machine.timezone.as_deref());
    let Machine { input, .. } = machine;
    let answers = match input.filter(|_| !options.yes) {
        Some(input) => {
            if input.terminal {
                say(&format!(
                    "{}\n{}\n\n",
                    shown(&wording::new_site_in(&shown_as)),
                    wording::ASK_ENTER
                ));
            }
            let asking = Asking {
                reader: input.reader,
                say: &mut say,
                colour,
                terminal: input.terminal,
            };
            match ask(&defaults, asking) {
                Ok(answers) => answers,
                Err(told) => {
                    say(&format!("{}\n", shown(&told)));
                    return 1;
                }
            }
        }
        None => {
            let answers = defaults.answers();
            for tag in without_words(&answers.languages) {
                say(&no_words(tag));
            }
            if !without_words(&answers.languages).is_empty() {
                say("\n");
            }
            answers
        }
    };
    let made = match make(Maker::New, &folder, &answers) {
        Ok(made) => made,
        Err(problem) => {
            say(&report.problem(&problem));
            return 1;
        }
    };
    say(&format!(
        "{}\n",
        shown(&wording::made_site(&shown_as, made.files))
    ));
    say(&format!("\n{}:\n", config::FILE_NAME));
    for line in made.settings.lines().filter(|line| !line.starts_with('#')) {
        say(&format!("  {}\n", shown(line)));
    }
    say("\n");
    let translated: Vec<&String> = (answers.languages.iter())
        .filter(|tag| lacks_shown_words(tag))
        .collect();
    for tag in &translated {
        let told = wording::translate_words(tag, &language_path(tag));
        say(&format!("{}\n", shown(&told)));
    }
    if !translated.is_empty() {
        say("\n");
    }
    let away = options.folder.as_ref().map(|_| shown_as.as_str());
    say(&format!("{}\n", shown(&wording::next_steps(away))));
    0
}

/// The last part of `folder`'s whole path, for a folder named as `.`.
fn own_name(folder: &Path) -> String {
    let whole = std::fs::canonicalize(folder).unwrap_or_else(|_| folder.to_path_buf());
    match whole.file_name() {
        Some(name) => name.to_string_lossy().into_owned(),
        None => whole.to_string_lossy().into_owned(),
    }
}

/// What Enter answers to each question.
struct Defaults {
    title: String,
    language: String,
    /// Whether the language is the machine's.
    machines_language: bool,
    /// The machine's zone, where it has one.
    timezone: Option<String>,
}

impl Defaults {
    fn of(folder: &str, locale: &Locale, timezone: Option<&str>) -> Defaults {
        let name = folder.rsplit(['/', '\\']).find(|piece| !piece.is_empty());
        let language = locale.language();
        Defaults {
            title: as_words(name.unwrap_or_default()),
            machines_language: language.is_some(),
            language: language.unwrap_or_else(|| "en".to_owned()),
            timezone: timezone.map(str::to_owned),
        }
    }

    fn answers(&self) -> Answers {
        Answers {
            title: self.title.clone(),
            author: None,
            address: None,
            languages: vec![self.language.clone()],
            timezone: self.timezone.clone(),
        }
    }
}

/// A folder's name read as words, `my-site` as `My Site`; `Site` where
/// it holds no letters.
pub(crate) fn as_words(name: &str) -> String {
    if !name.chars().any(char::is_alphabetic) {
        return "Site".to_owned();
    }
    let words: Vec<String> = name
        .split(['-', '_', ' ', '.'])
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut letters = word.chars();
            match letters.next() {
                Some(first) => first.to_uppercase().collect::<String>() + letters.as_str(),
                None => String::new(),
            }
        })
        .collect();
    words.join(" ")
}

/// The questions as they are put: shown to a person at a terminal, who
/// is asked again after an answer that can't be used, or answered by the
/// lines read with nothing shown.
pub(crate) struct Asking<'a> {
    pub reader: &'a mut dyn BufRead,
    pub say: &'a mut dyn FnMut(&str),
    pub colour: bool,
    pub terminal: bool,
}

impl Asking<'_> {
    /// Says `text` where a person is there to read it.
    fn show(&mut self, text: &str) {
        if self.terminal {
            (self.say)(text);
        }
    }

    fn block(&mut self, question: &str, hints: &[String]) {
        self.show(&report::question(self.colour, question));
        for hint in hints {
            self.show(&report::hint(self.colour, &shown(hint)));
        }
    }

    /// Gives what is typed on the answer line.
    fn reply(&mut self, default: Option<&str>) -> String {
        let default = default.map(|default| shown(&wording::ask_default(default)));
        self.show(&report::answer_line(self.colour, default.as_deref()));
        let mut line = String::new();
        // Nothing more to read is taken as Enter.
        let _ = self.reader.read_line(&mut line);
        line.trim().to_owned()
    }

    /// Tells a person of an answer that can't be used, for the question to
    /// be answered again. With nobody there to answer again, gives `told`.
    fn refused(&mut self, told: String) -> Result<(), String> {
        if !self.terminal {
            return Err(told);
        }
        self.show(&format!("  {}\n", shown(&told)));
        Ok(())
    }

    /// Asks `question` for the site's address, and gives it, or none where
    /// the answer is Enter.
    pub(crate) fn address(
        &mut self,
        question: &str,
        hints: &[String],
    ) -> Result<Option<String>, String> {
        self.block(question, hints);
        let address = loop {
            let typed = self.reply(None);
            if typed.is_empty() {
                break None;
            }
            match site_address(&typed) {
                Some(address) => break Some(address),
                None => self.refused(wording::not_a_site_address(&typed))?,
            }
        };
        self.show("\n");
        Ok(address)
    }
}

/// Asks each question until it has an answer it can use, or gives what
/// is told of an answer that can't be used where nobody is there to
/// answer again.
fn ask(defaults: &Defaults, mut asking: Asking) -> Result<Answers, String> {
    asking.block(wording::ASK_TITLE, &[]);
    let title = match asking.reply(Some(&defaults.title)) {
        typed if typed.is_empty() => defaults.title.clone(),
        typed => typed,
    };
    asking.show("\n");
    asking.block(wording::ASK_AUTHOR, &[wording::ASK_NONE_HINT.to_owned()]);
    let author = Some(asking.reply(None)).filter(|typed| !typed.is_empty());
    asking.show("\n");
    let address = asking.address(
        wording::ASK_ADDRESS,
        &[wording::ASK_ADDRESS_HINT.to_owned()],
    )?;
    let mut hints = vec![wording::ASK_LANGUAGE_HINT.to_owned()];
    if defaults.machines_language {
        hints.push(wording::ASK_MACHINES.to_owned());
    }
    asking.block(wording::ASK_LANGUAGE, &hints);
    let languages = loop {
        let typed = asking.reply(Some(&defaults.language));
        let typed = match typed.is_empty() {
            true => defaults.language.clone(),
            false => typed,
        };
        match languages(&typed) {
            Ok(languages) => break languages,
            Err(told) => asking.refused(told)?,
        }
    };
    asking.show("\n");
    for tag in without_words(&languages) {
        (asking.say)(&no_words(tag));
    }
    if !without_words(&languages).is_empty() {
        (asking.say)("\n");
    }
    let hint = match &defaults.timezone {
        Some(_) => wording::ASK_MACHINES,
        None => wording::ASK_ZONE_HINT,
    };
    asking.block(wording::ASK_ZONE, &[hint.to_owned()]);
    let timezone = loop {
        let typed = asking.reply(defaults.timezone.as_deref());
        if typed.is_empty() {
            break defaults.timezone.clone();
        }
        match jiff::tz::TimeZone::get(&typed) {
            Ok(_) => break Some(typed),
            Err(_) => {
                let nearest = crate::dates::nearest_zone(&typed);
                asking.refused(wording::not_a_time_zone(&typed, nearest.as_deref()))?;
            }
        }
    };
    asking.show("\n");
    Ok(Answers {
        title,
        author,
        address,
        languages,
        timezone,
    })
}

/// What is said of a language Statac has no words for.
fn no_words(tag: &str) -> String {
    format!(
        "{}\n{}\n",
        shown(&wording::no_words_for(tag)),
        wording::more(wording::LANGUAGE_FILES)
    )
}

/// `typed` as a site's address: `http` or `https` with a host, a final
/// slash taken off. None where it isn't one `statac.yaml` takes.
fn site_address(typed: &str) -> Option<String> {
    let given = url::Url::parse(typed).ok()?;
    let web = matches!(given.scheme(), "http" | "https");
    if !web || given.host_str().is_none_or(str::is_empty) {
        return None;
    }
    let address = typed.strip_suffix('/').unwrap_or(typed).to_owned();
    reads_back("url", &yaml::write::text(&address)).then_some(address)
}

/// The languages typed, parted by commas, or what to tell of the first
/// that can't be used.
fn languages(typed: &str) -> Result<Vec<String>, String> {
    let mut languages: Vec<String> = Vec::new();
    for code in typed
        .split(',')
        .map(str::trim)
        .filter(|code| !code.is_empty())
    {
        if !a_language_code(code) {
            return Err(wording::not_a_language_code(code));
        }
        if let Some(first) = (languages.iter()).find(|first| first.eq_ignore_ascii_case(code)) {
            return Err(wording::language_given_twice(code, first));
        }
        languages.push(code.to_owned());
    }
    match languages.is_empty() {
        true => Err(wording::not_a_language_code(typed)),
        false => Ok(languages),
    }
}

/// Whether `statac.yaml` takes `code` as a language's code.
pub(crate) fn a_language_code(code: &str) -> bool {
    reads_back("language", &yaml::write::text(code))
}

/// Whether `statac.yaml` holding `name: written` reads with no problem.
fn reads_back(name: &str, written: &str) -> bool {
    let mut sources = Sources::default();
    let source = sources.add(config::FILE_NAME, format!("{name}: {written}\n"));
    let (settings, problems) = config::read_source(&sources, source);
    problems.is_empty() && settings.get(name).is_some()
}

/// The languages among `languages` Statac has no words for.
fn without_words(languages: &[String]) -> Vec<&str> {
    (languages.iter())
        .map(String::as_str)
        .filter(|tag| date_words(tag).from == WordsFrom::Nowhere((*tag).to_owned()))
        .collect()
}

/// The words a site in `tag` writes its dates in, found as a build finds
/// them in a site with no `languages/` of its own.
fn date_words(tag: &str) -> DateWords {
    let mut sources = Sources::default();
    let mut problems = Vec::new();
    let tag_given = Sourced::new(tag, Origin::defaulted());
    let language = Languages::default().language(Some(tag_given), &mut sources, &mut problems);
    let from = match language.whence {
        crate::language::Whence::Statac(known) => WordsFrom::Statac(known.to_owned()),
        _ => WordsFrom::Nowhere(tag.to_owned()),
    };
    DateWords {
        words: language.words,
        from,
        zeros: true,
    }
}

/// Why `by` can make nothing in `folder`, if it can't: it is a link or a
/// file, it is a site already, or it holds something.
pub(crate) fn refusal(folder: &Path, shown_as: &str, by: Maker) -> Result<(), Problem> {
    let cant_write = |error: &io::Error| {
        Problem::nowhere(Kind::CantWrite {
            path: folder.to_path_buf(),
            reason: Reason::of(error),
        })
    };
    let found = match std::fs::symlink_metadata(folder) {
        Ok(found) => found,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(cant_write(&error)),
    };
    if found.is_symlink() {
        return Err(cant_write(&io::Error::from(Reason::Link)));
    }
    if !found.is_dir() {
        return Err(cant_write(&io::Error::from(io::ErrorKind::NotADirectory)));
    }
    if let Some(has) = site_mark(folder).map_err(|error| cant_write(&error))? {
        return Err(Problem::nowhere(Kind::AlreadyASite {
            by,
            folder: shown_as.to_owned(),
            has,
        }));
    }
    let mut names = Vec::new();
    for entry in std::fs::read_dir(folder).map_err(|error| cant_write(&error))? {
        let entry = entry.map_err(|error| cant_write(&error))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort_unstable();
    if names.is_empty() {
        return Ok(());
    }
    const NAMED: usize = 4;
    let more = names.len().saturating_sub(NAMED);
    names.truncate(NAMED);
    Err(Problem::nowhere(Kind::FolderNotEmpty {
        by,
        folder: shown_as.to_owned(),
        names,
        more,
    }))
}

/// What makes `folder` a site, if anything does: a `statac.yaml`, or a
/// `content` folder.
pub(crate) fn site_mark(folder: &Path) -> io::Result<Option<SiteMark>> {
    // Looked for by name first, so a folder is listed only where the file
    // may be there: a disk that ignores case finds `Statac.yaml` by that
    // name too, and only the listing gives its spelling.
    let maybe = match std::fs::symlink_metadata(folder.join(config::FILE_NAME)) {
        Ok(_) => true,
        Err(error) if error.kind() == io::ErrorKind::NotFound => false,
        Err(error) => return Err(error),
    };
    if maybe {
        for entry in std::fs::read_dir(folder)? {
            if entry?.file_name() == config::FILE_NAME {
                return Ok(Some(SiteMark::Settings));
            }
        }
    }
    let content = std::fs::symlink_metadata(folder.join("content"));
    Ok(content
        .is_ok_and(|found| found.is_dir())
        .then_some(SiteMark::Content))
}

/// Writes a new site from `answers` into `folder`, which must be empty
/// or not yet there. No link is followed, in the folder or inside it. Where
/// anything stops it part way, the folder is left as it was found: every
/// file written is removed, and every folder made, `folder` itself among
/// them. A refusal names `by` as the command to run again.
pub(crate) fn make(by: Maker, folder: &Path, answers: &Answers) -> Result<Made, Problem> {
    make_with(by, folder, answers, &mut platform::make_file)
}

/// Writes a file of the site's, by its path in the site's folder.
type Writer<'a> = dyn FnMut(&Path, &str, &[u8]) -> io::Result<()> + 'a;

/// [`make`], writing each file with `write`.
fn make_with(
    by: Maker,
    folder: &Path,
    answers: &Answers,
    write: &mut Writer<'_>,
) -> Result<Made, Problem> {
    refusal(folder, &folder.to_string_lossy(), by)?;
    let settings = settings(answers, by);
    let files = files(answers, &settings, by);
    let mut made = Written::default();
    let written = (|| -> Result<(), (PathBuf, io::Error)> {
        made.outer(folder)?;
        made.folder(folder, CONTENT)?;
        for (path, text) in &files {
            made.folders_to(folder, path)?;
            write(folder, path, text.as_bytes()).map_err(|error| (folder.join(path), error))?;
            made.files.push(path.clone());
        }
        Ok(())
    })();
    match written {
        Ok(()) => Ok(Made {
            files: files.len(),
            settings,
        }),
        Err((path, error)) => {
            made.undo(folder);
            Err(Problem::nowhere(Kind::CantWrite {
                path,
                reason: Reason::of(&error),
            }))
        }
    }
}

/// The folder a site's pages are written in, which a new site has empty.
const CONTENT: &str = "content";

/// What a run has made so far, to be undone where it stops part way.
#[derive(Default)]
struct Written {
    /// The site's folder and those above it that weren't there, outermost
    /// first.
    outer: Vec<PathBuf>,
    /// Folders inside the site, by their paths in it, in the order made.
    folders: Vec<String>,
    files: Vec<String>,
}

impl Written {
    /// Makes `folder` and each folder above it that isn't there.
    fn outer(&mut self, folder: &Path) -> Result<(), (PathBuf, io::Error)> {
        let mut missing = Vec::new();
        let mut at = Some(folder);
        while let Some(path) = at.filter(|path| !path.as_os_str().is_empty()) {
            match std::fs::symlink_metadata(path) {
                Ok(_) => break,
                Err(error) if error.kind() == io::ErrorKind::NotFound => missing.push(path),
                Err(error) => return Err((path.to_path_buf(), error)),
            }
            at = path.parent();
        }
        for path in missing.into_iter().rev() {
            match std::fs::create_dir(path) {
                Ok(()) => self.outer.push(path.to_path_buf()),
                // Made meanwhile by another run, so it isn't this one's to undo.
                Err(error)
                    if error.kind() == io::ErrorKind::AlreadyExists
                        && std::fs::symlink_metadata(path).is_ok_and(|found| found.is_dir()) => {}
                Err(error) => return Err((path.to_path_buf(), error)),
            }
        }
        Ok(())
    }

    /// Makes the folder `path` in `folder`, where it isn't made yet.
    fn folder(&mut self, folder: &Path, path: &str) -> Result<(), (PathBuf, io::Error)> {
        if self.folders.iter().any(|made| made == path) {
            return Ok(());
        }
        platform::make_folder(folder, path).map_err(|error| (folder.join(path), error))?;
        self.folders.push(path.to_owned());
        Ok(())
    }

    /// Makes each folder on the way to `path` in `folder` not made yet.
    fn folders_to(&mut self, folder: &Path, path: &str) -> Result<(), (PathBuf, io::Error)> {
        let Some((above, _)) = path.rsplit_once('/') else {
            return Ok(());
        };
        let mut end = 0;
        for piece in above.split('/') {
            end += piece.len();
            self.folder(folder, &above[..end])?;
            end += 1;
        }
        Ok(())
    }

    /// Removes everything made, the last first.
    fn undo(&self, folder: &Path) {
        for path in self.files.iter().rev() {
            let _ = platform::remove_file(folder, path);
        }
        for path in self.folders.iter().rev() {
            let _ = std::fs::remove_dir(platform::join(folder, path));
        }
        for path in self.outer.iter().rev() {
            let _ = std::fs::remove_dir(path);
        }
    }
}

/// The comment written above a setting that needs one.
const FEED_KEY_COMMENT: &str =
    "# What each post's id in a feed is made from: never change it once a feed is published.";

/// `statac.yaml` for a site answered as `answers`, made `by` a command:
/// what was answered, and for an imported site, whose feeds came across
/// with it, the key its feeds' ids are made from.
fn settings(answers: &Answers, by: Maker) -> String {
    let language = match answers.languages.as_slice() {
        [one] => yaml::write::text(one),
        many => {
            let codes: Vec<String> = many.iter().map(|code| yaml::write::text(code)).collect();
            format!("[{}]", codes.join(", "))
        }
    };
    let mut lines = vec![
        ("title", Some(yaml::write::text(&answers.title))),
        ("author", answers.author.as_deref().map(yaml::write::text)),
        ("url", answers.address.as_deref().map(yaml::write::text)),
        ("language", Some(language)),
        (
            "timezone",
            answers.timezone.as_deref().map(yaml::write::text),
        ),
    ];
    if by == Maker::Import {
        lines.push(("feed_key", Some(crate::feeds::mint_key())));
    }
    let mut text = String::new();
    for (name, value) in lines {
        let Some(value) = value else {
            continue;
        };
        if preset(name).is_some_and(|preset| yaml::write::text(&preset) == value) {
            continue;
        }
        if name == "feed_key" {
            text.push_str(FEED_KEY_COMMENT);
            text.push('\n');
        }
        text.push_str(&format!("{name}: {value}\n"));
    }
    text
}

/// Statac's own value for the setting `name` where `statac.yaml` doesn't
/// write it, as text.
fn preset(name: &str) -> Option<String> {
    let mut sources = Sources::default();
    let source = sources.add(config::FILE_NAME, String::new());
    let (settings, _) = config::read_source(&sources, source);
    settings.text(name).map(|text| text.value.to_owned())
}

/// Every file of a new site made `by` a command, by its path in the site:
/// `statac.yaml`, holding `settings`, and `.gitignore`, and for a site of
/// `statac new`, the words Statac shows beside the content in English for
/// each language it lacks them for.
fn files(answers: &Answers, settings: &str, by: Maker) -> Vec<(String, String)> {
    let mut files = vec![
        (config::FILE_NAME.to_owned(), settings.to_owned()),
        (".gitignore".to_owned(), ".statac/\n_site/\n".to_owned()),
    ];
    if by == Maker::New {
        for tag in (answers.languages.iter()).filter(|tag| lacks_shown_words(tag)) {
            files.push((language_path(tag), language_file()));
        }
    }
    files.sort_by(|(a, _), (b, _)| a.cmp(b));
    files
}

/// Whether Statac has words for `tag` and none for what it writes beside a
/// page's writing: a footnote's way back, or the callouts' names.
fn lacks_shown_words(tag: &str) -> bool {
    let found = date_words(tag);
    let words = &found.words;
    matches!(found.from, WordsFrom::Statac(_))
        && (words.back_to_the_text.is_none() || words.callouts.is_none())
}

/// Where a site keeps its own words for the language `tag`.
fn language_path(tag: &str) -> String {
    format!("{}/{tag}.yaml", crate::language::FOLDER)
}

/// A language's file in a new site: the words Statac writes beside a
/// page's writing, in English.
fn language_file() -> String {
    let english = crate::language::english();
    let mut text = String::from(
        "# The words Statac itself shows alongside your writing, in English for you to translate: \
         see /dates-and-languages/\n",
    );
    if let Some(label) = &english.back_to_the_text {
        text.push_str(&format!("back_to_the_text: {}\n", yaml::write::text(label)));
    }
    if let Some(names) = &english.callouts {
        let names: Vec<String> = names.iter().map(|name| yaml::write::text(name)).collect();
        text.push_str(&format!("callouts: [{}]\n", names.join(", ")));
    }
    text
}
