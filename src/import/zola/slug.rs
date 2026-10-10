//! How Zola makes a name ready for an address, by the settings'
//! `[slugify]`: `on`, `safe` or `off`.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

/// One of `[slugify]`'s ways.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Strategy {
    /// Small ASCII letters and figures, each run of anything else one
    /// hyphen, none at either end; a letter outside ASCII written in ASCII
    /// first.
    On,
    /// As written, with the characters no file name may hold on Windows,
    /// and `#`, taken out, and spaces and full stops taken off the end.
    Safe,
    /// As written.
    Off,
}

impl Strategy {
    pub fn named(name: &str) -> Option<Strategy> {
        match name {
            "on" => Some(Strategy::On),
            "safe" => Some(Strategy::Safe),
            "off" => Some(Strategy::Off),
            _ => None,
        }
    }

    /// `text` made ready, or none where Statac can't tell what Zola
    /// made of it: a letter outside ASCII, under `on`, that Statac can't
    /// write in ASCII as Zola does.
    pub fn made(self, text: &str) -> Option<String> {
        match self {
            Strategy::On => on(text),
            Strategy::Safe => Some(
                text.trim_end_matches([' ', '.'])
                    .chars()
                    .filter(|c| !"<>:\"/\\|?*#".contains(*c))
                    .collect(),
            ),
            Strategy::Off => Some(text.to_owned()),
        }
    }
}

/// Letters Zola writes in ASCII that don't come apart into a letter and
/// its accent.
const OWN_SPELLINGS: &[(char, &str)] = &[
    ('ß', "ss"),
    ('æ', "ae"),
    ('Æ', "AE"),
    ('œ', "oe"),
    ('Œ', "OE"),
    ('ø', "o"),
    ('Ø', "O"),
    ('ł', "l"),
    ('Ł', "L"),
    ('đ', "d"),
    ('Đ', "D"),
    ('ð', "d"),
    ('Ð', "D"),
    ('þ', "th"),
    ('Þ', "TH"),
    ('ı', "i"),
];

/// Whether `c` is a space, a dash or a quotation mark outside ASCII, which
/// Zola writes as ASCII punctuation, so as a hyphen.
fn punctuation(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '\u{2010}'..='\u{2027}' | '\u{00ab}' | '\u{00bb}' | '\u{2039}' | '\u{203a}'
        )
}

fn on(text: &str) -> Option<String> {
    let mut ascii = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii() {
            ascii.push(c);
        } else if let Some((_, spelled)) = OWN_SPELLINGS.iter().find(|(o, _)| *o == c) {
            ascii.push_str(spelled);
        } else if punctuation(c) {
            ascii.push(' ');
        } else {
            let bare: String = c.nfd().filter(|c| !is_combining_mark(*c)).collect();
            if !bare.is_ascii() || bare.is_empty() {
                return None;
            }
            ascii.push_str(&bare);
        }
    }
    let mut slug = String::with_capacity(ascii.len());
    let mut hyphen = true;
    for c in ascii.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            hyphen = false;
        } else if !hyphen {
            slug.push('-');
            hyphen = true;
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    Some(slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_made_ready_by_each_way() {
        let name = "Post 1: Café, Déjà Vu?";
        assert_eq!(
            Strategy::On.made(name).as_deref(),
            Some("post-1-cafe-deja-vu")
        );
        assert_eq!(
            Strategy::Safe.made(name).as_deref(),
            Some("Post 1 Café, Déjà Vu")
        );
        assert_eq!(Strategy::Off.made(name).as_deref(), Some(name));
        for (given, made) in [
            ("C++ & C#", Some("c-c")),
            ("  Spaces  ", Some("spaces")),
            ("Straße \u{2013} Œuvre", Some("strasse-oeuvre")),
            ("a_b.c", Some("a-b-c")),
            ("日記", None),
        ] {
            assert_eq!(Strategy::On.made(given).as_deref(), made, "{given}");
        }
        assert_eq!(Strategy::Safe.made("a<b>#c. .").as_deref(), Some("abc"));
    }
}
