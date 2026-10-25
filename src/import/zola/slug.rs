//! How Zola makes a name ready for an address, by the settings'
//! `[slugify]`: `on`, `safe` or `off`.

use deunicode::deunicode_char;

/// One of `[slugify]`'s ways.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum Strategy {
    /// Small ASCII letters and figures, each run of anything else one
    /// hyphen, none at either end; a character outside ASCII spelled in
    /// ASCII by `deunicode` first, as the `slug` crate Zola uses does.
    On,
    /// As written, with the characters no file name may hold on Windows,
    /// and `#`, taken out, and spaces and full stops taken off the end.
    Safe,
    /// As written.
    Off,
}

impl Strategy {
    pub(super) fn named(name: &str) -> Option<Strategy> {
        match name {
            "on" => Some(Strategy::On),
            "safe" => Some(Strategy::Safe),
            "off" => Some(Strategy::Off),
            _ => None,
        }
    }

    /// `text` made ready.
    pub(super) fn made(self, text: &str) -> String {
        match self {
            Strategy::On => on(text),
            Strategy::Safe => text
                .trim_end_matches([' ', '.'])
                .chars()
                .filter(|c| !"<>:\"/\\|?*#".contains(*c))
                .collect(),
            Strategy::Off => text.to_owned(),
        }
    }
}

fn on(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut hyphen = true;
    for c in text.chars() {
        let mut own = [0; 4];
        let spelled = match c.is_ascii() {
            true => &*c.encode_utf8(&mut own),
            // A character `deunicode` has no spelling for parts words.
            false => deunicode_char(c).unwrap_or("-"),
        };
        for c in spelled.chars() {
            if c.is_ascii_alphanumeric() {
                slug.push(c.to_ascii_lowercase());
                hyphen = false;
            } else if !hyphen {
                slug.push('-');
                hyphen = true;
            }
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_made_ready_by_each_way() {
        let name = "Post 1: Café, Déjà Vu?";
        assert_eq!(Strategy::On.made(name), "post-1-cafe-deja-vu");
        assert_eq!(Strategy::Safe.made(name), "Post 1 Café, Déjà Vu");
        assert_eq!(Strategy::Off.made(name), name);
        assert_eq!(Strategy::Safe.made("a<b>#c. ."), "abc");
    }

    #[test]
    fn on_spells_each_script_as_zola_does() {
        for (given, made) in [
            ("日本", "ri-ben"),
            ("東京タワー", "dong-jing-tawa"),
            ("ひらがな", "hiragana"),
            ("한국어", "hangugeo"),
            ("Привет мир", "privet-mir"),
            ("Ελληνικά", "ellenika"),
            ("العربية", "l-rby"),
            ("עברית", "bryt"),
            ("हिन्दी", "hindii"),
            ("ภาษาไทย", "phaasaaaithy"),
            ("ქართული", "k-art-uli"),
            ("😀 smile", "grinning-smile"),
            ("\u{1f1ec}\u{1f1e7} flag", "gb-flag"),
            ("C++ & C#", "c-c"),
            ("  Spaces  ", "spaces"),
            ("Straße \u{2013} Œuvre", "strasse-oeuvre"),
            ("a_b.c", "a-b-c"),
            ("Æúű--cool?", "aeuu-cool"),
            ("Đ ð þ ł ø ŉ", "d-d-th-l-o-n"),
            ("x\u{301}", "x"),
            ("\u{fb01}nance", "finance"),
            ("\u{201c}quotes\u{201d} «guillemets»", "quotes-guillemets"),
            ("€100 £5 ¥", "eur100-ps5-y"),
            ("½", "1-2"),
            ("zero\u{200b}width", "zero-width"),
            ("a\u{e000}b", "a-b"),
            ("!!!", ""),
            ("日本 and English", "ri-ben-and-english"),
            ("Ελλάδα-日本", "ellada-ri-ben"),
        ] {
            assert_eq!(Strategy::On.made(given), made, "{given}");
        }
    }
}
