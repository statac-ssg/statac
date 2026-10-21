//! The site's own commands: `before_build`, run before the site is read,
//! and `after_build`, run on the built site before it is published.
//!
//! Each is a line for the system's shell, run in the site's folder with
//! nothing to read from. What a command prints is kept, both streams as
//! one in the order written, and shown only where the command doesn't end
//! well.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::album::Progress;
use crate::config::Settings;
use crate::platform;
use crate::problem::{Ended, Hint, Kind, Named, Problem, Severity, WrongPlace};
use crate::publish::Staging;
use crate::source::Sources;

pub const BEFORE: &str = "before_build";
pub const AFTER: &str = "after_build";
/// The setting naming what `after_build` makes in the built site.
pub const MAKES: &str = "after_build_makes";
/// The setting naming what `before_build` is run again for.
pub const WATCHES: &str = "before_build_watches";

/// One of the site's commands, and what it is handed.
pub struct Hook<'a> {
    /// The setting it is written in.
    pub setting: &'static str,
    pub line: &'a str,
    /// The site's folder, which the command runs in.
    pub site: &'a Path,
    /// The folder the command is to act on.
    pub out: &'a Path,
    pub drafts: bool,
    /// Whether `statac serve` runs it, on a site being previewed.
    pub serve: bool,
}

/// A command that didn't end well: how it ended, and what it printed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Stopped {
    pub ended: Ended,
    pub said: String,
}

impl Hook<'_> {
    /// [`Hook::run`], with a terminal shown that the command is running
    /// for as long as it is: what the command prints is kept back, and a
    /// long one would otherwise look like a build that has stopped.
    pub fn run_saying(&self, progress: Progress) -> Result<(), Stopped> {
        if progress != Progress::Line {
            return self.run();
        }
        let say = |line: &str| {
            let mut err = std::io::stderr().lock();
            let _ = write!(err, "\r{line}\x1b[K");
            let _ = err.flush();
        };
        say(&crate::wording::running_hook(self.setting));
        let ended = self.run();
        say("");
        ended
    }

    /// Runs the command and waits for it.
    pub fn run(&self) -> Result<(), Stopped> {
        let not_started = |error: std::io::Error| Stopped {
            ended: Ended::NotStarted(crate::problem::Reason::of(&error)),
            said: String::new(),
        };
        // One pipe for both streams, so what was printed is read in the
        // order it was written.
        let (mut printed, out) = std::io::pipe().map_err(not_started)?;
        let err = out.try_clone().map_err(not_started)?;
        let mut command = platform::shell(self.line);
        command
            .current_dir(self.site)
            .env("STATAC_SITE", absolute(self.site))
            .env("STATAC_OUT", absolute(self.out))
            .env("STATAC_DRAFTS", if self.drafts { "yes" } else { "no" })
            .env("STATAC_SERVE", if self.serve { "yes" } else { "no" })
            .env("STATAC_HOOK", self.setting)
            .stdin(Stdio::null())
            .stdout(out)
            .stderr(err);
        let child = command.spawn();
        // The pipe ends only once every writing end is closed, and the
        // command holds two of them.
        drop(command);
        let mut child = child.map_err(not_started)?;
        let mut said = Vec::new();
        let _ = printed.read_to_end(&mut said);
        let status = child.wait().map_err(not_started)?;
        if status.success() {
            return Ok(());
        }
        Err(Stopped {
            ended: status.code().map_or(Ended::Cut, Ended::Code),
            said: String::from_utf8_lossy(&said).into_owned(),
        })
    }
}

/// A command handed a path may have gone to another folder before it reads
/// it.
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
}

/// What became of a site's `before_build`.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub enum Before {
    /// The site sets none, or the run is one that runs none.
    #[default]
    Unset,
    Ran,
    /// Not run, since the settings it is written in have an error.
    PassedOver,
}

/// What `after_build` did to the built site.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Changed {
    /// The files it added, those it wrote over, by their length or when
    /// they were last changed, and those it removed.
    pub added: usize,
    pub changed: usize,
    pub removed: usize,
}

/// What became of a site's own commands in one build.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Ran {
    pub before: Before,
    /// What `after_build` did, where it ran.
    pub after: Option<Changed>,
}

/// Runs the `before_build` of the site at `site`, read from its settings
/// alone since the command may write what the site is read from. The
/// command isn't run where the settings have an error, which the build
/// goes on to tell. `Err` holds a command that didn't end well, and the
/// settings it is placed in.
pub fn before(
    options: &crate::build::Options,
    progress: Progress,
    serve: bool,
) -> Result<Before, Box<(Sources, Problem)>> {
    let (site, out) = (options.site.as_path(), options.out());
    let mut sources = Sources::default();
    let (settings, problems) = crate::config::read(site, &mut sources);
    let stopped = problems
        .iter()
        .any(|problem| problem.kind.severity() == Severity::Error);
    let Some(line) = settings.text(BEFORE) else {
        // Settings that couldn't be read at all may still set one.
        let unread = stopped
            && settings
                .source
                .is_some_and(|source| sources.text(source).contains(BEFORE));
        return Ok(match unread {
            true => Before::PassedOver,
            false => Before::Unset,
        });
    };
    if stopped {
        return Ok(Before::PassedOver);
    }
    let hook = Hook {
        setting: BEFORE,
        line: line.value,
        site,
        out: &out,
        drafts: options.drafts,
        serve,
    };
    match hook.run_saying(progress) {
        Ok(()) => Ok(Before::Ran),
        Err(stopped) => {
            let problem = Problem::placed(told(BEFORE, stopped), line.origin.place);
            Err(Box::new((sources, problem)))
        }
    }
}

fn told(setting: &str, stopped: Stopped) -> Kind {
    Kind::HookStopped {
        hook: setting.to_owned(),
        ended: stopped.ended,
        said: stopped.said,
    }
}

/// Puts the site `staging` has written in the output's place, once the
/// site's `after_build` has run on it where the site sets one. Gives what
/// the command changed, and anything `after_build_makes` names that it
/// left nothing at. A command that doesn't end well publishes nothing.
pub fn publish(
    settings: &Settings,
    options: &crate::build::Options,
    mut staging: Staging,
    progress: Progress,
    serve: bool,
) -> Result<(Option<Changed>, Vec<Problem>), Problem> {
    let ran = after(settings, options, &mut staging, progress, serve)?;
    staging.finish()?;
    Ok(ran)
}

/// Runs the site's `after_build`, where it sets one, on the folder
/// `staging` has written, which must hold the whole site.
fn after(
    settings: &Settings,
    options: &crate::build::Options,
    staging: &mut Staging,
    progress: Progress,
    serve: bool,
) -> Result<(Option<Changed>, Vec<Problem>), Problem> {
    let Some(line) = settings.text(AFTER) else {
        return Ok((None, Vec::new()));
    };
    // Whatever an earlier build's command left in the folder goes first.
    staging.settle()?;
    let before = staging.files_to_compare()?;
    let hook = Hook {
        setting: AFTER,
        line: line.value,
        site: &options.site,
        out: staging.root(),
        drafts: options.drafts,
        serve,
    };
    hook.run_saying(progress)
        .map_err(|stopped| Problem::placed(told(AFTER, stopped), line.origin.place))?;
    let after = staging.files()?;
    let changed = Changed {
        added: after.names().filter(|file| !before.holds(file)).count(),
        changed: after.changed_since(&before).len(),
        removed: before.names().filter(|file| !after.holds(file)).count(),
    };
    let held = |rel: &str, folder: bool| {
        (!folder && after.holds(rel)) || after.names().any(|file| within(file, rel))
    };
    Ok((Some(changed), unmade(settings, held)))
}

/// Each place `after_build_makes` names that the built site has nothing
/// at. `held` says whether it has, of a path from its top and whether the
/// path is written as a folder.
fn unmade(settings: &Settings, held: impl Fn(&str, bool) -> bool) -> Vec<Problem> {
    let unmade = settings.texts(MAKES).iter().filter(|place| {
        let (rel, folder) = made(&place.value);
        !held(rel, folder)
    });
    let told = unmade.map(|place| {
        let kind = Kind::NothingMade {
            place: place.value.clone(),
        };
        Problem::placed(kind, place.origin.place)
    });
    told.collect()
}

/// Each place `after_build_makes` names that the built site in `folder`
/// has nothing at, where the command ran on an earlier build of it.
pub fn still_unmade(settings: &Settings, folder: &Path) -> Vec<Problem> {
    unmade(settings, |rel, as_folder| {
        let at = platform::join(folder, rel);
        match std::fs::symlink_metadata(&at) {
            Ok(found) if found.is_dir() => holds_a_file(&at),
            Ok(_) => !as_folder,
            Err(_) => false,
        }
    })
}

/// Whether `folder` holds a file at any depth.
fn holds_a_file(folder: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return false;
    };
    entries.flatten().any(|entry| match entry.file_type() {
        Ok(kind) if kind.is_dir() => holds_a_file(&entry.path()),
        Ok(_) => true,
        Err(_) => false,
    })
}

/// Whether any of `changed`, files of the site by their paths from its
/// folder with forward slashes, is one `before_build_watches` names or
/// lies in a folder it names.
pub fn watched(settings: &Settings, changed: &[String]) -> bool {
    settings.texts(WATCHES).iter().any(|watched| {
        let named = platform::folded(watched.value.trim_end_matches('/'));
        changed.iter().any(|file| {
            let file = platform::folded(file);
            file == named || within(&file, &named)
        })
    })
}

/// What an entry of `after_build_makes` names, from the top of the built
/// site, and whether it is written as a folder.
fn made(written: &str) -> (&str, bool) {
    let rel = written.strip_prefix('/').unwrap_or(written);
    match rel.strip_suffix('/') {
        Some(folder) => (folder, true),
        None => (rel, false),
    }
}

/// Whether `file` is inside `folder`, both paths with forward slashes.
fn within(file: &str, folder: &str) -> bool {
    file.strip_prefix(folder)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether `written` reads as a file or a folder below some folder: no
/// piece of it empty, climbing out, or naming a drive or a root.
pub fn a_place(written: &str) -> bool {
    let rel = written.strip_suffix('/').unwrap_or(written);
    !rel.is_empty() && platform::stays_inside(rel) && !rel.split('/').any(str::is_empty)
}

/// Whether `written` names a file or a folder in the built site, as an
/// entry of `after_build_makes` does: from its top, with or without the
/// `/` an address opens with.
pub fn a_built_place(written: &str) -> bool {
    a_place(written.strip_prefix('/').unwrap_or(written))
}

/// What is wrong with how the settings about the site's commands stand
/// together, for the site at `site`: one describing a command the site
/// doesn't set, and something `before_build_watches` names that the site
/// doesn't have.
pub fn check(site: &Path, settings: &Settings) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (about, command) in [(MAKES, AFTER), (WATCHES, BEFORE)] {
        if settings.text(command).is_none()
            && let Some(written) = settings.get(about)
        {
            let kind = Kind::WrongPlace {
                key: about.to_owned(),
                why: WrongPlace::NoCommand(command),
            };
            problems.push(Problem::placed(kind, written.origin.place));
        }
    }
    for watched in settings.texts(WATCHES) {
        let path = platform::join(site, &watched.value);
        let found = match watched.value.ends_with('/') {
            true => path.is_dir(),
            false => path.exists(),
        };
        if !found {
            let kind = Kind::MissingFile {
                path: watched.value.clone(),
                hint: None::<Hint>,
            };
            problems.push(Problem::placed(kind, watched.origin.place));
        }
    }
    problems
}

/// Each of `outputs`, a path in the built site and what is published
/// there, that stands where `after_build_makes` says the command makes
/// something. `base` is the path the site is published under.
pub fn claimed(settings: &Settings, outputs: &[(String, Named)], base: &str) -> Vec<Problem> {
    let mut problems = Vec::new();
    for place in settings.texts(MAKES) {
        let (rel, _) = made(&place.value);
        let folded = platform::folded(rel);
        let taken = outputs.iter().find(|(file, _)| {
            let file = platform::folded(file);
            file == folded || within(&file, &folded) || within(&folded, &file)
        });
        if let Some((file, source)) = taken {
            // Told as two things at one address whatever their spellings:
            // the command's own spelling isn't known until it has run.
            let kind = Kind::SameAddress {
                first: Box::new(source.clone()),
                second: Box::new(Named::MadeAfter(place.value.clone())),
                alike: platform::Alike::Exactly,
                address: format!("{base}/{file}"),
            };
            problems.push(Problem::placed(kind, place.origin.place));
        }
    }
    problems
}

/// How the system's shell spells the folder a command is to act on.
const HANDED: &str = match cfg!(windows) {
    true => "%STATAC_OUT%",
    false => "$STATAC_OUT",
};

/// Spellings of the site's own folder before a name, which leave the name
/// a path in the site's folder.
const FROM_THE_SITE: &[&str] = &[
    "$STATAC_SITE/",
    "${STATAC_SITE}/",
    "%STATAC_SITE%/",
    "%STATAC_SITE%\\",
    "./",
    ".\\",
];

/// Where `line` names `folder` as a path of its own and not as part of
/// another name: each by where the path begins, where the folder's name
/// begins, and where it ends.
fn naming(line: &str, folder: &str) -> Vec<(usize, usize, usize)> {
    let in_a_name = |c: char| c.is_alphanumeric() || matches!(c, '_' | '-' | '.');
    let mut found = Vec::new();
    for (at, _) in line.match_indices(folder) {
        let end = at + folder.len();
        if line[end..].chars().next().is_some_and(in_a_name) {
            continue;
        }
        let before = &line[..at];
        let from = FROM_THE_SITE
            .iter()
            .find(|spelt| before.ends_with(**spelt))
            .map_or(at, |spelt| at - spelt.len());
        let part_of_another = line[..from]
            .chars()
            .next_back()
            .is_some_and(|c| in_a_name(c) || matches!(c, '/' | '\\'));
        if !part_of_another {
            found.push((from, at, end));
        }
    }
    found
}

/// `line` with the folder Statac hands a command in place of each path
/// `named` gives, in quotes of its own where it isn't inside a pair.
fn rewritten(line: &str, named: &[(usize, usize, usize)]) -> String {
    let mut out = String::with_capacity(line.len());
    let mut done = 0;
    for &(from, _, end) in named {
        out.push_str(&line[done..from]);
        match line[..from].matches('"').count() % 2 {
            0 => out.push_str(&format!("\"{HANDED}\"")),
            _ => out.push_str(HANDED),
        }
        done = end;
    }
    out.push_str(&line[done..]);
    out
}

/// The warning for an `after_build` that names the folder the site is
/// published in, as the build was asked for it: the command runs before
/// the built site is put there. Placed at the folder's name.
pub fn names_output(
    settings: &Settings,
    sources: &Sources,
    options: &crate::build::Options,
) -> Option<Problem> {
    let line = settings.text(AFTER)?;
    let folder = match &options.out {
        Some(out) => out.to_string_lossy().into_owned(),
        None => crate::build::OUT.to_owned(),
    };
    let folder = folder.strip_prefix("./").unwrap_or(&folder);
    let folder = folder.trim_end_matches(['/', '\\']);
    if folder.is_empty() {
        return None;
    }
    let named = naming(line.value, folder);
    if named.is_empty() {
        return None;
    }
    // As the file writes it, the line may be quoted or run over several.
    let place = line.origin.place.map(|place| {
        let from = place.span.start as usize;
        let written = &sources.text(place.source)[from..place.span.end as usize];
        match naming(written, folder).first() {
            Some(&(_, name, end)) => crate::source::Place {
                source: place.source,
                span: crate::source::Span::new(from + name, from + end),
            },
            None => place,
        }
    });
    let kind = Kind::OutputFolderNamed {
        folder: folder.to_owned(),
        rewritten: rewritten(line.value, &named),
    };
    Some(Problem::placed(kind, place))
}

/// The note that `check` runs none of the commands the site sets, placed
/// at the first of them.
pub fn unrun_by_check(settings: &Settings) -> Option<Problem> {
    let set: Vec<_> = [BEFORE, AFTER]
        .into_iter()
        .filter_map(|hook| Some((hook, settings.text(hook)?)))
        .collect();
    let (_, first) = set.first()?;
    let kind = Kind::HookNotRun {
        hooks: set.iter().map(|(hook, _)| (*hook).to_owned()).collect(),
    };
    Some(Problem::placed(kind, first.origin.place))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(line: &str) -> Result<(), Stopped> {
        let site = std::env::temp_dir();
        Hook {
            setting: BEFORE,
            line,
            site: &site,
            out: Path::new("_site"),
            drafts: false,
            serve: false,
        }
        .run()
    }

    #[test]
    fn a_command_that_ends_well_is_quiet() {
        assert_eq!(run("echo built"), Ok(()));
        assert_eq!(run("echo one && echo two"), Ok(()));
    }

    #[test]
    fn both_streams_are_kept_in_the_order_written() {
        let line = match cfg!(windows) {
            true => "echo one& echo two 1>&2& echo three& exit /b 3",
            false => "echo one; echo two >&2; echo three; exit 3",
        };
        let stopped = run(line).unwrap_err();
        assert_eq!(stopped.ended, Ended::Code(3));
        let lines: Vec<&str> = stopped.said.lines().map(str::trim).collect();
        assert_eq!(lines, ["one", "two", "three"]);
    }

    #[test]
    fn a_command_the_shell_cant_find_ends_with_its_code_and_its_words() {
        let stopped = run("statac-no-such-command").unwrap_err();
        let Ended::Code(code) = stopped.ended else {
            panic!("{stopped:?}");
        };
        assert_ne!(code, 0);
        if cfg!(unix) {
            assert!(platform::command_not_found(code), "{code}");
        }
        assert!(
            stopped.said.contains("statac-no-such-command"),
            "{stopped:?}"
        );
    }

    #[test]
    fn a_command_is_told_where_it_is_and_what_it_is() {
        let line = match cfg!(windows) {
            true => {
                "if not \"%STATAC_DRAFTS%\"==\"no\" exit /b 1\
                 & if not \"%STATAC_SERVE%\"==\"no\" exit /b 1\
                 & if not \"%STATAC_HOOK%\"==\"before_build\" exit /b 1\
                 & if not exist \"%STATAC_SITE%\" exit /b 1\
                 & if \"%STATAC_OUT%\"==\"_site\" exit /b 1\
                 & exit /b 0"
            }
            false => {
                "test \"$STATAC_DRAFTS\" = no \
                 && test \"$STATAC_SERVE\" = no \
                 && test \"$STATAC_HOOK\" = before_build \
                 && test -d \"$STATAC_SITE\" \
                 && case \"$STATAC_OUT\" in /*/_site) exit 0 ;; *) exit 1 ;; esac"
            }
        };
        assert_eq!(run(line), Ok(()));
    }

    #[test]
    fn a_command_that_reads_finds_nothing_to_read() {
        let line = match cfg!(windows) {
            true => "(set /p answer=)& if defined answer exit /b 1& exit /b 0",
            false => "read answer; test -z \"$answer\"",
        };
        assert_eq!(run(line), Ok(()));
    }

    #[test]
    fn a_place_stays_below_its_folder() {
        for written in ["search/", "feed.json", "a/b/", "a/b.txt"] {
            assert!(a_place(written), "{written}");
            assert!(a_built_place(&format!("/{written}")), "{written}");
        }
        for written in [
            "", "/", "../a", "a/../b", "a//b", "/a", "C:/a", "a\\b", "./a",
        ] {
            assert!(!a_place(written), "{written}");
        }
        assert!(!a_built_place("//a"));
    }

    #[test]
    fn the_output_folder_is_found_only_as_a_path_of_its_own() {
        let named = |line: &str| !naming(line, "_site").is_empty();
        for line in [
            "index --site _site",
            "rsync -a _site/ host:www/",
            "index --site=./_site/",
            "gzip -r \"_site\"",
            "ls $STATAC_SITE/_site",
            "cd _site && ls",
            "(cd _site; ls)",
        ] {
            assert!(named(line), "{line}");
        }
        for line in [
            "./my_site.sh",
            "ls _sites/",
            "ls _site.old",
            "ls old/_site",
            "ls /var/www/_site",
            "index --site \"$STATAC_OUT\"",
            "index --site \"$STATAC_OUT/_site\"",
            "ls a_site",
        ] {
            assert!(!named(line), "{line}");
        }
        assert_eq!(naming("ls ./_site/a", "_site"), [(3, 5, 10)]);
    }

    #[test]
    fn the_command_is_offered_with_the_folder_it_is_handed() {
        let offered = |line: &str| rewritten(line, &naming(line, "_site"));
        assert_eq!(
            offered("index --site ./_site && gzip -r \"_site/a\""),
            format!("index --site \"{HANDED}\" && gzip -r \"{HANDED}/a\"")
        );
        assert_eq!(
            offered("rsync -a _site/ host:www/"),
            format!("rsync -a \"{HANDED}\"/ host:www/")
        );
    }

    #[test]
    fn what_a_command_makes_is_found_by_file_or_by_folder() {
        assert_eq!(made("/search/"), ("search", true));
        assert_eq!(made("feed.json"), ("feed.json", false));
        assert!(within("search/a.js", "search"));
        assert!(!within("searching/a.js", "search"));
        assert!(!within("search", "search"));
    }
}
