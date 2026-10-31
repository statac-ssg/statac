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
    assert_eq!(
        language("cosole"),
        Language::Unknown {
            nearest: Some("console")
        }
    );
    for word in ["qbasic", "glsl", "obj", "pc", "ssh", "gas"] {
        assert_eq!(
            language(word),
            Language::Unknown { nearest: None },
            "{word}"
        );
    }
}

#[test]
fn a_word_that_names_a_language_is_never_read_as_a_file() {
    let words = PLAIN
        .iter()
        .chain(languages::LANGUAGES)
        .chain(ALIASES.iter().map(|(alias, _)| alias));
    for word in words {
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
            matches!(
                first_word(word),
                FirstWord::Language(Language::Unknown { .. })
            ),
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
fn every_name_a_fence_can_use_is_a_grammar_statac_carries() {
    for (file, name) in FILES {
        assert!(languages::LANGUAGES.contains(name), "{file}");
        assert!(matches!(language(file), Language::Unknown { .. }), "{file}");
    }
    for name in NAMED_AS_THEIR_FILES {
        assert!(matches!(language(name), Language::Known(_)), "{name}");
    }
    for (ending, name) in ENDINGS {
        assert!(languages::LANGUAGES.contains(name), "{ending}");
        assert!(matches!(language(ending), Language::Unknown { .. }));
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
    let html = Highlighter::default().html("rust", RUST);
    assert_eq!(shown(&html), RUST);
    assert!(!html.contains("<span>"), "{html}");
    let classes = carried(&html);
    assert!(!classes.is_empty());
    for class in classes {
        let kind = class.strip_prefix(CLASS_PREFIX);
        assert!(kind.is_some_and(|kind| KINDS.contains(&kind)), "{class}");
    }
}

/// Sites' stylesheets, and the themes the docs give, colour these classes
/// by name: one renamed or taken away leaves their code uncoloured.
#[test]
fn the_classes_code_is_marked_with_are_the_nineteen_promised() {
    let classes = KINDS.map(|kind| format!("{CLASS_PREFIX}{kind}"));
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
    let html = Highlighter::default().html("rust", RUST);
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
    let html = Highlighter::default().html("rust", code);
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
    let html = Highlighter::default().html("html", code);
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
    let html = Highlighter::default().html("haxe", code);
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
fn code_of_no_grammar_carried_is_shown_as_it_is() {
    assert_eq!(coloured("no such language", "a < b"), "a &lt; b");
}

#[test]
fn every_scope_is_of_a_kind_there_is_and_every_kind_has_a_scope() {
    for (scope, kind) in SCOPES {
        assert!(kind.is_none_or(|kind| KINDS.contains(&kind)), "{scope}");
    }
    for kind in KINDS {
        assert!(SCOPES.iter().any(|(_, of)| *of == Some(kind)), "{kind}");
    }
}

#[test]
fn a_block_written_twice_is_coloured_once() {
    let highlighter = Highlighter::default();
    let first = highlighter.html("rust", RUST);
    let second = highlighter.html("rust", RUST);
    assert!(Arc::ptr_eq(&first, &second));
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
        let html = coloured(name, sample);
        assert_eq!(shown(&html), sample, "{name}");
        for class in carried(&html) {
            let kind = class.strip_prefix(CLASS_PREFIX);
            assert!(kind.is_some_and(|kind| KINDS.contains(&kind)), "{name}");
        }
    }
}
