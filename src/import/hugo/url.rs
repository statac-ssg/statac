//! Where Hugo publishes a page, by Hugo's own rules: a path made from the
//! file's place or a `[permalinks]` pattern, each name in it made ready for
//! an address as Hugo makes one.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::date::Date;

/// How the settings have Hugo make a name ready for an address.
#[derive(Copy, Clone, Debug)]
pub(super) struct Making {
    /// Small letters, unless `disablePathToLower`.
    pub lower: bool,
    /// Accents taken off, as `removePathAccents` asks.
    pub accents_removed: bool,
}

impl Making {
    /// `text` made ready for an address as Hugo makes it: letters, figures
    /// and marks of any script kept, as are `. / \ _ # + ~ - @`, and `%`
    /// before two hex digits; a run of spaces between them one hyphen, and
    /// everything else dropped.
    pub(super) fn made(&self, text: &str) -> String {
        let text: String = match self.accents_removed {
            true => text
                .nfd()
                .filter(|c| !is_combining_mark(*c))
                .nfc()
                .collect(),
            false => text.to_owned(),
        };
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::with_capacity(text.len());
        let mut hyphen_wanted = false;
        let mut was_hyphen = false;
        for (at, &c) in chars.iter().enumerate() {
            let hex = |n: usize| chars.get(n).is_some_and(char::is_ascii_hexdigit);
            let allowed = matches!(c, '.' | '/' | '\\' | '_' | '#' | '+' | '~' | '-' | '@')
                || c.is_alphanumeric()
                || is_combining_mark(c)
                || (c == '%' && hex(at + 1) && hex(at + 2));
            if allowed {
                was_hyphen = c == '-';
                if hyphen_wanted {
                    if !was_hyphen {
                        out.push('-');
                    }
                    hyphen_wanted = false;
                }
                out.push(c);
            } else if !out.is_empty() && !was_hyphen && c.is_whitespace() {
                hyphen_wanted = true;
            }
        }
        match self.lower {
            true => lowered(&out),
            false => out,
        }
    }

    /// A name from the content folder made ready: read first as [`read`]
    /// reads it, unless `disablePathToLower` has Hugo take it as it is.
    pub(super) fn named(&self, name: &str) -> String {
        match self.lower {
            true => self.made(&read(name)),
            false => self.made(name),
        }
    }

    /// `text` made ready and then read and written again as a URL, as Hugo
    /// does with what it puts into a `[permalinks]` pattern: a `#` that
    /// opens an empty fragment is lost.
    fn urlized(&self, text: &str) -> String {
        let mut made = self.made(text);
        if made.find('#').is_some_and(|at| at + 1 == made.len()) {
            made.pop();
        }
        made
    }

    /// A path made ready, as a whole: each piece as [`Making::made`], the
    /// slashes between them kept, none doubled.
    pub(super) fn path(&self, path: &str) -> String {
        let made = self.made(path);
        let mut tidy = format!("/{made}");
        while tidy.contains("//") {
            tidy = tidy.replace("//", "/");
        }
        tidy
    }
}

/// A name from the content folder as Hugo reads every one, before
/// making it ready: in small letters, each space a hyphen. A run of spaces,
/// or a space beside a hyphen, so keeps a hyphen for each.
pub(super) fn read(name: &str) -> String {
    lowered(name).replace(' ', "-")
}

/// `text` in small letters as Go makes them, a letter at a time: `Σ` is
/// `σ` wherever it stands, and `İ` is `i`.
fn lowered(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            'İ' => 'i'.to_lowercase(),
            c => c.to_lowercase(),
        })
        .collect()
}

/// `text` with `+` read as a space and each `%` with two hex digits as the
/// byte they stand for, as Go's `QueryUnescape` reads it.
fn unescaped(text: &str) -> String {
    let bytes = text.as_bytes();
    let hex = |b: Option<&u8>| b.and_then(|b| char::from(*b).to_digit(16));
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match (bytes[at], hex(bytes.get(at + 1)), hex(bytes.get(at + 2))) {
            (b'%', Some(high), Some(low)) => {
                out.push((high * 16 + low) as u8);
                at += 3;
                continue;
            }
            (b'+', _, _) => out.push(b' '),
            (b, _, _) => out.push(b),
        }
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The address of a page whose path, made ready, is `path`: as a folder,
/// or as its own `.html` file where the settings ask for that.
pub(super) fn finished(path: &str, ugly: bool) -> String {
    let bare = path.trim_end_matches('/');
    match (ugly, bare.is_empty()) {
        (_, true) => "/".to_owned(),
        (true, false) => format!("{bare}.html"),
        (false, false) => format!("{bare}/"),
    }
}

/// What a page gives a `[permalinks]` pattern.
pub(super) struct Pieces<'a> {
    pub date: Option<&'a Date>,
    pub title: &'a str,
    pub slug: Option<&'a str>,
    /// The file's name, without its ending or language, or the folder's
    /// for a page kept as a folder.
    pub file_name: &'a str,
    /// The folders the page is in that are lists, outermost first, each as
    /// [`read`] gives it.
    pub sections: &'a [String],
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

/// Where Hugo publishes the page of the term `term` by the `[permalinks]`
/// pattern `pattern`: its `:slug` and `:title` are the term's name, put in
/// as a page's are.
pub(super) fn term(pattern: &str, term: &str, making: &Making) -> String {
    let urlized = making.urlized(term);
    let filled = pattern
        .replace(":slug", &urlized)
        .replace(":title", &urlized);
    cleaned(&filled, making)
}

/// A filled-in pattern read as Go's `QueryUnescape` reads it, then made
/// ready as a whole. Hugo makes it ready without the slash at its end, so
/// a space left before that is dropped rather than made a hyphen.
fn cleaned(filled: &str, making: &Making) -> String {
    let read = unescaped(filled);
    let bare = read.trim_end_matches('/');
    let made = making.path(bare);
    match bare.len() < read.len() {
        true => format!("{}/", made.trim_end_matches('/')),
        false => made,
    }
}

/// `pattern` filled in from `pieces`, as [`cleaned`] gives it. The
/// placeholder Statac doesn't know where it holds one.
pub(super) fn expand(pattern: &str, pieces: &Pieces, making: &Making) -> Result<String, String> {
    let mut out = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some(colon) = rest.find(':') {
        out.push_str(&rest[..colon]);
        let after = &rest[colon + 1..];
        let mut length = after
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(after.len());
        if length == 0 {
            out.push(':');
            rest = after;
            continue;
        }
        let name = &after[..length];
        let slice = match after[length..].starts_with('[') {
            true => {
                let end = after[length..]
                    .find(']')
                    .ok_or_else(|| format!(":{name}"))?;
                let slice = &after[length + 1..length + end];
                length += end + 1;
                Some(slice)
            }
            false => None,
        };
        let filled =
            piece(name, slice, pieces, making).ok_or_else(|| format!(":{}", &after[..length]))?;
        out.push_str(&filled);
        rest = &after[length..];
    }
    out.push_str(rest);
    Ok(cleaned(&out, making))
}

/// One placeholder's value.
fn piece(name: &str, slice: Option<&str>, pieces: &Pieces, making: &Making) -> Option<String> {
    let date = pieces.date.copied().unwrap_or(Date::day_only(1, 1, 1));
    let civil = jiff::civil::Date::new(
        i16::try_from(date.year).ok()?,
        i8::try_from(date.month).ok()?,
        i8::try_from(date.day).ok()?,
    )
    .ok()?;
    let title = || making.urlized(pieces.title);
    let slug = || pieces.slug.map(|slug| making.urlized(slug));
    let file_name = || making.urlized(pieces.file_name);
    if slice.is_some() && name != "sections" {
        return None;
    }
    Some(match name {
        "year" => format!("{:04}", date.year),
        "month" => format!("{:02}", date.month),
        "monthname" => MONTHS[usize::from(date.month).checked_sub(1)?].to_owned(),
        "day" => format!("{:02}", date.day),
        "weekday" => civil.weekday().to_sunday_zero_offset().to_string(),
        "weekdayname" => {
            DAYS[usize::try_from(civil.weekday().to_sunday_zero_offset()).ok()?].to_owned()
        }
        "yearday" => civil.day_of_year().to_string(),
        "title" => title(),
        "slug" => slug().unwrap_or_else(title),
        "slugorfilename" | "slugorcontentbasename" => slug().unwrap_or_else(file_name),
        "filename" | "contentbasename" => file_name(),
        "section" => pieces.sections.first().cloned().unwrap_or_default(),
        "sections" => {
            let all = pieces.sections;
            let chosen = match slice {
                None => all,
                Some(slice) => {
                    let (from, to) = slice.split_once(':')?;
                    let at = |n: &str, or: usize| match n.trim() {
                        "" => Some(or),
                        "last" => Some(all.len().saturating_sub(1)),
                        n => n.parse::<usize>().ok(),
                    };
                    let (from, to) = (at(from, 0)?, at(to, all.len())?);
                    all.get(from.min(all.len())..to.min(all.len()).max(from.min(all.len())))?
                }
            };
            chosen.join("/")
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HUGO: Making = Making {
        lower: true,
        accents_removed: false,
    };

    #[test]
    fn a_name_is_made_ready_as_hugo_makes_it() {
        for (given, made) in [
            ("Post 1", "post-1"),
            ("Post 1: Café, Déjà Vu!", "post-1-café-déjà-vu"),
            ("  Spaces   around  ", "spaces-around"),
            ("a - b", "a-b"),
            ("C++ & C#", "c++-c#"),
            ("v1.2_final", "v1.2_final"),
            ("100%", "100"),
            ("%20a", "%20a"),
            ("привет мир", "привет-мир"),
            ("日記", "日記"),
            ("What? Why!", "what-why"),
        ] {
            assert_eq!(HUGO.made(given), made, "{given}");
        }
        let cased = Making {
            lower: false,
            accents_removed: true,
        };
        assert_eq!(cased.made("Café Crème"), "Cafe-Creme");
    }

    const AS_IT_IS: Making = Making {
        lower: false,
        accents_removed: false,
    };

    const ACCENTS_REMOVED: Making = Making {
        lower: true,
        accents_removed: true,
    };

    /// Each as Hugo 0.166 published `content/<name>.md`.
    #[test]
    fn a_name_from_the_content_folder_is_made_ready_as_hugo_makes_it() {
        // The name, then made ready by default, under `disablePathToLower`
        // and under `removePathAccents`.
        for (name, made, as_it_is, accents_removed) in [
            ("p04- - q", "p04----q", "p04--q", "p04----q"),
            ("a  b", "a--b", "a-b", "a--b"),
            (" lead", "-lead", "lead", "-lead"),
            ("trail ", "trail-", "trail", "trail-"),
            ("-lead", "-lead", "-lead", "-lead"),
            ("trail-", "trail-", "trail-", "trail-"),
            ("a--b", "a--b", "a--b", "a--b"),
            ("a\u{a0}b", "a-b", "a-b", "a-b"),
            ("a\tb", "a-b", "a-b", "a-b"),
            ("a & b", "a--b", "a-b", "a--b"),
            ("a&b", "ab", "ab", "ab"),
            ("a (1)", "a-1", "a-1", "a-1"),
            ("a, b", "a-b", "a-b", "a-b"),
            ("what? why!", "what-why", "what-why", "what-why"),
            ("C++ & C#", "c++--c#", "C++-C#", "c++--c#"),
            ("v1.2_final", "v1.2_final", "v1.2_final", "v1.2_final"),
            ("100%", "100", "100", "100"),
            ("a%20b", "a%20b", "a%20b", "a%20b"),
            ("a~b@c+d#e", "a~b@c+d#e", "a~b@c+d#e", "a~b@c+d#e"),
            ("a'b\"c", "abc", "abc", "abc"),
            ("Café Crème", "café-crème", "Café-Crème", "cafe-creme"),
            ("Straße", "straße", "Straße", "straße"),
            ("UPPER Case", "upper-case", "UPPER-Case", "upper-case"),
            ("İstanbul", "istanbul", "İstanbul", "istanbul"),
            ("ΟΔΟΣ", "οδοσ", "ΟΔΟΣ", "οδοσ"),
            ("привет мир", "привет-мир", "привет-мир", "привет-мир"),
            ("日本 語", "日本-語", "日本-語", "日本-語"),
        ] {
            assert_eq!(HUGO.named(name), made, "{name:?}");
            assert_eq!(AS_IT_IS.named(name), as_it_is, "{name:?}");
            assert_eq!(ACCENTS_REMOVED.named(name), accents_removed, "{name:?}");
        }
    }

    /// Each as Hugo 0.166 published `content/posts/<name>.md` with
    /// `/:filename/` as the pattern for posts.
    #[test]
    fn a_name_in_a_pattern_is_made_ready_then_read_as_a_url() {
        // The name, then made ready by default and under
        // `disablePathToLower`.
        for (name, made, as_it_is) in [
            ("p04- - q", "/p04--q/", "/p04--q/"),
            (" lead", "/lead/", "/lead/"),
            ("a & b", "/a-b/", "/a-b/"),
            ("C++ & C#", "/c-c/", "/C-C/"),
            ("C++ & C# 2", "/c-c#-2/", "/C-C#-2/"),
            ("a~b@c+d#e", "/a~b@c-d#e/", "/a~b@c-d#e/"),
            ("a%20b", "/a-b/", "/a-b/"),
            ("UPPER Case", "/upper-case/", "/UPPER-Case/"),
            ("İstanbul", "/istanbul/", "/İstanbul/"),
            ("x+", "/x/", "/x/"),
            ("+y", "/-y/", "/-y/"),
            ("z%20", "/z/", "/z/"),
            ("w %20", "/w-/", "/w-/"),
        ] {
            let pieces = Pieces {
                date: None,
                title: name,
                slug: Some(name),
                file_name: name,
                sections: &[],
            };
            for pattern in ["/:filename/", "/:slug/", "/:title/"] {
                let got = expand(pattern, &pieces, &HUGO);
                assert_eq!(got.as_deref(), Ok(made), "{pattern} {name:?}");
                let got = expand(pattern, &pieces, &AS_IT_IS);
                assert_eq!(got.as_deref(), Ok(as_it_is), "{pattern} {name:?}");
            }
        }
    }

    /// Each as Hugo 0.166 published the tag `<name>` with `/tt/:slug/` as
    /// the pattern for tags.
    #[test]
    fn a_term_in_a_pattern_is_made_ready_as_a_pages_name_is() {
        for (name, made) in [
            ("p04- - q", "/tt/p04--q/"),
            ("a & b", "/tt/a-b/"),
            ("C++ & C#", "/tt/c-c/"),
            ("C++ & C# 2", "/tt/c-c#-2/"),
            ("a~b@c+d#e", "/tt/a~b@c-d#e/"),
            ("a%20b", "/tt/a-b/"),
            ("100%", "/tt/100/"),
            ("a/b", "/tt/a/b/"),
            ("x+", "/tt/x/"),
            ("+y", "/tt/-y/"),
        ] {
            for pattern in ["/tt/:slug/", "/tt/:title/"] {
                assert_eq!(term(pattern, name, &HUGO), made, "{pattern} {name:?}");
            }
        }
    }

    #[test]
    fn a_pattern_is_filled_in_from_the_page() {
        let date = Date::day_only(2026, 1, 4);
        let sections = ["posts".to_owned(), "notes".to_owned()];
        let pieces = Pieces {
            date: Some(&date),
            title: "Post 1: Café",
            slug: None,
            file_name: "Post-1",
            sections: &sections,
        };
        for (pattern, filled) in [
            ("/:year/:month/:day/:title/", "/2026/01/04/post-1-café/"),
            (
                "/:monthname/:weekdayname/:weekday/:yearday/",
                "/january/sunday/0/4/",
            ),
            ("/:slug/", "/post-1-café/"),
            ("/:slugorfilename/", "/post-1/"),
            ("/:filename/", "/post-1/"),
            ("/:contentbasename/", "/post-1/"),
            ("/:section/:filename", "/posts/post-1"),
            ("/:sections/:filename/", "/posts/notes/post-1/"),
            ("/:sections[1:]/:filename/", "/notes/post-1/"),
            ("/:sections[last]/", "/:sections[last]"),
        ] {
            let got = expand(pattern, &pieces, &HUGO);
            match filled.starts_with("/:") {
                true => assert_eq!(got, Err(filled[1..].to_owned()), "{pattern}"),
                false => assert_eq!(got, Ok(filled.to_owned()), "{pattern}"),
            }
        }
        assert_eq!(
            expand("/:year/:unknown/", &pieces, &HUGO),
            Err(":unknown".into())
        );
    }

    #[test]
    fn an_address_ends_as_a_folder_or_a_file() {
        assert_eq!(finished("/posts/post-1", false), "/posts/post-1/");
        assert_eq!(finished("/posts/post-1/", true), "/posts/post-1.html");
        assert_eq!(finished("/", true), "/");
    }
}
