use super::*;

/// The text a block of coloured code shows, its tags taken out.
fn shown(html: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for c in html.chars() {
        match c {
            '<' => inside = true,
            '>' if inside => inside = false,
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

/// The classes a block of coloured code carries, in order.
fn carried(html: &str) -> Vec<&str> {
    html.split("<span class=\"")
        .skip(1)
        .map(|rest| rest.split('"').next().unwrap())
        .collect()
}

/// Whether `class` is the class of a kind there is.
fn is_a_kinds(class: &str) -> bool {
    let kind = class.strip_prefix(CLASS_PREFIX);
    kind.is_some_and(|kind| Kind::ALL.iter().any(|known| known.name() == kind))
}

/// The pieces a block of coloured code marks, as ranges in the code it
/// shows.
fn pieces_of(html: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut at = 0;
    let mut open: Option<(usize, Kind)> = None;
    let mut rest = html;
    while let Some(c) = rest.chars().next() {
        if let Some(after) = rest.strip_prefix("<span class=\"") {
            let (class, after) = after.split_once("\">").unwrap();
            let kind = Kind::ALL
                .iter()
                .find(|kind| class == format!("hl-{}", kind.name()));
            open = Some((at, *kind.unwrap()));
            rest = after;
        } else if let Some(after) = rest.strip_prefix("</span>") {
            let (start, kind) = open.take().unwrap();
            pieces.push(Piece {
                range: start..at,
                kind,
            });
            rest = after;
        } else if c == '&' {
            let (_, after) = rest.split_once(';').unwrap();
            at += 1;
            rest = after;
        } else {
            at += c.len_utf8();
            rest = &rest[c.len_utf8()..];
        }
    }
    pieces
}

const RUST: &str = "fn main() {\n    let x = \"a<b & c\"; // note\n\n}";

#[test]
fn a_fence_word_names_a_grammar_by_its_name_alias_or_common_spelling() {
    for (word, name) in [
        ("rust", "rust"),
        ("Rust", "rust"),
        ("rs", "rs"),
        ("console", "console"),
        ("c++", "c++"),
        ("sh", "sh"),
        ("py", "py"),
        ("make", "make"),
        ("cmd", "cmd"),
        ("js", "js"),
        ("cl", "common-lisp"),
        ("nasm", "asm"),
        ("golang", "go"),
        ("udiff", "diff"),
        ("octave", "matlab"),
        ("VB.NET", "vb"),
    ] {
        assert_eq!(language(word), Language::Known(name), "{word}");
    }
    for word in ["text", "txt", "plain", "TEXT"] {
        assert_eq!(language(word), Language::Plain, "{word}");
    }
    for word in ["cosole", "qbasic", "glsl", "obj", "pc", "ssh", "gas"] {
        assert_eq!(language(word), Language::Unknown, "{word}");
    }
}

#[test]
fn a_word_that_names_a_language_is_never_read_as_a_file() {
    for word in names() {
        assert_eq!(
            first_word(word),
            FirstWord::Language(language(word)),
            "{word}"
        );
        let capitals = word.to_uppercase();
        assert_eq!(
            first_word(&capitals),
            FirstWord::Language(language(word)),
            "{capitals}"
        );
    }
    for word in ["Makefile", "Dockerfile", "Justfile", "CODEOWNERS"] {
        assert!(
            matches!(first_word(word), FirstWord::Language(Language::Known(_))),
            "{word}"
        );
    }
}

#[test]
fn a_files_name_gives_its_language_by_its_whole_name_or_its_ending() {
    for (word, name) in [
        ("example.rs", "rs"),
        ("Cargo.toml", "toml"),
        ("src/example.RS", "rs"),
        (".env", "dotenv"),
        ("post-1.html", "html"),
        ("example.h", "c"),
        ("site/Gemfile", "ruby"),
        ("CMakeLists.txt", "cmake"),
        ("Cargo.lock", "toml"),
        (".zshrc", "zsh"),
        ("site/Makefile", "makefile"),
        ("docker/dockerfile", "dockerfile"),
        ("templates/blogroll.opml", "xml"),
        ("feed.rss", "xml"),
        ("feed.atom", "xml"),
        ("logo.svg", "xml"),
        ("style.xslt", "xsl"),
    ] {
        assert_eq!(
            first_word(word),
            FirstWord::File(Language::Known(name)),
            "{word}"
        );
    }
    // A file whose ending names no language is a file all the same, shown
    // as it is.
    for word in [
        "notes-1.txt",
        ".gitignore",
        "example.unknown",
        "example.pyy",
        "go.mod",
        "site/nginx.conf",
    ] {
        assert_eq!(first_word(word), FirstWord::File(Language::Plain), "{word}");
    }
    // A name with no ending, and no whole name known, names no file.
    for word in [
        "gemfile",
        "1.5",
        "v1.2",
        "{.rust}",
        "example.",
        "a:b.rs",
        "Caddyfile",
        "site/Caddyfile",
    ] {
        assert!(
            first_word(word) == FirstWord::Language(Language::Unknown),
            "{word}"
        );
    }
    // A file on disk is no word in the writing: capitals make no
    // difference to its whole name, and a name that gives no language
    // is plain.
    assert_eq!(file_language("Makefile"), Language::Known("makefile"));
    assert_eq!(file_language("gemfile"), Language::Known("ruby"));
    assert_eq!(file_language("example.RS"), Language::Known("rs"));
    assert_eq!(file_language("blogroll.opml"), Language::Known("xml"));
    for name in ["example", "rust", ".gitignore", "example.unknown"] {
        assert_eq!(file_language(name), Language::Plain, "{name}");
    }
}

#[test]
fn every_name_a_fence_can_use_is_a_grammar_the_crate_carries() {
    for (file, name) in FILES {
        assert!(languages::LANGUAGES.contains(name), "{file}");
        assert_eq!(language(file), Language::Unknown, "{file}");
    }
    for name in NAMED_AS_THEIR_FILES {
        assert!(matches!(language(name), Language::Known(_)), "{name}");
    }
    for (ending, name) in ENDINGS {
        assert!(languages::LANGUAGES.contains(name), "{ending}");
        assert_eq!(language(ending), Language::Unknown, "{ending}");
    }
    let carried: Vec<&str> = CARRIED.names.iter().map(|(name, _)| *name).collect();
    assert_eq!(carried, languages::LANGUAGES);
    for (alias, name) in ALIASES {
        assert!(languages::LANGUAGES.contains(name), "{alias}");
        assert!(!languages::LANGUAGES.contains(alias), "{alias}");
    }
    for word in PLAIN {
        assert!(!languages::LANGUAGES.contains(word), "{word}");
    }
}

#[test]
fn coloured_code_shows_the_same_text_with_one_class_to_a_piece() {
    let html = html("rust", RUST).unwrap();
    assert_eq!(shown(&html), RUST);
    assert!(!html.contains("<span>"), "{html}");
    let classes = carried(&html);
    assert!(!classes.is_empty());
    for class in classes {
        assert!(is_a_kinds(class), "{class}");
    }
}

/// Sites' stylesheets, and the themes the docs give, colour these classes
/// by name: one renamed or taken away leaves their code uncoloured.
#[test]
fn the_classes_code_is_marked_with_are_the_nineteen_promised() {
    let classes: Vec<String> = Kind::ALL
        .iter()
        .map(|kind| format!("{CLASS_PREFIX}{}", kind.name()))
        .collect();
    assert_eq!(
        classes,
        [
            "hl-comment",
            "hl-keyword",
            "hl-string",
            "hl-number",
            "hl-constant",
            "hl-function",
            "hl-type",
            "hl-variable",
            "hl-property",
            "hl-tag",
            "hl-attribute",
            "hl-operator",
            "hl-punctuation",
            "hl-heading",
            "hl-emphasis",
            "hl-strong",
            "hl-link",
            "hl-inserted",
            "hl-deleted",
        ]
    );
}

#[test]
fn each_piece_carries_the_class_of_its_kind_and_text_of_none_carries_none() {
    let html = html("rust", RUST).unwrap();
    // The spaces, the indentation and the blank line are of no kind.
    assert_eq!(
        &*html,
        "<span class=\"hl-keyword\">fn</span> <span class=\"hl-function\">main</span>\
         <span class=\"hl-punctuation\">()</span> <span class=\"hl-punctuation\">{</span>\n    \
         <span class=\"hl-keyword\">let</span> <span class=\"hl-variable\">x</span> \
         <span class=\"hl-operator\">=</span> <span class=\"hl-string\">&quot;a&lt;b &amp; c&quot;</span>\
         <span class=\"hl-punctuation\">;</span> <span class=\"hl-comment\">// note</span>\n\n\
         <span class=\"hl-punctuation\">}</span>"
    );
}

#[test]
fn a_piece_after_a_character_of_two_units_starts_where_it_should() {
    let code = "let s = \"\u{e9}\u{1d11e}\"; // \u{1d11e} note\nlet t = 1;";
    let html = html("rust", code).unwrap();
    assert_eq!(shown(&html), code);
    assert!(
        html.contains("<span class=\"hl-string\">&quot;\u{e9}\u{1d11e}&quot;</span>"),
        "{html}"
    );
    assert!(
        html.contains("<span class=\"hl-comment\">// \u{1d11e} note</span>\n"),
        "{html}"
    );
}

#[test]
fn a_grammar_colours_the_languages_it_includes() {
    let code = "<style>a { color: red; }</style>\n<script>let x = 1;</script>";
    let html = html("html", code).unwrap();
    assert!(
        html.contains("<span class=\"hl-property\">color</span>"),
        "{html}"
    );
    assert!(
        html.contains("<span class=\"hl-keyword\">let</span>"),
        "{html}"
    );
}

#[test]
fn code_after_a_haxe_string_in_single_quotes_is_coloured() {
    let code = "trace('a $b');";
    let html = html("haxe", code).unwrap();
    assert!(
        html.contains("<span class=\"hl-variable\">b</span>"),
        "{html}"
    );
    assert!(
        html.ends_with("<span class=\"hl-punctuation\">);</span>"),
        "{html}"
    );
}

#[test]
fn code_of_no_grammar_carried_is_not_coloured() {
    assert_eq!(html("no such language", "a < b"), None);
    assert_eq!(pieces("no such language", "a < b"), None);
    assert_eq!(html("Rust", "a < b"), None, "a name is in lower case");
}

#[test]
fn every_scope_is_of_a_kind_there_is_and_every_kind_has_a_scope() {
    for (scope, kind) in SCOPES {
        assert!(
            kind.is_none_or(|kind| Kind::ALL.iter().any(|known| known.name() == kind)),
            "{scope}"
        );
    }
    for kind in Kind::ALL {
        let name = kind.name();
        assert!(SCOPES.iter().any(|(_, of)| *of == Some(name)), "{name}");
    }
}

#[test]
fn a_line_is_too_long_past_the_longest_counted_in_characters() {
    let longest = "é".repeat(LONGEST_LINE);
    assert_eq!(too_long(&format!("a\n{longest}\r\nb")), None);
    assert_eq!(
        too_long(&format!("a\n{longest}é\nb")),
        Some(LONGEST_LINE + 1)
    );
}

#[test]
fn every_language_carried_colours_code() {
    let sample = "#include <stdio.h>\n/* c */ int main(void) { return 0; }\n\
                  # comment\nlet x = 'y'; def f(a): pass\n<a href=\"b\">c</a>\n\
                  $ echo \"$HOME\" | grep -v x > /dev/null\n- [ ] **bold** _it_ `code`\n";
    for name in languages::LANGUAGES {
        assert!(tokeniser(name).is_some(), "{name}");
        let html = html(name, sample).unwrap();
        assert_eq!(shown(&html), sample, "{name}");
        for class in carried(&html) {
            assert!(is_a_kinds(class), "{name}");
        }
        assert_eq!(pieces(name, sample).unwrap(), pieces_of(&html), "{name}");
    }
}

#[test]
fn each_piece_is_a_range_of_the_code_with_its_kind() {
    let pieces = pieces("rust", RUST).unwrap();
    let read: Vec<(&str, Kind)> = pieces
        .iter()
        .map(|piece| (&RUST[piece.range.clone()], piece.kind))
        .collect();
    assert_eq!(
        read,
        [
            ("fn", Kind::Keyword),
            ("main", Kind::Function),
            ("()", Kind::Punctuation),
            ("{", Kind::Punctuation),
            ("let", Kind::Keyword),
            ("x", Kind::Variable),
            ("=", Kind::Operator),
            ("\"a<b & c\"", Kind::String),
            (";", Kind::Punctuation),
            ("// note", Kind::Comment),
            ("}", Kind::Punctuation),
        ]
    );
}

#[test]
fn a_piece_of_a_kind_carried_over_lines_ends_at_each_line() {
    let code = "/* a\r\nb */ x";
    let pieces = pieces("c", code).unwrap();
    let comments: Vec<&str> = pieces
        .iter()
        .filter(|piece| piece.kind == Kind::Comment)
        .map(|piece| &code[piece.range.clone()])
        .collect();
    assert_eq!(comments, ["/* a\r", "b */"]);
    assert_eq!(pieces_of(&html("c", code).unwrap()), pieces);
}

#[test]
fn pieces_start_where_they_should_after_characters_of_two_units() {
    let code = "let s = \"\u{e9}\u{1d11e}\"; // \u{1d11e} note\nlet t = 1;";
    let pieces = pieces("rust", code).unwrap();
    assert!(
        pieces
            .iter()
            .any(|piece| &code[piece.range.clone()] == "// \u{1d11e} note"
                && piece.kind == Kind::Comment),
        "{pieces:?}"
    );
    assert_eq!(pieces_of(&html("rust", code).unwrap()), pieces);
}
