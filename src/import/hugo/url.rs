//! Where Hugo publishes a page, by Hugo's own rules: a path made from the
//! file's place or a `[permalinks]` pattern, each name in it made ready for
//! an address as Hugo makes one.

use crate::date::Date;

/// `text` made ready for an address as Hugo makes the names a site most
/// often holds: letters and digits kept, in small letters, as are
/// `. / \ _ # + ~ - @`; a run of spaces between them one hyphen, and
/// everything else dropped.
pub(super) fn made(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut hyphen_wanted = false;
    for c in text.chars() {
        if c.is_alphanumeric() || matches!(c, '.' | '/' | '\\' | '_' | '#' | '+' | '~' | '-' | '@')
        {
            if hyphen_wanted && c != '-' {
                out.push('-');
            }
            hyphen_wanted = false;
            out.push(c);
        } else if c.is_whitespace() && !out.is_empty() && !out.ends_with('-') {
            hyphen_wanted = true;
        }
    }
    out.to_lowercase()
}

/// A name from the content folder, or a term, as Hugo reads every one
/// before making it ready: in small letters, each space a hyphen.
pub(super) fn read(name: &str) -> String {
    name.to_lowercase().replace(' ', "-")
}

/// A name from the content folder made ready, read first as [`read`]
/// reads it.
pub(super) fn named(name: &str) -> String {
    made(&read(name))
}

/// Where Hugo publishes the page of the term `term` in the list `plural`
/// where no pattern places it.
pub(super) fn listed(plural: &str, term: &str) -> String {
    format!("{}/", path(&read(&format!("/{plural}/{term}"))))
}

/// A path made ready, as a whole: each piece as [`made`], the slashes
/// between them kept, none doubled.
pub(super) fn path(path: &str) -> String {
    let mut tidy = format!("/{}", made(path));
    while tidy.contains("//") {
        tidy = tidy.replace("//", "/");
    }
    tidy
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

/// Where Hugo publishes the page of the term titled `title` by the
/// `[permalinks]` pattern `pattern`: its `:slug` and `:title` are that
/// title made ready.
pub(super) fn term(pattern: &str, title: &str) -> String {
    let made = made(title);
    path(&pattern.replace(":slug", &made).replace(":title", &made))
}

/// `pattern` filled in from `pieces`, and made ready as a whole. The
/// placeholder Statac doesn't know where it holds one.
pub(super) fn expand(pattern: &str, pieces: &Pieces) -> Result<String, String> {
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
        let filled = piece(name, slice, pieces).ok_or_else(|| format!(":{}", &after[..length]))?;
        out.push_str(&filled);
        rest = &after[length..];
    }
    out.push_str(rest);
    Ok(path(&out))
}

/// One placeholder's value.
fn piece(name: &str, slice: Option<&str>, pieces: &Pieces) -> Option<String> {
    let date = pieces.date.copied().unwrap_or(Date::day_only(1, 1, 1));
    let civil = jiff::civil::Date::new(
        i16::try_from(date.year).ok()?,
        i8::try_from(date.month).ok()?,
        i8::try_from(date.day).ok()?,
    )
    .ok()?;
    // A page's title gives one piece of the path, so a slash in it is
    // not a slash in the path.
    let title = || made(&pieces.title.replace(['/', '\\'], "-"));
    let slug = || pieces.slug.map(made);
    let file_name = || made(pieces.file_name);
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

    #[test]
    fn a_name_is_made_ready_as_hugo_makes_it() {
        for (given, made_ready) in [
            ("Post 1", "post-1"),
            ("Post 1: Café, Déjà Vu!", "post-1-café-déjà-vu"),
            ("  Spaces   around  ", "spaces-around"),
            ("a - b", "a-b"),
            ("C++ & C#", "c++-c#"),
            ("v1.2_final", "v1.2_final"),
            ("node.js", "node.js"),
            ("mod_deflate", "mod_deflate"),
            ("3 B+", "3-b+"),
            ("a~b@c", "a~b@c"),
            ("a'b\"c", "abc"),
            ("100%", "100"),
            ("привет мир", "привет-мир"),
            ("日記", "日記"),
            ("What? Why!", "what-why"),
        ] {
            assert_eq!(made(given), made_ready, "{given}");
        }
    }

    /// A name from the content folder has each space a hyphen, beside a
    /// hyphen or another space too.
    #[test]
    fn a_name_from_the_content_folder_keeps_a_hyphen_for_each_space() {
        for (name, made_ready) in [
            ("a b", "a-b"),
            ("a  b", "a--b"),
            ("a & b", "a--b"),
            ("x - y", "x---y"),
            ("UPPER Case", "upper-case"),
        ] {
            assert_eq!(named(name), made_ready, "{name:?}");
        }
        assert_eq!(listed("tags", "Tips & Tricks"), "/tags/tips--tricks/");
        assert_eq!(term("/tt/:slug/", "Tips & Tricks"), "/tt/tips-tricks/");
    }

    /// Each as Hugo 0.166 published a page titled `<title>` with
    /// `/t/:title/` as the pattern for its list, and with `/t/:slug/` for a
    /// page without a slug.
    #[test]
    fn a_slash_in_a_title_is_a_hyphen() {
        for (title, made_ready) in [
            ("a/b", "/t/a-b/"),
            ("a\\b", "/t/a-b/"),
            ("a / b", "/t/a-b/"),
            ("a \\ b", "/t/a-b/"),
            ("/a", "/t/-a/"),
            ("a/", "/t/a-/"),
            ("a//b", "/t/a--b/"),
            ("A/B\\C", "/t/a-b-c/"),
        ] {
            let pieces = Pieces {
                date: None,
                title,
                slug: None,
                file_name: "x",
                sections: &[],
            };
            for pattern in ["/t/:title/", "/t/:slug/"] {
                let got = expand(pattern, &pieces);
                assert_eq!(got.as_deref(), Ok(made_ready), "{pattern} {title:?}");
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
            let got = expand(pattern, &pieces);
            match filled.starts_with("/:") {
                true => assert_eq!(got, Err(filled[1..].to_owned()), "{pattern}"),
                false => assert_eq!(got, Ok(filled.to_owned()), "{pattern}"),
            }
        }
        assert_eq!(expand("/:year/:unknown/", &pieces), Err(":unknown".into()));
    }

    #[test]
    fn an_address_ends_as_a_folder_or_a_file() {
        assert_eq!(finished("/posts/post-1", false), "/posts/post-1/");
        assert_eq!(finished("/posts/post-1/", true), "/posts/post-1.html");
        assert_eq!(finished("/", true), "/");
    }
}
