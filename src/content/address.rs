//! Where things land: addresses, the names a disk will write in them, and
//! the pattern language `permalink` is written in.

use super::slug::nfc;
use crate::date::Date;
use crate::platform::Unwritable;
use crate::problem::{AddressFault, Kind, Problem, Word};
use crate::source::{Place, Span};

/// The pieces a `permalink` pattern may ask for.
pub const PIECES: &[&str] = &["year", "month", "day", "slug", "folder", "name"];

/// Where something lands, its pieces composed the one way.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Address {
    /// From the top of the site, opening with `/`; a folder ends with `/`.
    path: String,
    /// Whether the address names a file itself, rather than the index of a folder.
    file: bool,
}

/// A fault in a written address, and the range of the text it is in.
pub type Fault = (AddressFault, Span);

impl Address {
    /// Reads an address. A trailing slash is a folder; with none, a dot in
    /// the last piece is a file and anything else a folder, so `/v2.0/` is a
    /// folder, `/404.html` a file and `/notes` a folder. Empty pieces are
    /// dropped, as the disk drops them.
    pub fn read(written: &str) -> Result<Address, Fault> {
        let (path, last) = Address::pieces(written)?;
        let file = !written.ends_with('/') && last.contains('.');
        Ok(Address::settled(path, file))
    }

    /// Reads an address that names a file whatever it is called.
    pub fn read_file(written: &str) -> Result<Address, Fault> {
        let (path, _) = Address::pieces(written)?;
        Ok(Address::settled(path, true))
    }

    fn pieces(written: &str) -> Result<(String, String), Fault> {
        let mut path = String::with_capacity(written.len() + 1);
        let mut last = String::new();
        let mut at = 0;
        for piece in written.split('/') {
            let start = at;
            at += piece.len() + 1;
            if piece.is_empty() {
                continue;
            }
            let composed = nfc(piece);
            if let Some(fault) = name_fault(&composed) {
                return Err((fault, Span::new(start, start + piece.len())));
            }
            path.push('/');
            path.push_str(&composed);
            last = composed;
        }
        Ok((path, last))
    }

    fn settled(mut path: String, file: bool) -> Address {
        if !file && !path.ends_with('/') {
            path.push('/');
        }
        Address { path, file }
    }

    /// An address for a page whose own could not be made, where nothing
    /// else will stand, while the rest of the site is read.
    pub fn made(path: &str) -> Address {
        Address {
            path: format!("/{}/", super::slug::made_slug(path)),
            file: false,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.path
    }

    pub fn is_file(&self) -> bool {
        self.file
    }

    /// The file this address writes, from the top of the built site.
    pub fn file(&self) -> String {
        let path = self.path.trim_start_matches('/');
        match self.file {
            true => path.to_owned(),
            false => format!("{path}index.html"),
        }
    }

    /// One spelling for every address a common disk holds as one file.
    pub fn key(&self) -> String {
        crate::platform::folded(&self.file())
    }

    /// The folder something beside this page lands in.
    pub fn folder(&self) -> &str {
        match self.file {
            true => &self.path[..=self.path.rfind('/').unwrap_or(0)],
            false => &self.path,
        }
    }
}

/// An address from the top of the site as it travels outside a browser,
/// in a feed, a sitemap or a redirect: every byte its path can't carry as
/// itself written as `%` and the byte, so a letter outside ASCII is the
/// bytes of its UTF-8. A `%` already there is left as written, and so is
/// what follows a `?` or a `#`.
pub fn on_the_wire(address: &str) -> String {
    let (path, rest) = address.split_at(address.find(['?', '#']).unwrap_or(address.len()));
    let mut out = String::with_capacity(address.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(char::from(byte)),
            _ if b"-._~!$&'()*+,;=:@/%".contains(&byte) => out.push(char::from(byte)),
            _ => {
                use std::fmt::Write as _;
                write!(out, "%{byte:02X}").expect("writing to a string can't fail");
            }
        }
    }
    out.push_str(rest);
    out
}

/// An address from the top of the site made whole with the site's `url`,
/// its path and all, and spelled as it travels.
pub fn whole(address: &str, url: &str) -> String {
    format!("{}{}", url.trim_end_matches('/'), on_the_wire(address))
}

/// Why one piece of an address is no name every disk will write, or no
/// name an address can carry. The disk's rules are `platform`'s, held on
/// every machine.
pub fn name_fault(piece: &str) -> Option<AddressFault> {
    if piece == "." || piece == ".." {
        return Some(AddressFault::Dots(piece.to_owned()));
    }
    if let Some(why) = crate::platform::unwritable(piece) {
        return Some(match why {
            Unwritable::Character(c) => AddressFault::Character(c),
            Unwritable::Control | Unwritable::NotText => AddressFault::Control,
            Unwritable::TrailingDot => AddressFault::TrailingSpaceOrDot('.'),
            Unwritable::TrailingSpace => AddressFault::TrailingSpaceOrDot(' '),
            Unwritable::Device(device) => AddressFault::Device(device),
        });
    }
    if piece.contains('%') {
        return Some(AddressFault::Percent);
    }
    if piece.contains('#') {
        return Some(AddressFault::Hash);
    }
    None
}

/// Everything wrong with a `permalink` pattern, each with where it is in
/// the pattern: a piece no pattern fills, a brace with no partner, another
/// tool's `:piece`, and the first piece no disk will write. Its pieces are
/// [`PIECES`] and the fields `declared` for the site's pages. A pattern
/// `handed` to many pages must ask for at least one piece, or every page
/// lands at one address.
pub fn pattern_faults(pattern: &str, handed: bool, declared: &[&str]) -> Vec<(Kind, Span)> {
    faults(pattern, handed, Fields::Declared(declared))
}

/// Everything wrong with the pattern a page making pages gives its items,
/// whose pieces are the items' own fields, so any name in braces is one.
pub fn item_pattern_faults(pattern: &str) -> Vec<(Kind, Span)> {
    faults(pattern, true, Fields::Any)
}

/// Which names in braces a pattern may ask for besides [`PIECES`].
#[derive(Copy, Clone)]
enum Fields<'a> {
    /// Any name, each an item's own field.
    Any,
    /// The fields declared for the site's pages.
    Declared(&'a [&'a str]),
}

impl<'a> Fields<'a> {
    fn has(self, token: &str) -> bool {
        match self {
            Fields::Any => is_field_name(token),
            Fields::Declared(declared) => declared.contains(&token),
        }
    }

    /// The piece nearest `written`, of [`PIECES`] and the fields declared.
    fn nearest(self, written: &str) -> Option<&'a str> {
        let declared = match self {
            Fields::Any => &[][..],
            Fields::Declared(declared) => declared,
        };
        crate::suggest::nearest(written, PIECES.iter().chain(declared).copied())
    }
}

fn faults(pattern: &str, handed: bool, fields: Fields) -> Vec<(Kind, Span)> {
    let items = matches!(fields, Fields::Any);
    let mut found = Vec::new();
    let unusable = |fault| Kind::UnusableUrl { fault };
    let mut chars = pattern.char_indices().peekable();
    let mut fills = false;
    while let Some((start, c)) = chars.next() {
        match c {
            ':' => {
                let word = colon_word(&pattern[..start], &pattern[start + 1..]);
                if word.is_empty() {
                    continue;
                }
                let span = Span::new(start, start + 1 + word.len());
                let ours = match word {
                    word if items => Some(word),
                    "title" | "name" | "filename" => Some("slug"),
                    word if PIECES.contains(&word) || fields.has(word) => Some(word),
                    word => fields.nearest(word),
                };
                let written = format!(":{word}");
                found.push((
                    match ours {
                        Some(ours) => Kind::OtherSpelling {
                            word: Word::AddressPiece,
                            written,
                            ours: format!("{{{ours}}}"),
                        },
                        None => unusable(AddressFault::UnknownPiece {
                            written,
                            nearest: None,
                        }),
                    },
                    span,
                ));
            }
            '}' => found.push((unusable(AddressFault::Stray), Span::new(start, start + 1))),
            '{' => {
                let Some(close) = pattern[start..].find('}') else {
                    found.push((
                        unusable(AddressFault::Unclosed),
                        Span::new(start, pattern.len()),
                    ));
                    break;
                };
                let end = start + close + 1;
                while chars.next_if(|&(at, _)| at < end).is_some() {}
                let token = &pattern[start + 1..end - 1];
                if PIECES.contains(&token) || fields.has(token) {
                    fills = true;
                    continue;
                }
                let nearest = match token {
                    "section" => Some("folder"),
                    "title" | "filename" => Some("slug"),
                    token => fields.nearest(token),
                };
                found.push((
                    unusable(AddressFault::UnknownPiece {
                        written: format!("{{{token}}}"),
                        nearest: nearest.map(|n| format!("{{{n}}}")),
                    }),
                    Span::new(start, end),
                ));
            }
            _ => {}
        }
    }
    if found.is_empty() {
        if let Some(fault) = first_unwritable(pattern) {
            found.push(fault);
        } else if handed && !fills {
            found.push((
                unusable(AddressFault::FillsNothing {
                    pattern: pattern.to_owned(),
                }),
                Span::new(0, pattern.len()),
            ));
        }
    }
    found
}

/// The word a colon opens, `:year`: a letter, then letters, figures and
/// `_`. Nothing where the colon stands inside a word, as in `a:b`.
fn colon_word<'a>(before: &str, rest: &'a str) -> &'a str {
    if before.ends_with(|c: char| c.is_alphanumeric())
        || !rest.starts_with(|c: char| c.is_ascii_alphabetic())
    {
        return "";
    }
    let end = rest
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    &rest[..end]
}

/// The first piece of a pattern with no other fault that no disk will
/// write, each `{piece}` standing for a plain name.
fn first_unwritable(pattern: &str) -> Option<(Kind, Span)> {
    let mut at = 0;
    for piece in pattern.split('/') {
        let start = at;
        at += piece.len() + 1;
        let named = filled(piece, |_| Ok::<_, ()>("x".to_owned())).unwrap_or_default();
        if named.is_empty() {
            continue;
        }
        if let Some(fault) = name_fault(&named) {
            return Some((
                Kind::UnusableUrl { fault },
                Span::new(start, start + piece.len()),
            ));
        }
    }
    None
}

/// Whether `token` can name an item's field in a pattern: anything that
/// leaves the braces and the pattern's folders alone.
fn is_field_name(token: &str) -> bool {
    !token.is_empty() && !token.contains(['{', '}', '/'])
}

/// A pattern with each `{name}` in it filled in by `fill`, in turn. The
/// pattern's braces are known to pair.
pub fn filled<E>(
    pattern: &str,
    mut fill: impl FnMut(&str) -> Result<String, E>,
) -> Result<String, E> {
    let mut out = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        out.push_str(&rest[..open]);
        out.push_str(&fill(&rest[open + 1..open + close])?);
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// What one page fills a pattern with.
pub struct Pieces<'a> {
    pub date: Option<&'a Date>,
    pub slug: &'a str,
    /// The last piece of the page's folder's address.
    pub folder: &'a str,
    /// The page's name exactly as written, any date included.
    pub name: &'a str,
}

/// Fills a well-formed pattern in for one page, or names the piece of a
/// date it asks for and the page hasn't got.
pub fn expand(pattern: &str, pieces: &Pieces) -> Result<String, AddressFault> {
    let mut out = pattern.to_owned();
    for (token, width) in [("{year}", 4), ("{month}", 2), ("{day}", 2)] {
        if !out.contains(token) {
            continue;
        }
        let Some(date) = pieces.date else {
            return Err(AddressFault::NoDate {
                piece: token.to_owned(),
            });
        };
        let n = match token {
            "{year}" => date.year,
            "{month}" => i32::from(date.month),
            _ => i32::from(date.day),
        };
        out = out.replace(token, &format!("{n:0width$}"));
    }
    // `{name}` is filled last, so a name holding braces stays its own name.
    Ok(out
        .replace("{slug}", pieces.slug)
        .replace("{folder}", pieces.folder)
        .replace("{name}", &nfc(pieces.name)))
}

/// Where a pattern is read from: the top of the pages of the language it
/// is written for. A language's folder is its top, and its patterns land
/// beneath that folder's address; anywhere else, they land as written.
pub struct Top {
    /// The folder's path under `content/`; empty at the top of `content/`.
    folder: String,
}

impl Top {
    /// The top at `folder`, its path under `content/`, with or without a
    /// closing `/`.
    pub fn new(folder: &str) -> Top {
        Top {
            folder: folder.trim_end_matches('/').to_owned(),
        }
    }

    /// The pattern to fill in: as written, or where it already opens with
    /// the top's folder, without it, warned of once at `place`.
    pub fn pattern<'a>(
        &self,
        pattern: &'a str,
        place: Option<Place>,
        problems: &mut Vec<Problem>,
    ) -> &'a str {
        if self.folder.is_empty() {
            return pattern;
        }
        let opened = pattern.trim_start_matches('/');
        let (first, rest) = opened.split_at(opened.find('/').unwrap_or(opened.len()));
        if !first.eq_ignore_ascii_case(&self.folder) {
            return pattern;
        }
        let without = match rest {
            "" => "/",
            rest => rest,
        };
        if let Some(place) = place {
            let problem = Problem::at(
                Kind::LanguageFolderInPattern {
                    pattern: pattern.to_owned(),
                    folder: self.address(""),
                    without: without.to_owned(),
                },
                place,
            );
            if !problems.contains(&problem) {
                problems.push(problem);
            }
        }
        without
    }

    /// A pattern filled in, read from the top: beneath the folder's
    /// address, or as it is at the top of `content/`.
    pub fn address(&self, filled: &str) -> String {
        match self.folder.is_empty() {
            true => filled.to_owned(),
            false => format!(
                "/{}{}",
                super::slug::tidied_path(&self.folder),
                filled.trim_start_matches('/')
            ),
        }
    }
}

/// Whether `written` is a full address, opening with a scheme such as
/// `https:` or `mailto:`, or with `//`, rather than an address on the site.
pub fn is_full(written: &str) -> bool {
    if written.starts_with("//") {
        return true;
    }
    let Some((scheme, _)) = written.split_once(':') else {
        return false;
    };
    scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// `written` with each `%XX` read as the byte it stands for, where the
/// bytes read are text; else as written.
pub fn decoded(written: &str) -> String {
    let bytes = written.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[at] == b'%'
            && let (Some(high), Some(low)) = (
                bytes.get(at + 1).copied().and_then(hex),
                bytes.get(at + 2).copied().and_then(hex),
            )
        {
            out.push((high * 16 + low) as u8);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| written.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_travels_with_what_its_path_cant_carry_written_as_bytes() {
        for (written, travelling) in [
            ("/posts/post-1/", "/posts/post-1/"),
            ("/blog/5-μs/", "/blog/5-%CE%BCs/"),
            ("/posts/café 4/", "/posts/caf%C3%A9%204/"),
            ("/مقالات/", "/%D9%85%D9%82%D8%A7%D9%84%D8%A7%D8%AA/"),
            ("/a%20b/", "/a%20b/"),
            ("/blog/5-%CE%BCs/", "/blog/5-%CE%BCs/"),
            ("/search/?q=café#top", "/search/?q=café#top"),
        ] {
            assert_eq!(on_the_wire(written), travelling, "{written}");
        }
        assert_eq!(
            whole("/posts/café/", "https://example.org/sub/"),
            "https://example.org/sub/posts/caf%C3%A9/"
        );
        assert_eq!(whole("/", "https://example.org"), "https://example.org/");
    }

    fn address(written: &str) -> (String, bool) {
        let address = Address::read(written).unwrap();
        (address.as_str().to_owned(), address.is_file())
    }

    #[test]
    fn a_trailing_slash_or_no_dot_is_a_folder() {
        let cases = [
            ("/notes/", "/notes/", false),
            ("/notes", "/notes/", false),
            ("notes/", "/notes/", false),
            ("/v2.0/", "/v2.0/", false),
            ("/404.html", "/404.html", true),
            ("/robots.txt", "/robots.txt", true),
            ("/", "/", false),
            ("", "/", false),
            ("//notes//a/", "/notes/a/", false),
            ("/Notes/Café/", "/Notes/Café/", false),
            ("/cafe\u{301}/", "/café/", false),
        ];
        for (written, path, file) in cases {
            assert_eq!(address(written), (path.to_owned(), file), "{written}");
        }
    }

    #[test]
    fn an_address_writes_one_file() {
        let cases = [
            ("/notes/", "notes/index.html"),
            ("/", "index.html"),
            ("/404.html", "404.html"),
            ("/Notes/", "Notes/index.html"),
        ];
        for (written, file) in cases {
            assert_eq!(Address::read(written).unwrap().file(), file, "{written}");
        }
        assert_eq!(
            Address::read("/Notes/").unwrap().key(),
            Address::read("/notes/index.html").unwrap().key()
        );
        assert_eq!(Address::read_file("/CNAME").unwrap().file(), "CNAME");
        assert_eq!(Address::read("/notes/a.html").unwrap().folder(), "/notes/");
        assert_eq!(Address::read("/notes/a/").unwrap().folder(), "/notes/a/");
    }

    #[test]
    fn a_name_no_disk_will_write_is_refused_where_it_is() {
        use AddressFault::*;
        let cases = [
            ("/a/../b/", Dots("..".into()), ".."),
            ("/a/./", Dots(".".into()), "."),
            ("/a\\b/", Character('\\'), "a\\b"),
            ("/a:b/", Character(':'), "a:b"),
            ("/what?/", Character('?'), "what?"),
            ("/a\u{7}/", Control, "a\u{7}"),
            ("/notes./", TrailingSpaceOrDot('.'), "notes."),
            ("/notes /", TrailingSpaceOrDot(' '), "notes "),
            ("/100%/", Percent, "100%"),
            ("/a#b/", Hash, "a#b"),
            ("/con/", Device("con".into()), "con"),
            ("/NUL.txt", Device("NUL".into()), "NUL.txt"),
            ("/notes/lpt1.tar.gz", Device("lpt1".into()), "lpt1.tar.gz"),
        ];
        for (written, fault, piece) in cases {
            let (found, span) = Address::read(written).unwrap_err();
            assert_eq!(found, fault, "{written}");
            assert_eq!(&written[span.range()], piece, "{written}");
        }
        assert!(Address::read("/console/").is_ok());
        assert!(Address::read("/$10/").is_ok());
    }

    #[test]
    fn every_piece_of_a_pattern_is_filled() {
        let date = crate::date::Date::day_only(2024, 1, 2);
        let pieces = Pieces {
            date: Some(&date),
            slug: "post-1",
            folder: "notes",
            name: "2024-01-02-Post 1",
        };
        let cases = [
            ("/{year}/{month}/{day}/{slug}/", "/2024/01/02/post-1/"),
            ("/{folder}/{slug}.html", "/notes/post-1.html"),
            ("/{name}/", "/2024-01-02-Post 1/"),
            ("/blog/{year}/{slug}/", "/blog/2024/post-1/"),
            ("/about/", "/about/"),
        ];
        for (pattern, expanded) in cases {
            assert_eq!(expand(pattern, &pieces).unwrap(), expanded, "{pattern}");
        }
        let undated = Pieces {
            date: None,
            ..pieces
        };
        assert_eq!(
            expand("/{year}/{slug}/", &undated),
            Err(AddressFault::NoDate {
                piece: "{year}".into()
            })
        );
        let braced = Pieces {
            name: "{slug}",
            ..undated
        };
        assert_eq!(expand("/{name}/", &braced).unwrap(), "/{slug}/");
    }

    #[test]
    fn an_items_pattern_takes_any_name_as_a_piece() {
        let unusable = |fault| Kind::UnusableUrl { fault };
        let cases = [
            ("/people/{key}/", None),
            ("/{year}/{isbn}.html", None),
            (
                "/people/",
                Some((
                    unusable(AddressFault::FillsNothing {
                        pattern: "/people/".into(),
                    }),
                    "/people/",
                )),
            ),
            (
                "/people/:key/",
                Some((
                    Kind::OtherSpelling {
                        word: Word::AddressPiece,
                        written: ":key".into(),
                        ours: "{key}".into(),
                    },
                    ":key",
                )),
            ),
            (
                "/people/{}/",
                Some((
                    unusable(AddressFault::UnknownPiece {
                        written: "{}".into(),
                        nearest: None,
                    }),
                    "{}",
                )),
            ),
            (
                "/people/{key/",
                Some((unusable(AddressFault::Unclosed), "{key/")),
            ),
            (
                "/people/{key}./",
                Some((unusable(AddressFault::TrailingSpaceOrDot('.')), "{key}.")),
            ),
        ];
        for (pattern, expected) in cases {
            let faults = item_pattern_faults(pattern);
            let found = faults
                .first()
                .map(|(kind, span)| (kind.clone(), &pattern[span.range()]));
            assert_eq!(found, expected, "{pattern}");
        }
        let filled = filled("/{a}/x-{b}.html", |token| Ok::<_, ()>(token.to_uppercase()));
        assert_eq!(filled, Ok("/A/x-B.html".to_owned()));
    }

    #[test]
    fn a_patterns_faults_are_found_where_they_are() {
        let unusable = |fault| Kind::UnusableUrl { fault };
        let cases = [
            (
                "/{yaer}/{slug}/",
                unusable(AddressFault::UnknownPiece {
                    written: "{yaer}".into(),
                    nearest: Some("{year}".into()),
                }),
                "{yaer}",
            ),
            (
                "/{section}/{slug}/",
                unusable(AddressFault::UnknownPiece {
                    written: "{section}".into(),
                    nearest: Some("{folder}".into()),
                }),
                "{section}",
            ),
            (
                "/{year/{slug}/",
                unusable(AddressFault::UnknownPiece {
                    written: "{year/{slug}".into(),
                    nearest: None,
                }),
                "{year/{slug}",
            ),
            ("/{slug/", unusable(AddressFault::Unclosed), "{slug/"),
            ("/slug}/", unusable(AddressFault::Stray), "}"),
            (
                "/:year/:title/",
                Kind::OtherSpelling {
                    word: Word::AddressPiece,
                    written: ":year".into(),
                    ours: "{year}".into(),
                },
                ":year",
            ),
            (
                "/a:b/{slug}/",
                unusable(AddressFault::Character(':')),
                "a:b",
            ),
            (
                "/{slug}./",
                unusable(AddressFault::TrailingSpaceOrDot('.')),
                "{slug}.",
            ),
        ];
        for (pattern, kind, at) in cases {
            let faults = pattern_faults(pattern, false, &[]);
            assert_eq!(faults[0].0, kind, "{pattern}");
            assert_eq!(&pattern[faults[0].1.range()], at, "{pattern}");
        }
        assert_eq!(pattern_faults("/:year/:title/", false, &[]).len(), 2);
        assert!(pattern_faults("/{year}/{month}/{day}/{slug}/", true, &[]).is_empty());
        assert!(pattern_faults("/about/", false, &[]).is_empty());
        assert_eq!(
            pattern_faults("/blog/", true, &[]),
            [(
                unusable(AddressFault::FillsNothing {
                    pattern: "/blog/".into()
                }),
                Span::new(0, 6)
            )]
        );
    }

    #[test]
    fn a_pages_pattern_takes_a_declared_field_as_a_piece() {
        let declared = ["isbn", "book.author"];
        let unusable = |fault| Kind::UnusableUrl { fault };
        let cases = [
            ("/books/{isbn}/", None),
            ("/{year}/{isbn}.html", None),
            ("/books/{book.author}/", None),
            (
                "/books/{book.autor}/",
                Some((
                    unusable(AddressFault::UnknownPiece {
                        written: "{book.autor}".into(),
                        nearest: Some("{book.author}".into()),
                    }),
                    "{book.autor}",
                )),
            ),
            (
                "/books/{book}/",
                Some((
                    unusable(AddressFault::UnknownPiece {
                        written: "{book}".into(),
                        nearest: None,
                    }),
                    "{book}",
                )),
            ),
            (
                "/books/:isbn/",
                Some((
                    Kind::OtherSpelling {
                        word: Word::AddressPiece,
                        written: ":isbn".into(),
                        ours: "{isbn}".into(),
                    },
                    ":isbn",
                )),
            ),
            (
                "/books/{ibsn}/",
                Some((
                    unusable(AddressFault::UnknownPiece {
                        written: "{ibsn}".into(),
                        nearest: Some("{isbn}".into()),
                    }),
                    "{ibsn}",
                )),
            ),
            (
                "/books/{isbn}./",
                Some((unusable(AddressFault::TrailingSpaceOrDot('.')), "{isbn}.")),
            ),
        ];
        for (pattern, expected) in cases {
            let faults = pattern_faults(pattern, true, &declared);
            let found = faults
                .first()
                .map(|(kind, span)| (kind.clone(), &pattern[span.range()]));
            assert_eq!(found, expected, "{pattern}");
        }
        assert!(!pattern_faults("/books/{isbn}/", true, &[]).is_empty());
    }

    #[test]
    fn a_full_address_opens_with_a_scheme_or_two_slashes() {
        let cases = [
            ("https://example.org/", true),
            ("http://example.org", true),
            ("mailto:name@example.org", true),
            ("//example.org/post-1/", true),
            ("/post-1/", false),
            ("post-1/", false),
            ("/a:b/", false),
            ("1a:b", false),
            ("", false),
        ];
        for (written, full) in cases {
            assert_eq!(is_full(written), full, "{written}");
        }
    }

    #[test]
    fn an_address_written_with_percent_codes_is_read_as_its_characters() {
        let cases = [
            ("/caf%C3%A9/", "/café/"),
            ("/caf%c3%a9/", "/café/"),
            ("/a%2/", "/a%2/"),
            ("/a%zz/", "/a%zz/"),
            ("/%FF/", "/%FF/"),
            ("/post-1/", "/post-1/"),
        ];
        for (written, read) in cases {
            assert_eq!(decoded(written), read, "{written}");
        }
        assert_eq!(
            on_the_wire(&decoded("/blog/5-%CE%BCs/")),
            "/blog/5-%CE%BCs/"
        );
    }
}
