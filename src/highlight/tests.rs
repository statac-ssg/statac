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
    ] {
        assert_eq!(
            first_word(word),
            FirstWord::File(Language::Known(name)),
            "{word}"
        );
    }
    assert_eq!(first_word("notes-1.txt"), FirstWord::File(Language::Plain));
    // An ending that names no language names no file.
    for word in [
        ".gitignore",
        "example.unknown",
        "vb.net",
        "example.pyy",
        "gemfile",
        "1.5",
        "v1.2",
        "{.rust}",
        "example.",
        "a:b.rs",
        "Caddyfile",
        "site/Makefile",
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
    for &style in NAMES {
        let html = Highlighter::new(style).unwrap().html("rust", RUST);
        assert_eq!(shown(&html), RUST, "{style}");
        assert!(!html.contains("<span>"), "{style}: {html}");
        let css = stylesheet(style).unwrap();
        let classes = carried(&html);
        assert!(!classes.is_empty(), "{style}");
        for class in classes {
            assert!(!class.contains(' '), "{style}: {class}");
            assert!(css.contains(&format!(".{class} {{")), "{style}: {class}");
        }
    }
}

#[test]
fn each_piece_carries_the_class_of_its_kind_and_text_of_none_carries_none() {
    let html = Highlighter::new("ocean").unwrap().html("rust", RUST);
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
fn code_is_marked_the_same_in_every_style() {
    let html = |style| Highlighter::new(style).unwrap().html("rust", RUST);
    for &style in NAMES {
        assert_eq!(html(style), html(NAMES[0]), "{style}");
    }
}

#[test]
fn a_piece_after_a_character_of_two_units_starts_where_it_should() {
    let code = "let s = \"\u{e9}\u{1d11e}\"; // \u{1d11e} note\nlet t = 1;";
    let html = Highlighter::new("ocean").unwrap().html("rust", code);
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
    let html = Highlighter::new("ocean").unwrap().html("html", code);
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
    let highlighter = Highlighter::new("gruvbox").unwrap();
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
fn each_style_is_written_as_a_rule_to_a_kind_whose_colour_a_property_can_set() {
    let styles: Vec<&str> = styles::STYLES.iter().map(|style| style.name).collect();
    assert_eq!(styles, NAMES);
    assert_eq!(SETTING, [NAMES, &[OFF]].concat());
    assert!(Highlighter::new(OFF).is_none());
    assert_eq!(stylesheet(OFF), None);
    for &name in NAMES {
        let css = stylesheet(name).unwrap_or_else(|| panic!("{name}"));
        assert!(css.contains(&format!("Code colours: {name},")), "{name}");
        let rules: Vec<&str> = css.lines().filter(|line| line.starts_with('.')).collect();
        assert_eq!(rules.len(), KINDS.len() + 1, "{name}");
        for (kind, rule) in KINDS.iter().zip(&rules[1..]) {
            let opens = format!(".hl-{kind} {{ color: var(--code-{kind}, ");
            assert!(rule.starts_with(&opens), "{name}: {rule}");
        }
    }
    assert!(stylesheet("ocean").unwrap().contains(
        ".hl-comment { color: var(--code-comment, light-dark(#6a6d76, #8e98a0)); }\n\
         .hl-keyword { color: var(--code-keyword, light-dark(#80657b, #b48ead)); }\n"
    ));
    assert!(stylesheet("gruvbox").unwrap().contains(
        ".hl-comment { color: var(--code-comment, light-dark(#786c60, #9a8c7f)); \
         font-style: italic; }\n"
    ));
    assert_eq!(stylesheet("dracula"), None);
}

#[test]
fn code_takes_the_look_of_the_nearest_data_theme_above_it() {
    // An element takes a custom property from the nearest element above
    // that sets it, so each rule reaches the code below it until another
    // `data-theme` stands between: a light part of a dark page is light,
    // a dark part of a light page dark. With none anywhere the property
    // is unset and the reader's own setting chooses.
    const NEAREST: &str = "[data-theme=\"light\"] { --statac-code-scheme: light; }\n\
                           [data-theme=\"dark\"] { --statac-code-scheme: dark; }\n\n\
                           .code {\n  color-scheme: var(--statac-code-scheme, light dark);\n";
    for (name, colours) in [
        (
            "ocean",
            "  color: var(--code-text, light-dark(#4f5b66, #c0c5ce));\n  \
             background-color: var(--code-background, light-dark(#eff1f5, #2b303b));\n}\n\n\
             .hl-comment {",
        ),
        (
            "gruvbox",
            "  color: var(--code-text, light-dark(#3c3836, #ebdbb2));\n  \
             background-color: var(--code-background, light-dark(#fbf1c7, #282828));\n}\n\n\
             .hl-comment {",
        ),
    ] {
        let css = stylesheet(name).unwrap();
        let (opening, rules) = css.split_once("*/\n\n").unwrap();
        assert!(opening.starts_with("/* Code colours: "), "{name}: {css}");
        assert_eq!(
            rules
                .strip_prefix(NEAREST)
                .map(|rest| &rest[..colours.len()]),
            Some(colours),
            "{name}"
        );
        // Nothing else in the stylesheet says which look the code takes.
        assert_eq!(css.matches("color-scheme").count(), 1, "{name}");
        assert_eq!(css.matches("data-theme").count(), 2, "{name}");
    }
}

#[test]
fn every_colour_written_can_be_read_on_its_background() {
    for style in styles::STYLES {
        let kinds = style.kinds.iter().map(|(colours, _)| colours);
        for colours in kinds.chain([&style.text]) {
            for (text, on) in colours.iter().zip(style.background) {
                let (text, on) = (colour(text), colour(on));
                let (Some(text), Some(on)) = (legible(text, on).solid(), on.solid()) else {
                    continue;
                };
                let name = style.name;
                assert!(contrast(text, on) >= LEGIBLE, "{name}: {text:?} on {on:?}");
            }
        }
    }
}

#[test]
fn a_colour_that_can_be_read_is_left_and_one_too_faint_is_moved_the_least() {
    let (paper, ink) = (colour("#eff1f5"), colour("#2b303b"));
    assert_eq!(legible(ink, paper), ink);
    assert_eq!(legible(paper, ink), paper);
    let green = colour("#a3be8c");
    assert_eq!(legible(green, ink), green);
    let darker = legible(green, paper);
    let apart = contrast(darker.solid().unwrap(), paper.solid().unwrap());
    assert!((LEGIBLE..LEGIBLE + 0.2).contains(&apart), "{apart}");
    let faint = colour("#a3be8c80");
    assert_eq!(legible(faint, paper), faint);
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

#[test]
fn a_colour_both_looks_share_is_written_once() {
    let solid = colour("#123456");
    assert_eq!(both(solid, solid), "#123456");
    let faint = colour("#12345630");
    assert_eq!(both(solid, faint), "light-dark(#123456, #12345630)");
    assert_eq!(opaque(faint).hex(), "#123456");
    assert_eq!(Colour::from_hex("#12345"), None);
    assert_eq!(Colour::from_hex("123456"), None);
}
