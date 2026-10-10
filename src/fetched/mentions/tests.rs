use super::*;

fn mention(words: &str) -> Mention {
    Mention {
        id: Some(2028899),
        kind: Said::Replied,
        author: Some("A".into()),
        author_url: Some("https://a.example/".into()),
        face: Some("0123456789abcdef.jpg".into()),
        url: "https://a.example/post-1/".into(),
        target: "https://example.org/post-1/".into(),
        published: Some(Date::day_only(2026, 9, 3)),
        fetched: Date::day_only(2026, 10, 6),
        words: Some(words.into()),
    }
}

/// The file `mention` is written as, read back.
fn read_back(mention: &Mention) -> (Option<Mention>, Vec<Problem>) {
    let mut sources = Sources::default();
    let source = sources.add("fetched/mentions/2028899.yaml", mention.text());
    let mut problems = Vec::new();
    let read = super::mention(
        &sources,
        source,
        mention.id,
        &["0123456789abcdef.jpg"],
        &mut problems,
    );
    (read.map(|(mention, _)| mention), problems)
}

#[test]
fn a_mention_is_written_as_quoted_text_field_by_field() {
    assert_eq!(
        mention("Words.").text(),
        "kind: \"replied\"\nauthor: \"A\"\nauthor_url: \"https://a.example/\"\n\
         face: \"0123456789abcdef.jpg\"\nurl: \"https://a.example/post-1/\"\n\
         target: \"https://example.org/post-1/\"\npublished: \"2026-09-03\"\n\
         fetched: \"2026-10-06\"\nwords: \"Words.\"\n"
    );
    let bare = Mention {
        author: None,
        author_url: None,
        face: None,
        published: None,
        words: None,
        ..mention("")
    };
    assert_eq!(
        bare.text(),
        "kind: \"replied\"\nurl: \"https://a.example/post-1/\"\n\
         target: \"https://example.org/post-1/\"\nfetched: \"2026-10-06\"\n"
    );
}

#[test]
fn whatever_a_stranger_sent_reads_back_exactly() {
    for words in [
        "\"Quoted\" and 'quoted'",
        "a \\ backslash, \\n written out, and \\\" both",
        "key: value, and a: colon",
        "# not a comment",
        "- not a list",
        "= =today @youtube(\"abcdefghijk\") {{ site.title }} {% if x %}",
        "&anchor *alias !tag | > ? [ ] { } , % `",
        "Grüße, привет, 你好, مرحبا, 🙂",
        "\u{FEFF}a mark \u{FFFE} and \u{2028} separators \u{2029}",
        "   ",
        "",
    ] {
        let sent = mention(words);
        let (read, problems) = read_back(&sent);
        assert_eq!(problems, [], "{words:?}");
        assert_eq!(read, Some(sent), "{words:?}");
    }
}

#[test]
fn a_strangers_text_loses_its_controls_and_its_line_breaks_in_words() {
    assert_eq!(
        clean("  A\u{0}\u{7}\u{1b}[31m\u{85}\u{9f}B\t ", false),
        "A[31mB"
    );
    assert_eq!(clean("a\u{202E}b\u{2066}c\u{200F}d\u{061C}", false), "abcd");
    assert_eq!(clean("one\ntwo\r\nthree\rfour", false), "onetwothreefour");
    assert_eq!(
        clean("one\ntwo\r\nthree\rfour\u{2028}five", true),
        "one two three four five"
    );
    assert_eq!(words(" \n "), None);
    assert_eq!(words("Words.\n").as_deref(), Some("Words."));
}

#[test]
fn words_are_cut_at_a_character_with_an_ellipsis() {
    let long = "é".repeat(LONGEST_WORDS);
    assert_eq!(words(&long), Some(long.clone()));
    let longer = format!("{long}a");
    let cut = words(&longer).unwrap();
    assert_eq!(cut.chars().count(), LONGEST_WORDS);
    assert!(cut.ends_with("é…"), "{cut}");
    let spaced = format!("{} {}", "a".repeat(LONGEST_WORDS - 2), "b".repeat(10));
    assert_eq!(
        words(&spaced).unwrap(),
        format!("{}…", "a".repeat(LONGEST_WORDS - 2))
    );
}

#[test]
fn only_http_and_https_are_web_addresses() {
    for address in ["https://a.example/", "http://a.example/post-1/?q=1#a"] {
        assert!(web_address(address), "{address}");
    }
    for address in [
        "javascript:alert(1)",
        "JAVASCRIPT:alert(1)",
        "data:text/html,<script>",
        "ftp://a.example/",
        "//a.example/",
        "/post-1/",
        "https://",
        "",
    ] {
        assert!(!web_address(address), "{address}");
    }
}

#[test]
fn a_target_is_on_the_site_at_a_path_boundary() {
    let site = "https://example.org/";
    for (target, path) in [
        ("https://example.org/post-1/", "/post-1/"),
        ("http://example.org/post-1/", "/post-1/"),
        ("https://www.example.org/post-1/", "/post-1/"),
        ("https://EXAMPLE.org/post-1", "/post-1"),
        ("https://example.org", "/"),
        ("https://example.org/post-1/?ref=a#b", "/post-1/"),
    ] {
        assert_eq!(on_site(target, site).as_deref(), Some(path), "{target}");
    }
    let site = "https://www.example.org/blog/";
    assert_eq!(
        on_site("https://example.org/blog/post-1/", site).as_deref(),
        Some("/post-1/")
    );
    assert_eq!(
        on_site("https://example.org/blog", site).as_deref(),
        Some("/")
    );
    for target in [
        "https://example.org/blogging/post-1/",
        "https://example.org/post-1/",
        "https://example.com/blog/post-1/",
        "https://example.org.a.example/blog/",
        "https://example.org:8080/blog/post-1/",
        "javascript:alert(1)",
        "",
    ] {
        assert_eq!(on_site(target, site), None, "{target}");
    }
}

#[test]
fn a_relays_id_is_a_whole_number_above_nought_in_digits() {
    assert_eq!(whole_id("2028899"), Some(2028899));
    for id in [
        "0",
        "-1",
        "1e3",
        "1.0",
        "+1",
        "../x",
        "",
        "99999999999999999999999",
    ] {
        assert_eq!(whole_id(id), None, "{id}");
    }
    assert_eq!(id_of("mentions/2028899.yaml"), Some(2028899));
    assert_eq!(id_of("mentions/notes.yaml"), None);
    assert_eq!(id_of("stills/2028899.yaml"), None);
}

#[test]
fn a_face_is_named_by_the_hash_of_its_address() {
    let name = face_name("https://a.example/a.jpg");
    assert_eq!(name.len(), 20);
    assert!(a_face_name(&name), "{name}");
    assert_eq!(name, face_name("https://a.example/a.jpg"));
    assert_ne!(name, face_name("https://a.example/b.jpg"));
    for name in [
        "0123456789ABCDEF.jpg",
        "0123456789abcde.jpg",
        "../0123456789abcd.jpg",
    ] {
        assert!(!a_face_name(name), "{name}");
    }
}
