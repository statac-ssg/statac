//! The title Hugo gives a page it makes for a term, which a `[permalinks]`
//! pattern's `:slug` and `:title` are made from: the term's name in
//! capitals by the settings' `titleCaseStyle`, or as it is written under
//! `capitalizeListTitles = false`.

use icu_properties::CodePointMapData;
use icu_properties::props::{GeneralCategory, GeneralCategoryGroup};

/// What a word starts with.
const LETTERS_AND_NUMBERS: GeneralCategoryGroup =
    GeneralCategoryGroup::Letter.union(GeneralCategoryGroup::Number);

/// Whether `c` is in `group`, by its Unicode general category.
fn is(group: GeneralCategoryGroup, c: char) -> bool {
    group.contains(CodePointMapData::<GeneralCategory>::new().get(c))
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

/// The words AP's style leaves in small letters within a title.
const SMALL_WORDS: &[&str] = &[
    "a", "an", "and", "as", "at", "but", "by", "en", "for", "if", "in", "nor", "of", "on", "or",
    "per", "the", "to", "vs", "vs.", "via", "v", "v.",
];

/// The words Chicago's style leaves in small letters within a title
/// besides AP's.
const PREPOSITIONS: &[&str] = &[
    "with",
    "from",
    "into",
    "during",
    "including",
    "until",
    "against",
    "among",
    "throughout",
    "despite",
    "towards",
    "upon",
    "concerning",
    "about",
    "over",
    "through",
    "before",
    "between",
    "after",
    "since",
    "without",
    "under",
    "within",
    "along",
    "following",
    "across",
    "beyond",
    "around",
    "down",
    "near",
    "above",
];

/// How the settings have Hugo put a title in capitals.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(super) enum Style {
    /// The Associated Press's, Hugo's own: each word starts with a
    /// capital, but for a few short words within.
    Ap,
    /// The Chicago Manual's: as AP's, and prepositions within too.
    Chicago,
    /// Go's: each word starts with a capital.
    Go,
    /// The first letter alone a capital.
    FirstUpper,
    /// As it is written.
    None,
}

impl Style {
    /// The style `titleCaseStyle` names, in any capitals: AP's for any
    /// name Hugo doesn't know.
    pub(super) fn named(name: &str) -> Style {
        match lowered(name).as_str() {
            "chicago" => Style::Chicago,
            "go" => Style::Go,
            "firstupper" => Style::FirstUpper,
            "none" => Style::None,
            _ => Style::Ap,
        }
    }

    /// `text` in this style's capitals, as Hugo 0.166 gives them.
    pub(super) fn titled(self, text: &str) -> String {
        match self {
            Style::Ap => headline(text, |word| SMALL_WORDS.contains(&word)),
            Style::Chicago => headline(text, |word| {
                SMALL_WORDS.contains(&word) || PREPOSITIONS.contains(&word)
            }),
            Style::Go => {
                let mut previous = ' ';
                (text.chars())
                    .map(|c| match separates(std::mem::replace(&mut previous, c)) {
                        true => title(c),
                        false => c,
                    })
                    .collect()
            }
            Style::FirstUpper => {
                let mut chars = text.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", upper(first), chars.as_str()),
                    None => String::new(),
                }
            }
            Style::None => text.to_owned(),
        }
    }
}

/// `text` with each word starting with a capital but for those `small`
/// names within, as the `prose` library Hugo uses gives it. That finds
/// each word again in a copy of `text` with its curly quotes, dashes and
/// ellipses written plainly, from a byte as far on from the last as that
/// word had characters, and decides by the bytes around where it finds
/// it; so this does too.
fn headline(text: &str, small: impl Fn(&str) -> bool) -> String {
    let mut plain = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\u{201c}' | '\u{201d}' => plain.push('"'),
            '\u{2018}' | '\u{2019}' => plain.push('\''),
            '\u{2013}' | '\u{2014}' => plain.push('-'),
            '\u{2026}' => plain.push_str("..."),
            c => plain.push(c),
        }
    }
    let bytes = plain.as_bytes();
    // The byte at `at`, or else the first.
    let byte = |at: isize| {
        let at = usize::try_from(at).ok().and_then(|at| bytes.get(at));
        at.or(bytes.first()).copied().unwrap_or_default()
    };
    let mut out = String::with_capacity(text.len());
    let mut copied = 0;
    let mut from: isize = 0;
    for (start, word) in words(text) {
        out.push_str(&text[copied..start]);
        copied = start + word.len();
        let found = usize::try_from(from)
            .ok()
            .and_then(|from| bytes.get(from..))
            .and_then(|rest| (rest.windows(word.len())).position(|w| w == word.as_bytes()));
        let at = from + found.map_or(-1, |n| n as isize);
        let length = word.chars().count() as isize;
        from = at + length;
        let lower = lowered(word);
        let within = at != 0 && from != bytes.len() as isize;
        if within
            && small(&lower)
            && matches!(byte(at - 1), b' ' | b'-' | b'/')
            && !matches!(byte(at - 2), b':' | b'-')
            && (byte(at + length) != b'-' || byte(at - 1) == b'-')
        {
            out.push_str(&lower);
        } else {
            let mut chars = word.chars();
            out.extend(chars.next().map(title));
            out.push_str(chars.as_str());
        }
    }
    out.push_str(&text[copied..]);
    out
}

/// Each word of `text`, with the byte it starts at, as the `prose`
/// library finds them: from a letter or number up to a space, a hyphen or
/// a slash.
fn words(text: &str) -> Vec<(usize, &str)> {
    let mut words = Vec::new();
    let mut from = 0;
    while let Some(start) = text[from..].find(|c| is(LETTERS_AND_NUMBERS, c)) {
        let start = from + start;
        let end = (text[start..].find(['\t', '\n', '\x0c', '\r', ' ', '-', '/']))
            .map_or(text.len(), |end| start + end);
        words.push((start, &text[start..end]));
        from = end;
    }
    words
}

/// Whether Go's `strings.Title` takes `c` to end a word: a space, or an
/// ASCII character other than a letter, a digit or `_`.
fn separates(c: char) -> bool {
    match c.is_ascii() {
        true => !c.is_ascii_alphanumeric() && c != '_',
        false => c.is_whitespace(),
    }
}

/// `c` as a capital as Go makes one, a character for a character: where
/// Unicode makes it more than one, Go keeps it, but for a Greek letter
/// with an iota beneath, which takes the capital with the iota beneath.
fn upper(c: char) -> char {
    let mut upper = c.to_uppercase();
    match (upper.next(), upper.next()) {
        (Some(one), None) => one,
        _ => match c {
            '\u{1f80}'..='\u{1f87}' | '\u{1f90}'..='\u{1f97}' | '\u{1fa0}'..='\u{1fa7}' => {
                char::from_u32(u32::from(c) + 8).unwrap_or(c)
            }
            'ᾳ' | 'ῃ' | 'ῳ' => char::from_u32(u32::from(c) + 9).unwrap_or(c),
            c => c,
        },
    }
}

/// `c` as it starts a word in a title, as Go makes it: as [`upper`], but
/// for the letters written as two, `ǆ` as `ǅ`, and Georgian letters, kept.
fn title(c: char) -> char {
    match c {
        'Ǆ'..='ǆ' => 'ǅ',
        'Ǉ'..='ǉ' => 'ǈ',
        'Ǌ'..='ǌ' => 'ǋ',
        'Ǳ'..='ǳ' => 'ǲ',
        '\u{10d0}'..='\u{10fa}' | '\u{10fd}'..='\u{10ff}' => c,
        c => upper(c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each as Hugo 0.166 gave it, by its `title` function under each
    /// `titleCaseStyle`.
    #[test]
    fn a_title_is_in_capitals_as_hugo_gives_them() {
        // The text, then as AP's style, Chicago's, Go's and `firstupper`
        // give it.
        for (text, ap, chicago, go, first_upper) in [
            (
                "the art of war",
                "The Art of War",
                "The Art of War",
                "The Art Of War",
                "The art of war",
            ),
            (
                "into the wild",
                "Into the Wild",
                "Into the Wild",
                "Into The Wild",
                "Into the wild",
            ),
            ("x from y", "X From Y", "X from Y", "X From Y", "X from y"),
            ("x over y", "X Over Y", "X over Y", "X Over Y", "X over y"),
            ("x: the y", "X: The Y", "X: The Y", "X: The Y", "X: the y"),
            (
                "x - the y",
                "X - The Y",
                "X - The Y",
                "X - The Y",
                "X - the y",
            ),
            ("x-the-y", "X-the-Y", "X-the-Y", "X-The-Y", "X-the-y"),
            ("the-x", "The-X", "The-X", "The-X", "The-x"),
            ("x the-y", "X The-Y", "X The-Y", "X The-Y", "X the-y"),
            ("x-the y", "X-the Y", "X-the Y", "X-The Y", "X-the y"),
            ("x/of/y", "X/of/Y", "X/of/Y", "X/Of/Y", "X/of/y"),
            ("x vs. y", "X vs. Y", "X vs. Y", "X Vs. Y", "X vs. y"),
            ("x v. y", "X v. Y", "X v. Y", "X V. Y", "X v. y"),
            ("x of", "X Of", "X Of", "X Of", "X of"),
            ("of", "Of", "Of", "Of", "Of"),
            ("éa a x", "Éa A X", "Éa A X", "Éa A X", "Éa a x"),
            ("a’a a x", "A’a A X", "A’a A X", "A’a A X", "A’a a x"),
            (
                "x \u{2013} the y",
                "X \u{2013} The Y",
                "X \u{2013} The Y",
                "X \u{2013} The Y",
                "X \u{2013} the y",
            ),
            (
                "x “the” y",
                "X “The” Y",
                "X “The” Y",
                "X “the” Y",
                "X “the” y",
            ),
            ("x … of y", "X … of Y", "X … of Y", "X … Of Y", "X … of y"),
            ("x İn y", "X in Y", "X in Y", "X İn Y", "X İn y"),
            ("ა x", "ა X", "ა X", "ა X", "Ა x"),
            ("ǆx ǆ", "ǅx ǅ", "ǅx ǅ", "ǅx ǅ", "Ǆx ǆ"),
            ("ᾳx", "ᾼx", "ᾼx", "ᾼx", "ᾼx"),
            ("ßx", "ßx", "ßx", "ßx", "ßx"),
            ("ıx", "Ix", "Ix", "Ix", "Ix"),
            ("ςx", "Σx", "Σx", "Σx", "Σx"),
            ("2nd of x", "2nd of X", "2nd of X", "2nd Of X", "2nd of x"),
            ("x ½ of y", "X ½ of Y", "X ½ of Y", "X ½ Of Y", "X ½ of y"),
            ("x_of y", "X_of Y", "X_of Y", "X_of Y", "X_of y"),
            (
                "o'neil of x",
                "O'neil of X",
                "O'neil of X",
                "O'Neil Of X",
                "O'neil of x",
            ),
            (
                "x\u{a0}of y",
                "X\u{a0}of Y",
                "X\u{a0}of Y",
                "X\u{a0}Of Y",
                "X\u{a0}of y",
            ),
            ("x\tof y", "X\tOf Y", "X\tOf Y", "X\tOf Y", "X\tof y"),
            ("x (of) y", "X (Of) Y", "X (Of) Y", "X (Of) Y", "X (of) y"),
            (
                "UPPER case",
                "UPPER Case",
                "UPPER Case",
                "UPPER Case",
                "UPPER case",
            ),
            ("a & b", "A & B", "A & B", "A & B", "A & b"),
            (
                "e\u{301}clair",
                "E\u{301}clair",
                "E\u{301}clair",
                "E\u{301}clair",
                "E\u{301}clair",
            ),
        ] {
            assert_eq!(Style::Ap.titled(text), ap, "{text:?}");
            assert_eq!(Style::Chicago.titled(text), chicago, "{text:?}");
            assert_eq!(Style::Go.titled(text), go, "{text:?}");
            assert_eq!(Style::FirstUpper.titled(text), first_upper, "{text:?}");
            assert_eq!(Style::None.titled(text), text, "{text:?}");
        }
        for (name, style) in [
            ("Chicago", Style::Chicago),
            ("go", Style::Go),
            ("FirstUpper", Style::FirstUpper),
            ("none", Style::None),
            ("ap", Style::Ap),
            ("anything", Style::Ap),
        ] {
            assert_eq!(Style::named(name), style, "{name}");
        }
    }
}
