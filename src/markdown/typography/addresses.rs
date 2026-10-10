//! Addresses written bare among the words, found as GitHub Flavored
//! Markdown finds them: `www.` and a domain, `http://`, `https://` or
//! `ftp://` and a host, and a mail address, with or without `mailto:` or
//! `xmpp:` before it. An address reaches the page as the author typed it, so typography
//! leaves each one alone, and the writing links each one.
//!
//! Where one may start, and what is left off its end, follow the format's
//! reference reader, cmark-gfm (`extensions/autolink.c`), where its spec's
//! prose says less: a scheme follows anything but a letter, so
//! `"https://example.org"` is an address, and a mail address follows
//! anything at all.
//!
//! One end is Statac's own: a character that shows nothing, or a closing
//! quote or bracket with no opening one inside the address, is left to the
//! sentence, where cmark-gfm would carry it into the link, which then
//! points nowhere.

use std::ops::Range;

/// An address written bare.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) struct Address {
    /// Where it is written, within the words searched.
    pub at: Range<usize>,
    /// What a link to it writes before it: `http://` for a `www.` address,
    /// `mailto:` for a mail address written without a scheme.
    pub scheme: &'static str,
    /// Whether it is a mail address, which an open `[` doesn't stop being
    /// linked.
    pub mail: bool,
}

/// Each address written bare in `words`, in the order written. `before`
/// is the character the words follow: a space where they open a line.
pub(super) fn found(words: &str, before: char) -> Vec<Address> {
    let mut found = Vec::new();
    let mut previous = before;
    let mut read = Read::default();
    let mut at = 0;
    while let Some(character) = words[at..].chars().next() {
        if let Some((length, scheme, mail)) = address_at(words, at, previous, &mut read) {
            let end = at + length;
            previous = words[..end].chars().next_back().unwrap_or(previous);
            found.push(Address {
                at: at..end,
                scheme,
                mail,
            });
            at = end;
            continue;
        }
        previous = character;
        at += character.len_utf8();
    }
    found
}

/// How far a `www.` address and a mail address with no scheme were read
/// before they turned out to be none. One starting inside what was read
/// fails for the same reason, so it is not read again: without this, words
/// that keep almost being an address are read over and over.
#[derive(Default)]
struct Read {
    www: usize,
    mailbox: usize,
}

/// How long the address starting at `at` in `words`, just after
/// `previous`, is, if one does; the scheme a link to it writes before it;
/// and whether it is a mail address.
fn address_at(
    words: &str,
    at: usize,
    previous: char,
    read: &mut Read,
) -> Option<(usize, &'static str, bool)> {
    let rest = &words[at..];
    // `www.` starts a line, follows a space, or follows what emphasis or a
    // bracket opens with.
    if at >= read.www && (previous.is_ascii_whitespace() || "*_~(".contains(previous)) {
        match www(rest) {
            Ok(length) => return Some((length, "http://", false)),
            Err(failed) => read.www = at + failed,
        }
    }
    if !previous.is_ascii_alphabetic()
        && let Some(length) = url(rest)
    {
        return Some((length, "", false));
    }
    let scheme = ["mailto:", "xmpp:"]
        .into_iter()
        .find(|scheme| rest.starts_with(scheme) && !previous.is_ascii_alphanumeric());
    if scheme.is_none() && at < read.mailbox {
        return None;
    }
    match mailbox(rest, scheme) {
        Ok(length) => Some((length, if scheme.is_some() { "" } else { "mailto:" }, true)),
        Err(failed) => {
            if scheme.is_none() {
                read.mailbox = at + failed;
            }
            None
        }
    }
}

/// How long the `www.` address `rest` opens with is, or how far it was
/// read before it wasn't one.
fn www(rest: &str) -> Result<usize, usize> {
    let after = rest.strip_prefix("www.").ok_or(0usize)?;
    match domain(after) {
        Ok(domain) => Ok(path_end(rest, "www.".len() + domain)),
        Err(failed) => Err("www.".len() + failed),
    }
}

/// How long the `http://`, `https://` or `ftp://` address `rest` opens
/// with is. Its host needs no full stop, so `http://localhost:8080` is one.
fn url(rest: &str) -> Option<usize> {
    let scheme = ["http://", "https://", "ftp://"]
        .into_iter()
        .find(|scheme| {
            rest.get(..scheme.len())
                .is_some_and(|written| written.eq_ignore_ascii_case(scheme))
        })?
        .len();
    let host = &rest[scheme..];
    if !host.starts_with(char::is_alphanumeric) {
        return None;
    }
    let (end, names) = names(host, |c| c.is_alphanumeric() || "_-".contains(c));
    let host_names = &names[names.len().saturating_sub(2)..];
    if host_names
        .iter()
        .any(|name| host[name.clone()].contains('_'))
    {
        return None;
    }
    Some(path_end(rest, scheme + end))
}

/// How long the domain `text` opens with is: two or more names of letters,
/// digits, underscores and hyphens with a full stop between each, and no
/// underscore in the last two, which name the host. Where there is none,
/// how far it was read.
fn domain(text: &str) -> Result<usize, usize> {
    let (end, names) = names(text, |c| c.is_alphanumeric() || "_-".contains(c));
    match names.len().checked_sub(2).map(|from| &names[from..]) {
        Some(last_two)
            if last_two
                .iter()
                .all(|name| !text[name.clone()].contains('_')) =>
        {
            Ok(end)
        }
        _ => Err(end),
    }
}

/// The names `text` opens with, each a run of characters `in_a_name`
/// takes with a full stop between each, and where the last one ends. A
/// full stop with no name after it is left out.
fn names(text: &str, in_a_name: impl Fn(char) -> bool) -> (usize, Vec<Range<usize>>) {
    let mut names = Vec::new();
    let mut end = 0;
    loop {
        let length: usize = text[end..]
            .chars()
            .take_while(|c| in_a_name(*c))
            .map(char::len_utf8)
            .sum();
        if length == 0 {
            break;
        }
        names.push(end..end + length);
        end += length;
        match text[end..].strip_prefix('.') {
            Some(after) if after.starts_with(&in_a_name) => end += 1,
            _ => break,
        }
    }
    (end, names)
}

/// Where an address whose domain ends at `from` ends: at the first space or
/// `<`, less what GFM leaves off the end of one.
fn path_end(rest: &str, from: usize) -> usize {
    let end = rest[from..]
        .find(|c: char| c.is_whitespace() || c == '<')
        .map_or(rest.len(), |at| from + at);
    kept(&rest[..end])
}

/// How much of an address is kept once its end is read as the sentence's
/// rather than the address's: a mark of punctuation or a straight quote, a
/// closing bracket or typographic quote no opening one inside it matches, a
/// semicolon, with the entity it seems to end, such as `&hl;`, where it
/// ends one, and a character that shows nothing.
fn kept(address: &str) -> usize {
    let count = |wanted: &[char]| address.chars().filter(|c| wanted.contains(c)).count();
    let mut unmatched: Vec<(char, usize)> = CLOSING
        .iter()
        .map(|&(closing, opening)| {
            let unmatched = count(&[closing]).saturating_sub(count(opening));
            (closing, unmatched)
        })
        .collect();
    let mut end = address.len();
    while let Some(last) = address[..end].chars().next_back() {
        let before = end - last.len_utf8();
        match last {
            '?' | '!' | '.' | ',' | ':' | '*' | '_' | '~' | '\'' | '"' => end = before,
            ';' => {
                let name = address[..before]
                    .bytes()
                    .rev()
                    .take_while(u8::is_ascii_alphabetic)
                    .count();
                match before.checked_sub(name + 1) {
                    Some(at) if name > 0 && address.as_bytes()[at] == b'&' => end = at,
                    _ => end = before,
                }
            }
            c if is_format(c) => end = before,
            c => match unmatched.iter_mut().find(|(closing, _)| *closing == c) {
                Some((_, unmatched)) if *unmatched > 0 => {
                    *unmatched -= 1;
                    end = before;
                }
                _ => break,
            },
        }
    }
    end
}

/// Each closing bracket or quote an address may end in, and the characters
/// that open it: in some languages the same quote closes one pair and
/// opens another.
const CLOSING: [(char, &[char]); 19] = [
    (')', &['(']),
    ('\u{201d}', &['\u{201c}', '\u{201e}']),
    ('\u{201c}', &['\u{201e}']),
    ('\u{2019}', &['\u{2018}', '\u{201a}']),
    ('\u{2018}', &['\u{201a}']),
    ('\u{bb}', &['\u{ab}']),
    ('\u{ab}', &['\u{bb}']),
    ('\u{203a}', &['\u{2039}']),
    ('\u{2039}', &['\u{203a}']),
    ('\u{300d}', &['\u{300c}']),
    ('\u{300f}', &['\u{300e}']),
    ('\u{3011}', &['\u{3010}']),
    ('\u{3009}', &['\u{3008}']),
    ('\u{300b}', &['\u{300a}']),
    ('\u{3015}', &['\u{3014}']),
    ('\u{ff09}', &['\u{ff08}']),
    ('\u{ff3d}', &['\u{ff3b}']),
    ('\u{ff5d}', &['\u{ff5b}']),
    ('\u{ff63}', &['\u{ff62}']),
];

/// Whether `c` is a format character, Unicode's general category Cf:
/// direction marks and embeddings, zero-width spaces and joiners, and the
/// like, which show nothing.
fn is_format(c: char) -> bool {
    matches!(
        c as u32,
        0x00AD
            | 0x0600..=0x0605
            | 0x061C
            | 0x06DD
            | 0x070F
            | 0x0890..=0x0891
            | 0x08E2
            | 0x180E
            | 0x200B..=0x200F
            | 0x202A..=0x202E
            | 0x2060..=0x2064
            | 0x2066..=0x206F
            | 0xFEFF
            | 0xFFF9..=0xFFFB
            | 0x110BD
            | 0x110CD
            | 0x13430..=0x1343F
            | 0x1BCA0..=0x1BCA3
            | 0x1D173..=0x1D17A
            | 0xE0001
            | 0xE0020..=0xE007F
    )
}

/// How long the mail address `rest` opens with is, written after
/// `scheme` where it has one, or how far it was read before it wasn't one.
/// Its domain is letters, figures, `-` and `_`, with a full stop before a
/// letter or a figure at least once, and ends in a letter. An `xmpp:`
/// address can name a resource after a `/`.
fn mailbox(rest: &str, scheme: Option<&str>) -> Result<usize, usize> {
    let scheme_length = scheme.map_or(0, str::len);
    let local = rest[scheme_length..]
        .bytes()
        .take_while(|byte| byte.is_ascii_alphanumeric() || b".-_+".contains(byte))
        .count();
    let at = scheme_length + local;
    if local == 0 || rest.as_bytes().get(at) != Some(&b'@') {
        return Err(at);
    }
    let bytes = rest.as_bytes();
    let mut end = at + 1;
    let mut stops = 0;
    while let Some(&byte) = bytes.get(end) {
        if byte == b'.' && bytes.get(end + 1).is_some_and(u8::is_ascii_alphanumeric) {
            stops += 1;
        } else if !(byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_') {
            break;
        }
        end += 1;
    }
    if stops == 0 || !bytes[end - 1].is_ascii_alphabetic() {
        return Err(at);
    }
    if scheme == Some("xmpp:")
        && let Some(after) = rest[end..].strip_prefix('/')
    {
        let resource = after
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || b"@.".contains(byte))
            .count();
        let resource = after[..resource].trim_end_matches('.').len();
        if resource > 0 {
            end += 1 + resource;
        }
    }
    Ok(end)
}

#[cfg(test)]
mod tests {
    use super::found;

    fn addresses(words: &str) -> Vec<&str> {
        found(words, ' ')
            .into_iter()
            .map(|address| &words[address.at])
            .collect()
    }

    #[test]
    fn www_starts_a_line_follows_a_space_or_what_emphasis_or_a_bracket_opens() {
        assert_eq!(addresses("www.a.org"), ["www.a.org"]);
        assert_eq!(addresses("see www.a.org"), ["www.a.org"]);
        assert_eq!(addresses("see\twww.a.org"), ["www.a.org"]);
        assert_eq!(addresses("(www.a.org)"), ["www.a.org"]);
        assert_eq!(addresses("~www.a.org"), ["www.a.org"]);
        assert_eq!(found("www.a.org", '*').len(), 1);
        assert_eq!(found("www.a.org", '_').len(), 1);
        for before in ['x', '1', '"', '>', '/', '\u{a0}', '\u{3000}'] {
            assert!(found("www.a.org", before).is_empty(), "{before:?}");
        }
        assert!(addresses("xwww.a.org").is_empty());
    }

    #[test]
    fn a_scheme_follows_anything_but_a_letter() {
        for written in [
            "\"https://a.org\"",
            "1https://a.org",
            "\u{a0}https://a.org",
            "éhttps://a.org",
        ] {
            assert_eq!(addresses(written), ["https://a.org"], "{written}");
        }
        assert!(addresses("xhttps://a.org").is_empty());
        assert!(found("https://a.org", 'X').is_empty());
        assert_eq!(found("https://a.org", '_').len(), 1);
    }

    /// A mail address is as much of the letters, figures and `.+-_` before
    /// its `@` as there is, whatever comes before them; `mailto:` and
    /// `xmpp:` belong to it only after something other than a letter or a
    /// figure.
    #[test]
    fn a_mail_address_follows_anything() {
        assert_eq!(addresses("/a@b.c"), ["a@b.c"]);
        assert_eq!(addresses("x\"a.b@c.d"), ["a.b@c.d"]);
        assert_eq!(addresses("foo:bar@c.d"), ["bar@c.d"]);
        assert_eq!(addresses("(mailto:a@b.c)"), ["mailto:a@b.c"]);
        let found = found("xmailto:a@b.c", ' ');
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].at.clone(), found[0].scheme), (8..13, "mailto:"));
        assert_eq!(addresses("1xmpp:a@b.c"), ["a@b.c"]);
    }

    /// GFM asks no more of a domain than this, so `www.` and a name with
    /// no full stop in it is no address.
    #[test]
    fn a_domain_is_two_names_or_more_with_no_underscore_in_the_last_two() {
        assert_eq!(addresses("www._a.b.org"), ["www._a.b.org"]);
        assert!(addresses("www.a_b.org").is_empty());
        assert!(addresses("www.a.b_c").is_empty());
        assert!(addresses("www.org").is_empty());
        assert_eq!(addresses("HTTPS://a.org"), ["HTTPS://a.org"]);
        assert_eq!(addresses("https://bücher.de/x"), ["https://bücher.de/x"]);
    }

    /// A scheme's host needs no full stop, as cmark-gfm has it, but starts
    /// with a letter or a figure and has no underscore in its last two
    /// names.
    #[test]
    fn a_scheme_and_a_host_are_an_address() {
        assert_eq!(
            addresses("http://localhost:4000/"),
            ["http://localhost:4000/"]
        );
        assert_eq!(addresses("ftp://a.org/f.txt."), ["ftp://a.org/f.txt"]);
        assert_eq!(addresses("FTP://a"), ["FTP://a"]);
        assert!(addresses("gopher://a.org").is_empty());
        assert!(addresses("http://-a.org").is_empty());
        assert!(addresses("http://_a.org").is_empty());
        assert!(addresses("http://a_b").is_empty());
        assert!(addresses("http:// a.org").is_empty());
        assert_eq!(addresses("https://_a.b.org"), Vec::<&str>::new());
        assert_eq!(addresses("https://a_b.c.org"), ["https://a_b.c.org"]);
    }

    /// Statac's own end: a character that shows nothing, and a closing
    /// quote or bracket no opening one inside the address matches.
    #[test]
    fn what_shows_nothing_and_an_unmatched_closing_quote_are_left_off() {
        let cases = [
            ("\u{201c}http://a.org\u{201d}).", "http://a.org"),
            ("\u{202a}https://a.org/4f\u{202c}", "https://a.org/4f"),
            ("https://a.org/\u{200e}", "https://a.org/"),
            ("https://a.org/x\u{200d}\u{2060}.", "https://a.org/x"),
            (
                "https://a.org/\u{201c}x\u{201d}",
                "https://a.org/\u{201c}x\u{201d}",
            ),
            ("\u{ab}https://a.org/x\u{bb}", "https://a.org/x"),
            ("\u{bb}https://a.org/x\u{ab}", "https://a.org/x"),
            ("\u{201e}https://a.org/x\u{201c}", "https://a.org/x"),
            ("https://a.org/\u{ab}x\u{bb}", "https://a.org/\u{ab}x\u{bb}"),
            ("https://a.org/it\u{2019}s", "https://a.org/it\u{2019}s"),
            ("\u{300c}https://a.org/\u{300d}", "https://a.org/"),
            (
                "https://a.org/x\u{201d}\u{202c}.\u{201d}",
                "https://a.org/x",
            ),
        ];
        for (written, wanted) in cases {
            assert_eq!(addresses(written), [wanted], "{written:?}");
        }
    }

    #[test]
    fn the_sentences_punctuation_is_left_off_the_end() {
        assert_eq!(addresses("www.a.org/x?!.,:*_~ rest"), ["www.a.org/x"]);
        assert_eq!(addresses("www.a.org/x'\"\".' rest"), ["www.a.org/x"]);
        assert_eq!(addresses("www.a.org/x'y"), ["www.a.org/x'y"]);
        assert_eq!(addresses("www.a.org/x;"), ["www.a.org/x"]);
        assert_eq!(addresses("www.a.org/x;;."), ["www.a.org/x"]);
        assert_eq!(addresses("www.a.org/x&ab;"), ["www.a.org/x"]);
        assert_eq!(addresses("www.a.org/x&a1;"), ["www.a.org/x&a1"]);
        assert_eq!(addresses("www.a.org/x&;"), ["www.a.org/x&"]);
        assert_eq!(addresses("(www.a.org/(x)))."), ["www.a.org/(x)"]);
        assert_eq!(addresses("https://a.org/a--b..."), ["https://a.org/a--b"]);
    }

    #[test]
    fn a_mail_address_ends_in_a_domain() {
        assert_eq!(addresses("a+b@c.d"), ["a+b@c.d"]);
        assert_eq!(addresses("a@-b_.c-d"), ["a@-b_.c-d"]);
        assert!(addresses("a@b.c1").is_empty());
        assert!(addresses("/a@4.0.1/").is_empty());
        assert!(addresses("a@b.-c").is_empty());
        assert_eq!(addresses("a@b.c:d"), ["a@b.c"]);
        assert!(addresses("a@c+d.e").is_empty());
        assert!(addresses("a@c..d").is_empty());
        assert!(addresses("a@cd").is_empty());
        assert_eq!(addresses("xmpp:a@b.c/d.e/f"), ["xmpp:a@b.c/d.e"]);
        assert_eq!(addresses("xmpp:a@b.c/."), ["xmpp:a@b.c"]);
        assert_eq!(addresses("mailto:a@b.c/d"), ["mailto:a@b.c"]);
    }
}
