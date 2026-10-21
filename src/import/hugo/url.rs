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
            true => out.to_lowercase(),
            false => out,
        }
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
    /// The folders the page is in that are lists, outermost first.
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

/// `pattern` filled in from `pieces`, then made ready as a whole. The
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
    Ok(making.path(&out))
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
    let title = || making.made(pieces.title);
    let slug = || pieces.slug.map(|slug| making.made(slug));
    let file_name = || making.made(pieces.file_name);
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
