//! Sass compiled to CSS, and everything Sass says, placed in the file and
//! at the text it is about. Sass reads only within `assets/`.

mod fault;

use std::cell::{Cell, RefCell};
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Component, Path, PathBuf};

use codemap::SpanLoc;

use crate::problem::{Kind, Problem, Reach, Reason};
use crate::source::{Place, Sources, Span};

pub use fault::{SassFault, SassType};

pub(super) struct Compiled {
    pub(super) css: Result<String, Box<grass::Error>>,
    /// Whether Sass reached for a file outside `assets/`.
    pub(super) refused: bool,
    /// What `@warn` and `@debug` said, and Sass's own warnings, where each
    /// was said.
    pub(super) warnings: Vec<(SpanLoc, String)>,
}

/// The CSS `path` compiles to, reading nothing outside `assets`.
pub(super) fn compile(path: &Path, assets: &Path) -> Compiled {
    let files = Confined::new(assets);
    let heard = Heard::default();
    let options = grass::Options::default().fs(&files).logger(&heard);
    let css = grass::from_path(path, &options);
    Compiled {
        css,
        refused: files.refused.get(),
        warnings: heard.0.into_inner(),
    }
}

/// Everything Sass would otherwise print, kept for the problem list.
#[derive(Default, Debug)]
struct Heard(RefCell<Vec<(SpanLoc, String)>>);

impl grass::Logger for Heard {
    fn debug(&self, location: SpanLoc, message: &str) {
        self.0.borrow_mut().push((location, message.to_owned()));
    }

    fn warn(&self, location: SpanLoc, message: &str) {
        self.0.borrow_mut().push((location, message.to_owned()));
    }
}

/// A warning for something Sass said.
pub(super) fn warned(loc: &SpanLoc, message: &str, site: &Path, sources: &mut Sources) -> Problem {
    Problem::at(
        Kind::SassWarning {
            message: unquoted(message.trim()).to_owned(),
        },
        placed(loc, site, sources),
    )
}

/// A message without the quotes of the CSS string Sass gives `@warn "..."` as.
fn unquoted(message: &str) -> &str {
    ['"', '\'']
        .into_iter()
        .find_map(|quote| {
            message
                .strip_prefix(quote)
                .and_then(|inner| inner.strip_suffix(quote))
        })
        .unwrap_or(message)
}

/// Where `loc` is, with its file added to `sources`.
fn placed(loc: &SpanLoc, site: &Path, sources: &mut Sources) -> Place {
    at(loc, bytes(loc), site, sources)
}

/// `span` of the file `loc` is in, with the file added to `sources`.
fn at(loc: &SpanLoc, span: Range<usize>, site: &Path, sources: &mut Sources) -> Place {
    let text = loc.file.source().to_owned();
    let source = sources.add(shown(Path::new(loc.file.name()), site), text);
    Place {
        source,
        span: Span::new(span.start, span.end),
    }
}

/// The bytes of its file `loc` covers.
fn bytes(loc: &SpanLoc) -> Range<usize> {
    let text = loc.file.source();
    let start = offset(text, loc.begin.line, loc.begin.column);
    start..offset(text, loc.end.line, loc.end.column).max(start)
}

/// `file` as a path in the site. Sass may name a file by its real path,
/// past any link in the site's.
fn shown(file: &Path, site: &Path) -> PathBuf {
    let real = fs::canonicalize(site).unwrap_or_else(|_| site.to_path_buf());
    file.strip_prefix(site)
        .or_else(|_| file.strip_prefix(&real))
        .unwrap_or(file)
        .to_path_buf()
}

/// The files Sass may read: those whose real path, past every link, is
/// inside `assets/`. Anything else Sass is told isn't there.
#[derive(Debug)]
struct Confined {
    written: PathBuf,
    real: PathBuf,
    refused: Cell<bool>,
}

impl Confined {
    fn new(assets: &Path) -> Confined {
        let written = std::path::absolute(assets).unwrap_or_else(|_| assets.to_path_buf());
        Confined {
            real: fs::canonicalize(&written).unwrap_or_else(|_| written.clone()),
            written: lexically_normal(&written),
            refused: Cell::new(false),
        }
    }

    /// `path` if it may be read.
    fn inside(&self, path: &Path) -> Option<PathBuf> {
        let absolute = std::path::absolute(path).ok()?;
        let lexical = lexically_normal(&absolute);
        let within = match fs::canonicalize(&lexical) {
            Ok(real) => real.starts_with(&self.real).then_some(real),
            Err(_) => None,
        };
        if within.is_none()
            && (!lexical.starts_with(&self.written) || fs::symlink_metadata(&lexical).is_ok())
        {
            self.refused.set(true);
        }
        within
    }
}

impl grass::Fs for Confined {
    fn is_dir(&self, path: &Path) -> bool {
        self.inside(path).is_some_and(|real| real.is_dir())
    }

    fn is_file(&self, path: &Path) -> bool {
        self.inside(path).is_some_and(|real| real.is_file())
    }

    /// Read at its real path through no link, so a link put in place of
    /// the file or a folder on the way since it was asked about is refused.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let at = self.inside(path).and_then(|real| {
            let within = real.strip_prefix(&self.real).ok()?;
            crate::platform::slashed(within).ok()
        });
        match at {
            Some(at) => {
                let mut bytes = Vec::new();
                let mut file = crate::platform::open_file(&self.real, &at)?;
                io::Read::read_to_end(&mut file, &mut bytes)?;
                Ok(bytes)
            }
            None => Err(io::ErrorKind::NotFound.into()),
        }
    }
}

fn lexically_normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Why Sass stopped compiling `path`, placed where it stopped. `site` is
/// the site's folder, against which the files Sass names are shown.
/// `refused` says Sass reached outside `assets/`, which is why it stopped.
pub(super) fn stopped(
    error: grass::Error,
    refused: bool,
    path: &Path,
    site: &Path,
    sources: &mut Sources,
) -> Problem {
    let shown = |file: &Path| shown(file, site);
    match error.kind() {
        grass::ErrorKind::ParseError { message, loc, .. } => {
            if !refused {
                let file = Path::new(loc.file.name());
                let assets = site.join(super::ASSETS);
                let (fault, span) =
                    fault::read(&message, loc.file.source(), bytes(&loc), file, &assets);
                let place = at(&loc, span, site, sources);
                return Problem::at(Kind::SassStopped { fault }, place);
            }
            let mut place = placed(&loc, site, sources);
            let inside = quoted(sources.spelling(place));
            let start = place.span.start as usize;
            place.span = Span::new(start + inside.start, start + inside.end);
            let reach = Reach::Load(sources.spelling(place).into());
            Problem::at(Kind::OutsideSite { reach }, place)
        }
        grass::ErrorKind::IoError(error) => Problem::nowhere(Kind::CantRead {
            path: shown(path),
            reason: Reason::of(&error),
        }),
        grass::ErrorKind::FromUtf8Error(_) => Problem::nowhere(Kind::CantRead {
            path: shown(path),
            reason: Reason::NotText,
        }),
        _ => Problem::nowhere(Kind::CantRead {
            path: shown(path),
            reason: Reason::NotForSass,
        }),
    }
}

/// Where the name sits in a load Sass refused: inside the first quotes,
/// or the whole of `text` where it has none. Sass places the refusal at a
/// whole `@use` rule, but only at the name of an `@import`.
fn quoted(text: &str) -> std::ops::Range<usize> {
    let Some(open) = text.find(['"', '\'']) else {
        return 0..text.len();
    };
    let quote = text.as_bytes()[open];
    let start = open + 1;
    let end = text[start..]
        .find(quote as char)
        .map_or(text.len(), |at| start + at);
    start..end
}

/// The byte offset of a line and column, both counted from 0 and the
/// column in characters.
fn offset(text: &str, line: usize, column: usize) -> usize {
    let line_start = text
        .split_inclusive('\n')
        .take(line)
        .map(str::len)
        .sum::<usize>();
    let rest = &text[line_start..];
    line_start
        + rest
            .char_indices()
            .nth(column)
            .map_or(rest.len(), |(at, _)| at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_place_counts_characters_on_its_line() {
        let text = "a { }\n// Café\n$x: ;\n";
        assert_eq!(offset(text, 0, 0), 0);
        assert_eq!(offset(text, 1, 7), "a { }\n// Café".len());
        assert_eq!(offset(text, 2, 4), "a { }\n// Café\n$x: ".len());
    }
}
