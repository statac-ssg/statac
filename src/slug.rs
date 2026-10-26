//! Names made into the pieces of an address.

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::{compose, is_combining_mark};

/// Lower case, words joined by hyphens, everything else dropped, with a
/// Latin letter's accent folded to its plain letter on the way.
///
/// A letter or figure of any script is kept, so `привет` stays `привет` and
/// `日記` stays `日記`. The fold is Latin's alone, because `ò` is `o` in
/// every language that writes it while `й` is not `и`. Arabic's vowel marks
/// and tatweel are dropped as a Latin accent is: a writer may leave the
/// vowels out, and the tatweel only stretches the line. Empty where the name
/// holds no letter or figure.
pub fn slugify(text: &str) -> String {
    let mut slug = String::with_capacity(text.len());
    let mut hyphen = true;
    let mut kept = false;
    for c in lowered(text).chars() {
        kept = kept_in_a_name(c, kept);
        if kept {
            slug.push(c);
            hyphen = false;
        } else if !hyphen {
            slug.push('-');
            hyphen = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

/// A heading's words as the id a link to it ends with: lower case, each
/// space and hyphen a hyphen and each underscore kept, a letter or figure
/// of any script kept, and everything else dropped. Unlike a slug, the
/// hyphens stay as typed, so a link written from the heading's words lands;
/// and another script's marks stay in the order they were typed, as the
/// link spells them. Empty where the words hold no letter or figure.
pub fn anchor(text: &str) -> String {
    let mut anchor = String::with_capacity(text.len());
    let mut kept = false;
    for c in folded(text.trim()).to_lowercase().chars() {
        kept = match c {
            ' ' | '-' | '_' => {
                anchor.push(if c == '_' { '_' } else { '-' });
                false
            }
            _ => kept_in_a_name(c, kept),
        };
        if kept {
            anchor.push(c);
        }
    }
    anchor
}

/// A folder's path under `content/` as its address spells it, each name
/// made a slug. A name with no letter or figure keeps its own spelling.
pub fn tidied_path(path: &str) -> String {
    let mut tidied = String::with_capacity(path.len());
    for name in path.split('/').filter(|name| !name.is_empty()) {
        let slug = slugify(name);
        tidied.push_str(if slug.is_empty() { name } else { &slug });
        tidied.push('/');
    }
    tidied
}

/// A slug made from something with no letter or figure in it: eight hex
/// digits of a hash of `path`, the same on every machine.
pub fn made_slug(path: &str) -> String {
    // FNV-1a: stable across builds and platforms, unlike the standard hasher.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{:08x}", hash >> 32)
}

/// The text with its characters composed the one way, Unicode's NFC. A
/// disk may hand back `é` as `e` and a separate accent; both must reach one
/// address.
pub fn nfc(text: &str) -> String {
    text.nfc().collect()
}

/// Whether a character stays in a name: a letter or figure of any script,
/// or a mark or joiner on one just kept. A zero-width non-joiner is part of
/// a Persian word, as a zero-width joiner is of many.
fn kept_in_a_name(c: char, after_a_kept_one: bool) -> bool {
    c.is_alphanumeric()
        || (after_a_kept_one && (is_combining_mark(c) || matches!(c, '\u{200c}' | '\u{200d}')))
}

/// Folded, lower-cased as a whole (so a Greek sigma ending a word is told
/// from one inside it), and composed.
fn lowered(text: &str) -> String {
    nfc(&folded(text).to_lowercase())
}

/// The Latin letters whose accent is part of the letter itself, so nothing
/// comes apart from them, each with its plain spelling.
fn plain_letter(c: char) -> Option<&'static str> {
    Some(match c {
        'ß' | 'ẞ' => "ss",
        'æ' | 'Æ' => "ae",
        'œ' | 'Œ' => "oe",
        'ø' | 'Ø' => "o",
        'đ' | 'Đ' | 'ð' | 'Ð' => "d",
        'þ' | 'Þ' => "th",
        'ł' | 'Ł' => "l",
        'ħ' | 'Ħ' => "h",
        'ŋ' | 'Ŋ' => "n",
        'ŧ' | 'Ŧ' => "t",
        'ı' => "i",
        'ĸ' => "k",
        _ => return None,
    })
}

/// A letter typed whole, without its marks.
fn latin_base(c: char) -> char {
    std::iter::once(c).nfd().next().unwrap_or(c)
}

/// Latin, or the punctuation and figures every script shares.
fn latin_or_common(c: char) -> bool {
    matches!(c as u32,
        0..=0x36F | 0x1E00..=0x1EFF | 0x2C60..=0x2C7F | 0xA720..=0xA7FF
        | 0xAB30..=0xAB6F | 0xFB00..=0xFB06 | 0xFF21..=0xFF5A)
        || !c.is_alphabetic()
}

/// Arabic's vowel marks, the dagger alef and the tatweel.
fn arabic_vowel_or_stretch(c: char) -> bool {
    matches!(c, '\u{64b}'..='\u{65f}' | '\u{670}' | '\u{640}')
}

/// Every Latin accent folded to its plain letter. A letter of another
/// script keeps its marks as typed; a mark on anything else is dropped.
fn folded(text: &str) -> String {
    let mut plain = String::with_capacity(text.len());
    let mut keeps_its_marks = false;
    for typed in text.chars() {
        // A hamza or madda typed apart from its alef makes a letter, and stays.
        if arabic_vowel_or_stretch(typed)
            && plain
                .chars()
                .next_back()
                .and_then(|letter| compose(letter, typed))
                .is_none()
        {
            continue;
        }
        if is_combining_mark(typed) {
            if keeps_its_marks {
                plain.push(typed);
            }
            continue;
        }
        keeps_its_marks = typed.is_alphabetic() && !latin_or_common(typed);
        if keeps_its_marks {
            plain.push(typed);
            continue;
        }
        let base = latin_base(typed);
        match plain_letter(base) {
            Some(letters) => plain.push_str(letters),
            None => plain.push(base),
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_lower_case_words_joined_by_hyphens() {
        let cases = [
            ("Post 1", "post-1"),
            ("  Post   1  ", "post-1"),
            ("post_1", "post-1"),
            ("Post: The Sequel!", "post-the-sequel"),
            ("C++ & Rust", "c-rust"),
            ("2024-01-02", "2024-01-02"),
            ("---", ""),
            ("", ""),
            ("Apple_Pie", "apple-pie"),
        ];
        for (name, slug) in cases {
            assert_eq!(slugify(name), slug, "{name:?}");
        }
    }

    #[test]
    fn latin_accents_fold_to_their_plain_letters() {
        let cases = [
            ("Café", "cafe"),
            ("Post Mòr", "post-mor"),
            ("Ölçü", "olcu"),
            ("Straße", "strasse"),
            ("Ærø", "aero"),
            ("Łódź", "lodz"),
            ("Þór", "thor"),
            ("Tiếng Việt", "tieng-viet"),
            ("naïve façade", "naive-facade"),
            // Typed apart, as a disk may hand a name back.
            ("Cafe\u{301}", "cafe"),
        ];
        for (name, slug) in cases {
            assert_eq!(slugify(name), slug, "{name:?}");
        }
    }

    #[test]
    fn names_in_other_scripts_are_kept() {
        let cases = [
            ("Привет мир", "привет-мир"),
            ("日記", "日記"),
            ("한국어 글", "한국어-글"),
            ("Ελληνικά ΟΔΟΣ", "ελληνικά-οδος"),
            ("हिन्दी", "हिन्दी"),
            ("پارتیشن\u{200c}بندی", "پارتیشن\u{200c}بندی"),
            ("й", "й"),
        ];
        for (name, slug) in cases {
            assert_eq!(slugify(name), slug, "{name:?}");
        }
    }

    #[test]
    fn a_name_typed_apart_reaches_the_same_slug() {
        let pairs = [
            ("и\u{306}", "й"),
            ("\u{1112}\u{1161}\u{11ab}", "한"),
            ("e\u{323}\u{302}", "ệ"),
        ];
        for (apart, whole) in pairs {
            assert_eq!(slugify(apart), slugify(whole), "{apart:?}");
            assert_eq!(nfc(apart), nfc(whole), "{apart:?}");
        }
    }

    #[test]
    fn arabic_vowels_and_the_tatweel_fold_out() {
        assert_eq!(slugify("الصَّلاة"), slugify("الصلاة"));
        assert_eq!(slugify("كتـــاب"), "كتاب");
        // A hamza typed apart from its alef is the letter, not a vowel.
        assert_eq!(slugify("\u{627}\u{654}"), "\u{623}");
    }

    #[test]
    fn a_folders_path_is_tidied_name_by_name() {
        assert_eq!(tidied_path("My Notes/Post Mòr/"), "my-notes/post-mor/");
        assert_eq!(tidied_path("posts"), "posts/");
        assert_eq!(tidied_path(""), "");
        assert_eq!(tidied_path("---/a"), "---/a/");
    }

    #[test]
    fn a_made_slug_is_eight_hex_digits_and_stable() {
        let made = made_slug("content/---.md");
        assert_eq!(made.len(), 8);
        assert!(made.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(made, made_slug("content/---.md"));
        assert_ne!(made, made_slug("content/----.md"));
    }
}
