//! `statac new`: four questions, and a whole site written from the
//! answers into an empty folder or a new one.
//!
//! The starter's files are written exactly as `starter/` keeps them, so
//! one left untouched can be told by its bytes. What the answers shape is
//! made here: `statac.yaml`, `.gitignore`, the words the templates show in
//! `data/words.yaml`, the words Statac shows in `languages/` for a language
//! it has them in English alone for, the home page, and the sample pages,
//! which carry dates.
//!
//! Everything `new` learns from the machine comes in a [`Machine`], so
//! that a run can be repeated exactly. [`make`] writes a site from
//! [`Answers`] and says nothing. `statac new --update` is [`update`].

mod differ;
mod past;
mod samples;
mod starter;
#[cfg(test)]
mod tests;
pub mod update;
mod words;

use std::borrow::Cow;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::config;
use crate::date::Date;
use crate::dates::format::{self, DateWords, Format};
use crate::language::Languages;
use crate::origin::{Origin, Sourced};
use crate::platform;
use crate::problem::{Kind, Maker, Problem, SiteMark, WordsFrom, system_reason};
use crate::report::{self, Report, shown};
use crate::source::Sources;
use crate::wording;
use crate::yaml::{self, Resolved, Value as Yaml};
pub(crate) use starter::has_snippet;
use update::{CARD, Starter};

/// What `statac new` is asked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    /// The folder the site is made in; the folder Statac is run in where
    /// none is given.
    pub folder: Option<PathBuf>,
    /// Whether every default is taken without asking.
    pub yes: bool,
    /// What `--update` asks, where it is given: the site in the folder
    /// brought up to date in place of a new one.
    pub update: Option<update::Update>,
    /// Whether `--cards` is given: the sharing card's files written, into
    /// the site in the folder, or with a new site.
    pub cards: bool,
}

/// What `statac new` learns from the machine it runs on.
pub struct Machine<'a> {
    /// Where the answers are read from; none where every default is taken.
    pub input: Option<Input<'a>>,
    pub locale: Locale,
    /// The machine's time zone, by its name in the time zone database.
    pub timezone: Option<String>,
    /// The day on the machine's clock, in its zone.
    pub today: Date,
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

/// The four answers a site is made from.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Answers {
    pub title: String,
    /// The site's address, `http` or `https` with a host, and no slash at
    /// its end.
    pub address: Option<String>,
    /// Codes `statac.yaml` takes, the main language first, each once.
    pub languages: Vec<String>,
    /// An example `date_format` reads in the main language's words.
    pub date_format: String,
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
    if let Some(asked) = options.update {
        let folder = options.folder.as_deref().unwrap_or(Path::new("."));
        return update::run(folder, asked, out, colour);
    }
    let mut say = |text: &str| {
        let _ = out.write_all(text.as_bytes());
        let _ = out.flush();
    };
    let folder = options.folder.clone().unwrap_or_else(|| PathBuf::from("."));
    if options.cards && matches!(site_mark(&folder), Ok(Some(_))) {
        return cards_on(&folder, &Starter::OWN, out, colour);
    }
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
    let defaults = Defaults::of(&shown_as, &machine.locale);
    let Machine {
        input,
        timezone,
        today,
        ..
    } = machine;
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
    let made = make(
        Maker::New,
        &folder,
        &answers,
        timezone.as_deref(),
        today,
        Writes::starter(options.cards),
    );
    let made = match made {
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
    match &timezone {
        Some(zone) => say(&format!("{}\n", wording::times_in(zone))),
        None => say(&format!("{}\n", wording::NO_ZONE_HERE)),
    }
    say(&format!("\n{}:\n", config::FILE_NAME));
    for line in made.settings.lines().filter(|line| !line.starts_with('#')) {
        say(&format!("  {}\n", shown(line)));
    }
    say("\n");
    let translated: Vec<&String> = (answers.languages.iter())
        .filter(|tag| !english(tag))
        .collect();
    for tag in &translated {
        let file = lacks_shown_words(tag).then(|| language_path(tag));
        let told = wording::translate_words(tag, file.as_deref());
        say(&format!("{told}\n"));
    }
    if !translated.is_empty() {
        say("\n");
    }
    let away = options.folder.as_ref().map(|_| shown_as.as_str());
    say(&format!("{}\n", shown(&wording::next_steps(away))));
    0
}

/// Writes the sharing card's files into the site in `folder`, held to
/// `starter`, and says what it wrote on `out`. Nothing is written where the
/// site describes a card already, or holds a file of the author's where one
/// of the card's would go. The description is written last, so that a run
/// stopped part way leaves no card described. Gives the exit code: 0, or 1
/// where a file couldn't be read or written.
fn cards_on(folder: &Path, starter: &Starter, out: &mut dyn Write, colour: bool) -> u8 {
    let sources = Sources::default();
    let report = Report::new(&sources, folder, colour);
    let problem = |kind: Kind| report.problem(&Problem::nowhere(kind));
    let mut said = String::new();
    let mut failed = false;
    let root = match std::fs::canonicalize(folder) {
        Ok(root) => root,
        Err(error) => {
            let _ = out.write_all(
                problem(Kind::CantRead {
                    path: folder.to_path_buf(),
                    reason: system_reason(&error),
                })
                .as_bytes(),
            );
            return 1;
        }
    };
    let mut found = update::survey(&root, starter);
    found.retain(|found| starter.is_a_cards(found.path));
    found.sort_by_key(|found| found.path == CARD);
    let described = (found.iter())
        .any(|found| found.path == CARD && !matches!(found.is, update::Is::NotHere { .. }));
    let mut to_write = Vec::new();
    for found in &found {
        if described && found.path != CARD {
            continue;
        }
        match &found.is {
            update::Is::NotHere { current } => to_write.push((found.path, *current)),
            update::Is::Linked(link) => {
                said.push_str(&shown(&wording::left_for_a_link(found.path, link)));
                said.push('\n');
            }
            update::Is::Unread(reason) => {
                failed = true;
                said.push_str(&problem(Kind::CantRead {
                    path: found.path.into(),
                    reason: reason.clone(),
                }));
            }
            // A file as a starter wrote it is in nothing's way, but a
            // description is the site's card whatever it holds.
            update::Is::Newest | update::Is::Older { .. } if found.path != CARD => {}
            update::Is::Newest
            | update::Is::Older { .. }
            | update::Is::Yours { .. }
            | update::Is::NotText { .. } => {
                said.push_str(&wording::yours_untouched(found.path));
                said.push('\n');
            }
        }
    }
    if said.is_empty() {
        for (path, bytes) in to_write {
            match update::write_missing(&root, path, bytes) {
                Ok(()) => {
                    said.push_str(&wording::wrote_file(path));
                    said.push('\n');
                }
                Err(error) => {
                    failed = true;
                    said.push_str(&problem(Kind::CantWrite {
                        path: path.into(),
                        reason: system_reason(&error),
                    }));
                    break;
                }
            }
        }
    }
    let _ = out.write_all(said.as_bytes());
    let _ = out.flush();
    u8::from(failed)
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
}

impl Defaults {
    fn of(folder: &str, locale: &Locale) -> Defaults {
        let name = folder.rsplit(['/', '\\']).find(|piece| !piece.is_empty());
        Defaults {
            title: as_words(name.unwrap_or_default()),
            language: locale.language().unwrap_or_else(|| "en".to_owned()),
        }
    }

    fn answers(&self) -> Answers {
        let languages = vec![self.language.clone()];
        let date_format = dates_offered(&languages[0])[0].1.clone();
        Answers {
            title: self.title.clone(),
            address: None,
            languages,
            date_format,
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
    let address = asking.address(
        wording::ASK_ADDRESS,
        &[wording::ASK_ADDRESS_HINT.to_owned()],
    )?;
    asking.block(
        wording::ASK_LANGUAGE,
        &[wording::ASK_LANGUAGE_HINT.to_owned()],
    );
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
    let offered = dates_offered(&languages[0]);
    let mut hints: Vec<String> = (offered.iter())
        .map(|(number, example)| wording::ask_date_choice(*number, example))
        .collect();
    hints.push(wording::ASK_DATES_OWN.to_owned());
    asking.block(wording::ASK_DATES, &hints);
    let first = offered[0].0.to_string();
    let date_format = loop {
        let typed = asking.reply(Some(&first));
        let typed = match typed.is_empty() {
            true => first.clone(),
            false => typed,
        };
        if let Some((_, example)) = (offered.iter()).find(|(number, _)| number.to_string() == typed)
        {
            break example.clone();
        }
        match Format::for_the_site(&typed, &date_words(&languages[0])) {
            Ok(_) => break typed,
            Err(refusal) => asking.refused(wording::headline(&refusal.into_kind()))?,
        }
    };
    asking.show("\n");
    Ok(Answers {
        title,
        address,
        languages,
        date_format,
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
    reads_back("url", &quoted(&address)).then_some(address)
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
    reads_back("language", &quoted(code))
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
        house: format::house_example(tag),
    }
}

/// The examples of how dates look offered for a site in `tag`, each by
/// its number: the language's own way, which a site naming no
/// `date_format` writes; the month first, in English alone; and the year
/// first, which is all there is where Statac has no words for the
/// language's months.
pub(crate) fn dates_offered(tag: &str) -> Vec<(u8, String)> {
    let date_words = date_words(tag);
    let mut examples = Vec::new();
    if let Some(months) = &date_words.words.months {
        examples.push(Format::house(&date_words).shown());
        if english(tag) {
            examples.push(format!("{} 31, 2025", months[11]));
        }
    }
    examples.push(YEAR_FIRST.to_owned());
    (1..).zip(examples).collect()
}

/// Whether `tag` is English, in whichever region.
fn english(tag: &str) -> bool {
    let language = tag.split('-').next().unwrap_or(tag);
    language.eq_ignore_ascii_case("en")
}

/// Why `by` can make nothing in `folder`, if it can't: it is a link or a
/// file, it is a site already, or it holds something.
pub(crate) fn refusal(folder: &Path, shown_as: &str, by: Maker) -> Result<(), Problem> {
    let cant_write = |error: &io::Error| {
        Problem::nowhere(Kind::CantWrite {
            path: folder.to_path_buf(),
            reason: system_reason(error),
        })
    };
    let found = match std::fs::symlink_metadata(folder) {
        Ok(found) => found,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(cant_write(&error)),
    };
    if found.is_symlink() {
        return Err(cant_write(&io::Error::other("it is a link")));
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
    for entry in std::fs::read_dir(folder)? {
        if entry?.file_name() == config::FILE_NAME {
            return Ok(Some(SiteMark::Settings));
        }
    }
    let content = std::fs::symlink_metadata(folder.join("content"));
    Ok(content
        .is_ok_and(|found| found.is_dir())
        .then_some(SiteMark::Content))
}

/// Which of a new site's files [`make`] writes.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Writes {
    /// The starter's, and those made from the answers.
    Starter,
    /// Those, and the sharing card's.
    StarterAndCards,
    /// `statac.yaml` alone.
    Settings,
}

impl Writes {
    /// The starter, with the sharing card's files where `cards` asks.
    fn starter(cards: bool) -> Writes {
        match cards {
            true => Writes::StarterAndCards,
            false => Writes::Starter,
        }
    }
}

/// Writes a new site from `answers` into `folder`, which must be empty
/// or not yet there, with `timezone` in its settings where the machine
/// has one and its sample pages dated from `today`. A site made by an
/// import has no sample pages: the old site's take their place. No link
/// is followed, in the folder or inside it. Where anything stops it part
/// way, the folder is left as it was found: every file written is
/// removed, and every folder made, `folder` itself among them. A refusal
/// names `by` as the command to run again. `writes` says which files.
pub(crate) fn make(
    by: Maker,
    folder: &Path,
    answers: &Answers,
    timezone: Option<&str>,
    today: Date,
    writes: Writes,
) -> Result<Made, Problem> {
    make_with(
        by,
        folder,
        answers,
        timezone,
        today,
        writes,
        &mut platform::make_file,
    )
}

/// Writes a file of the site's, by its path in the site's folder.
type Writer<'a> = dyn FnMut(&Path, &str, &[u8]) -> io::Result<()> + 'a;

/// [`make`], writing each file with `write`.
fn make_with(
    by: Maker,
    folder: &Path,
    answers: &Answers,
    timezone: Option<&str>,
    today: Date,
    writes: Writes,
    write: &mut Writer<'_>,
) -> Result<Made, Problem> {
    refusal(folder, &folder.to_string_lossy(), by)?;
    let settings = settings(answers, timezone);
    let samples = (by == Maker::New).then_some(today);
    let files = files(answers, &settings, samples, writes);
    let mut made = Written::default();
    let written = (|| -> Result<(), (PathBuf, io::Error)> {
        made.outer(folder)?;
        for (path, bytes) in &files {
            made.folders_to(folder, path)?;
            write(folder, path, bytes).map_err(|error| (folder.join(path), error))?;
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
                reason: system_reason(&error),
            }))
        }
    }
}

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
            std::fs::create_dir(path).map_err(|error| (path.to_path_buf(), error))?;
            self.outer.push(path.to_path_buf());
        }
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
            let on_the_way = &above[..end];
            end += 1;
            if self.folders.iter().any(|made| made == on_the_way) {
                continue;
            }
            platform::make_folder(folder, on_the_way)
                .map_err(|error| (folder.join(on_the_way), error))?;
            self.folders.push(on_the_way.to_owned());
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

/// `statac.yaml` for a site answered as `answers`.
fn settings(answers: &Answers, timezone: Option<&str>) -> String {
    let language = match answers.languages.as_slice() {
        [one] => quoted(one),
        many => {
            let codes: Vec<String> = many.iter().map(|code| quoted(code)).collect();
            format!("[{}]", codes.join(", "))
        }
    };
    let mut lines = vec![
        ("title", Some(quoted(&answers.title))),
        ("url", answers.address.as_deref().map(quoted)),
        ("language", Some(language)),
        ("timezone", timezone.map(quoted)),
        ("date_format", Some(quoted(&answers.date_format))),
    ];
    for (name, example) in shorter_dates(&answers.languages[0], &answers.date_format) {
        lines.push((name, Some(quoted(&example))));
    }
    lines.push(("feed_key", Some(crate::feeds::mint_key())));
    let mut text = String::new();
    for (name, value) in lines {
        let Some(value) = value else {
            continue;
        };
        if preset(name).is_some_and(|preset| quoted(&preset) == value) {
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

/// The year-first date, and the shorter examples that go with it.
const YEAR_FIRST: &str = "2025-12-31";
const YEAR_FIRST_SHORTER: [&str; 3] = [YEAR_FIRST, "12-31", "2025-12-31 15:04"];

/// The shorter examples of a date the starter's templates hand the `date`
/// filter, each by the name `statac.yaml` keeps it under as a value of the
/// author's own: the day, the month and the year, the day and the month,
/// and the day with the time. They follow `date_format`, an example in the
/// words of `tag`, in its order, where it comes apart into them, and are
/// the year-first ones where it doesn't.
fn shorter_dates(tag: &str, date_format: &str) -> [(&'static str, String); 3] {
    let words = date_words(tag);
    let reads = |example: &String| Format::for_a_template(example, &words).is_ok();
    let derived = Some(date_format)
        .filter(|example| *example != YEAR_FIRST)
        .and_then(|example| Format::for_the_site(example, &words).ok())
        .and_then(|format| format.shorter())
        .map(|shorter| [shorter.short, shorter.day, shorter.stamp])
        .filter(|examples| examples.iter().all(reads));
    let [short, day, stamp] = derived.unwrap_or_else(|| YEAR_FIRST_SHORTER.map(str::to_owned));
    [
        ("date_short", short),
        ("date_day", day),
        ("date_stamp", stamp),
    ]
}

/// Statac's own value for the setting `name` where `statac.yaml` doesn't
/// write it, as text.
fn preset(name: &str) -> Option<String> {
    let mut sources = Sources::default();
    let source = sources.add(config::FILE_NAME, String::new());
    let (settings, _) = config::read_source(&sources, source);
    settings.text(name).map(|text| text.value.to_owned())
}

/// `text` as a YAML value that reads back as the same text: as it is
/// where it can be, and in double quotes where it can't.
fn quoted(text: &str) -> String {
    let mut sources = Sources::default();
    let source = sources.add("value.yaml", format!("value: {text}\n"));
    let plain = yaml::read(&sources, source).is_ok_and(|node| match &node.value {
        Yaml::Mapping(entries) => match entries.as_slice() {
            [entry] => {
                entry.value.resolved() == Some(Resolved::Str)
                    && matches!(&entry.value.value, Yaml::Scalar(read) if read == text)
            }
            _ => false,
        },
        _ => false,
    });
    if plain && !text.contains('\n') {
        return text.to_owned();
    }
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if c.is_control() => out.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Every file of a new site `writes` asks for, by its path in the site:
/// the starter's, the sharing card's, those made from the answers, and the
/// sample pages with what is kept beside them, dated from `samples` where
/// it gives a day.
fn files<'a>(
    answers: &Answers,
    settings: &str,
    samples: Option<Date>,
    writes: Writes,
) -> Vec<(String, Cow<'a, [u8]>)> {
    let settings_file = (config::FILE_NAME.to_owned(), Cow::Owned(settings.into()));
    let asked: &[starter::File] = match writes {
        Writes::Settings => return vec![settings_file],
        Writes::StarterAndCards => starter::CARDS,
        Writes::Starter => &[],
    };
    let mut files: Vec<(String, Cow<[u8]>)> = (starter::FILES.iter().chain(asked))
        .filter(|file| {
            let sample = starter::is_a_samples(file.path) && file.path != starter::POSTS_PAGE;
            samples.is_some() || !sample
        })
        .map(|file| (file.path.to_owned(), Cow::Borrowed(file.bytes)))
        .collect();
    files.push(settings_file);
    let mut add = |path: String, text: String| files.push((path, Cow::Owned(text.into_bytes())));
    add(".gitignore".to_owned(), ".statac/\n_site/\n".to_owned());
    add("data/words.yaml".to_owned(), words_file(&answers.languages));
    for (at, tag) in answers.languages.iter().enumerate() {
        let path = match at {
            0 => "content/index.md".to_owned(),
            _ => format!("content/{tag}/index.md"),
        };
        let page = first_page(&path, samples.is_some());
        add(path, page);
        // Each language lists its own tags, on a page of its own.
        if let (1.., Some(tags)) = (at, starter::file(starter::TAGS_PAGE)) {
            let text = String::from_utf8_lossy(tags).into_owned();
            add(format!("content/{tag}/tags.md"), text);
        }
    }
    for (path, page) in samples.map(samples::pages).unwrap_or_default() {
        add(path, page);
    }
    // A sample post has a footnote, which shows the words.
    if samples.is_some() {
        for tag in (answers.languages.iter()).filter(|tag| lacks_shown_words(tag)) {
            add(language_path(tag), language_file());
        }
    }
    files.sort_by(|(a, _), (b, _)| a.cmp(b));
    files
}

/// Every file a new site holds, by its path in the site, in the order of
/// their paths, with the sharing card's where `cards` asks: the site
/// `statac new` makes in English where every default is taken.
pub fn files_of_a_new_site(cards: bool) -> Vec<(String, Vec<u8>)> {
    let defaults = Defaults {
        title: "Site".to_owned(),
        language: "en".to_owned(),
    };
    let answers = defaults.answers();
    let settings = settings(&answers, None);
    let day = Date::day_only(2026, 1, 1);
    files(&answers, &settings, Some(day), Writes::starter(cards))
        .into_iter()
        .map(|(path, bytes)| (path, bytes.into_owned()))
        .collect()
}

/// The file a new site is given for each language but English, at its
/// path for the language `tag`.
pub fn language_file_of_a_new_site(tag: &str) -> (String, String) {
    (language_path(tag), language_file())
}

/// The words the starter's templates and snippets show readers, in
/// English, each by the name `data/words.yaml` keeps it under.
pub fn words() -> &'static [(&'static str, &'static str)] {
    words::WORDS
}

/// `data/words.yaml`: every word the starter reads, in English, under each
/// of the site's languages.
fn words_file(languages: &[String]) -> String {
    let mut text = String::from(
        "# The words the templates and snippets show readers, in each language: see /new/\n",
    );
    for tag in languages {
        text.push_str(&format!("{}:\n", quoted(tag)));
        for (name, english) in words::WORDS {
            text.push_str(&format!("  {name}: {}\n", quoted(english)));
        }
    }
    text
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
        text.push_str(&format!("back_to_the_text: {}\n", quoted(label)));
    }
    if let Some(names) = &english.callouts {
        let names: Vec<String> = names.iter().map(|name| quoted(name)).collect();
        text.push_str(&format!("callouts: [{}]\n", names.join(", ")));
    }
    text
}

/// The home page, or a language's, written at `path`, its own list kept
/// to one page where `one_page` says.
fn first_page(path: &str, one_page: bool) -> String {
    let one_page = match one_page {
        true => {
            "# The home page shows the newest of each folder itself, so its own list is kept to \
             one page: see /lists/\nposts_per_page: 0\n"
        }
        false => "",
    };
    home_page(
        path,
        one_page,
        "a new site",
        " Posts are in `content/posts/`, notes in `content/notes/` and photographs in \
         `content/photos/`; delete a folder you don't want and it leaves the site.",
    )
}

/// The home page, or a language's, of a site made by an import, written
/// at `path`: the starter's, naming only the folders of posts that came
/// across, each by its path under `content/`, and none where none did.
pub(crate) fn imported_first_page(path: &str, folders: &[String]) -> String {
    let named = |kind: &str| -> Vec<String> {
        let last = |folder: &String| folder.rsplit('/').next().unwrap_or(folder).to_owned();
        (folders.iter())
            .filter(|folder| match kind {
                "posts" => !matches!(last(folder).as_str(), "notes" | "photos"),
                kind => last(folder) == kind,
            })
            .map(|folder| match folder.is_empty() {
                true => "`content/`".to_owned(),
                false => format!("`content/{folder}/`"),
            })
            .collect()
    };
    let kinds: Vec<String> = [
        ("posts", "posts"),
        ("notes", "notes"),
        ("photos", "photographs"),
    ]
    .iter()
    .map(|(kind, word)| (word, named(kind)))
    .filter(|(_, folders)| !folders.is_empty())
    .map(|(word, folders)| format!("{word} in {}", wording::listed(&folders)))
    .collect();
    let mut kept = wording::listed(&kinds).replacen(" in ", " are in ", 1);
    if let Some(first) = kept.get_mut(..1) {
        first.make_ascii_uppercase();
        kept = format!(" {kept}.");
    }
    home_page(path, "", "an imported site", &kept)
}

/// A home page of the starter's at `path`, for `site`, with `lines` in
/// its front matter and `kept`, where the site's writing is, in its words.
fn home_page(path: &str, lines: &str, site: &str, kept: &str) -> String {
    format!(
        "---\ngreeting: Hello.\n{lines}---\n\
         This is the home page of {site}, the file `{path}`. Write here what the site \
         is, in place of these words.\n\
         \n\
         To see the site as you change it, run `statac serve` in its folder.{kept} How each \
         page looks is in `templates/`, `snippets/` and `assets/site.css`, and the words the \
         pages show around the writing are in `data/words.yaml`. Every file is yours to \
         change.\n"
    )
}
