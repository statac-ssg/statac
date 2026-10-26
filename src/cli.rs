//! The command line: what the author asked for, and the answer.
//!
//! Options are written `--name=value`, and may come before or after the command.

pub(crate) mod search;

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::album::Progress;
use crate::build::{self, Mode, Options, Outcome};
use crate::date::{self, Date, Fault};
use crate::import;
use crate::new;
use crate::problem::Quiet;
use crate::problem::{Code, CommandHint, Expected, Found, Kind, Missing, OptionHint, Problem};
use crate::refresh;
use crate::report::Report;
use crate::serve;
use crate::serve::watch::{self, How};
use crate::source::Sources;
use crate::stamp;
use crate::suggest;
use crate::wording;

/// What the command line asks for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Request {
    Build {
        options: Options,
        /// Whether where the build spent its time is shown after it.
        timings: bool,
    },
    Check {
        options: Options,
        /// Whether the problems are shown as JSON, and nothing else.
        json: bool,
    },
    Serve {
        options: serve::Options,
        /// Whether nothing is to be fetched.
        offline: bool,
    },
    Stamp(stamp::Options),
    Refresh(refresh::Options),
    New(new::Options),
    Import(import::Options),
    Search(crate::search::Options),
    Explain(Code),
    /// A language server for an editor, on standard input and output.
    Lsp,
    /// The usage screen, for the command named before `--help`, or whole.
    Help(Option<&'static str>),
    Version,
}

const COMMANDS: [&str; 10] = [
    "build", "check", "serve", "stamp", "refresh", "new", "import", "search", "explain", "lsp",
];

/// Commands Statac once had, each with the setting that does its work now.
const FORMER_COMMANDS: [(&str, &str); 1] = [("refuse", crate::config::refused::SETTING)];

/// The commands that work on a site, named by the folder after the command.
const SITE_COMMANDS: [&str; 6] = ["build", "check", "serve", "stamp", "refresh", "search"];

/// The options that take a value, and the commands that take each.
const OPTIONS: [(&str, &[&str]); 7] = [
    ("--out", &["build"]),
    ("--url", &["build", "check"]),
    (
        "--today",
        &[
            "build", "check", "serve", "stamp", "refresh", "import", "search",
        ],
    ),
    ("--port", &["serve"]),
    ("--watch", &["serve"]),
    ("--from", &["import"]),
    ("--language", &["search"]),
];

/// The ways `--watch` names of watching the site's folder.
const WAYS: [&str; 3] = ["auto", "native", "poll"];

/// Options that take no value, and the commands that take each.
const FLAGS: [(&str, &[&str]); 6] = [
    ("--drafts", &["build", "check", "search"]),
    ("--json", &["check"]),
    ("--timings", &["build"]),
    ("--no-drafts", &["serve"]),
    ("--offline", &["serve"]),
    ("--yes", &["new"]),
];

/// Options Statac once had for writing its own files into a site.
const RETIRED_OPTIONS: [&str; 4] = ["--cards", "--update", "--missing", "--no-starter"];

/// The option Statac once had for the site's folder, which the folder
/// after the command names now.
const RETIRED_SITE: &str = "--site";

/// Options any command takes, which take no value.
const QUESTIONS: [&str; 2] = ["--help", "--version"];

/// Spellings other tools use for what Statac spells one way.
const OTHER_SPELLINGS: [(&str, &str); 7] = [
    ("help", "--help"),
    ("-h", "--help"),
    ("-help", "--help"),
    ("version", "--version"),
    ("-v", "--version"),
    ("-V", "--version"),
    ("-version", "--version"),
];

/// The first line of the usage screen.
const INTRO: &str = "Statac builds a website from a folder of Markdown.\n";

/// Each command's entry on the usage screen, as the screen sets it.
const USAGE: [(&str, &str); 10] = [
    (
        "build",
        "  statac build [folder]  Read the site and write it out, ready to publish.\n",
    ),
    (
        "check",
        "  statac check [folder]  Read the site and show every problem with it.\n",
    ),
    (
        "serve",
        "  statac serve [folder]  Build the site and serve it on this computer,
                         building it again each time you save.
",
    ),
    (
        "stamp",
        "  statac stamp [folder]  Write the date over every stamp, such as =today, in
                         the front matter of the site's pages.
",
    ),
    (
        "refresh",
        "  statac refresh [folder]
                         Fetch what the site keeps from other sites, such
                         as a video's thumbnail or a webmention, into
                         fetched/. Webmentions are asked for with the
                         token in STATAC_WEBMENTION_TOKEN, or else in
                         the site's .statac/token.
",
    ),
    (
        "new",
        "  statac new [folder]    Ask five questions and write a new site's settings
                         from the answers into the folder, which must be
                         empty or not there yet.
",
    ),
    (
        "import",
        "  statac import <folder> [new folder]
                         Write a new site from a site another tool made,
                         into the new folder, which must be empty or not
                         there yet; without one, beside the old site, its
                         name ending -statac. The old site is only read.
",
    ),
    (
        "search",
        "  statac search <words> [folder]
                         Show the pages a reader of the site would find
                         for the words, best first, each language's
                         apart on a site in several.
",
    ),
    (
        "explain",
        "  statac explain <name>  Explain a kind of problem. Its name is the last part
                         of the problem's More: link.
",
    ),
    (
        "lsp",
        "  statac lsp             Tell an editor the site's problems as you type. The
                         editor starts it.
",
    ),
];

/// What the usage screen says, after the entries of the commands that work
/// on a site, of where they work without a folder.
const NO_FOLDER: &str = "
Without a folder, statac build, check, serve, stamp, refresh and search
work on the site the current folder is in.
";

/// Each option's entry on the usage screen, as the screen sets it.
const OPTION_LINES: [(&str, &str); 15] = [
    (
        "--out",
        "  --out=<folder>         Where statac build writes the built site. Without
                         it, _site in the site's folder.
",
    ),
    (
        "--url",
        "  --url=<url>            The site's full URL for this run, in place of url
                         in statac.yaml, as a preview of the site needs.
",
    ),
    (
        "--today",
        "  --today=<date>         The day the site is built on, as 2026-10-04, for
                         site.today, the day stamps are counted from, and
                         the day statac refresh records. Without it, today
                         in the site's zone.
",
    ),
    (
        "--drafts",
        "  --drafts               Build drafts and scheduled posts as if they were
                         published.
",
    ),
    (
        "--json",
        "  --json                 Show every problem as JSON, for an editor or another
                         program to read, and nothing else.
",
    ),
    (
        "--timings",
        "  --timings              Show where statac build spent its time, once it
                         has finished.
",
    ),
    (
        "--no-drafts",
        "  --no-drafts            Leave drafts and scheduled posts out of what
                         statac serve shows, as statac build does.
",
    ),
    (
        "--offline",
        "  --offline              Fetch nothing as statac serve starts. Without it,
                         statac serve fetches what is new once, as statac
                         refresh does, after the first build with no
                         errors.
",
    ),
    (
        "--yes",
        "  --yes                  Take every answer statac new offers without asking.\n",
    ),
    (
        "--from",
        "  --from=<tool>          The tool that made the site statac import reads:
                         jekyll, hugo, zola or eleventy. Without it, told
                         from the site's files.
",
    ),
    (
        "--port",
        "  --port=<number>        The port statac serve answers on. Without it,
                         3000, or the next port that is free.
",
    ),
    (
        "--watch",
        "  --watch=<way>          How statac serve learns of a save: native, by
                         the system's own reports; poll, by looking at
                         every file twice a second; or auto, which
                         polls only where reports can't be relied on.
                         Without it, auto.
",
    ),
    (
        "--language",
        "  --language=<code>      Search only the pages in this language, named
                         by its code in statac.yaml.
",
    ),
    ("--help", "  --help                 Show this.\n"),
    (
        "--version",
        "  --version              Show which version of Statac this is.\n",
    ),
];

/// The usage screen: whole, or for one command, its entries and the
/// options it takes.
fn help(command: Option<&str>) -> String {
    let takes = |option: &str| {
        command.is_none_or(|command| {
            QUESTIONS.contains(&option)
                || OPTIONS
                    .iter()
                    .chain(&FLAGS)
                    .any(|(taken, takers)| *taken == option && takers.contains(&command))
        })
    };
    let mut screen = format!("{INTRO}\nUsage:\n");
    for (of, entry) in USAGE {
        if command.is_none_or(|command| command == of) {
            screen.push_str(entry);
        }
    }
    if command.is_none_or(|command| SITE_COMMANDS.contains(&command)) {
        screen.push_str(NO_FOLDER);
    }
    screen.push_str("\nOptions:\n");
    for (option, entry) in OPTION_LINES {
        if takes(option) {
            screen.push_str(entry);
        }
    }
    screen
}

/// Where a run writes, and whether each stream takes colour.
pub struct Terminal<'a> {
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
    pub colour_out: bool,
    pub colour_err: bool,
    /// How a long build says how far it has got.
    pub progress: Progress,
}

/// The stack of each thread that builds pages. The deepest writing read,
/// in a build without optimisation, uses under a fifth of it.
pub const STACK_SIZE: usize = 8 << 20;

/// Does what `args` (the program's name left off) ask, and gives the exit
/// code: 0 for a clean run or warnings only, 1 for errors, 2 for a mistake
/// on the command line, and 101 for a fault of Statac's own.
pub fn run(args: impl IntoIterator<Item = OsString>, terminal: &mut Terminal) -> u8 {
    // Fails only where a pool already exists, which then is the one used.
    let _ = rayon::ThreadPoolBuilder::new()
        .stack_size(STACK_SIZE)
        .build_global();
    let args: Vec<OsString> = args.into_iter().collect();
    let unreadable: Vec<bool> = args.iter().map(|arg| arg.to_str().is_none()).collect();
    let args: Vec<String> = args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let request = match parse_with(&args, &unreadable) {
        Ok(request) => request,
        Err(problems) => {
            let sources = Sources::default();
            let report = Report::new(&sources, Path::new(""), terminal.colour_err);
            say(terminal.err, &report.problems(&problems));
            return 2;
        }
    };
    let command = match &request {
        Request::Help(_) | Request::Version | Request::Explain(_) | Request::Lsp => None,
        Request::Build { .. } => Some("build"),
        Request::Check { .. } => Some("check"),
        Request::Stamp(_) => Some("stamp"),
        Request::Refresh(_) => Some("refresh"),
        Request::Import(_) => Some("import"),
        Request::New(_) => Some("new"),
        Request::Serve { .. } => Some("serve"),
        Request::Search(_) => Some("search"),
    };
    guarded(command, terminal, |terminal| {
        act(request, command, terminal)
    })
}

/// Runs `command`, and after Rust's own report of a panic in it, which is
/// what a report of the fault needs, says on the same stream that the
/// fault is Statac's own. A panic ends the run with the code Rust gives it.
fn guarded(
    command: Option<&str>,
    terminal: &mut Terminal,
    command_runs: impl FnOnce(&mut Terminal) -> u8,
) -> u8 {
    crate::publish::nothing_in_place();
    let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| command_runs(terminal)));
    ran.unwrap_or_else(|_| {
        let lines: &[&str] = match command {
            Some("build") if crate::publish::in_place() => &[wording::BUILD_STOPPED],
            Some("build") => &[wording::BUILD_STOPPED, wording::WROTE_NOTHING],
            Some("check") => &[wording::CHECK_STOPPED],
            _ => &[],
        };
        for line in lines.iter().chain(&[wording::REPORT_FAULT]) {
            say(terminal.err, &format!("{line}\n"));
        }
        101
    })
}

/// Does what `request` asks.
fn act(mut request: Request, command: Option<&str>, terminal: &mut Terminal) -> u8 {
    let json = matches!(request, Request::Check { json: true, .. });
    let timings = matches!(request, Request::Build { timings: true, .. });
    if let Some(command) = command
        && !json
    {
        let badge = crate::report::badge(terminal.colour_out, command);
        say(terminal.out, &badge);
    }
    let given_alone = match &request {
        Request::Search(options) => crate::search::folder_for_words(options),
        _ => None,
    };
    if let Some(problem) = given_alone {
        let sources = Sources::default();
        let report = Report::new(&sources, Path::new(""), terminal.colour_out);
        say(terminal.out, &report.problems(&[problem]));
        return 1;
    }
    if let Some(site) = site_of(&mut request)
        && let Err(problem) = find_site(site)
    {
        let sources = Sources::default();
        let report = Report::new(&sources, Path::new(""), terminal.colour_out);
        // Said as each command says it can't read the folder it was given.
        let said = match request {
            Request::Build { .. } | Request::Serve { .. } => {
                let outcome = Outcome::unread(problem);
                report.of_build(&outcome, Path::new(""), Duration::ZERO)
            }
            Request::Stamp(_) | Request::Search(_) => report.problems(&[problem]),
            _ if json => report.check_json(&[problem], &Quiet::default()),
            _ => report.check(&[problem], &Quiet::default()),
        };
        say(terminal.out, &said);
        return 1;
    }
    match request {
        Request::Help(command) => say(terminal.out, &help(command)),
        Request::Version => say(
            terminal.out,
            &format!("Statac {}\n", env!("CARGO_PKG_VERSION")),
        ),
        Request::Explain(code) => say(terminal.out, &code.explained()),
        Request::Lsp => {
            let input = std::io::BufReader::new(std::io::stdin());
            return crate::lsp::run(input, terminal.out);
        }
        Request::Build { options, .. } | Request::Check { options, .. } => {
            let started = Instant::now();
            let outcome = build::run_saying(&options, terminal.progress);
            let problems = &outcome.problems;
            let report = Report::new(outcome.sources(), &options.site, terminal.colour_out);
            if json {
                say(terminal.out, &report.check_json(problems, &outcome.quiet));
                return u8::from(outcome.stopped());
            }
            let said = match options.mode {
                Mode::Build => {
                    let took = started.elapsed().saturating_sub(outcome.waited);
                    report.of_build(&outcome, &options.out(), took)
                }
                Mode::Check => {
                    // A stopped run leaves pages out, so what it carries is unknown.
                    let javascript = match outcome.stopped() {
                        true => &[],
                        false => outcome.javascript.as_slice(),
                    };
                    report.check_with(problems, &outcome.quiet, javascript)
                }
            };
            say(terminal.out, &report.included(outcome.included));
            say(terminal.out, &said);
            if timings {
                say(
                    terminal.out,
                    &format!("\n{}", report.timings(&outcome.timings)),
                );
            }
            if outcome.stopped() {
                return 1;
            }
        }
        Request::Stamp(options) => {
            let outcome = stamp::run(&options);
            let nowhere = Sources::default();
            let sources = outcome.sources().unwrap_or(&nowhere);
            let report = Report::new(sources, &options.site, terminal.colour_out);
            // The problems come between what was stamped and that nothing
            // was, which is the only thing said after an error.
            let stamps = report.stamps(&outcome.pass);
            let problems = report.problems(&outcome.problems);
            let gap = if stamps.is_empty() || problems.is_empty() {
                ""
            } else {
                "\n"
            };
            let said = match (
                outcome.pass.unsettled,
                stamps.is_empty() && problems.is_empty(),
            ) {
                (true, _) => format!("{problems}{gap}{stamps}"),
                (false, true) if outcome.pass.unrecorded => String::new(),
                (false, true) => format!("{}\n", wording::NOTHING_TO_STAMP),
                (false, false) => format!("{stamps}{gap}{problems}"),
            };
            say(terminal.out, &said);
            let said = match outcome.pass.unrecorded {
                true => format!("{}\n", wording::IDS_NOT_WRITTEN),
                false => String::new(),
            };
            say(terminal.out, &said);
            if !outcome.problems.is_empty() || outcome.pass.unsettled {
                return 1;
            }
        }
        Request::Refresh(options) => {
            let token = token(&options.site, terminal.out);
            let options = refresh::Options { token, ..options };
            return refresh::run(
                &options,
                &crate::fetch::fetch,
                terminal.out,
                terminal.colour_out,
            );
        }
        Request::Search(options) => {
            let outcome = search::run(&options);
            let nowhere = Sources::default();
            let sources = outcome.sources().unwrap_or(&nowhere);
            let report = Report::new(sources, &options.site, terminal.colour_out);
            if !outcome.errors.is_empty() {
                say(terminal.out, &report.problems(&outcome.errors));
                return 1;
            }
            // A code the site doesn't declare is a mistake on the command
            // line, told as one is.
            if let Some(mistake) = &outcome.mistake {
                let report = Report::new(&nowhere, Path::new(""), terminal.colour_err);
                say(
                    terminal.err,
                    &report.problems(std::slice::from_ref(mistake)),
                );
                return 2;
            }
            say(terminal.out, &report.found(&options.words, &outcome.found));
        }
        Request::Import(options) => {
            let stdin = std::io::stdin();
            let answering = std::io::IsTerminal::is_terminal(&stdin);
            let mut reader = stdin.lock();
            let input = new::Input {
                reader: &mut reader,
                terminal: answering,
            };
            return import::run(&options, Some(input), terminal.out, terminal.colour_out);
        }
        Request::New(options) => {
            let stdin = std::io::stdin();
            let answering = std::io::IsTerminal::is_terminal(&stdin);
            let mut reader = stdin.lock();
            let machine = new::Machine {
                input: Some(new::Input {
                    reader: &mut reader,
                    terminal: answering,
                }),
                locale: new::Locale::of_this_machine(),
                timezone: new::zone_of_this_machine(),
            };
            return new::run(&options, machine, terminal.out, terminal.colour_out);
        }
        Request::Serve { options, offline } => {
            let token = token(&options.site, terminal.out);
            let options = serve::Options {
                fetcher: (!offline).then(|| serve::Fetcher::new(crate::fetch::fetch)),
                token,
                ..options
            };
            let Some(mut serving) = serve::start(&options, terminal.out, terminal.colour_out)
            else {
                return 1;
            };
            // Serving ends when the author ends the program.
            while serving.round(terminal.out) {}
            return 1;
        }
    }
    0
}

/// The site's folder of a request that works on a site.
fn site_of(request: &mut Request) -> Option<&mut PathBuf> {
    match request {
        Request::Build { options, .. } | Request::Check { options, .. } => Some(&mut options.site),
        Request::Serve { options, .. } => Some(&mut options.site),
        Request::Stamp(options) => Some(&mut options.site),
        Request::Refresh(options) => Some(&mut options.site),
        Request::Search(options) => Some(&mut options.site),
        Request::New(_)
        | Request::Import(_)
        | Request::Explain(_)
        | Request::Lsp
        | Request::Help(_)
        | Request::Version => None,
    }
}

/// Settles `site`, as the command line left it, on the folder of the site
/// to work on. A folder named is taken as named, and refused where it is
/// there and isn't a site; one that isn't there is left for the command
/// to tell. Where none is named, the site is the nearest folder that is
/// one, from the current folder up, named from the current folder.
fn find_site(site: &mut PathBuf) -> Result<(), Problem> {
    let no_site = |folder: String| Problem::nowhere(Kind::NoSiteHere { folder });
    if !site.as_os_str().is_empty() {
        let there = std::fs::metadata(&*site).is_ok_and(|found| found.is_dir());
        return match there && matches!(new::site_mark(site), Ok(None)) {
            true => Err(no_site(site.display().to_string())),
            false => Ok(()),
        };
    }
    let cant_read = |path: PathBuf, error: &std::io::Error| {
        Problem::nowhere(Kind::CantRead {
            path,
            reason: crate::problem::Reason::of(error),
        })
    };
    let here = std::env::current_dir().map_err(|error| cant_read(PathBuf::from("."), &error))?;
    for (up, folder) in here.ancestors().enumerate() {
        let named = match up {
            0 => PathBuf::from("."),
            _ => std::iter::repeat_n("..", up).collect(),
        };
        match new::site_mark(folder) {
            Ok(Some(_)) => {
                *site = named;
                return Ok(());
            }
            Ok(None) => {}
            Err(error) => return Err(cant_read(named, &error)),
        }
    }
    Err(no_site(".".into()))
}

/// The relay's token for the site at `site`, from the environment or the
/// site's own file, saying on `out` why a file that is there isn't used.
fn token(site: &Path, out: &mut dyn Write) -> Option<refresh::Token> {
    let variable = std::env::var(refresh::TOKEN_VARIABLE).ok();
    let (token, unused) = refresh::token(site, variable);
    if let Some(line) = unused {
        say(out, &format!("{}\n", crate::report::shown(&line)));
    }
    token
}

/// A stream that can't be written to is let go: a reader that stops
/// reading, as `head` does, is no mistake of the author's.
fn say(stream: &mut dyn Write, text: &str) {
    let _ = stream.write_all(text.as_bytes());
    let _ = stream.flush();
}

/// Reads the command line, or gives every mistake in it, in order.
pub fn parse(args: &[String]) -> Result<Request, Vec<Problem>> {
    parse_with(args, &[])
}

/// A folder or path given on the command line that Statac can't use: one
/// holding bytes that aren't UTF-8 (`unreadable`, its lossy reading
/// being `written`), or a control character.
fn unusable_name(written: &str, unreadable: bool) -> Option<Kind> {
    let written = written.to_owned();
    if unreadable {
        Some(Kind::UnreadableName { written })
    } else if written.chars().any(char::is_control) {
        Some(Kind::UnusableName { written })
    } else {
        None
    }
}

/// [`parse`], given for each of `args` whether the system's own reading
/// of it held bytes that aren't UTF-8, which `args` then has replaced.
fn parse_with(args: &[String], unreadable: &[bool]) -> Result<Request, Vec<Problem>> {
    let unreadable = |at: usize| unreadable.get(at).copied().unwrap_or(false);
    if args.is_empty() {
        return Ok(Request::Help(None));
    }
    if let Some(asked) = args.iter().position(|arg| arg == "--help") {
        let command = command_at(args)
            .filter(|&at| at < asked)
            .and_then(|at| COMMANDS.into_iter().find(|&command| command == args[at]));
        return Ok(Request::Help(command));
    }
    if args.iter().any(|arg| arg == "--version") {
        return Ok(Request::Version);
    }
    let at = command_at(args);
    let command = at.map(|i| args[i].as_str());
    let known = command.filter(|command| COMMANDS.contains(command));
    let mut problems = Vec::new();
    let mut mistake = |kind| problems.push(Problem::nowhere(kind));
    let mut site = None;
    // Whether a folder for the site was written, usable or not.
    let mut site_named = false;
    let mut out = None;
    let mut url = None;
    let mut today = None;
    let mut drafts = false;
    let mut json = false;
    let mut timings = false;
    let mut no_drafts = false;
    let mut offline = false;
    let mut yes = false;
    let mut port = None;
    let mut watch = None;
    let mut language = None;
    let wrong = |option: &str, expected, written: &str| Kind::WrongKind {
        field: option.into(),
        expected,
        found: Found::Written(written.into()),
        respelt: None,
    };
    let mut name = None;
    // The words `search` looks for.
    let mut words = None;
    // The folder `new` makes the site in.
    let mut folder = None;
    // The folders `import` reads from and writes into.
    let mut old = None;
    // Whether a folder to read from was written, usable or not.
    let mut old_named = false;
    let mut new_folder = None;
    let mut from = None;
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if !is_option(arg) {
            if Some(i - 1) == at {
                if known.is_none() {
                    mistake(Kind::UnknownCommand {
                        written: arg.into(),
                        hint: command_hint(arg),
                    });
                }
            } else if known == Some("explain") && name.is_none() {
                name = Some(arg);
                if Code::from_name(arg).is_none() {
                    mistake(Code::unknown(arg));
                }
            } else if known == Some("search") && words.is_none() {
                words = Some(arg);
            } else if known == Some("new") && folder.is_none() {
                match unusable_name(arg, unreadable(i - 1)) {
                    Some(kind) => mistake(kind),
                    None => folder = Some(PathBuf::from(arg)),
                }
            } else if known == Some("import") && old.is_none() {
                old_named = true;
                match unusable_name(arg, unreadable(i - 1)) {
                    Some(kind) => mistake(kind),
                    None => old = Some(PathBuf::from(arg)),
                }
            } else if known == Some("import") && new_folder.is_none() {
                match unusable_name(arg, unreadable(i - 1)) {
                    Some(kind) => mistake(kind),
                    None => new_folder = Some(PathBuf::from(arg)),
                }
            } else if known.is_some_and(|command| SITE_COMMANDS.contains(&command)) && !site_named {
                site_named = true;
                match unusable_name(arg, unreadable(i - 1)) {
                    Some(kind) => mistake(kind),
                    None => site = Some(PathBuf::from(arg)),
                }
            } else if known.is_some() {
                mistake(Kind::UnexpectedArgument {
                    written: arg.into(),
                });
            }
            // After an unknown command, what follows can't be judged.
            continue;
        }

        let (written_name, value) = match arg.split_once('=') {
            Some((written_name, value)) => (written_name, Some(value)),
            None => (arg, None),
        };
        if written_name == RETIRED_SITE {
            // Its folder given apart is taken with it, as an option's value is.
            let folder = match value {
                None if takes_apart(args, i, at) => {
                    i += 1;
                    Some(args[i - 1].clone())
                }
                _ => value.filter(|value| !value.is_empty()).map(str::to_owned),
            };
            let hint = match known {
                Some("explain" | "lsp") => hint(written_name, value, known),
                _ => OptionHint::FolderAfterCommand {
                    command: known.map(str::to_owned),
                    folder,
                },
            };
            mistake(Kind::UnknownOption {
                written: arg.into(),
                hint,
            });
            continue;
        }
        if RETIRED_OPTIONS.contains(&written_name) {
            mistake(Kind::UnknownOption {
                written: arg.into(),
                hint: OptionHint::Retired,
            });
            continue;
        }
        if let Some(&(flag, takers)) = FLAGS.iter().find(|(f, _)| *f == written_name) {
            if value.is_some() {
                mistake(Kind::UnknownOption {
                    written: arg.into(),
                    hint: OptionHint::Similar(flag.into()),
                });
            } else if let Some(command) = known
                && !takers.contains(&command)
            {
                let hint = match (command, flag) {
                    ("serve", "--drafts") => OptionHint::ShownAlready,
                    _ => OptionHint::ForOther {
                        command: command.into(),
                        option: flag.into(),
                        takers: takers.into(),
                    },
                };
                mistake(Kind::UnknownOption {
                    written: arg.into(),
                    hint,
                });
            } else {
                let given = match flag {
                    "--drafts" => &mut drafts,
                    "--json" => &mut json,
                    "--timings" => &mut timings,
                    "--no-drafts" => &mut no_drafts,
                    "--yes" => &mut yes,
                    _ => &mut offline,
                };
                if std::mem::replace(given, true) {
                    mistake(Kind::OptionGivenTwice {
                        option: flag.into(),
                    });
                }
            }
            continue;
        }
        let Some(&(option, takers)) = OPTIONS.iter().find(|(o, _)| *o == written_name) else {
            mistake(Kind::UnknownOption {
                written: arg.into(),
                hint: hint(written_name, value, known),
            });
            continue;
        };
        // `--out public` is the value given apart, and taken with its option.
        let apart = match value {
            None if takes_apart(args, i, at) => {
                i += 1;
                Some(args[i - 1].clone())
            }
            _ => None,
        };
        if let Some(command) = known
            && !takers.contains(&command)
        {
            mistake(Kind::UnknownOption {
                written: arg.into(),
                hint: OptionHint::ForOther {
                    command: command.into(),
                    option: option.into(),
                    takers: takers.into(),
                },
            });
            continue;
        }
        let value = match value {
            Some(value) if !value.is_empty() => value,
            _ => {
                mistake(Kind::MissingValue {
                    written: arg.into(),
                    option: option.into(),
                    value: apart,
                });
                continue;
            }
        };
        if option == "--today" {
            match (today, day(option, value)) {
                (Some(_), _) => mistake(Kind::OptionGivenTwice {
                    option: option.into(),
                }),
                (None, Ok(date)) => today = Some(date),
                (None, Err(kind)) => mistake(kind),
            }
            continue;
        }
        if option == "--url" {
            match (&url, crate::config::site_address(value)) {
                (Some(_), _) => mistake(Kind::OptionGivenTwice {
                    option: option.into(),
                }),
                (None, true) => url = Some(value.to_owned()),
                (None, false) => {
                    let https = format!("https://{value}");
                    let respelt = (!value.contains("://") && crate::config::site_address(&https))
                        .then(|| https.into_boxed_str());
                    mistake(Kind::WrongKind {
                        field: option.into(),
                        expected: Expected::SiteAddress,
                        found: Found::Written(value.into()),
                        respelt,
                    });
                }
            }
            continue;
        }
        if option == "--port" {
            match (port, value.parse::<u16>()) {
                (Some(_), _) => mistake(Kind::OptionGivenTwice {
                    option: option.into(),
                }),
                (None, Ok(number)) if number > 0 => port = Some(number),
                (None, _) => mistake(wrong(
                    option,
                    Expected::Between { from: 1, to: 65535 },
                    value,
                )),
            }
            continue;
        }
        if option == "--from" {
            match (from, import::Tool::from_word(value)) {
                (Some(_), _) => mistake(Kind::OptionGivenTwice {
                    option: option.into(),
                }),
                (None, Some(tool)) => from = Some(tool),
                (None, None) => {
                    let tools = import::Tool::ALL
                        .iter()
                        .map(|t| t.word().to_owned())
                        .collect();
                    mistake(wrong(option, Expected::OneOf(tools), value));
                }
            }
            continue;
        }
        // Read against the codes the site declares once it is read.
        if option == "--language" {
            match language {
                Some(_) => mistake(Kind::OptionGivenTwice {
                    option: option.into(),
                }),
                None => language = Some(value.to_owned()),
            }
            continue;
        }
        if option == "--watch" {
            let how = match value {
                "auto" => Some(How::Auto),
                "native" => Some(How::Native),
                "poll" => Some(How::Poll { every: watch::POLL }),
                _ => None,
            };
            match (watch, how) {
                (Some(_), _) => mistake(Kind::OptionGivenTwice {
                    option: option.into(),
                }),
                (None, Some(how)) => watch = Some(how),
                (None, None) => {
                    let ways = WAYS.iter().map(|way| way.to_string()).collect();
                    mistake(wrong(option, Expected::OneOf(ways), value));
                }
            }
            continue;
        }
        match (&out, unusable_name(value, unreadable(i - 1))) {
            (Some(_), _) => mistake(Kind::OptionGivenTwice {
                option: option.into(),
            }),
            (None, Some(kind)) => mistake(kind),
            (None, None) => out = Some(PathBuf::from(value)),
        }
    }
    if command.is_none() {
        problems.insert(0, Problem::nowhere(Kind::MissingCommand));
    }
    if known == Some("explain") && name.is_none() {
        problems.push(Problem::nowhere(Kind::MissingName));
    }
    if known == Some("search") && words.is_none_or(|words| words.trim().is_empty()) {
        problems.push(Problem::nowhere(Kind::NothingToSearch { site: None }));
    }
    if known == Some("import") && !old_named {
        problems.push(Problem::nowhere(Kind::MissingFolder));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    // A site named by neither is left empty, to be found as the command runs.
    let options = |mode| Options {
        mode,
        site: site.clone().unwrap_or_default(),
        out: out.clone(),
        today,
        drafts,
        url: url.clone(),
    };
    Ok(match known {
        Some("build") => Request::Build {
            options: options(Mode::Build),
            timings,
        },
        Some("check") => Request::Check {
            options: options(Mode::Check),
            json,
        },
        Some("serve") => Request::Serve {
            options: serve::Options {
                site: site.unwrap_or_default(),
                port: port.unwrap_or(serve::PORT),
                watch: watch.unwrap_or(How::Auto),
                settle: watch::Settle::default(),
                today,
                drafts: !no_drafts,
                fetcher: None,
                token: None,
            },
            offline,
        },
        Some("stamp") => Request::Stamp(stamp::Options {
            site: site.unwrap_or_default(),
            today,
        }),
        Some("refresh") => Request::Refresh(refresh::Options {
            site: site.unwrap_or_default(),
            today,
            token: None,
        }),
        Some("new") => Request::New(new::Options { folder, yes }),
        Some("import") => Request::Import(import::Options {
            old: old.expect("checked above"),
            new: new_folder,
            from,
            today,
        }),
        Some("search") => Request::Search(crate::search::Options {
            site: site.unwrap_or_default(),
            words: words.expect("checked above").to_owned(),
            today,
            drafts,
            language,
        }),
        Some("lsp") => Request::Lsp,
        _ => Request::Explain(name.and_then(Code::from_name).expect("checked above")),
    })
}

/// A day written as ISO 8601, `2026-10-04`, given to `option`.
fn day(option: &str, written: &str) -> Result<Date, Kind> {
    let wrong = |respelt: Option<String>| Kind::WrongKind {
        field: option.into(),
        expected: Expected::Date,
        found: Found::Written(written.into()),
        respelt: respelt.map(String::into_boxed_str),
    };
    let missing = |missing| Kind::NoSuchDate {
        written: written.into(),
        missing,
    };
    match date::read(written) {
        // The build has one day, so a time or an offset would be dropped unsaid.
        Ok(date) if date.time.is_some() || date.offset.is_some() => {
            Err(wrong(Some(date.iso_day())))
        }
        Ok(date) => Ok(date),
        Err(Fault::NotIso { iso }) => Err(wrong(iso)),
        Err(Fault::Unclear { readings }) => Err(Kind::UnclearDate {
            written: written.into(),
            readings: Box::new(readings),
        }),
        Err(Fault::NoSuchDay { day, month, year }) => {
            Err(missing(Missing::Day { day, month, year }))
        }
        Err(Fault::NoSuchMonth(month)) => Err(missing(Missing::Month(month))),
        Err(Fault::NoSuchTime(time)) => Err(missing(Missing::Time(time))),
        Err(Fault::NoSuchOffset(offset)) => Err(missing(Missing::Offset(offset))),
    }
}

fn is_option(arg: &str) -> bool {
    arg.starts_with('-') && arg != "-"
}

/// The command is the first argument that is neither an option nor an
/// option's value given apart.
fn command_at(args: &[String]) -> Option<usize> {
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].as_str();
        i += 1;
        if !is_option(arg) {
            return Some(i - 1);
        }
        let takes_value = arg == RETIRED_SITE || OPTIONS.iter().any(|(o, _)| *o == arg);
        if takes_value && takes_apart(args, i, None) {
            i += 1;
        }
    }
    None
}

/// Whether the argument at `i` reads as the value of the option before it.
fn takes_apart(args: &[String], i: usize, command: Option<usize>) -> bool {
    args.get(i).is_some_and(|next| !is_option(next)) && command != Some(i)
}

/// What to say of `written`, which names no command.
fn command_hint(written: &str) -> CommandHint {
    if let Some(&(_, setting)) = FORMER_COMMANDS
        .iter()
        .find(|(former, _)| *former == written)
    {
        return CommandHint::Setting(setting.into());
    }
    match suggest::nearest(written, COMMANDS).or_else(|| other_spelling(written)) {
        Some(near) => CommandHint::Similar(near.into()),
        None => CommandHint::None,
    }
}

fn other_spelling(written: &str) -> Option<&'static str> {
    OTHER_SPELLINGS
        .iter()
        .find(|(other, _)| *other == written)
        .map(|&(_, ours)| ours)
}

/// What can be said about an option Statac doesn't have, given the command
/// it was meant for.
fn hint(written: &str, value: Option<&str>, command: Option<&str>) -> OptionHint {
    if let Some(ours) = other_spelling(written) {
        return OptionHint::Similar(ours.into());
    }
    let taken: Vec<&'static str> = OPTIONS
        .iter()
        .chain(&FLAGS)
        .filter(|(_, takers)| command.is_none_or(|command| takers.contains(&command)))
        .map(|&(option, _)| option)
        .collect();
    let candidates = taken.iter().copied().chain(QUESTIONS);
    if let Some(near) = suggest::nearest(written, candidates) {
        return OptionHint::Similar(match value {
            Some(value)
                if !QUESTIONS.contains(&near) && !FLAGS.iter().any(|(flag, _)| *flag == near) =>
            {
                format!("{near}={value}")
            }
            _ => near.into(),
        });
    }
    match command {
        Some(command) => OptionHint::Takes {
            command: command.into(),
            options: taken.into(),
        },
        None => OptionHint::None,
    }
}

#[cfg(test)]
mod tests;
