//! `statac refuse`: turning a webmention away for good, or everything
//! from a sender. Only `fetched/` is read, so a site with an error
//! elsewhere can still be rid of a mention.
//!
//! The list of refused is written first, so a run stopped part way never
//! leaves a mention removed but still welcome; then each mention is
//! removed and recorded `absent`, then each face no mention left names,
//! and last the record.

use std::collections::{BTreeSet, HashSet};
use std::io::Write;
use std::path::PathBuf;

use crate::build;
use crate::date::Date;
use crate::fetched::mentions::{self, Mention};
use crate::fetched::refused::{self, Entry, Reach, Spot};
use crate::fetched::{self as kept, Held, Line};
use crate::problem::{Kind, Problem, Reason};
use crate::refresh::{keep, unwritten};
use crate::report::{Quiet, Report, shown};
use crate::source::Sources;
use crate::wording;

/// What `statac refuse` is asked.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Options {
    /// The site's folder.
    pub site: PathBuf,
    /// The address of the mention or the sender, as the author gave it.
    pub address: String,
    /// The day the refusal is recorded on; today on the machine's clock
    /// where none is given, since the site's zone is in a file this
    /// doesn't read.
    pub today: Option<Date>,
}

/// Turns away what `options` names, saying what it removed on `out`.
/// Gives the exit code: 0 where it refused, or had refused already; 1
/// where `fetched/` has an error, nothing kept is from the address, or
/// something couldn't be written or removed.
pub fn run(options: &Options, out: &mut dyn Write, colour: bool) -> u8 {
    let mut say = |line: &str| {
        let _ = out.write_all(line.as_bytes());
        let _ = out.flush();
    };
    if let Err(problem) = build::a_folder(&options.site) {
        let sources = Sources::default();
        let report = Report::new(&sources, &options.site, colour);
        say(&report.check(&[problem], &Quiet::default()));
        return 1;
    }
    let mut sources = Sources::default();
    let (fetched, mut problems) = kept::read(&options.site, &mut sources);
    let held = mentions::kept(&fetched, &mut sources, &mut problems);
    let report = Report::new(&sources, &options.site, colour);
    if build::stops(&problems) {
        say(&report.check(&problems, &Quiet::default()));
        return 1;
    }
    if !problems.is_empty() {
        say(&format!("{}\n", report.problems(&problems)));
    }
    let address = mentions::clean(&options.address, true);
    let Some(spot) = Spot::of(&address) else {
        say(&format!("{}\n", shown(&wording::nothing_refused(&address))));
        return 1;
    };
    let from = |mention: &Mention| {
        refused::at_or_under(&mention.url, &spot)
            || (mention.author_url.as_deref())
                .is_some_and(|sender| refused::at_or_under(sender, &spot))
    };
    let matching: Vec<&(String, Mention, _)> = held.iter().filter(|(_, m, _)| from(m)).collect();
    let reach = match matching[..] {
        [] if fetched.refused.refusing(&address, None).is_some() => {
            say(&format!("{}\n", shown(&wording::refused_already(&address))));
            return 0;
        }
        [] => {
            say(&format!("{}\n", shown(&wording::nothing_refused(&address))));
            let urls = held.iter().map(|(_, mention, _)| mention.url.as_str());
            if let Some(near) = crate::suggest::nearest(&address, urls) {
                say(&format!("{}\n", shown(&wording::did_you_mean(near))));
            }
            return 1;
        }
        [(_, only, _)] if Spot::of(&only.url).as_ref() == Some(&spot) => Reach::Mention,
        _ => Reach::Sender,
    };

    let mut paths: Vec<&str> = matching.iter().map(|(path, _, _)| path.as_str()).collect();
    paths.sort_unstable();
    let named_by = |mentions: &mut dyn Iterator<Item = &Mention>| -> BTreeSet<String> {
        let faces = mentions.filter_map(|mention| mention.face.as_deref());
        faces
            .map(|face| format!("{}/{face}", mentions::FACES))
            .collect()
    };
    let folder = options.site.join(kept::FOLDER);
    let others = held.iter().filter(|(_, mention, _)| !from(mention));
    let kept_faces = named_by(&mut others.map(|(_, mention, _)| mention));
    let faces = named_by(&mut matching.iter().map(|(_, mention, _)| mention));
    let faces: Vec<String> = faces.difference(&kept_faces).cloned().collect();
    let mut told = Vec::new();
    // Each file is asked what it is before anything is written, so a link
    // in a mention's place stops the run with nothing changed.
    for path in paths
        .iter()
        .copied()
        .chain(faces.iter().map(String::as_str))
    {
        if let Err(error) = crate::platform::no_link_to(&folder, path) {
            told.push(unremoved(path, &error));
        }
    }
    if !told.is_empty() {
        say(&report.problems(&told));
        return 1;
    }
    let today = options.today.unwrap_or_else(today);

    let mut list = fetched.refused.clone();
    list.set(Entry {
        reach,
        address: address.clone(),
        day: today,
    });
    if let Err(error) = keep(&folder, refused::NAME, list.text().as_bytes()) {
        say(&unwritten(&report, &refused::shown(), &error));
        return 1;
    }

    let mut record = fetched.record.clone();
    let mut standing: HashSet<&str> = (fetched.files.iter())
        .map(|(path, _)| path.as_str())
        .collect();
    let mut removed = Vec::new();
    for path in &paths {
        match crate::platform::remove_file(&folder, path) {
            Ok(()) => {
                let held = Held::Absent;
                record.set(path, Line { held, day: today });
                standing.remove(path);
                removed.push(path.to_string());
            }
            Err(error) => told.push(unremoved(path, &error)),
        }
    }
    let mentions_removed = removed.len();
    let mut left = (held.iter())
        .filter(|(path, _, _)| standing.contains(path.as_str()))
        .map(|(_, mention, _)| mention);
    let named = named_by(&mut left);
    for face in faces.iter().filter(|face| !named.contains(*face)) {
        match crate::platform::remove_file(&folder, face) {
            Ok(()) => {
                standing.remove(face.as_str());
                removed.push(face.clone());
            }
            Err(error) => told.push(unremoved(face, &error)),
        }
    }

    let text = record.text(|path| standing.contains(path));
    let mut code = u8::from(!told.is_empty());
    let written = keep(&folder, kept::RECORD, text.as_bytes());

    let headline = match reach {
        Reach::Mention => wording::refused_one(&address, mentions_removed),
        Reach::Sender => wording::refused_sender(&address, mentions_removed),
    };
    say(&format!("{}\n", shown(&headline)));
    for path in &removed {
        let file = format!("{}/{path}", kept::FOLDER);
        say(&format!("{}\n", shown(&wording::removed_file(&file))));
    }
    if !told.is_empty() {
        say(&report.problems(&told));
    }
    if let Err(error) = written {
        say(&unwritten(&report, &kept::record_shown(), &error));
        code = 1;
    }
    code
}

/// The problem with a file of `fetched/` that couldn't be removed.
fn unremoved(path: &str, error: &std::io::Error) -> Problem {
    Problem::nowhere(Kind::CantRemove {
        path: format!("{}/{path}", kept::FOLDER).into(),
        reason: Reason::of(error),
    })
}

/// Today on the machine's clock.
fn today() -> Date {
    let now = jiff::Zoned::now();
    Date::day_only(
        i32::from(now.year()),
        now.month().unsigned_abs(),
        now.day().unsigned_abs(),
    )
}
