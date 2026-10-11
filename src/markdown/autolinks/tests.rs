use super::{Written, written_at};
use crate::language::bundled;
use crate::markdown::typography::{bare_addresses, blocks};
use crate::markdown::{
    Call, Document, Kind, Node, Options, Placement, Snippets, Typography, read, set_typography,
    to_html,
};
use crate::source::{Place, Sources, Span};

struct Named;

impl Snippets for Named {
    fn html(&self, call: &Call, _: Place, _: Placement, _: Option<&str>) -> String {
        format!("<call {}>", call.name.value)
    }
}

fn read_text(text: &str) -> (Sources, Document) {
    let mut sources = Sources::default();
    let id = sources.add("content/post-1.md", text.to_owned());
    let document = read(&sources, id, Span::new(0, text.len()), &|_| Some(false), "");
    (sources, document)
}

fn html_of(text: &str) -> String {
    let (_, document) = read_text(text);
    let options = Options {
        snippets: &Named,
        back_to_the_text: None,
        digits: None,
        right_to_left: false,
        highlighter: None,
    };
    to_html(&document, &options).trim_end().to_owned()
}

/// The words of a one-paragraph document as HTML.
fn line(text: &str) -> String {
    let html = html_of(text);
    html.strip_prefix("<p>")
        .and_then(|rest| rest.strip_suffix("</p>"))
        .unwrap_or(&html)
        .to_owned()
}

fn links(document: &Document) -> Vec<&Node> {
    let mut found = Vec::new();
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| {
            if matches!(node.kind, Kind::Link { .. }) {
                found.push(node);
            }
        });
    }
    found
}

/// Every example of GitHub Flavored Markdown's autolinks extension, each
/// line its own paragraph, with mail addresses at example domains.
#[test]
fn bare_addresses_are_linked_as_github_flavored_markdown_links_them() {
    let cases = [
        (
            "www.commonmark.org",
            r#"<a href="http://www.commonmark.org">www.commonmark.org</a>"#,
        ),
        (
            "Visit www.commonmark.org/help for more information.",
            r#"Visit <a href="http://www.commonmark.org/help">www.commonmark.org/help</a> for more information."#,
        ),
        (
            "Visit www.commonmark.org.",
            r#"Visit <a href="http://www.commonmark.org">www.commonmark.org</a>."#,
        ),
        (
            "Visit www.commonmark.org/a.b.",
            r#"Visit <a href="http://www.commonmark.org/a.b">www.commonmark.org/a.b</a>."#,
        ),
        (
            "www.google.com/search?q=Markup+(business)",
            r#"<a href="http://www.google.com/search?q=Markup+(business)">www.google.com/search?q=Markup+(business)</a>"#,
        ),
        (
            "www.google.com/search?q=Markup+(business)))",
            r#"<a href="http://www.google.com/search?q=Markup+(business)">www.google.com/search?q=Markup+(business)</a>))"#,
        ),
        (
            "(www.google.com/search?q=Markup+(business))",
            r#"(<a href="http://www.google.com/search?q=Markup+(business)">www.google.com/search?q=Markup+(business)</a>)"#,
        ),
        (
            "(www.google.com/search?q=Markup+(business)",
            r#"(<a href="http://www.google.com/search?q=Markup+(business)">www.google.com/search?q=Markup+(business)</a>"#,
        ),
        (
            "www.google.com/search?q=(business))+ok",
            r#"<a href="http://www.google.com/search?q=(business))+ok">www.google.com/search?q=(business))+ok</a>"#,
        ),
        (
            "www.google.com/search?q=commonmark&hl=en",
            r#"<a href="http://www.google.com/search?q=commonmark&amp;hl=en">www.google.com/search?q=commonmark&amp;hl=en</a>"#,
        ),
        (
            "www.google.com/search?q=commonmark&hl;",
            r#"<a href="http://www.google.com/search?q=commonmark">www.google.com/search?q=commonmark</a>&amp;hl;"#,
        ),
        (
            "www.commonmark.org/he<lp",
            r#"<a href="http://www.commonmark.org/he">www.commonmark.org/he</a>&lt;lp"#,
        ),
        (
            "http://commonmark.org",
            r#"<a href="http://commonmark.org">http://commonmark.org</a>"#,
        ),
        (
            "(Visit https://encrypted.google.com/search?q=Markup+(business))",
            r#"(Visit <a href="https://encrypted.google.com/search?q=Markup+(business)">https://encrypted.google.com/search?q=Markup+(business)</a>)"#,
        ),
        (
            "foo@bar.example.org",
            r#"<a href="mailto:foo@bar.example.org">foo@bar.example.org</a>"#,
        ),
        (
            "hello@mail+xyz.example.org isn't valid, but hello+xyz@mail.example.org is.",
            r#"hello@mail+xyz.example.org isn't valid, but <a href="mailto:hello+xyz@mail.example.org">hello+xyz@mail.example.org</a> is."#,
        ),
        (
            "a.b-c_d@a.b",
            r#"<a href="mailto:a.b-c_d@a.b">a.b-c_d@a.b</a>"#,
        ),
        (
            "a.b-c_d@a.b.",
            r#"<a href="mailto:a.b-c_d@a.b">a.b-c_d@a.b</a>."#,
        ),
        ("a.b-c_d@a.b-", "a.b-c_d@a.b-"),
        ("a.b-c_d@a.b_", "a.b-c_d@a.b_"),
        (
            "mailto:foo@bar.example.org",
            r#"<a href="mailto:foo@bar.example.org">mailto:foo@bar.example.org</a>"#,
        ),
        (
            "mailto:a.b-c_d@a.b",
            r#"<a href="mailto:a.b-c_d@a.b">mailto:a.b-c_d@a.b</a>"#,
        ),
        (
            "mailto:a.b-c_d@a.b.",
            r#"<a href="mailto:a.b-c_d@a.b">mailto:a.b-c_d@a.b</a>."#,
        ),
        (
            "mailto:a.b-c_d@a.b/",
            r#"<a href="mailto:a.b-c_d@a.b">mailto:a.b-c_d@a.b</a>/"#,
        ),
        ("mailto:a.b-c_d@a.b-", "mailto:a.b-c_d@a.b-"),
        ("mailto:a.b-c_d@a.b_", "mailto:a.b-c_d@a.b_"),
        (
            "xmpp:foo@bar.example.org",
            r#"<a href="xmpp:foo@bar.example.org">xmpp:foo@bar.example.org</a>"#,
        ),
        (
            "xmpp:foo@bar.example.org.",
            r#"<a href="xmpp:foo@bar.example.org">xmpp:foo@bar.example.org</a>."#,
        ),
        (
            "xmpp:foo@bar.example.org/txt",
            r#"<a href="xmpp:foo@bar.example.org/txt">xmpp:foo@bar.example.org/txt</a>"#,
        ),
        (
            "xmpp:foo@bar.example.org/txt@bin",
            r#"<a href="xmpp:foo@bar.example.org/txt@bin">xmpp:foo@bar.example.org/txt@bin</a>"#,
        ),
        (
            "xmpp:foo@bar.example.org/txt@bin.example.com",
            r#"<a href="xmpp:foo@bar.example.org/txt@bin.example.com">xmpp:foo@bar.example.org/txt@bin.example.com</a>"#,
        ),
        (
            "xmpp:foo@bar.example.org/txt/bin",
            r#"<a href="xmpp:foo@bar.example.org/txt">xmpp:foo@bar.example.org/txt</a>/bin"#,
        ),
    ];
    for (written, wanted) in cases {
        assert_eq!(line(written), wanted, "{written}");
    }
}

#[test]
fn an_address_starts_a_line_a_word_or_what_emphasis_or_a_bracket_opens() {
    let cases = [
        (
            "*www.a.org*",
            r#"<em><a href="http://www.a.org">www.a.org</a></em>"#,
        ),
        (
            "~~www.a.org~~",
            r#"<del><a href="http://www.a.org">www.a.org</a></del>"#,
        ),
        (
            "a\nwww.a.org",
            "a\n<a href=\"http://www.a.org\">www.a.org</a>",
        ),
        ("xwww.a.org", "xwww.a.org"),
        (
            "\"https://a.org\"",
            r#"&quot;<a href="https://a.org">https://a.org</a>&quot;"#,
        ),
        (
            "http://localhost:4000/",
            r#"<a href="http://localhost:4000/">http://localhost:4000/</a>"#,
        ),
        (
            "ftp://a.org/f.txt.",
            r#"<a href="ftp://a.org/f.txt">ftp://a.org/f.txt</a>."#,
        ),
        (
            "# See www.a.org",
            r#"<h1>See <a href="http://www.a.org">www.a.org</a></h1>"#,
        ),
        (
            "a <span>see www.a.org</span>",
            r#"a <span>see <a href="http://www.a.org">www.a.org</a></span>"#,
        ),
        ("a <span>www.a.org</span>", "a <span>www.a.org</span>"),
    ];
    for (written, wanted) in cases {
        assert_eq!(line(written), wanted, "{written}");
    }
}

/// Where an address may start and what is left off its end, as cmark-gfm
/// reads them where the spec's prose says less.
#[test]
fn an_address_starts_and_ends_as_the_reference_reader_has_it() {
    let cases = [
        (
            "Read \"https://a.org\".",
            r#"Read &quot;<a href="https://a.org">https://a.org</a>&quot;."#,
        ),
        (
            "1https://a.org",
            r#"1<a href="https://a.org">https://a.org</a>"#,
        ),
        ("xhttps://a.org", "xhttps://a.org"),
        (
            "'www.a.org/x' and \"www.b.org\"",
            "'www.a.org/x' and &quot;www.b.org&quot;",
        ),
        (
            "(see 'https://a.org/x').",
            r#"(see '<a href="https://a.org/x">https://a.org/x</a>')."#,
        ),
        (
            "www.a.org/x; www.b.org/y&z1;",
            r#"<a href="http://www.a.org/x">www.a.org/x</a>; <a href="http://www.b.org/y&amp;z1">www.b.org/y&amp;z1</a>;"#,
        ),
        (
            "x/a@b.example.org",
            r#"x/<a href="mailto:a@b.example.org">a@b.example.org</a>"#,
        ),
        (
            "xmailto:a@b.example.org",
            r#"xmailto:<a href="mailto:a@b.example.org">a@b.example.org</a>"#,
        ),
    ];
    for (written, wanted) in cases {
        assert_eq!(line(written), wanted, "{written}");
    }
}

/// Statac's own end, earlier than cmark-gfm's: what shows nothing, and a
/// typographic closing quote or bracket no opening one inside the address
/// matches, stay in the text after the link.
#[test]
fn a_link_ends_before_what_shows_nothing_and_an_unmatched_closing_quote() {
    let cases = [
        (
            "(should be \u{201c}http://local.projectname.com\u{201d}).",
            "(should be \u{201c}<a href=\"http://local.projectname.com\">http://local.projectname.com</a>\u{201d}).",
        ),
        (
            "\u{202a}https://tools.wmflabs.org/paste/view/raw/4625f46a\u{202c}",
            "\u{202a}<a href=\"https://tools.wmflabs.org/paste/view/raw/4625f46a\">https://tools.wmflabs.org/paste/view/raw/4625f46a</a>\u{202c}",
        ),
        (
            "See:https://gnunet.org/\u{200e}",
            "See:<a href=\"https://gnunet.org/\">https://gnunet.org/</a>\u{200e}",
        ),
        (
            "\u{ab}https://a.org/x\u{bb} and \u{ab}www.a.org\u{bb}",
            "\u{ab}<a href=\"https://a.org/x\">https://a.org/x</a>\u{bb} and \u{ab}www.a.org\u{bb}",
        ),
        (
            "https://a.org/\u{201c}x\u{201d}",
            "<a href=\"https://a.org/%E2%80%9Cx%E2%80%9D\">https://a.org/\u{201c}x\u{201d}</a>",
        ),
    ];
    for (written, wanted) in cases {
        assert_eq!(line(written), wanted, "{written:?}");
    }
}

/// cmark-gfm links no `www.` or scheme address from a `[` to the next
/// `]`, whether or not they make a link, but links a mail address there.
#[test]
fn an_open_bracket_holds_off_all_but_a_mail_address() {
    let cases = [
        ("[see www.a.org]", "[see www.a.org]"),
        ("[see https://a.org", "[see https://a.org"),
        (
            "[see a@b.example.org]",
            r#"[see <a href="mailto:a@b.example.org">a@b.example.org</a>]"#,
        ),
        (
            "[1] www.a.org",
            r#"[1] <a href="http://www.a.org">www.a.org</a>"#,
        ),
        (
            "\\[see www.a.org",
            r#"[see <a href="http://www.a.org">www.a.org</a>"#,
        ),
        ("[a [b] www.a.org", "[a [b] www.a.org"),
        (
            "[a] [b]] www.a.org",
            r#"[a] [b]] <a href="http://www.a.org">www.a.org</a>"#,
        ),
        (
            "https://a.org/[x www.b.org",
            r#"<a href="https://a.org/%5Bx">https://a.org/[x</a> <a href="http://www.b.org">www.b.org</a>"#,
        ),
        (
            "[a](/b/) www.a.org",
            r#"<a href="/b/">a</a> <a href="http://www.a.org">www.a.org</a>"#,
        ),
        ("*[see* www.a.org", "<em>[see</em> www.a.org"),
    ];
    for (written, wanted) in cases {
        assert_eq!(line(written), wanted, "{written}");
    }
}

#[test]
fn an_address_is_never_linked_where_it_isnt_the_writing() {
    let cases = [
        ("`www.a.org`", "<p><code>www.a.org</code></p>"),
        (
            "[see www.a.org](/a/)",
            r#"<p><a href="/a/">see www.a.org</a></p>"#,
        ),
        (
            "<https://a.org>",
            r#"<p><a href="https://a.org">https://a.org</a></p>"#,
        ),
        (
            "<a href=\"/a/\">see www.a.org</a>",
            "<p><a href=\"/a/\">see www.a.org</a></p>",
        ),
        ("<code>www.a.org</code>", "<p><code>www.a.org</code></p>"),
        (
            "$www.a.org$",
            "<p><math><mi>w</mi><mi>w</mi><mi>w</mi><mi>.</mi><mi>a</mi><mi>.</mi><mi>o</mi>\
             <mi>r</mi><mi>g</mi></math></p>",
        ),
        (
            "```\nwww.a.org\n```",
            "<pre class=\"code\"><code>www.a.org</code></pre>",
        ),
        ("<div>\nwww.a.org\n</div>", "<div>\nwww.a.org\n</div>"),
        (
            "![www.a.org](a.png)",
            r#"<p><img src="a.png" alt="www.a.org" loading="lazy" decoding="async"></p>"#,
        ),
        ("www\\.a.org", "<p>www.a.org</p>"),
        ("@card(www.a.org)", "<call card>"),
    ];
    for (written, wanted) in cases {
        assert_eq!(html_of(written), wanted, "{written}");
    }
    let (_, document) = read_text("A @card(www.a.org, https://a.org) b.\n");
    assert!(links(&document).is_empty());
}

#[test]
fn a_link_made_from_an_address_is_placed_at_the_address() {
    let text =
        "This &amp; That \\* at www.a.org, then a@b.example.org.\n\n[^1]\n\n[^1]: See www.c.org\n";
    let (sources, document) = read_text(text);
    let placed: Vec<_> = links(&document)
        .into_iter()
        .map(|link| {
            let words: Vec<_> = link
                .children
                .iter()
                .map(|child| sources.spelling(child.place))
                .collect();
            (sources.spelling(link.place), words)
        })
        .collect();
    assert_eq!(
        placed,
        [
            ("www.a.org", vec!["www.a.org"]),
            ("a@b.example.org", vec!["a@b.example.org"]),
            ("www.c.org", vec!["www.c.org"]),
        ]
    );
    let mut texts = Vec::new();
    document.root.walk(&mut |node| {
        if let Kind::Text(text) = &node.kind {
            texts.push((text.clone(), sources.spelling(node.place).to_owned()));
        }
    });
    for (text, spelling) in [
        ("This & That * at ", "This &amp; That \\* at "),
        (", then ", ", then "),
    ] {
        assert!(
            texts.contains(&(text.to_owned(), spelling.to_owned())),
            "{texts:?}"
        );
    }
}

/// Typography finds the addresses linking does, so once they are linked
/// none is left among the writing for typography to keep as typed, and
/// the words inside each link reach the page as typed.
#[test]
fn typography_and_linking_agree_on_every_address() {
    let samples = [
        "www.a.org.www.b.org and www.a_b.www.c.org",
        "(www.a.org/(x))) \"www.b.org\" 'it's' www.c.org/a--b...",
        "a_b@c.example.org_x www.x_(www.b.org mailto:a@b.c/d xmpp:a@b.c/d.e/f",
        "*www.a.org*_www.b.org_ ~www.c.org~ https://a.org/it's--here",
        "This &amp; www.a.org&amp;b www.b.org/&hl; www.c.org/x&amp;y;",
        "x www.a.org\\-b www.b.org/\\\"q a.b-c_d@a.b- www.d.org<e",
        "\u{201c}http://a.org/it's\u{201d}\u{202c} http://localhost:8080/a--b ftp://a.org/\u{200e}",
    ];
    let words = bundled("en").expect("bundled").1;
    for sample in samples {
        let (sources, mut document) = read_text(sample);
        let mut left = Vec::new();
        blocks(&mut document.root, &mut |nodes, source| {
            for bare in bare_addresses(nodes, sources.text(source)) {
                if bare.linkable {
                    left.push(bare.destination);
                }
            }
        });
        assert!(left.is_empty(), "{sample}: {left:?}");
        let linked =
            |document: &Document| -> Vec<Node> { links(document).into_iter().cloned().collect() };
        let before = linked(&document);
        set_typography(&mut document, &sources, &Typography::of(&words));
        assert_eq!(linked(&document), before, "{sample}");
    }
}

#[test]
fn a_character_is_found_where_it_was_typed() {
    assert_eq!(written_at("a www.b.c", "a www.b.c", 2), Some(2));
    assert_eq!(written_at("a*b www.c.d", "a\\*b www.c.d", 4), Some(5));
    assert_eq!(written_at("a & www.b.c", "a &amp; www.b.c", 4), Some(8));
    assert_eq!(written_at("é www.b.c", "&eacute; www.b.c", 3), Some(9));
    assert_eq!(written_at("é www.b.c", "&#233; www.b.c", 3), Some(7));
    assert_eq!(written_at("&x; é", "&x; &eacute;", 4), Some(4));
    assert_eq!(written_at("&amp; www", "&amp;amp; www", 6), Some(10));
    assert_eq!(written_at("ab www", "a\u{b}b www", 3), Some(4));
    assert_eq!(written_at("ab", "xy", 1), None);
}

/// A place asked for after another is read on from it, and is where it
/// would be found asked alone, whatever the order of asking.
#[test]
fn a_character_is_found_the_same_asked_in_any_order() {
    let cases = [
        ("a*b www.c.d * e", "a\\*b www.c.d \\* e"),
        ("a & www.b.c & d", "a &amp; www.b.c &#38; d"),
        ("é www.b.c é", "&eacute; www.b.c &#233;"),
        ("&x; é", "&x; &eacute;"),
        ("ab www cd", "a\u{b}b www c\u{b}d"),
        ("ab", "xy"),
    ];
    for (text, spelling) in cases {
        let places: Vec<usize> = text
            .char_indices()
            .map(|(at, _)| at)
            .chain([text.len()])
            .collect();
        let alone: Vec<_> = places
            .iter()
            .map(|at| written_at(text, spelling, *at))
            .collect();
        let written = Written::new(text, spelling);
        let onwards: Vec<_> = places.iter().map(|at| written.at(*at)).collect();
        assert_eq!(onwards, alone, "{text}");
        let back: Vec<_> = places.iter().rev().map(|at| written.at(*at)).collect();
        assert_eq!(
            back,
            alone.iter().rev().copied().collect::<Vec<_>>(),
            "{text}"
        );
        let twice: Vec<_> = places
            .iter()
            .map(|at| (written.at(*at), written.at(*at)))
            .collect();
        assert!(
            twice
                .iter()
                .zip(&alone)
                .all(|((a, b), alone)| a == alone && b == alone)
        );
    }
}

#[test]
fn every_address_of_one_line_is_placed_at_itself() {
    let text = "É \\* www.a.org &amp; é x@b.example.org, \\[ www.c.org ö &#233; d@e.example.org www.f.org.\n";
    let (sources, document) = read_text(text);
    let placed: Vec<_> = links(&document)
        .into_iter()
        .map(|link| {
            let words: String = link
                .children
                .iter()
                .map(|child| sources.spelling(child.place))
                .collect();
            (sources.spelling(link.place), words)
        })
        .collect();
    let addresses = [
        "www.a.org",
        "x@b.example.org",
        "www.c.org",
        "d@e.example.org",
        "www.f.org",
    ];
    let expected: Vec<_> = addresses.iter().map(|a| (*a, (*a).to_owned())).collect();
    assert_eq!(placed, expected);
}
