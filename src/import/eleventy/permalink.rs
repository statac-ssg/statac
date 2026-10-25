//! A `permalink` holding template tags, worked out as Eleventy would where
//! it uses only what Statac can know: `page.fileSlug`, `page.filePathStem`,
//! `page.date` through Liquid's `date` filter, the page's own fields, text
//! in quotes, and the filters `slugify` and `slug`.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::date::Date;

/// What a page gives its `permalink`.
pub(super) struct Facts<'a> {
    /// The file's name without its ending or the date it opens with, or
    /// its folder's name where the file is `index`.
    pub file_slug: &'a str,
    /// The file's path from the top of the input folder, with `/` before
    /// it and no ending: `/posts/post-1`.
    pub file_path_stem: &'a str,
    pub date: Option<&'a Date>,
    /// The page's fields that hold text or a number, as text.
    pub fields: &'a [(String, String)],
}

/// `permalink` with each `{{ }}` worked out; or, where one can't be, that
/// tag as written.
pub(super) fn fill(permalink: &str, facts: &Facts) -> Result<String, String> {
    let mut out = String::with_capacity(permalink.len());
    let mut rest = permalink;
    loop {
        let next = [rest.find("{{"), rest.find("{%"), rest.find("{#")]
            .into_iter()
            .flatten()
            .min();
        let Some(open) = next else {
            out.push_str(rest);
            return Ok(out);
        };
        out.push_str(&rest[..open]);
        let after = &rest[open..];
        let close = match &after[..2] {
            "{{" => "}}",
            "{%" => "%}",
            _ => "#}",
        };
        let Some(end) = after[2..].find(close).map(|n| n + 4) else {
            return Err(after.to_owned());
        };
        let written = &after[..end];
        if close != "}}" {
            return Err(written.to_owned());
        }
        let inner = written[2..end - 2]
            .trim_start_matches('-')
            .trim_end_matches('-')
            .trim();
        out.push_str(&expression(inner, facts).ok_or_else(|| written.to_owned())?);
        rest = &after[end..];
    }
}

/// `inner` split at each `|` outside quotes.
fn filters(inner: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut quote = None;
    let mut start = 0;
    for (at, c) in inner.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '"' | '\'') => quote = Some(c),
            (None, '|') => {
                parts.push(inner[start..at].trim());
                start = at + 1;
            }
            _ => {}
        }
    }
    parts.push(inner[start..].trim());
    parts
}

/// Text in quotes, as it reads.
fn literal(text: &str) -> Option<String> {
    let quote = text.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    let inner = text.strip_prefix(quote)?.strip_suffix(quote)?;
    (text.len() > 1 && !inner.contains(quote)).then(|| inner.to_owned())
}

/// What `{{ inner }}` writes, where Statac can tell.
fn expression(inner: &str, facts: &Facts) -> Option<String> {
    let parts = filters(inner);
    let (first, rest) = parts.split_first()?;
    let mut value = match *first {
        "page.fileSlug" => Value::Text(facts.file_slug.to_owned()),
        "page.filePathStem" => Value::Text(facts.file_path_stem.to_owned()),
        "page.date" => Value::Date(*facts.date?),
        name if literal(name).is_some() => Value::Text(literal(name)?),
        name => {
            let name = name.strip_prefix("page.").map_or(name, |_| "");
            let (_, text) = facts.fields.iter().find(|(field, _)| field == name)?;
            Value::Text(text.clone())
        }
    };
    for filter in rest {
        let (name, argument) = match filter.split_once(':') {
            Some((name, argument)) => (name.trim(), Some(argument.trim())),
            None => (*filter, None),
        };
        value = match (name, argument, value) {
            ("slugify", None, Value::Text(text)) => Value::Text(slugify(&text)),
            ("slug", None, Value::Text(text)) => Value::Text(slug(&text)),
            ("date", Some(format), Value::Date(date)) => {
                Value::Text(strftime(&date, &literal(format)?)?)
            }
            _ => return None,
        };
    }
    match value {
        Value::Text(text) => Some(text),
        Value::Date(_) => None,
    }
}

enum Value {
    Text(String),
    Date(Date),
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

/// `date` written by `format`, as Liquid's `date` filter writes it, where
/// the format uses only the day's pieces Statac knows.
fn strftime(date: &Date, format: &str) -> Option<String> {
    let mut out = String::new();
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let month = MONTHS[usize::from(date.month.checked_sub(1)?)];
        match chars.next()? {
            'Y' => out.push_str(&date.year.to_string()),
            'y' => out.push_str(&format!("{:02}", date.year.rem_euclid(100))),
            'm' => out.push_str(&format!("{:02}", date.month)),
            'd' => out.push_str(&format!("{:02}", date.day)),
            'e' => out.push_str(&format!("{:>2}", date.day)),
            'B' => out.push_str(month),
            'b' => out.push_str(&month[..3]),
            '%' => out.push('%'),
            _ => return None,
        }
    }
    Some(out)
}

/// A letter with its accent taken off, and the few Latin letters that are
/// written as others.
fn plain_letters(text: &str) -> String {
    text.nfd()
        .filter(|c| !is_combining_mark(*c))
        .flat_map(|c| {
            let spelled = match c {
                'ß' => "ss",
                'æ' => "ae",
                'Æ' => "AE",
                'œ' => "oe",
                'Œ' => "OE",
                'ø' => "o",
                'Ø' => "O",
                'đ' => "d",
                'Đ' => "D",
                'ł' => "l",
                'Ł' => "L",
                'þ' => "th",
                'Þ' => "TH",
                _ => "",
            };
            match spelled.is_empty() {
                true => vec![c],
                false => spelled.chars().collect(),
            }
        })
        .collect()
}

/// Eleventy's `slugify` filter: accents off, `&` as `and`, apostrophes
/// dropped, small letters, and every other run of characters but letters
/// and figures one hyphen.
fn slugify(text: &str) -> String {
    let text = plain_letters(text)
        .replace('&', " and ")
        .replace(['\'', '’'], "");
    let mut out = String::new();
    let mut gap = false;
    for c in text.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    out
}

/// Eleventy's older `slug` filter: accents off, `&` as `and`, every
/// character but ASCII letters, figures, spaces and `$*_+~.()'"!-:@`
/// dropped, runs of spaces and hyphens one hyphen, and small letters.
fn slug(text: &str) -> String {
    let kept: String = plain_letters(text)
        .replace('&', "and")
        .chars()
        .map(|c| if c == '-' { ' ' } else { c })
        .filter(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || "$*_+~.()'\"!:@".contains(*c))
        .collect();
    kept.split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts<'a>(date: Option<&'a Date>, fields: &'a [(String, String)]) -> Facts<'a> {
        Facts {
            file_slug: "post-1",
            file_path_stem: "/posts/post-1",
            date,
            fields,
        }
    }

    #[test]
    fn a_permalink_is_filled_where_every_tag_is_known() {
        let date = Date::day_only(2026, 1, 2);
        let fields = [("title".to_owned(), "Post 1: Café & Tea".to_owned())];
        let facts = facts(Some(&date), &fields);
        assert_eq!(
            fill(
                "/{{ page.date | date: \"%Y/%m\" }}/{{ title | slugify }}/",
                &facts
            ),
            Ok("/2026/01/post-1-cafe-and-tea/".to_owned())
        );
        assert_eq!(
            fill("/{{ title | slug }}/", &facts),
            Ok("/post-1:-cafe-and-tea/".to_owned())
        );
        assert_eq!(
            fill("{{ page.filePathStem }}.html", &facts),
            Ok("/posts/post-1.html".to_owned())
        );
        assert_eq!(
            fill("/{{ page.fileSlug | upcase }}/", &facts),
            Err("{{ page.fileSlug | upcase }}".to_owned())
        );
        assert_eq!(
            fill("/{% if a %}a{% endif %}/", &facts),
            Err("{% if a %}".to_owned())
        );
        let undated = super::Facts {
            date: None,
            ..facts
        };
        assert!(fill("/{{ page.date | date: '%Y' }}/", &undated).is_err());
    }
}
