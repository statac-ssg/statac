use super::super::{Call, Options, Placement, Snippets, give_heading_ids, read, to_html};
use super::{Typography, set_typography, set_words, tidied};
use crate::language::{Words, bundled};
use crate::source::{Place, Sources, Span};

/// Writes a call as an element naming it, its writing inside.
struct Named;

impl Snippets for Named {
    fn html(&self, call: &Call, _: Place, _: Placement, body: Option<&str>) -> String {
        match body {
            Some(body) => format!("<{0}>{body}</{0}>", call.name.value),
            None => format!("<{}>", call.name.value),
        }
    }
}

fn words(tag: &str) -> std::sync::Arc<Words> {
    bundled(tag).expect("bundled").1
}

/// French as a site's own file sets it: inner quotes of another shape, a
/// space inside the outer ones, and the marks a typed space is kept with.
fn french() -> Words {
    let file = "quotes: « » “ ”\n\
                no_break_before: \"; : ! ? »\"\n\
                no_break_after: «\n\
                inside_quotes: \"\\u00A0\"\n";
    let mut sources = Sources::default();
    let id = sources.add("languages/fr.yaml", file.to_owned());
    let (words, problems) = crate::language::read_onto(&sources, id, (*words("fr")).clone());
    assert!(problems.is_empty(), "{problems:?}");
    words
}

fn set_in(words: &Words, markdown: &str) -> String {
    let mut sources = Sources::default();
    let id = sources.add("content/a.md", markdown.to_owned());
    let body = Span::new(0, markdown.len());
    let mut document = read(&sources, id, body, &|name| Some(name == "box"), "");
    give_heading_ids(&mut document, "content/a.md", markdown);
    set_typography(&mut document, &sources, &Typography::of(words));
    let options = Options {
        snippets: &Named,
        back_to_the_text: None,
        digits: None,
        right_to_left: false,
        highlighter: None,
    };
    to_html(&document, &options)
}

/// The words of a one-paragraph document, set in `tag`.
fn line(tag: &str, markdown: &str) -> String {
    let html = set_in(&words(tag), markdown);
    html.strip_prefix("<p>")
        .and_then(|rest| rest.strip_suffix("</p>"))
        .unwrap_or(&html)
        .to_owned()
}

fn each(tag: &str, cases: &[(&str, &str)]) {
    for (written, wanted) in cases {
        assert_eq!(line(tag, written), *wanted, "{tag}: {written}");
    }
}

#[test]
fn quotes_are_set_in_english_marks_and_an_apostrophe_is_always_one() {
    each(
        "en",
        &[
            ("\"Yes,\" she said.", "\u{201c}Yes,\u{201d} she said."),
            ("'Yes,' she said.", "\u{2018}Yes,\u{2019} she said."),
            ("It's the cat's.", "It\u{2019}s the cat\u{2019}s."),
            ("In the '90s.", "In the \u{2019}90s."),
            ("Rock 'n' roll", "Rock \u{2018}n\u{2019} roll"),
            (
                "\"It's 'mine'\"",
                "\u{201c}It\u{2019}s \u{2018}mine\u{2019}\u{201d}",
            ),
            (
                "[Post 1](/b/)'s title",
                "<a href=\"/b/\">Post 1</a>\u{2019}s title",
            ),
        ],
    );
}

#[test]
fn hyphens_and_full_stops_are_set_as_dashes_and_an_ellipsis() {
    each(
        "en",
        &[
            ("a - b", "a - b"),
            ("a -- b", "a \u{2013} b"),
            ("a --- b", "a \u{2014} b"),
            ("a ---- b", "a \u{2013}\u{2013} b"),
            ("a ----- b", "a \u{2014}\u{2013} b"),
            ("a ------ b", "a \u{2014}\u{2014} b"),
            ("a ------- b", "a \u{2014}\u{2013}\u{2013} b"),
            ("well-known", "well-known"),
            ("Wait...", "Wait\u{2026}"),
            ("Wait....", "Wait\u{2026}."),
            ("Wait......", "Wait\u{2026}\u{2026}"),
            ("a.. b", "a.. b"),
            ("pages 10--20", "pages 10\u{2013}20"),
        ],
    );
}

/// Each language's own marks, outer and inner.
#[test]
fn each_language_sets_its_own_marks() {
    let sentence = "\"Say 'yes'\" -- it's done...";
    for (tag, wanted) in [
        (
            "en",
            "\u{201c}Say \u{2018}yes\u{2019}\u{201d} \u{2013} it\u{2019}s done\u{2026}",
        ),
        (
            "de",
            "\u{201e}Say \u{201a}yes\u{2018}\u{201c} \u{2013} it\u{2019}s done\u{2026}",
        ),
        (
            "es",
            "\u{201c}Say \u{2018}yes\u{2019}\u{201d} \u{2013} it\u{2019}s done\u{2026}",
        ),
        (
            "fa",
            "\u{ab}Say \u{2039}yes\u{203a}\u{bb} \u{2013} it\u{2019}s done\u{2026}",
        ),
        (
            "fr",
            "\u{ab}Say \u{ab}yes\u{bb}\u{bb} \u{2013} it\u{2019}s done\u{2026}",
        ),
        (
            "ja",
            "\u{300c}Say \u{300e}yes\u{300f}\u{300d} \u{2013} it\u{2019}s done\u{2026}",
        ),
        (
            "ko",
            "\u{201c}Say \u{2018}yes\u{2019}\u{201d} \u{2013} it\u{2019}s done\u{2026}",
        ),
        (
            "ar",
            "\u{201d}Say \u{2019}yes\u{2018}\u{201c} \u{2013} it\u{2019}s done\u{2026}",
        ),
    ] {
        assert_eq!(line(tag, sentence), wanted, "{tag}");
    }
}

/// With no marks of its own a language keeps its quotes as typed, though
/// an apostrophe, a dash and an ellipsis are still set.
#[test]
fn a_language_with_no_marks_keeps_its_quotes_straight() {
    let none = Words::default();
    let html = set_in(&none, "\"Oui,\" dit-il, 'bien' -- c'est tout...");
    assert_eq!(
        html,
        "<p>&quot;Oui,&quot; dit-il, 'bien' \u{2013} c\u{2019}est tout\u{2026}</p>"
    );
}

/// The space a language writes inside its quotes goes inside the outer
/// pair typography sets, never the inner pair, and never doubles a space
/// the writer typed there.
#[test]
fn a_language_spaces_the_quotes_it_sets() {
    let spaced = Words {
        inside_quotes: Some('\u{202f}'),
        ..french()
    };
    for (written, wanted) in [
        (
            "Il a dit \"beaucoup de neige\".",
            "Il a dit \u{ab}\u{202f}beaucoup de neige\u{202f}\u{bb}.",
        ),
        (
            "\"C'est 'magnifique', vraiment\" !",
            "\u{ab}\u{202f}C\u{2019}est \u{201c}magnifique\u{201d}, vraiment\u{202f}\u{bb}\u{a0}!",
        ),
        ("\"*Oui*\"", "\u{ab}\u{202f}<em>Oui</em>\u{202f}\u{bb}"),
        (
            "Il a dit \" beaucoup \".",
            "Il a dit \u{ab}\u{a0}beaucoup\u{a0}\u{bb}.",
        ),
        (
            "Il a dit \u{ab}\u{202f}Oui\u{202f}\u{bb}.",
            "Il a dit \u{ab}\u{202f}Oui\u{202f}\u{bb}.",
        ),
        (
            "Un \u{e9}cran de 13\" suffit.",
            "Un \u{e9}cran de 13&quot; suffit.",
        ),
    ] {
        assert_eq!(
            set_in(&spaced, written),
            format!("<p>{wanted}</p>"),
            "{written}"
        );
    }
    let unspaced = Words {
        inside_quotes: None,
        ..french()
    };
    assert_eq!(
        set_in(&unspaced, "Il a dit \"neige\"."),
        "<p>Il a dit \u{ab}neige\u{bb}.</p>"
    );
    let straight = Words {
        quotes: None,
        ..french()
    };
    assert_eq!(
        set_in(&straight, "Il a dit \"neige\"."),
        "<p>Il a dit &quot;neige&quot;.</p>",
        "no space inside quotes left straight"
    );
}

#[test]
fn double_quotes_pair_from_the_inside_out() {
    each(
        "en",
        &[(
            "\"One \"two\" and \"three\" four.\"",
            "\u{201c}One \u{201c}two\u{201d} and \u{201c}three\u{201d} four.\u{201d}",
        )],
    );
}

/// Where the neighbours can't tell a quote's side, between two words of
/// Chinese or Japanese or two spaces, quotes take turns.
#[test]
fn quotes_nothing_around_can_tell_take_turns() {
    each(
        "en",
        &[
            (
                "\u{4ed6}\u{8bf4}\"\u{4f60}\u{597d}\"\u{7136}\u{540e}\u{3002}",
                "\u{4ed6}\u{8bf4}\u{201c}\u{4f60}\u{597d}\u{201d}\u{7136}\u{540e}\u{3002}",
            ),
            (
                "A \" b \" c \" d \" e.",
                "A \u{201c} b \u{201d} c \u{201c} d \u{201d} e.",
            ),
        ],
    );
    each(
        "ja",
        &[(
            "\u{3053}\u{308c}\u{306f}\"Getting started\"\u{3067}\u{3059}",
            "\u{3053}\u{308c}\u{306f}\u{300c}Getting started\u{300d}\u{3067}\u{3059}",
        )],
    );
}

/// A quote that closes nothing, an inch mark most often, is left as typed;
/// so is one that took its turn to open and found nothing to close it.
#[test]
fn a_quote_that_pairs_with_nothing_is_left_straight() {
    assert_eq!(
        set_in(
            &words("en"),
            "An 11\" laptop and a 1/4\" jack.\n\nAnd \" alone.\n"
        ),
        "<p>An 11&quot; laptop and a 1/4&quot; jack.</p>\n<p>And &quot; alone.</p>"
    );
}

/// A quotation running on into the next paragraph opens each one and
/// closes only the last.
#[test]
fn a_quotation_running_on_opens_each_paragraph() {
    assert_eq!(
        set_in(&words("en"), "\"Come and see,\n\n\"It's frozen.\""),
        "<p>\u{201c}Come and see,</p>\n<p>\u{201c}It\u{2019}s frozen.\u{201d}</p>"
    );
}

/// A single quote with Chinese, Japanese or Korean straight after it
/// closes the one open, as a Korean particle follows a quotation; with
/// none open it is still an apostrophe.
#[test]
fn a_single_quote_before_a_particle_closes() {
    each(
        "ko",
        &[
            (
                "\u{adf8}\u{b294} '\u{adf8}\u{b807}\u{b2e4}'\u{ace0} \u{b9d0}\u{d588}\u{b2e4}.",
                "\u{adf8}\u{b294} \u{2018}\u{adf8}\u{b807}\u{b2e4}\u{2019}\u{ace0} \
                 \u{b9d0}\u{d588}\u{b2e4}.",
            ),
            (
                "\u{b098}\u{b294} 'Python'\u{c744} \u{c4f4}\u{b2e4}.",
                "\u{b098}\u{b294} \u{2018}Python\u{2019}\u{c744} \u{c4f4}\u{b2e4}.",
            ),
            ("Kim's\u{ace0}", "Kim\u{2019}s\u{ace0}"),
        ],
    );
}

/// A space the writer typed against a mark the language keeps it with
/// can't break there; none is added where none was typed.
#[test]
fn a_typed_space_is_kept_with_its_mark() {
    let unspaced = Words {
        inside_quotes: None,
        ..french()
    };
    for (written, wanted) in [
        ("Bonjour !", "Bonjour\u{a0}!"),
        ("Bonjour!", "Bonjour!"),
        (
            "Il a dit : \u{ab} Oui \u{bb} ; bon.",
            "Il a dit\u{a0}: \u{ab}\u{a0}Oui\u{a0}\u{bb}\u{a0}; bon.",
        ),
        ("Il a dit \"Oui\" !", "Il a dit \u{ab}Oui\u{bb}\u{a0}!"),
        ("Bonjour  !", "Bonjour \u{a0}!"),
        ("Et toi *?*", "Et toi\u{a0}<em>?</em>"),
        ("Et *toi* ?", "Et <em>toi</em>\u{a0}?"),
        (
            "Voir [ceci](/ici/) !",
            "Voir <a href=\"/ici/\">ceci</a>\u{a0}!",
        ),
        (
            "Tapez `?` ou `a ?`",
            "Tapez <code>?</code> ou <code>a ?</code>",
        ),
        ("Bonjour <b>!</b>", "Bonjour <b>!</b>"),
        ("Bonjour\n!", "Bonjour\n!"),
        (
            "Un <code>a ! b</code> ici : oui.",
            "Un <code>a ! b</code> ici\u{a0}: oui.",
        ),
    ] {
        assert_eq!(
            set_in(&unspaced, written),
            format!("<p>{wanted}</p>"),
            "{written}"
        );
    }
    assert_eq!(line("en", "Bonjour !"), "Bonjour !", "English keeps none");
}

/// Code, raw HTML, maths, calls and their arguments, and whatever reaches
/// the page as typed, are never set.
#[test]
fn what_typography_never_touches() {
    each(
        "en",
        &[
            ("`\"a\" -- b...`", "<code>&quot;a&quot; -- b...</code>"),
            ("<b title=\"x\">it's</b>", "<b title=\"x\">it\u{2019}s</b>"),
            (
                "Run <code>awk '{print \" MB\";}' -- x...</code> -- it's \"done\".",
                "Run <code>awk '{print &quot; MB&quot;;}' -- x...</code> \u{2013} it\u{2019}s \
                 \u{201c}done\u{201d}.",
            ),
            (
                "<KBD class=\"k\">it's</KBD> and <samp>'a'</samp> then 'b'",
                "<KBD class=\"k\">it's</KBD> and <samp>'a'</samp> then \u{2018}b\u{2019}",
            ),
            (
                "$a' -- b...$ isn't",
                "<math><msup><mi>a</mi><mo>\u{2032}</mo></msup><mo>\u{2212}</mo>\
                 <mo lspace=\"0\" rspace=\"0\">\u{2212}</mo><mi>b</mi><mi>.</mi><mi>.</mi>\
                 <mi>.</mi></math> isn\u{2019}t",
            ),
            (
                "Say @quote(\"it's -- here...\") 'now'",
                "Say <quote> \u{2018}now\u{2019}",
            ),
            (
                "\\@quote(\"it's\") -- here",
                "@quote(&quot;it's&quot;) \u{2013} here",
            ),
            (
                "\\\"Hi\\\" -- \\'a\\' \\-- \\.\\.\\.",
                "&quot;Hi&quot; \u{2013} 'a' -- ...",
            ),
            (
                "&quot;Hi&quot; &#39;a&#39; &#45;- &#x2E;..",
                "&quot;Hi&quot; 'a' -- ...",
            ),
            (
                "[it's](/a--b/ \"x\") and <https://a.org/a--b...>",
                "<a href=\"/a--b/\" title=\"x\">it\u{2019}s</a> and \
                 <a href=\"https://a.org/a--b...\">https://a.org/a--b...</a>",
            ),
            (
                "See https://a.org/it's--here... and www.b.org/c--d -- ok",
                "See <a href=\"https://a.org/it&#x27;s--here\">https://a.org/it's--here</a>\u{2026} \
                 and <a href=\"http://www.b.org/c--d\">www.b.org/c--d</a> \u{2013} ok",
            ),
        ],
    );
    let code = "```\n\"Yes\" -- it's...\n```\n";
    assert_eq!(
        set_in(&words("en"), code),
        "<pre class=\"code\"><code>&quot;Yes&quot; -- it's...</code></pre>"
    );
    let html = "<div title=\"it's\">\n\"Yes\" -- it's...\n</div>\n";
    assert_eq!(set_in(&words("en"), html), html.trim_end());
    let display = "$$\n\"a\" -- b...\n$$\n";
    assert!(
        set_in(&words("en"), display).contains(
            "<mo>\u{2212}</mo><mo lspace=\"0\" rspace=\"0\">\u{2212}</mo><mi>b</mi><mi>.</mi>\
             <mi>.</mi><mi>.</mi>"
        ),
        "{}",
        set_in(&words("en"), display)
    );
}

/// A quote reaching the page as typed pairs with none outside it, and
/// those outside pair as though it weren't there.
#[test]
fn a_quote_left_as_typed_pairs_with_none_outside() {
    each(
        "en",
        &[
            (
                "\"Run <code>echo \"hi</code> now,\" she said.",
                "\u{201c}Run <code>echo &quot;hi</code> now,\u{201d} she said.",
            ),
            (
                "\"Run `echo \"hi` now,\" she said.",
                "\u{201c}Run <code>echo &quot;hi</code> now,\u{201d} she said.",
            ),
            (
                "\"Say \\\" now,\" she said.",
                "\u{201c}Say &quot; now,\u{201d} she said.",
            ),
        ],
    );
}

/// Every block's words are set: headings, lists, tables, quotations,
/// footnotes, a picture's description, and the writing a call wraps.
#[test]
fn every_block_of_writing_is_set() {
    let html = set_in(
        &words("en"),
        "# It's \"here\"\n\n- it's\n\n| it's |\n| --- |\n| a -- b |\n\n> \"q\"\n\n\
         A note.[^1]\n\n[^1]: It's...\n\n@box()\nIt's -- wrapped.\n@end\n\n\
         ![It's \"a\"](a.png \"It's -- \\\"b\\\"\")\n",
    );
    for wanted in [
        "<h1 id=\"its-here\">It\u{2019}s \u{201c}here\u{201d}</h1>",
        "<li>it\u{2019}s</li>",
        "<th>it\u{2019}s</th>",
        "<td>a \u{2013} b</td>",
        "<p>\u{201c}q\u{201d}</p>",
        "It\u{2019}s\u{2026}",
        "It\u{2019}s \u{2013} wrapped.",
        "alt=\"It\u{2019}s \u{201c}a\u{201d}\"",
        "title=\"It\u{2019}s \u{2013} \u{201c}b\u{201d}\"",
    ] {
        assert!(html.contains(wanted), "{wanted}\n{html}");
    }
}

/// A control character left out of the writing leaves the words around it
/// set as though it had never been typed.
#[test]
fn writing_beside_a_control_character_is_set() {
    each(
        "en",
        &[
            ("it's\u{b}", "it\u{2019}s"),
            ("\"a\u{b}b\" -- c", "\u{201c}ab\u{201d} \u{2013} c"),
        ],
    );
}

/// A call standing alone in place of a paragraph leaves the writing
/// around it set as ever.
#[test]
fn writing_after_a_call_standing_alone_is_set() {
    assert_eq!(
        set_in(&words("en"), "@note()\n\nIt's here...\n"),
        "<note>\n<p>It\u{2019}s here\u{2026}</p>"
    );
}

/// A heading's id is made from its words as typed, so a link to it lands
/// whatever the language does with its punctuation.
#[test]
fn a_headings_id_is_made_from_its_words_as_typed() {
    let html = set_in(&french(), "## Bonjour !\n\n## A -- B\n");
    assert!(
        html.contains("<h2 id=\"bonjour-\">Bonjour\u{a0}!</h2>"),
        "{html}"
    );
    assert!(
        html.contains("<h2 id=\"a----b\">A \u{2013} B</h2>"),
        "{html}"
    );
}

/// Words handed to a call are set as writing of their own, everything but
/// the words left as typed.
#[test]
fn words_of_their_own_are_set_and_the_rest_left_as_typed() {
    let fr = french();
    let en = words("en");
    for (typography, written, wanted) in [
        (
            Typography::of(&fr),
            "Il a dit \"Oui\" *toi* !",
            "Il a dit \u{ab}\u{a0}Oui\u{a0}\u{bb} *toi*\u{a0}!",
        ),
        (
            Typography::of(&en),
            "It's `a--b` -- *it's* <i>x</i>...",
            "It\u{2019}s `a--b` \u{2013} *it\u{2019}s* <i>x</i>\u{2026}",
        ),
        (
            Typography::of(&en),
            "See www.a--b.org -- or \\\"this\\\"",
            "See www.a--b.org \u{2013} or \\\"this\\\"",
        ),
    ] {
        assert_eq!(set_words(written, &typography), wanted, "{written}");
    }
}

#[test]
fn a_title_is_set_as_words_of_its_own() {
    let fr = french();
    let typography = Typography::of(&fr);
    assert_eq!(
        tidied("Le lac, \"enfin\" !", &typography),
        "Le lac, \u{ab}\u{a0}enfin\u{a0}\u{bb}\u{a0}!"
    );
    assert_eq!(
        tidied("See www.a--b.org -- or https://c--d.org/e--f", &typography),
        "See www.a--b.org \u{2013} or https://c--d.org/e--f"
    );
}
