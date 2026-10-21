use super::*;
use crate::highlight::Highlighter;
use crate::origin::Style;
use crate::problem::{CallFault, Kind as ProblemKind, MathsFault, Unmatched};
use crate::source::{LineColumn, Place};

/// Writes every call as an empty element naming it and saying where it sits.
struct StandIn;

impl Snippets for StandIn {
    fn html(&self, call: &Call, _: Place, placement: Placement, body: Option<&str>) -> String {
        let at = match (placement, call.within) {
            (Placement::Standalone, _) => "alone",
            (_, Within::Prose) => "prose",
            (_, Within::Html) => "html",
            (_, Within::Attribute) => "attribute",
        };
        match body {
            Some(body) => format!("<call {}={at}>\n{body}\n</call>", call.name.value),
            None => format!("<call {}={at}>", call.name.value),
        }
    }
}

/// `box` wraps writing, `nothing` is no snippet of the site's, and every
/// other name is a snippet that wraps none.
fn wraps(name: &str) -> Option<bool> {
    match name {
        "box" => Some(true),
        "nothing" => None,
        _ => Some(false),
    }
}

fn read_text(text: &str) -> (Sources, Document) {
    let mut sources = Sources::default();
    let id = sources.add("content/post-1.md", text.to_owned());
    let body = crate::yaml::split(text).expect("closed front matter").body;
    let document = read(&sources, id, body, &wraps, "");
    (sources, document)
}

fn html_of(text: &str) -> String {
    let (_, document) = read_text(text);
    to_html(
        &document,
        &Options {
            snippets: &StandIn,
            back_to_the_text: Some("back to the text"),
            digits: None,
            right_to_left: false,
            highlighter: None,
        },
    )
}

#[test]
fn a_right_to_left_page_turns_the_arrow_back_and_keeps_its_name() {
    let (_, document) = read_text("Words.[^1]\n\n[^1]: A note.\n");
    let html = to_html(
        &document,
        &Options {
            snippets: &StandIn,
            back_to_the_text: Some("back to the text"),
            digits: Some(&['۰', '۱', '۲', '۳', '۴', '۵', '۶', '۷', '۸', '۹']),
            right_to_left: true,
            highlighter: None,
        },
    );
    assert_eq!(
        html,
        "<p>Words.<sup class=\"footnote\"><a href=\"#note-1\" id=\"back-1\">۱</a></sup></p>\n\
         <section class=\"footnotes\">\n  <ol>\n    <li id=\"note-1\">\n      \
         <p>A note. <a href=\"#back-1\" class=\"back\" aria-label=\"back to the text\">\u{21aa}</a></p>\n    \
         </li>\n  </ol>\n</section>"
    );
}

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/src/markdown/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{path}"))
}

const FIXTURES: [&str; 14] = [
    "calls.md",
    "html-calls.md",
    "callouts.md",
    "maths.md",
    "spans.md",
    "documents/blocks.md",
    "documents/code.md",
    "documents/footnotes.md",
    "documents/html.md",
    "documents/links.md",
    "documents/lists.md",
    "documents/prose.md",
    "documents/quotations.md",
    "documents/tables.md",
];

fn all_nodes(document: &Document) -> Vec<&Node> {
    let mut out = Vec::new();
    for top in std::iter::once(&document.root).chain(&document.notes) {
        top.walk(&mut |node| out.push(node));
    }
    out
}

/// Each call in the writing: its name, what surrounds it, and its spelling.
fn calls(sources: &Sources, document: &Document) -> Vec<(String, Within, String)> {
    all_nodes(document)
        .into_iter()
        .filter_map(|node| match &node.kind {
            Kind::Call(call) => Some((
                call.name.value.clone(),
                call.within,
                sources.spelling(node.place).to_owned(),
            )),
            _ => None,
        })
        .collect()
}

fn first_text(value: &Value) -> String {
    match value {
        Value::Text(text) => text.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
    }
}

fn line_column(sources: &Sources, place: Place) -> (usize, usize) {
    let LineColumn { line, column } = sources.line_column(place.source, place.span.start as usize);
    (line, column)
}

#[test]
fn calls_are_masked_and_put_back_with_their_places() {
    let text = fixture("calls.md");
    let (sources, document) = read_text(&text);
    let found: Vec<String> = all_nodes(&document)
        .into_iter()
        .filter_map(|node| match &node.kind {
            Kind::Call(call) => Some(format!(
                "{}:{}",
                call.name.value,
                call.arguments
                    .first()
                    .map(|a| first_text(&a.value.value))
                    .unwrap_or_default()
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        found,
        [
            "image:a.jpg",
            "image:b.jpg",
            "image:c.jpg",
            "icon:arrow",
            "card:post",
            "say:*not emphasis*",
            "note:quoted",
            "image:g.jpg",
            "image:h.jpg",
            "icon:star",
        ]
    );
    for (name, _, spelling) in calls(&sources, &document) {
        assert!(spelling.starts_with(&format!("@{name}(")), "{spelling}");
        assert!(spelling.ends_with(')'), "{spelling}");
    }
    let card = all_nodes(&document)
        .into_iter()
        .find(|node| matches!(&node.kind, Kind::Call(call) if call.name.value == "card"))
        .unwrap();
    assert_eq!(line_column(&sources, card.place), (11, 23));
    let end = sources.line_column(card.place.source, card.place.span.end as usize);
    assert_eq!((end.line, end.column), (12, 13));

    let html = html_of(&text);
    for kept in [
        "<p>A call mid-sentence <call image=prose> and on.</p>\n<call image=alone>\n",
        "<code>@image('e.jpg')</code>",
        "<pre class=\"code\"><code>@image('indented-code.jpg')</code></pre>",
        "<code class=\"language-text\">@image('fenced.jpg')</code>",
        "<math><mi>x</mi><mo>+</mo><mi>@</mi><mi>y</mi>",
        "Shown as text: @image('d.jpg') and",
        "<p>Arguments the parser would read: <call say=prose>.</p>",
        "An email <a href=\"mailto:name@example.org\">name@example.org</a>(x) and an address \
         <a href=\"https://example.org/@name(x)\">https://example.org/@name(x)</a> are not calls.",
    ] {
        assert!(html.contains(kept), "lost {kept:?} from\n{html}");
    }
    let say = all_nodes(&document)
        .into_iter()
        .find_map(|node| match &node.kind {
            Kind::Call(call) if call.name.value == "say" => Some(call.clone()),
            _ => None,
        })
        .unwrap();
    let given: Vec<_> = say.unnamed().map(|v| first_text(&v.value)).collect();
    assert_eq!(
        given,
        ["*not emphasis*", "[not a link](x)", "<b>not html</b>"]
    );

    let problems: Vec<_> = document
        .problems
        .iter()
        .map(|p| (p.headline(), line_column(&sources, p.place.unwrap())))
        .collect();
    assert_eq!(
        problems,
        [(
            "Malformed call: `@image(` is never closed".to_owned(),
            (39, 13)
        )]
    );
}

#[test]
fn calls_in_raw_html_keep_out_of_code_and_comments() {
    let text = fixture("html-calls.md");
    let (sources, document) = read_text(&text);
    let found: Vec<_> = calls(&sources, &document)
        .into_iter()
        .map(|(_, within, spelling)| (within, spelling))
        .collect();
    assert_eq!(
        found,
        [
            (Within::Html, "@image('in-a-div.jpg')".to_owned()),
            (
                Within::Attribute,
                "@asset('in-an-attribute.png')".to_owned()
            ),
            (Within::Prose, "@image('after-code.jpg')".to_owned()),
            (Within::Prose, "@image('in-a-span.jpg')".to_owned()),
        ]
    );
    let html = html_of(&text);
    for kept in [
        "<p><call image=html></p>",
        "<img src=\"<call asset=attribute>\">",
        "<code>@image('in-code.jpg')</code>",
        "<span>@image('escaped-in-html.jpg')</span>",
        "\"@image('in-style.jpg')\"",
        "\"@image('in-script.jpg')\"",
        "@image('in-pre.jpg')\n\n@image('in-pre-after-a-blank-line.jpg')",
        "<code>@image('inline-code.jpg')</code>",
    ] {
        assert!(html.contains(kept), "lost {kept:?} from\n{html}");
    }
    // Writing inside raw HTML's code reaches the page as typed.
    let literal = all_nodes(&document)
        .into_iter()
        .any(|node| matches!(&node.kind, Kind::Literal(text) if text == "\"quotes\" -- left"));
    assert!(literal);
}

#[test]
fn callouts_are_their_own_nodes() {
    let (_, document) = read_text(&fixture("callouts.md"));
    let kinds: Vec<CalloutKind> = all_nodes(&document)
        .into_iter()
        .filter_map(|node| match node.kind {
            Kind::Callout(kind) => Some(kind),
            _ => None,
        })
        .collect();
    assert_eq!(
        kinds,
        [
            CalloutKind::Note,
            CalloutKind::Tip,
            CalloutKind::Important,
            CalloutKind::Warning,
            CalloutKind::Caution,
            CalloutKind::Note,
        ]
    );
    let quotations = all_nodes(&document)
        .into_iter()
        .filter(|node| matches!(node.kind, Kind::Quotation))
        .count();
    assert_eq!(quotations, 2);
}

#[test]
fn maths_is_its_own_nodes() {
    let (_, document) = read_text(&fixture("maths.md"));
    let maths: Vec<(bool, String)> = all_nodes(&document)
        .into_iter()
        .filter_map(|node| match &node.kind {
            Kind::Maths { display, text, .. } => Some((*display, text.trim().to_owned())),
            _ => None,
        })
        .collect();
    assert_eq!(
        maths,
        [
            (false, "a^2 + b^2 = c^2".to_owned()),
            (true, r"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}".to_owned()),
            (true, "x = 1".to_owned()),
        ]
    );
}

fn uses_maths(text: &str) -> Vec<String> {
    let (_, document) = read_text(text);
    all_nodes(&document)
        .into_iter()
        .filter_map(|node| match &node.kind {
            Kind::Maths { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn dollars_are_maths_by_pandocs_rule() {
    let cases: [(&str, &[&str]); 17] = [
        ("It cost $30 from one and _$60_ from another.\n", &[]),
        ("The $99-$199 range.\n", &[]),
        ("Paying $330 plus $85 shipping.\n", &[]),
        (
            "About $50, or closer to [$100](https://example.org/).\n",
            &[],
        ),
        ("Inline $a^2 + b^2$ maths.\n", &["a^2 + b^2"]),
        ("A dollar with space: $ not maths $.\n", &[]),
        ("Escaped: \\$5 and \\$6.\n", &[]),
        ("Glued: a$b$ and /$c$.\n", &[]),
        (
            "Add it to $PATH with `echo 'PATH=$HOME/bin:$PATH'` now.\n",
            &[],
        ),
        ("Use <code>$_GET</code> and <code>$_POST</code>.\n", &[]),
        (
            "A [report](http://example.org/$File/a) and [b](http://example.org/$File/b).\n",
            &[],
        ),
        (
            "    ./configure --prefix=$HOME \\\n        LDFLAGS=\"-rpath=$HOME/lib\"\n",
            &[],
        ),
        ("$$\nx = 1\n$$\n", &["\nx = 1\n"]),
        ("Not $$a\nb$$ over lines mid-sentence.\n", &[]),
        ("From `run $$i` to `copy $$r`.\n", &[]),
        (
            "A path: <code>$.results</code> or <code>$.items</code>.\n",
            &[],
        ),
        ("Less: $x<y$ and $$a<b$$.\n", &["x<y", "a<b"]),
    ];
    for (text, expected) in cases {
        assert_eq!(uses_maths(text), expected, "{text:?}");
    }
    assert!(
        html_of("Use <code>$_GET</code> and <code>$_POST</code>.\n")
            .contains("<code>$_GET</code> and <code>$_POST</code>")
    );
}

#[test]
fn display_maths_in_a_quotation_loses_its_markers() {
    assert_eq!(uses_maths("> $$\n> x = 1\n> $$\n"), ["\nx = 1\n"]);
    assert_eq!(uses_maths("- $$\n  x = 1\n  $$\n"), ["\nx = 1\n"]);
}

#[test]
fn maths_is_written_as_mathml() {
    assert_eq!(
        html_of("Inline $x^2$ here.\n\n$$\n\\frac{1}{2}\n$$\n"),
        "<p>Inline <math><msup><mi>x</mi><mn>2</mn></msup></math> here.</p>\n\
         <p><math display=\"block\"><mfrac><mn>1</mn><mn>2</mn></mfrac></math></p>"
    );
    let (_, document) = read_text("It cost $5 and then $10 more, \\$x\\$.\n");
    assert!(!crate::markdown::uses_maths(&document));
    assert!(!html_of("It cost $5 and then $10 more.\n").contains("<math"));
}

/// Equations number on through the page, and a reference finds an
/// equation written after it.
#[test]
fn equations_number_on_through_the_page() {
    let text = "See $\\eqref{b}$.\n\n\
                $$\n\\begin{equation} x \\end{equation}\n$$\n\n\
                $$\n\\begin{equation}\\label{b} y \\end{equation}\n$$\n";
    let (_, document) = read_text(text);
    assert!(document.problems.is_empty(), "{:?}", document.problems);
    let html = html_of(text);
    assert!(html.contains("<a href=\"#b\">(2)</a>"), "{html}");
}

#[test]
fn maths_is_mathml_wherever_the_writing_holds_it() {
    let text = "# The $x^2$ bound\n\n| a |\n|---|\n| $y$ |\n\nA note.[^n]\n\n\
                @box()\n$z$\n@end\n\n> $$\n> v\n> $$\n\n[^n]: See $w$.\n";
    let (_, mut document) = read_text(text);
    assert!(document.problems.is_empty(), "{:?}", document.problems);
    give_heading_ids(&mut document, "content/post-1.md", text);
    let html = to_html(
        &document,
        &Options {
            snippets: &StandIn,
            back_to_the_text: None,
            digits: None,
            right_to_left: false,
            highlighter: None,
        },
    );
    for written in [
        "<h1 id=\"the-x2-bound\">The <math><msup><mi>x</mi><mn>2</mn></msup></math> bound</h1>",
        "<td><math><mi>y</mi></math></td>",
        "<call box=alone>\n<p><math><mi>z</mi></math></p>\n</call>",
        "<p><math display=\"block\"><mi>v</mi></math></p>",
        "See <math><mi>w</mi></math>",
    ] {
        assert!(html.contains(written), "{written:?} not in\n{html}");
    }
    assert_eq!(headings(&document)[0].text, "The x^2 bound");
}

/// The excerpt and opening words of writing that opens with maths: the
/// excerpt is HTML, so it holds the MathML; the opening words are plain
/// text, so they hold the maths as written.
#[test]
fn an_excerpt_holds_the_maths_and_the_opening_words_its_source() {
    let (_, document) = read_text("Area $\\pi r^2$ here.\n\nMore.\n");
    let written = write(
        &document,
        &Options {
            snippets: &StandIn,
            back_to_the_text: None,
            digits: None,
            right_to_left: false,
            highlighter: None,
        },
    );
    assert_eq!(
        written.excerpt.as_deref(),
        Some("<p>Area <math><mi>π</mi><msup><mi>r</mi><mn>2</mn></msup></math> here.</p>")
    );
    assert_eq!(written.opening.as_deref(), Some("Area \\pi r^2 here."));
    assert!(written.maths);
    let (_, plain) = read_text("No maths.\n");
    let written = write(
        &plain,
        &Options {
            snippets: &StandIn,
            back_to_the_text: None,
            digits: None,
            right_to_left: false,
            highlighter: None,
        },
    );
    assert!(!written.maths);
}

#[test]
fn malformed_maths_is_placed_at_the_exact_text() {
    let cases: [(&str, MathsFault, &str, (usize, usize)); 14] = [
        (
            "A $x + \\foo{y}$ b.\n",
            MathsFault::UnknownCommand("\\foo".into()),
            "\\foo",
            (1, 8),
        ),
        (
            "$\\frac{1}{2 b$\n",
            MathsFault::Unclosed {
                opening: Some("{".into()),
                closing: "}".into(),
            },
            "{",
            (1, 10),
        ),
        (
            "$$\n\\begin{pmatrix} a & b \\end{bmatrix}\n$$\n",
            MathsFault::Mismatched {
                written: "\\end{bmatrix}".into(),
                expected: "\\end{pmatrix}".into(),
            },
            "\\end{bmatrix}",
            (2, 23),
        ),
        (
            "$\\left( x$\n",
            MathsFault::Unclosed {
                opening: Some("\\left(".into()),
                closing: "\\right)".into(),
            },
            "\\left(",
            (1, 2),
        ),
        (
            "$x + } y$\n",
            MathsFault::ClosesNothing("}".into()),
            "}",
            (1, 6),
        ),
        (
            "$\\sqrt[3$\n",
            MathsFault::Unclosed {
                opening: Some("[".into()),
                closing: "]".into(),
            },
            "[",
            (1, 7),
        ),
        ("$a^b^c$\n", MathsFault::Twice('^'), "^", (1, 5)),
        (
            "$\\frac{1}$ b.\n",
            MathsFault::MissingBefore("$".into()),
            "$",
            (1, 10),
        ),
        (
            "$\\begin{matrx} a \\end{matrx}$\n",
            MathsFault::UnknownEnvironment("matrx".into()),
            "matrx",
            (1, 9),
        ),
        (
            "$\\color{redd} x$\n",
            MathsFault::UnknownColour("redd".into()),
            "redd",
            (1, 9),
        ),
        (
            "> $$\n> \\begin{cases}\n> x \\\\\n> $$\n",
            MathsFault::Unclosed {
                opening: Some("\\begin{cases}".into()),
                closing: "\\end{cases}".into(),
            },
            "\\begin{cases}",
            (2, 3),
        ),
        (
            "- $$\r\n  x +\r\n  \\foo y\r\n  $$\r\n",
            MathsFault::UnknownCommand("\\foo".into()),
            "\\foo",
            (3, 3),
        ),
        (
            "$$\n\\begin{equation}\\label{a} x \\end{equation}\n$$\n\nSee $\\eqref{b}$.\n",
            MathsFault::NoSuchLabel("b".into()),
            "\\eqref{b}",
            (5, 6),
        ),
        (
            "$\\hspace{1qq}$\n",
            MathsFault::Unreadable("{1qq}".into()),
            "{1qq}",
            (1, 9),
        ),
    ];
    for (text, found, spelling, at) in cases {
        let (sources, document) = read_text(text);
        assert_eq!(
            document.problems.len(),
            1,
            "{text:?}: {:?}",
            document.problems
        );
        let problem = &document.problems[0];
        assert_eq!(
            problem.kind,
            ProblemKind::MalformedMaths { found },
            "{text:?}"
        );
        let place = problem.place.unwrap();
        assert_eq!(sources.spelling(place), spelling, "{text:?}");
        assert_eq!(line_column(&sources, place), at, "{text:?}");
    }
    // A control character, left out of the maths, moves nothing after it.
    for text in ["A $\u{b}x + \\foo$.\n", "$$\nx +\n\\foo\u{b} y\n$$\n"] {
        let (sources, document) = read_text(text);
        let place = document
            .problems
            .iter()
            .find(|p| matches!(p.kind, ProblemKind::MalformedMaths { .. }))
            .and_then(|p| p.place)
            .unwrap();
        assert_eq!(sources.spelling(place), "\\foo", "{text:?}");
    }
}

#[test]
fn a_definition_needs_a_space_after_its_colon() {
    let cases = [
        ("Line one\n::FFFF:C0A8:B67\nline three\n", false),
        ("A picture.\n\n:-)\n\nAfter.\n", false),
        ("Term\n:no space\n", false),
        ("Term\n: with a space\n", true),
        ("Term\n:\twith a tab\n", true),
        ("> Term\n> :-)\n", false),
    ];
    for (text, definition) in cases {
        let (_, document) = read_text(text);
        let found = all_nodes(&document)
            .into_iter()
            .any(|node| matches!(node.kind, Kind::DefinitionList));
        assert_eq!(found, definition, "{text:?}");
    }
    assert!(html_of("A picture.\n\n:-)\n").contains("<p>:-)</p>"));
}

#[test]
fn tabs_after_a_closing_fence_still_close_it() {
    let cases = [
        "```js\nlet a = 1;\n```\t\n\nAfter the fence.\n",
        "```js\nlet a = 1;\n``` \t \n\nAfter the fence.\n",
        "~~~\nlet a = 1;\n~~~\t\n\nAfter the fence.\n",
        "```js\nlet a = 1;\n```\n\nAfter the fence.\n",
    ];
    for text in cases {
        let (sources, document) = read_text(text);
        let code: Vec<_> = all_nodes(&document)
            .into_iter()
            .filter(|node| matches!(node.kind, Kind::CodeBlock { .. }))
            .collect();
        assert_eq!(code.len(), 1, "{text:?}");
        assert!(
            sources.spelling(code[0].place).ends_with("let a = 1;\n```")
                || sources.spelling(code[0].place).ends_with("~~~")
        );
        assert!(
            html_of(text).ends_with("<p>After the fence.</p>"),
            "{text:?}"
        );
    }
    // A tab inside a code block is the code's.
    assert!(html_of("```\na\tb\n```\n").contains("a\tb"));
}

#[test]
fn private_use_characters_are_written_where_the_author_wrote_them() {
    let all: String = ('\u{E000}'..='\u{E001}')
        .chain('\u{E010}'..='\u{E01F}')
        .collect();
    let cases = [
        (
            "<div class=\"icon\">\u{E000}</div>\n".to_owned(),
            "<div class=\"icon\">\u{E000}</div>".to_owned(),
        ),
        (
            "Plain \u{E000}\u{E001} words @url(/about/).\n".to_owned(),
            "<p>Plain \u{E000}\u{E001} words /about/.</p>".to_owned(),
        ),
        (
            "A <span title=\"\u{E000}\">\u{E001}</span> and \u{E010}.\n".to_owned(),
            "<p>A <span title=\"\u{E000}\">\u{E001}</span> and \u{E010}.</p>".to_owned(),
        ),
        (
            "<a href=\"@url(/about/)\" title=\"\u{E001}\">\u{E000}</a>\n".to_owned(),
            "<p><a href=\"/about/\" title=\"\u{E001}\">\u{E000}</a></p>".to_owned(),
        ),
        (
            "`\u{E000}` and\n\n```\n\u{E000}\u{E001}\n```\n".to_owned(),
            "<p><code>\u{E000}</code> and</p>\n\
             <pre class=\"code\"><code>\u{E000}\u{E001}</code></pre>"
                .to_owned(),
        ),
        (
            "[\u{E000} *\u{E001}*](/a\u{E010}/ \"\u{E011}\") ![\u{E012}](b.jpg)\n".to_owned(),
            "<p><a href=\"/a%EE%80%90/\" title=\"\u{E011}\">\u{E000} <em>\u{E001}</em></a> \
             <img src=\"b.jpg\" alt=\"\u{E012}\" loading=\"lazy\" decoding=\"async\"></p>"
                .to_owned(),
        ),
        (
            "[\u{E000}] and [a][\u{E000}].\n\n[\u{E000}]: /a/\n".to_owned(),
            "<p><a href=\"/a/\">\u{E000}</a> and <a href=\"/a/\">a</a>.</p>".to_owned(),
        ),
        (
            "# Heading \u{E000} {#a\u{E001}}\n\n| \u{E010} |\n| - |\n| \u{E011} |\n".to_owned(),
            "<h1 id=\"a\u{E001}\">Heading \u{E000}</h1>\n<div class=\"scroll\">\n  <table>\n    \
             <thead>\n      <tr>\n        <th>\u{E010}</th>\n      </tr>\n    </thead>\n    \
             <tr>\n      <td>\u{E011}</td>\n    </tr>\n  </table>\n</div>"
                .to_owned(),
        ),
        (
            "Words.[^\u{E000}]\n\n[^\u{E000}]: A note \u{E001}.\n".to_owned(),
            "<p>Words.<sup class=\"footnote\"><a href=\"#note-1\" id=\"back-1\">1</a></sup></p>\n\
             <section class=\"footnotes\">\n  <ol>\n    <li id=\"note-1\">\n      \
             <p>A note \u{E001}. <a href=\"#back-1\" class=\"back\" \
             aria-label=\"back to the text\">\u{21a9}</a></p>\n    \
             </li>\n  </ol>\n</section>"
                .to_owned(),
        ),
        (
            "@x(\"\u{E000}\") $\u{E001}$\n\n@box()\nInside \u{E010} @url(/a/).\n@end\n".to_owned(),
            "<p><call x=prose> <math><mi>\u{E001}</mi></math></p>\n\
             <call box=alone>\n<p>Inside \u{E010} /a/.</p>\n</call>"
                .to_owned(),
        ),
        (
            format!("{all}\n\n`{all}`\n"),
            format!("<p>{all}</p>\n<p><code>{all}</code></p>"),
        ),
    ];
    for (text, html) in cases {
        let (_, document) = read_text(&text);
        assert!(document.problems.is_empty(), "{:?}", document.problems);
        assert_eq!(html_of(&text), html, "{text:?}");
    }
    let handed = arguments("@x(\"\u{E000}\", a=\u{E001}\u{E010})\n");
    let values: Vec<&Value> = handed.iter().map(|(_, value, ..)| value).collect();
    assert_eq!(
        values,
        [
            &Value::Text("\u{E000}".to_owned()),
            &Value::Text("\u{E001}\u{E010}".to_owned())
        ]
    );
}

/// The calls a text holds, and its HTML with every call written by the
/// stand-in.
fn masked(text: &str) -> (Vec<(String, Within, String)>, String) {
    let (sources, document) = read_text(text);
    (calls(&sources, &document), html_of(text))
}

#[test]
fn calls_are_found_only_where_they_are_calls() {
    type Case = (
        &'static str,
        &'static [(&'static str, Within)],
        &'static str,
    );
    let cases: [Case; 27] = [
        (
            "A @x(1) b.\n",
            &[("@x(1)", Within::Prose)],
            "<p>A <call x=prose> b.</p>",
        ),
        ("@x(1)\n", &[("@x(1)", Within::Prose)], "<call x=alone>"),
        (
            "*@x(1)*\n",
            &[("@x(1)", Within::Prose)],
            "<p><em><call x=prose></em></p>",
        ),
        (
            "name@example.org(x)\n",
            &[],
            "<p><a href=\"mailto:name@example.org\">name@example.org</a>(x)</p>",
        ),
        ("@x\n", &[], "<p>@x</p>"),
        ("@x (1)\n", &[], "<p>@x (1)</p>"),
        ("Shown: \\@x(*1*).\n", &[], "<p>Shown: @x(*1*).</p>"),
        (
            "Not escaped: \\\\@x(1).\n",
            &[],
            "<p>Not escaped: \\@x(1).</p>",
        ),
        ("`@x(1)`\n", &[], "<p><code>@x(1)</code></p>"),
        ("$@x(1)$\n", &[], "<p><math><mi>@</mi><mi>x</mi>"),
        ("[a](@x(1))\n", &[], "<p><a href=\"@x(1)\">a</a></p>"),
        (
            "![@x(1)](a.jpg)\n",
            &[],
            "<p><img src=\"a.jpg\" alt=\"@x(1)\" loading=\"lazy\" decoding=\"async\"></p>",
        ),
        (
            "<div>\n<p>@x(1)</p>\n</div>\n",
            &[("@x(1)", Within::Html)],
            "<div>\n<p><call x=html></p>\n</div>",
        ),
        (
            "<div>\n<img src=\"@x(1)\">\n</div>\n",
            &[("@x(1)", Within::Attribute)],
            "<div>\n<img src=\"<call x=attribute>\">\n</div>",
        ),
        (
            "<div>\n<style>a { b: \"@x(1)\" }</style>\n</div>\n",
            &[],
            "<style>a { b: \"@x(1)\" }</style>",
        ),
        (
            "<div>\n<script>let a = \"@x(1)\";</script>\n</div>\n",
            &[],
            "<script>let a = \"@x(1)\";</script>",
        ),
        ("<pre>\n@x(1)\n</pre>\n", &[], "<pre>\n@x(1)\n</pre>"),
        ("<pre\n>\n@x(1)\n</pre>\n", &[], "<pre\n>\n@x(1)\n</pre>"),
        (
            "<div\n>\n@x(1)\n</div>\n",
            &[("@x(1)", Within::Html)],
            "<div\n>\n<call x=html>\n</div>",
        ),
        (
            "<div>\n<code>@x(1)</code>\n</div>\n",
            &[],
            "<code>@x(1)</code>",
        ),
        (
            "<div>\n<!-- @x(1) --> <p>@y(2)</p>\n</div>\n",
            &[("@y(2)", Within::Html)],
            "<div>\n <p><call y=html></p>\n</div>",
        ),
        (
            "<div>\n<!-- @x(1) --!>\n<p>@y(2)</p>\n</div>\n",
            &[("@y(2)", Within::Html)],
            "<p><call y=html></p>",
        ),
        (
            "<div>\n<!---> <p>@y(2)</p>\n</div>\n",
            &[("@y(2)", Within::Html)],
            "<p><call y=html></p>",
        ),
        (
            "A <!--> <img src=\"@y(2)\">.\n",
            &[("@y(2)", Within::Attribute)],
            "<img src=\"<call y=attribute>\">",
        ),
        (
            "A <span>@x(1)</span>.\n",
            &[("@x(1)", Within::Prose)],
            "<p>A <span><call x=prose></span>.</p>",
        ),
        (
            "A <code>@x(1)</code>.\n",
            &[],
            "<p>A <code>@x(1)</code>.</p>",
        ),
        (
            "<div>\n<span>\\@x(1)</span>\n</div>\n",
            &[],
            "<span>@x(1)</span>",
        ),
    ];
    for (text, expected, html) in cases {
        let (found, written) = masked(text);
        let found: Vec<_> = found
            .into_iter()
            .map(|(_, within, spelling)| (spelling, within))
            .collect();
        let expected: Vec<_> = expected
            .iter()
            .map(|(s, w)| ((*s).to_owned(), *w))
            .collect();
        assert_eq!(found, expected, "{text:?}");
        assert!(written.contains(html), "{text:?} wrote\n{written}");
    }
}

/// A call that is a paragraph of its own stands alone wherever the
/// paragraph is written; one sharing its paragraph, or written straight
/// into a tight list item, doesn't.
#[test]
fn a_call_on_a_line_of_its_own_stands_alone_in_every_container() {
    let cases = [
        ("@x(1)\n", "<call x=alone>"),
        ("1. One.\n\n   @x(1)\n", "<p>One.</p>\n    <call x=alone>"),
        (
            "- One.\n\n  @x(1)\n\n  Two.\n",
            "<call x=alone>\n    <p>Two.</p>",
        ),
        ("- @x(1)\n\n- Two.\n", "<li>\n    <call x=alone>\n  </li>"),
        (
            "One\n: Two.\n\n  @x(1)\n",
            "<p>Two.</p>\n    <call x=alone>",
        ),
        ("> @x(1)\n", "<blockquote>\n  <call x=alone>\n</blockquote>"),
        ("> One.\n>\n> @x(1)\n", "<p>One.</p>\n  <call x=alone>"),
        ("> [!NOTE]\n> @x(1)\n", "<call x=alone>"),
        ("> [!NOTE]\n> One.\n>\n> @x(1)\n", "<call x=alone>"),
        ("- One.\n  @x(1)\n", "<call x=prose>"),
        ("- @x(1)\n", "<li><call x=prose></li>"),
    ];
    for (text, html) in cases {
        let written = html_of(text);
        assert!(written.contains(html), "{text:?} wrote\n{written}");
    }
}

#[test]
fn a_call_over_several_lines_keeps_every_container_whole() {
    let cases: [(&str, &str); 9] = [
        (
            "Before @card(post,\n  wide=true) after.\n",
            "<p>Before <call card=prose> after.</p>",
        ),
        (
            "> Before @card(post,\n>   wide=true) after.\n",
            "<blockquote>\n  <p>Before <call card=prose> after.</p>\n</blockquote>",
        ),
        (
            "> > Before @card(post,\n> >   wide=true) after.\n",
            "<blockquote>\n  <blockquote>\n    <p>Before <call card=prose> after.</p>\n  </blockquote>\n</blockquote>",
        ),
        (
            "- Before @card(post,\n  wide=true) after.\n- Next.\n",
            "<ul>\n  <li>Before <call card=prose> after.</li>\n  <li>Next.</li>\n</ul>",
        ),
        (
            "> - Before @card(post,\n>   wide=true) after.\n",
            "<blockquote>\n  <ul>\n    <li>Before <call card=prose> after.</li>\n  </ul>\n</blockquote>",
        ),
        (
            "> [!NOTE]\n> Before @card(post,\n>   wide=true) after.\n",
            "<aside class=\"callout note\">\n  <p>Before <call card=prose> after.</p>\n</aside>",
        ),
        (
            "A note.[^n]\n\n[^n]: Before @card(post,\n    wide=true) after.\n",
            "<p>Before <call card=prose> after. <a href=\"#back-1\"",
        ),
        (
            "Before\n@card(post,\n  wide=true)\nafter.\n",
            "<p>Before\n<call card=prose>\nafter.</p>",
        ),
        (
            "<div>\n@card(post,\n  wide=true)\n</div>\n",
            "<div>\n<call card=html>\n</div>",
        ),
    ];
    for (text, html) in cases {
        let (found, written) = masked(text);
        assert_eq!(found.len(), 1, "{text:?}");
        let (sources, document) = read_text(text);
        let node = all_nodes(&document)
            .into_iter()
            .find(|node| matches!(node.kind, Kind::Call(_)))
            .unwrap();
        let Kind::Call(card) = &node.kind else {
            unreachable!()
        };
        assert!(
            sources.spelling(node.place).starts_with("@card(post,\n"),
            "{text:?}"
        );
        assert_eq!(
            first_text(&card.named("wide").unwrap().value),
            "true",
            "{text:?}"
        );
        assert!(written.contains(html), "{text:?} wrote\n{written}");
    }
}

#[test]
fn a_code_blocks_caption_heads_it_in_a_figure() {
    let cases = [
        (
            "```rust\nlet x = 1;\n```\n",
            "<pre class=\"code\"><code class=\"language-rust\">let x = 1;</code></pre>",
        ),
        (
            "```rust A <caption> & more\nlet x = 1;\n```\n",
            "<figure class=\"code\"><figcaption>A &lt;caption&gt; &amp; more</figcaption>\
             <pre><code class=\"language-rust\">let x = 1;</code></pre></figure>",
        ),
        (
            "~~~ text \"Quoted whole\"\nas it is\n~~~\n",
            "<figure class=\"code\"><figcaption>Quoted whole</figcaption>\
             <pre><code class=\"language-text\">as it is</code></pre></figure>",
        ),
        (
            "```\"No language\"\nas it is\n```\n",
            "<figure class=\"code\"><figcaption>No language</figcaption>\
             <pre><code>as it is</code></pre></figure>",
        ),
        (
            "```css {\n.a {}\n```\n",
            "<pre class=\"code\"><code class=\"language-css\">.a {}</code></pre>",
        ),
        (
            "```\"\"\nas it is\n```\n",
            "<pre class=\"code\"><code class=\"language-&quot;&quot;\">as it is</code></pre>",
        ),
    ];
    for (text, html) in cases {
        assert_eq!(html_of(text), html, "{text:?}");
    }
}

#[test]
fn a_call_over_several_lines_in_code_keeps_the_codes_indentation() {
    let cases = [
        (
            "- An item.\n\n      @card(post,\n        wide=true)\n",
            "<pre class=\"code\"><code>@card(post,\n  wide=true)</code></pre>",
        ),
        (
            "- An item.\n\n  ```\n  @card(post,\n    wide=true)\n  ```\n",
            "<pre class=\"code\"><code>@card(post,\n  wide=true)</code></pre>",
        ),
        (
            "> ```\n> @card(post,\n>   wide=true)\n> ```\n",
            "<pre class=\"code\"><code>@card(post,\n  wide=true)</code></pre>",
        ),
        (
            "    @card(post,\n      wide=true)\n",
            "<pre class=\"code\"><code>@card(post,\n  wide=true)</code></pre>",
        ),
        (
            "A `@card(post,\n  wide=true)` span.\n",
            "<code>@card(post,   wide=true)</code>",
        ),
        (
            "<pre>\n@card(post,\n  wide=true)\n</pre>\n",
            "<pre>\n@card(post,\n  wide=true)\n</pre>",
        ),
    ];
    for (text, html) in cases {
        let (found, written) = masked(text);
        assert!(found.is_empty(), "{text:?}");
        assert!(written.contains(html), "{text:?} wrote\n{written}");
    }
}

/// The arguments of the one call in `text`: each name, value, the value's
/// spelling and style.
fn arguments(text: &str) -> Vec<(Option<String>, Value, String, Style)> {
    let (sources, document) = read_text(text);
    assert!(
        document
            .problems
            .iter()
            .all(|p| p.kind.severity() != crate::problem::Severity::Error),
        "{text:?}: {:?}",
        document.problems
    );
    let call = all_nodes(&document)
        .into_iter()
        .find_map(|node| match &node.kind {
            Kind::Call(call) => Some(call.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no call in {text:?}"));
    call.arguments
        .iter()
        .map(|a| {
            let place = a.value.origin.place.unwrap();
            (
                a.name.as_ref().map(|n| n.value.clone()),
                a.value.value.clone(),
                sources.spelling(place).to_owned(),
                a.value.origin.style,
            )
        })
        .collect()
}

#[test]
fn arguments_are_read_with_their_places() {
    use Style::{DoubleQuoted as D, Plain as P, SingleQuoted as S};
    let text = |s: &str| Value::Text(s.to_owned());
    let named = |s: &str| Some(s.to_owned());
    type Case = (
        &'static str,
        Vec<(Option<String>, Value, &'static str, Style)>,
    );
    let cases: Vec<Case> = vec![
        ("@x()", vec![]),
        ("@x( )", vec![]),
        ("@x(a.jpg)", vec![(None, text("a.jpg"), "a.jpg", P)]),
        ("@x('a.jpg')", vec![(None, text("a.jpg"), "'a.jpg'", S)]),
        ("@x(\"a, b\")", vec![(None, text("a, b"), "\"a, b\"", D)]),
        ("@x('it\\'s')", vec![(None, text("it's"), "'it\\'s'", S)]),
        ("@x(it's)", vec![(None, text("it's"), "it's", P)]),
        (
            "@x(  a  ,  b  )",
            vec![(None, text("a"), "a", P), (None, text("b"), "b", P)],
        ),
        ("@x(a,)", vec![(None, text("a"), "a", P)]),
        ("@x(800)", vec![(None, Value::Number(800.0), "800", P)]),
        ("@x(-1.5)", vec![(None, Value::Number(-1.5), "-1.5", P)]),
        ("@x(3.10)", vec![(None, Value::Number(3.1), "3.10", P)]),
        ("@x(1e3)", vec![(None, text("1e3"), "1e3", P)]),
        ("@x('800')", vec![(None, text("800"), "'800'", S)]),
        ("@x(true)", vec![(None, Value::Bool(true), "true", P)]),
        ("@x(FALSE)", vec![(None, Value::Bool(false), "FALSE", P)]),
        ("@x(yes)", vec![(None, text("yes"), "yes", P)]),
        (
            "@x(width=800)",
            vec![(named("width"), Value::Number(800.0), "800", P)],
        ),
        (
            "@x(width = 800)",
            vec![(named("width"), Value::Number(800.0), "800", P)],
        ),
        (
            "@x(alt=\"A, B\")",
            vec![(named("alt"), text("A, B"), "\"A, B\"", D)],
        ),
        ("@x(a==b)", vec![(None, text("a==b"), "a==b", P)]),
        (
            "@x(https://example.org/a)",
            vec![(
                None,
                text("https://example.org/a"),
                "https://example.org/a",
                P,
            )],
        ),
        (
            "@x(mailto:a@b.c)",
            vec![(None, text("mailto:a@b.c"), "mailto:a@b.c", P)],
        ),
        (
            "@x(poster=//example.org/a.jpg)",
            vec![(
                named("poster"),
                text("//example.org/a.jpg"),
                "//example.org/a.jpg",
                P,
            )],
        ),
        (
            "@x(a.jpg, The (old) one)",
            vec![
                (None, text("a.jpg"), "a.jpg", P),
                (None, text("The (old) one"), "The (old) one", P),
            ],
        ),
        (
            "@x('a (b, c) d')",
            vec![(None, text("a (b, c) d"), "'a (b, c) d'", S)],
        ),
        ("@x('@y(1)')", vec![(None, text("@y(1)"), "'@y(1)'", S)]),
        (
            "@x(a,\n  b)",
            vec![(None, text("a"), "a", P), (None, text("b"), "b", P)],
        ),
        (
            "@x(one\n  two)",
            vec![(None, text("one\ntwo"), "one\n  two", P)],
        ),
        (
            "> @x(a,\n> 'b')",
            vec![(None, text("a"), "a", P), (None, text("b"), "'b'", S)],
        ),
    ];
    for (text, expected) in cases {
        let found = arguments(text);
        let expected: Vec<_> = expected
            .into_iter()
            .map(|(n, v, s, style)| (n, v, s.to_owned(), style))
            .collect();
        assert_eq!(found, expected, "{text:?}");
    }
}

#[test]
fn a_name_and_its_value_are_placed_apart() {
    let (sources, document) = read_text("Text @image(caption='Post 1').\n");
    let call = all_nodes(&document)
        .into_iter()
        .find_map(|node| match &node.kind {
            Kind::Call(call) => Some(call.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(sources.spelling(call.name.origin.place.unwrap()), "image");
    let argument = &call.arguments[0];
    let name = argument.name.as_ref().unwrap();
    assert_eq!(sources.spelling(name.origin.place.unwrap()), "caption");
    assert_eq!(
        line_column(&sources, argument.value.origin.place.unwrap()),
        (1, 21)
    );
    assert_eq!(
        call.named("caption").unwrap().value,
        Value::Text("Post 1".into())
    );
    assert_eq!(call.unnamed().count(), 0);
}

#[test]
fn a_colon_names_an_argument_with_a_warning() {
    let text = "@crop('a.jpg', caption: 'Post 1', alt:\"B\")\n";
    let found = arguments(text);
    let names: Vec<_> = found.iter().map(|(n, ..)| n.clone()).collect();
    assert_eq!(names, [None, Some("caption".into()), Some("alt".into())]);
    let (sources, document) = read_text(text);
    let problems: Vec<_> = document
        .problems
        .iter()
        .map(|p| (p.headline(), sources.spelling(p.place.unwrap()).to_owned()))
        .collect();
    assert_eq!(
        problems,
        [
            (
                "Colon in call: `caption:` was read as `caption=`".to_owned(),
                "caption:".to_owned()
            ),
            (
                "Colon in call: `alt:` was read as `alt=`".to_owned(),
                "alt:".to_owned()
            ),
        ]
    );
    assert_eq!(
        document.problems[0].kind.severity(),
        crate::problem::Severity::Warning
    );
    // A call shown in code is nobody's mistake.
    let (_, document) = read_text("`@crop(caption: 'Post 1')`\n");
    assert!(document.problems.is_empty());
}

#[test]
fn malformed_calls_are_placed_at_the_exact_text() {
    let cases: [(&str, CallFault, &str); 9] = [
        (
            "A @image('a.jpg' and on.\n",
            CallFault::Unclosed("@image(".into()),
            "@image(",
        ),
        (
            "@image(a.jpg\n\nAfter.\n",
            CallFault::Unclosed("@image(".into()),
            "@image(",
        ),
        (
            "A @image(a `b` c) d.\n",
            CallFault::Unclosed("@image(".into()),
            "@image(",
        ),
        (
            "> @image(a,\n>\n> b)\n",
            CallFault::Unclosed("@image(".into()),
            "@image(",
        ),
        (
            "@image(\n",
            CallFault::Unclosed("@image(".into()),
            "@image(",
        ),
        ("@image('a.jpg)\n", CallFault::UnclosedQuote('\''), "'"),
        ("@image(\"a,\nb\")\n", CallFault::UnclosedQuote('"'), "\""),
        ("@image('a.jpg' b, c)\n", CallFault::TextAfterQuote, "b"),
        (
            "Une @café(a) ici.\n",
            CallFault::Name("café".into()),
            "@café",
        ),
    ];
    for (text, found, spelling) in cases {
        let (sources, document) = read_text(text);
        assert_eq!(
            document.problems.len(),
            1,
            "{text:?}: {:?}",
            document.problems
        );
        let problem = &document.problems[0];
        assert_eq!(
            problem.kind,
            ProblemKind::MalformedCall { found },
            "{text:?}"
        );
        assert_eq!(
            sources.spelling(problem.place.unwrap()),
            spelling,
            "{text:?}"
        );
        assert!(
            !all_nodes(&document)
                .iter()
                .any(|n| matches!(n.kind, Kind::Call(_)))
        );
    }
    for quiet in [
        "`@image('a.jpg'`\n",
        "    @image(\n",
        "```\n@image('a\n```\n",
        "<div>\n<code>@image(</code>\n</div>\n",
        "A <!-- @image( --> comment.\n",
        "Shown: \\@image( never closed.\n",
    ] {
        let (_, document) = read_text(quiet);
        assert!(
            document.problems.is_empty(),
            "{quiet:?}: {:?}",
            document.problems
        );
    }
}

#[test]
fn every_node_kind_is_placed_at_its_own_text() {
    let mut seen = std::collections::BTreeSet::new();
    for name in FIXTURES {
        for crlf in [false, true] {
            let mut text = fixture(name);
            if crlf {
                text = text.replace('\n', "\r\n");
            }
            let (sources, document) = read_text(&text);
            for top in std::iter::once(&document.root).chain(&document.notes) {
                check(&sources, top, None, &mut seen, name);
            }
        }
    }
    let every = [
        "document",
        "paragraph",
        "heading",
        "quotation",
        "callout",
        "list",
        "item",
        "definition-list",
        "term",
        "definition",
        "code-block",
        "html-block",
        "rule",
        "table",
        "table-row",
        "table-cell",
        "footnote-definition",
        "text",
        "literal",
        "code",
        "html",
        "maths",
        "soft-break",
        "hard-break",
        "emphasis",
        "strong",
        "strikethrough",
        "link",
        "image",
        "footnote-reference",
        "task",
        "call",
    ];
    let seen: Vec<_> = seen.into_iter().collect();
    let mut every = every.to_vec();
    every.sort_unstable();
    assert_eq!(seen, every);
}

fn kind_name(kind: &Kind) -> &'static str {
    match kind {
        Kind::Document => "document",
        Kind::Paragraph { .. } => "paragraph",
        Kind::Heading { .. } => "heading",
        Kind::Quotation => "quotation",
        Kind::Callout(_) => "callout",
        Kind::List { .. } => "list",
        Kind::Item => "item",
        Kind::DefinitionList => "definition-list",
        Kind::Term => "term",
        Kind::Definition => "definition",
        Kind::CodeBlock { .. } => "code-block",
        Kind::HtmlBlock => "html-block",
        Kind::Rule => "rule",
        Kind::Table { .. } => "table",
        Kind::TableRow { .. } => "table-row",
        Kind::TableCell => "table-cell",
        Kind::FootnoteDefinition { .. } => "footnote-definition",
        Kind::Text(_) => "text",
        Kind::Literal(_) => "literal",
        Kind::Code(_) => "code",
        Kind::Html(_) => "html",
        Kind::Maths { .. } => "maths",
        Kind::SoftBreak => "soft-break",
        Kind::HardBreak => "hard-break",
        Kind::Emphasis => "emphasis",
        Kind::Strong => "strong",
        Kind::Strikethrough => "strikethrough",
        Kind::Link { .. } => "link",
        Kind::Image { .. } => "image",
        Kind::FootnoteReference { .. } => "footnote-reference",
        Kind::Task { .. } => "task",
        Kind::Call(_) => "call",
    }
}

fn unescaped(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && chars.peek().is_some_and(char::is_ascii_punctuation) {
            out.push(chars.next().unwrap());
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether a node's place holds the kind of text it is made from.
fn written_as(node: &Node, s: &str) -> bool {
    let t = s.trim_start_matches([' ', '\t']);
    let starts_list =
        |t: &str| t.starts_with(['-', '*', '+']) || t.starts_with(|c: char| c.is_ascii_digit());
    match &node.kind {
        Kind::Document => true,
        Kind::Paragraph { .. } => {
            !s.is_empty()
                && !s.starts_with(char::is_whitespace)
                && node
                    .children
                    .first()
                    .is_none_or(|c| c.place.span.start == node.place.span.start)
                && node
                    .children
                    .last()
                    .is_none_or(|c| c.place.span.end == node.place.span.end)
        }
        Kind::Heading { .. } => t.starts_with('#') || s.trim_end().ends_with(['=', '-']),
        Kind::Quotation | Kind::Callout(_) => t.starts_with('>'),
        Kind::List { .. } | Kind::Item => starts_list(t),
        Kind::DefinitionList | Kind::Term | Kind::Table { .. } => !s.trim().is_empty(),
        Kind::Definition => t.starts_with(':'),
        Kind::CodeBlock { fenced: true, .. } => t.starts_with("```") || t.starts_with("~~~"),
        Kind::CodeBlock { text, .. } => s.contains(text.lines().next().unwrap_or_default()),
        Kind::HtmlBlock => t.starts_with('<'),
        Kind::Rule => s.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')),
        Kind::TableRow { .. } => !s.trim().is_empty() && !s.trim_end().contains('\n'),
        Kind::TableCell => !s.starts_with(' ') && (!s.ends_with('|') || s.ends_with("\\|")),
        Kind::FootnoteDefinition { .. } => s.starts_with("[^"),
        Kind::Text(text) => s == text || unescaped(s) == *text || s.contains('&'),
        Kind::Literal(text) => s == text || s.strip_prefix('\\') == Some(text),
        Kind::Code(_) => s.starts_with('`') && s.ends_with('`'),
        Kind::Html(html) => html.trim_end_matches(['\n', '\r']) == s.replace("\\@", "@"),
        Kind::Maths { .. } => s.starts_with('$') && s.ends_with('$'),
        Kind::SoftBreak => s.chars().all(char::is_whitespace),
        Kind::HardBreak => s.ends_with('\n') && (s.starts_with('\\') || s.starts_with("  ")),
        Kind::Emphasis => {
            s.len() > 2
                && (s.starts_with('*') && s.ends_with('*')
                    || s.starts_with('_') && s.ends_with('_'))
        }
        Kind::Strong => {
            s.starts_with("**") && s.ends_with("**") || s.starts_with("__") && s.ends_with("__")
        }
        Kind::Strikethrough => s.starts_with('~') && s.ends_with('~'),
        Kind::Link { .. } => {
            let words: String = node
                .children
                .iter()
                .filter_map(|child| match &child.kind {
                    Kind::Text(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect();
            s.starts_with('[') || s.starts_with('<') || s == words
        }
        Kind::Image { .. } => s.starts_with("!["),
        Kind::FootnoteReference { label, .. } => {
            s.to_lowercase() == format!("[^{}]", label.to_lowercase())
        }
        Kind::Task { done } => s == if *done { "[x]" } else { "[ ]" },
        Kind::Call(call) => {
            s.starts_with(&format!("@{}(", call.name.value))
                && (s.ends_with(')') || call.wraps() && s.ends_with("@end"))
        }
    }
}

fn check(
    sources: &Sources,
    node: &Node,
    parent: Option<Span>,
    seen: &mut std::collections::BTreeSet<&'static str>,
    file: &str,
) {
    seen.insert(kind_name(&node.kind));
    let span = node.place.span;
    let s = sources.spelling(node.place);
    assert!(written_as(node, s), "{file}: {:?} at {s:?}", node.kind);
    if let Some(parent) = parent {
        assert!(
            parent.start <= span.start && span.end <= parent.end,
            "{file}: {:?} at {s:?} {span:?} outside its parent {parent:?}",
            node.kind
        );
    }
    let mut last = 0;
    for child in &node.children {
        assert!(
            child.place.span.start >= last,
            "{file}: {:?} overlaps the one before",
            child.kind
        );
        last = child.place.span.end;
        let inside = (!matches!(node.kind, Kind::Document)).then_some(span);
        check(sources, child, inside, seen, file);
    }
}

#[test]
fn line_feeds_and_carriage_returns_read_alike() {
    for name in FIXTURES {
        let text = fixture(name);
        let crlf = text.replace('\n', "\r\n");
        let (sources, document) = read_text(&text);
        let (crlf_sources, crlf_document) = read_text(&crlf);
        let shape = |sources: &Sources, document: &Document| -> Vec<String> {
            all_nodes(document)
                .into_iter()
                .filter(|node| {
                    !matches!(
                        node.kind,
                        Kind::Text(_)
                            | Kind::Html(_)
                            | Kind::CodeBlock { .. }
                            | Kind::Maths { .. }
                            | Kind::Literal(_)
                    )
                })
                .map(|node| {
                    let (line, column) = line_column(sources, node.place);
                    format!("{} {line}:{column}", kind_name(&node.kind))
                })
                .collect()
        };
        assert_eq!(
            shape(&sources, &document),
            shape(&crlf_sources, &crlf_document),
            "{name}"
        );
        let calls_of = |s: &Sources, d: &Document| -> Vec<String> {
            calls(s, d).into_iter().map(|(name, ..)| name).collect()
        };
        assert_eq!(
            calls_of(&sources, &document),
            calls_of(&crlf_sources, &crlf_document)
        );
    }
}

/// The author's HTML in a list item or a quotation is indented to its
/// depth, except where the spaces would show: inside `<pre>` or
/// `<textarea>` in either case, but not inside an element whose name only
/// starts the same.
#[test]
fn html_keeps_its_lines_where_the_spaces_would_show() {
    let cases = [
        (
            "- One.\n\n  <PRE>\n  a\n    b\n  </PRE>\n",
            "    <PRE>\na\n  b\n</PRE>\n  </li>",
        ),
        (
            "> <textarea>\n> a\n>   b\n> </textarea>\n",
            "  <textarea>\na\n  b\n</textarea>\n</blockquote>",
        ),
        (
            "> <pre>\n> a <prefix>b</prefix>\n>   c\n> </pre>\n",
            "  <pre>\na <prefix>b</prefix>\n  c\n</pre>\n</blockquote>",
        ),
        (
            "- One.\n\n  <preview>\n  a\n  </preview>\n",
            "    <preview>\n    a\n    </preview>\n  </li>",
        ),
    ];
    for (text, html) in cases {
        let written = html_of(text);
        assert!(written.contains(html), "{text:?} wrote\n{written}");
    }
}

#[test]
fn documents_are_written_as_expected() {
    for name in [
        "code",
        "footnotes",
        "html",
        "links",
        "lists",
        "prose",
        "quotations",
        "tables",
        "blocks",
    ] {
        let written = html_of(&fixture(&format!("documents/{name}.md")));
        let expected = fixture(&format!("documents/{name}.html"));
        assert_eq!(written, expected, "{name}");
    }
}

/// A comment in HTML ends where a browser ends it, and one written in the
/// text of an element such as `<textarea>` is that text.
#[test]
fn a_comment_in_html_ends_where_a_browser_ends_it() {
    for opened in ["<!-- a --!>", "<!-->", "<!--->"] {
        let text = format!("<div>\n{opened}\n<p>Shown</p>\n<!-- b -->\n</div>\n");
        assert_eq!(html_of(&text), "<div>\n<p>Shown</p>\n</div>", "{opened}");
    }
    let text = "One <!-- a --!> two -->.\n";
    assert_eq!(html_of(text), "<p>One <!-- a --!> two -->.</p>");
    for name in crate::html::RAW_TEXT.iter().chain(&["script"]) {
        let raw = format!("<{name}><!-- a --></{name}>");
        let text = format!("<div>\n{raw}\n</div>\n");
        assert_eq!(html_of(&text), format!("<div>\n{raw}\n</div>"), "{name}");
    }
}

#[test]
fn footnotes_are_numbered_by_first_use_and_gathered_at_the_foot() {
    let (_, document) = read_text(&fixture("documents/footnotes.md"));
    let notes: Vec<(String, u32)> = document
        .notes
        .iter()
        .map(|note| match &note.kind {
            Kind::FootnoteDefinition { label, number } => (label.clone(), *number),
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        notes,
        [
            ("a".to_owned(), 1),
            ("b".to_owned(), 2),
            ("case".to_owned(), 3),
            ("Twice".to_owned(), 4),
        ]
    );
    let markers: Vec<(u32, u32)> = all_nodes(&document)
        .into_iter()
        .filter_map(|node| match node.kind {
            Kind::FootnoteReference { number, repeat, .. } => Some((number, repeat)),
            _ => None,
        })
        .collect();
    assert_eq!(markers, [(1, 1), (2, 1), (1, 2), (3, 1), (4, 1)]);
    assert!(
        !all_nodes(&Document {
            root: document.root.clone(),
            notes: Vec::new(),
            definitions: Vec::new(),
            in_site: Vec::new(),
            problems: Vec::new(),
            held_in_paragraph: Vec::new(),
            held_open: Vec::new(),
        })
        .iter()
        .any(|node| matches!(node.kind, Kind::FootnoteDefinition { .. }))
    );
}

#[test]
fn plain_text_is_the_words_alone() {
    let text = "# A *heading*\n\nOne `two` three\nfour <b>five</b> @x(six) $7$.[^n]\n\n- Item.\n\n```\ncode\n```\n\n![A picture](a.jpg)\n\n[^n]: A note.\n";
    let (_, document) = read_text(text);
    assert_eq!(
        plain_text(&document.root),
        "A heading\nOne two three four five  7.\nItem.\ncode"
    );
    assert_eq!(plain_text(&document.notes[0]), "A note.");
}

/// Emphasis and a link run on with the words around them, so a heading's
/// text in the contents reads as it does on the page.
#[test]
fn plain_text_runs_on_through_emphasis_and_links() {
    let text = "## *Hello* world\n\nSee [this](/a/) or **that**, *a*b.\n";
    let (_, document) = read_text(text);
    let texts: Vec<String> = headings(&document).into_iter().map(|h| h.text).collect();
    assert_eq!(texts, ["Hello world"]);
    assert_eq!(
        plain_text(&document.root),
        "Hello world\nSee this or that, ab."
    );
}

/// A script or a style in a heading shows nothing, so its text is no part
/// of the heading's id or its entry in the contents.
#[test]
fn a_heading_leaves_out_the_text_of_a_script_or_a_style() {
    let text = "## Setup <script>x()</script>\n\n## Look <style>b { c: d }</style>\n";
    let (_, mut document) = read_text(text);
    give_heading_ids(&mut document, "content/post-1.md", text);
    let found: Vec<(String, String)> = headings(&document)
        .into_iter()
        .map(|h| (h.id.unwrap().to_owned(), h.text))
        .collect();
    assert_eq!(
        found,
        [
            ("setup".to_owned(), "Setup".to_owned()),
            ("look".to_owned(), "Look".to_owned()),
        ]
    );
}

#[test]
fn links_and_headings_are_found_with_their_places() {
    let text = "# One\n\nA [link](/post-1/) and ![a picture](a.jpg).[^n]\n\n## Two `code`\n\n[^n]: A [note link](/post-2/).\n";
    let (sources, document) = read_text(text);
    let found: Vec<_> = links(&document)
        .into_iter()
        .map(|link| {
            (
                link.destination.to_owned(),
                sources.spelling(link.place).to_owned(),
                link.picture,
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("/post-1/".to_owned(), "[link](/post-1/)".to_owned(), false),
            ("a.jpg".to_owned(), "![a picture](a.jpg)".to_owned(), true),
            (
                "/post-2/".to_owned(),
                "[note link](/post-2/)".to_owned(),
                false
            ),
        ]
    );
    let found: Vec<_> = headings(&document)
        .into_iter()
        .map(|h| (h.level, h.text, sources.spelling(h.place).to_owned(), h.id))
        .collect();
    assert_eq!(
        found,
        [
            (1, "One".to_owned(), "# One".to_owned(), None),
            (2, "Two code".to_owned(), "## Two `code`".to_owned(), None),
        ]
    );
}

#[test]
fn every_heading_is_given_an_id_a_link_can_land_on() {
    let text = "# Café au *lait*\n\n## Parts\n\n## Parts\n\n## parts-1\n\n## Привет мир\n\n\
                ## a -- b_c\n\n## ???\n\n## note-1\n\nA claim.[^a]\n\n[^a]: Because.\n\n    ## Noted\n";
    let (_, mut document) = read_text(text);
    give_heading_ids(&mut document, "content/post-1.md", text);
    let ids: Vec<String> = headings(&document)
        .into_iter()
        .map(|h| h.id.unwrap().to_owned())
        .collect();
    assert_eq!(
        ids,
        [
            "cafe-au-lait",
            "parts",
            "parts-1",
            "parts-1-1",
            "привет-мир",
            "a----b_c",
            &crate::content::slug::made_slug("content/post-1.md#7"),
            "note-1-1",
        ]
    );
    let html = to_html(
        &document,
        &Options {
            snippets: &StandIn,
            back_to_the_text: None,
            digits: None,
            right_to_left: false,
            highlighter: None,
        },
    );
    assert!(html.starts_with("<h1 id=\"cafe-au-lait\">Café au <em>lait</em></h1>"));
    assert!(html.contains("<li id=\"note-1\">"), "{html}");
    assert!(html.contains("<h2 id=\"noted\">Noted</h2>"), "{html}");
}

/// An id the author gives in braces is kept as written, even where another
/// heading has it too; an id made from the words steps aside for it, and
/// braces holding anything but an id and classes are words.
#[test]
fn braces_ending_a_heading_give_its_id_and_classes() {
    let text = "## Parts\n\n## Version 1.0 notes {#version-1.0-notes}\n\n## Other {#parts .wide .note}\n\n\
                ## Again {#parts}\n\nSetext {.plain}\n------\n\n## Set {a, b}\n\n## Bare {word}\n\n\
                ## Escaped \\{#kept}\n\n## Empty {}\n\n## Mixed {#x key=value}\n\n\
                - ## In a list {#listed}\n\n> ## Quoted {not this}\n\nA paragraph {#not-a-heading}\n\n\
                ```\n## code {x}\n```\n";
    let (_, mut document) = read_text(text);
    give_heading_ids(&mut document, "content/post-1.md", text);
    let found: Vec<(String, String)> = headings(&document)
        .into_iter()
        .map(|h| (h.id.unwrap().to_owned(), h.text))
        .collect();
    let expected = [
        ("parts-1", "Parts"),
        ("version-1.0-notes", "Version 1.0 notes"),
        ("parts", "Other"),
        ("parts", "Again"),
        ("setext", "Setext"),
        ("set-a-b", "Set {a, b}"),
        ("bare-word", "Bare {word}"),
        ("escaped-kept", "Escaped {#kept}"),
        ("empty-", "Empty {}"),
        ("mixed-x-keyvalue", "Mixed {#x key=value}"),
        ("listed", "In a list"),
        ("quoted-not-this", "Quoted {not this}"),
    ];
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(id, text)| ((*id).to_owned(), (*text).to_owned()))
        .collect();
    assert_eq!(found, expected);
    let html = to_html(
        &document,
        &Options {
            snippets: &StandIn,
            back_to_the_text: None,
            digits: None,
            right_to_left: false,
            highlighter: None,
        },
    );
    for written in [
        "<h2 id=\"parts\" class=\"wide note\">Other</h2>",
        "<h2 id=\"setext\" class=\"plain\">Setext</h2>",
        "<p>A paragraph {#not-a-heading}</p>",
        "<code>## code {x}</code>",
    ] {
        assert!(html.contains(written), "{written} in {html}");
    }
}

/// The id, classes and rows of each table in the writing, its own or
/// one inside a quotation or a list.
fn tables(text: &str) -> Vec<(Option<String>, Vec<String>, usize)> {
    let (_, document) = read_text(text);
    assert!(document.problems.is_empty(), "{:?}", document.problems);
    let mut found = Vec::new();
    document.root.walk(&mut |node| {
        if let Kind::Table { id, classes, .. } = &node.kind {
            found.push((id.clone(), classes.clone(), node.children.len()));
        }
    });
    found
}

/// A line of braces under a table's last row gives the table an id and
/// classes, as braces ending a heading do, wherever the table stands.
#[test]
fn braces_under_a_tables_last_row_give_its_id_and_classes() {
    let under = |line: &str| format!("| a | b |\n|---|---|\n| 1 | 2 |\n{line}\n");
    let given = |id: Option<&str>, classes: &[&str]| {
        let classes = classes.iter().map(|class| (*class).to_owned()).collect();
        vec![(id.map(str::to_owned), classes, 2)]
    };
    for (line, id, classes) in [
        ("{.wide}", None, &["wide"][..]),
        ("{#x}", Some("x"), &[][..]),
        ("{.wide #x .b}", Some("x"), &["wide", "b"][..]),
        ("  {#x}\t", Some("x"), &[][..]),
        ("{#x #y}", Some("y"), &[][..]),
    ] {
        assert_eq!(tables(&under(line)), given(id, classes), "{line}");
        let crlf = under(line).replace('\n', "\r\n");
        assert_eq!(tables(&crlf), given(id, classes), "{line}");
    }
    let quoted = "> | a | b |\n> |---|---|\n> | 1 | 2 |\n> {#x}\n";
    let listed = "- An item:\n\n  | a | b |\n  |---|---|\n  | 1 | 2 |\n  {#x}\n";
    let wrapped = "@box()\n| a | b |\n|---|---|\n| 1 | 2 |\n{#x}\n@end\n";
    for text in [quoted, listed, wrapped] {
        assert_eq!(tables(text), given(Some("x"), &[]), "{text}");
    }
    assert_eq!(
        tables("| a | b |\n|---|---|\n{.wide}\n"),
        [(None, vec!["wide".to_owned()], 1)]
    );
}

/// Any other line under a table is the row GFM reads it as, and nothing
/// is told: braces holding anything but an id and classes, braces with a
/// row after them, braces between bars and braces escaped.
#[test]
fn braces_that_give_a_table_nothing_are_a_row() {
    for (line, rows) in [
        ("{foo}", 3),
        ("{.a b}", 3),
        ("{}", 3),
        ("{#}", 3),
        ("{.x}\n| 3 | 4 |", 4),
        ("| {.x} |", 3),
        ("{.x} |", 3),
        ("\\{.x}", 3),
        ("{.a\\.b}", 3),
        ("{.a`b`}", 3),
        ("{.a<b>}", 3),
        ("{.a $b$}", 3),
        ("{.a @x()}", 3),
        ("{.a} {.b}", 3),
    ] {
        let text = format!("| a | b |\n|---|---|\n| 1 | 2 |\n{line}\n");
        assert_eq!(tables(&text), [(None, vec![], rows)], "{line}");
    }
    assert_eq!(
        html_of("| a |\n|---|\n| 1 |\n\\{.x}\n"),
        "<div class=\"scroll\">\n  <table>\n    <thead>\n      <tr>\n        <th>a</th>\n      \
         </tr>\n    </thead>\n    <tr>\n      <td>1</td>\n    </tr>\n    <tr>\n      \
         <td>{.x}</td>\n    </tr>\n  </table>\n</div>"
    );
}

/// A table's id and classes are written on it, escaped, the id first.
#[test]
fn a_tables_id_and_classes_are_written_on_it() {
    assert_eq!(
        html_of("| a |\n|:-:|\n| 1 |\n{.b\"c #x .d}\n"),
        "<div class=\"scroll\">\n  <table id=\"x\" class=\"b&quot;c d\">\n    <thead>\n      \
         <tr>\n        <th style=\"text-align: center\">a</th>\n      </tr>\n    </thead>\n    \
         <tr>\n      <td style=\"text-align: center\">1</td>\n    </tr>\n  </table>\n</div>"
    );
    assert_eq!(
        html_of("| a |\n|---|\n{.wide}\n"),
        "<div class=\"scroll\">\n  <table class=\"wide\">\n    <thead>\n      <tr>\n        \
         <th>a</th>\n      </tr>\n    </thead>\n  </table>\n</div>"
    );
}

/// A written id something above already has is told at the second, the
/// first heading shown; an id made from words steps aside instead.
#[test]
fn an_id_written_twice_or_a_footnotes_is_told() {
    let text = "## One {#route}\n\n## Two {#route}\n\n## Route\n\n\
                ## Three {#note-1}\n\nA claim.[^a]\n\n[^a]: Because.\n";
    let (sources, mut document) = read_text(text);
    give_heading_ids(&mut document, "content/post-1.md", text);
    let told: Vec<(String, String, Option<String>)> = document
        .problems
        .iter()
        .map(|problem| {
            let first = match &problem.kind {
                ProblemKind::IdUsedTwice {
                    taken: crate::problem::Taken::Heading(first),
                    ..
                } => Some(sources.spelling(*first).to_owned()),
                _ => None,
            };
            (
                problem.headline(),
                sources.spelling(problem.place.unwrap()).to_owned(),
                first,
            )
        })
        .collect();
    assert_eq!(
        told,
        [
            (
                "Id used twice: a heading above has the id `route`".to_owned(),
                "#route".to_owned(),
                Some("## One {#route}".to_owned())
            ),
            (
                "Id used twice: a footnote has the id `note-1`".to_owned(),
                "#note-1".to_owned(),
                None
            ),
        ]
    );
    let ids: Vec<String> = headings(&document)
        .into_iter()
        .map(|h| h.id.unwrap().to_owned())
        .collect();
    assert_eq!(ids, ["route", "route", "route-1", "note-1"]);
}

/// A table's id takes part as a heading's does: told where something above
/// has it, and stepped aside from by an id made from a heading's words. A
/// table holding it first is shown by its braces.
#[test]
fn a_tables_id_written_twice_is_told() {
    let text = "## A {#x}\n\n| a |\n|---|\n| 1 |\n{#x}\n\n| b |\n|---|\n| 2 |\n{#y}\n\n\
                ## Y\n\n## B {#y}\n\n| c |\n|---|\n| 3 |\n{#note-1}\n\nA claim.[^a]\n\n\
                [^a]: Because.\n";
    let (sources, mut document) = read_text(text);
    give_heading_ids(&mut document, "content/post-1.md", text);
    let told: Vec<(String, Option<String>, String, Option<String>)> = document
        .problems
        .iter()
        .map(|problem| {
            let first = match &problem.kind {
                ProblemKind::IdUsedTwice {
                    taken:
                        crate::problem::Taken::Heading(first) | crate::problem::Taken::Table(first),
                    ..
                } => Some(sources.spelling(*first).to_owned()),
                _ => None,
            };
            (
                problem.headline(),
                problem.suggestion(),
                sources.spelling(problem.place.unwrap()).to_owned(),
                first,
            )
        })
        .collect();
    let line = |headline: &str, suggestion: &str, at: &str, first: Option<&str>| {
        (
            headline.to_owned(),
            Some(suggestion.to_owned()),
            at.to_owned(),
            first.map(str::to_owned),
        )
    };
    assert_eq!(
        told,
        [
            line(
                "Id used twice: a heading above has the id `x`",
                "Give this table an id of its own.",
                "#x",
                Some("## A {#x}")
            ),
            line(
                "Id used twice: a table above has the id `y`",
                "Give this heading an id of its own.",
                "#y",
                Some("#y")
            ),
            line(
                "Id used twice: a footnote has the id `note-1`",
                "Give this table an id of its own.",
                "#note-1",
                None
            ),
        ]
    );
    let ids: Vec<String> = headings(&document)
        .into_iter()
        .map(|h| h.id.unwrap().to_owned())
        .collect();
    assert_eq!(ids, ["x", "y-1", "y"]);
}

/// A note nothing points at is shown nowhere, and told at its label, with
/// a marker that has no note offered where its label is near; counted
/// across the whole page, the writing a call wraps included.
#[test]
fn a_footnote_nothing_points_at_is_told() {
    let text = "Figures.[^source] Twice.[^twice] Inside the box, a marker.[^boxed]\n\n\
                @box()\n[^boxed]: Written inside, pointed at outside.\n\n[^lost]: Inside, unused.\n@end\n\n\
                [^sources]: The yearly report.\n\n[^twice]: The first, which a later one replaces.\n\n\
                [^twice]: The second.\n";
    let (sources, document) = read_text(text);
    let told: Vec<(String, String, Option<String>)> = document
        .problems
        .iter()
        .map(|problem| {
            (
                problem.headline(),
                sources.spelling(problem.place.unwrap()).to_owned(),
                problem.suggestion(),
            )
        })
        .collect();
    let line = |headline: &str, at: &str, suggestion: Option<&str>| {
        (
            headline.to_owned(),
            at.to_owned(),
            suggestion.map(str::to_owned),
        )
    };
    assert_eq!(
        told,
        [
            line(
                "Missing definition: there is no `[^source]:` line",
                "[^source]",
                Some("Did you mean `[^sources]`?")
            ),
            line(
                "Footnote never shown: nothing points at `[^lost]`",
                "[^lost]",
                None
            ),
            line(
                "Footnote never shown: nothing points at `[^sources]`",
                "[^sources]",
                Some("Is it the note for `[^source]`?")
            ),
            line(
                "Definition written twice: the first `[^twice]:` line isn't shown",
                "[^twice]",
                Some("Give one of them another name, or take one out.")
            ),
        ]
    );
}

/// Of notes with one label, whatever its case, each but the last is told
/// at its `[^label]`, pointing at the last, which is the one shown: with a
/// marker pointing at the label or none.
#[test]
fn a_footnote_written_twice_is_told_at_the_one_not_shown() {
    let told = |text: &str| -> Vec<(String, String, Option<String>)> {
        let (sources, document) = read_text(text);
        (document.problems.iter())
            .map(|problem| {
                let mut shown = problem.related().into_iter().map(|related| {
                    assert_eq!(related.label, "Shown instead");
                    let place = related.place.unwrap();
                    let line = line_column(&sources, place).0;
                    format!("{} on line {line}", sources.spelling(place))
                });
                (
                    problem.headline(),
                    sources.spelling(problem.place.unwrap()).to_owned(),
                    shown.next(),
                )
            })
            .collect()
    };
    let twice = |label: &str, at: &str, shown: &str| {
        (
            format!("Definition written twice: the first `[^{label}]:` line isn't shown"),
            at.to_owned(),
            Some(shown.to_owned()),
        )
    };
    assert_eq!(
        told("A marker.[^n]\n\n[^n]: The first.\n\n[^n]: The second.\n"),
        [twice("n", "[^n]", "[^n] on line 5")]
    );
    assert_eq!(
        told("A marker.[^n]\n\n[^n]: One.\n[^n]: Two.\n[^N]: Three.\n"),
        [
            twice("n", "[^n]", "[^N] on line 5"),
            twice("n", "[^n]", "[^N] on line 5"),
        ]
    );
    assert_eq!(
        told("No marker.\n\n[^Note]: The first.\n\n[^note]: The second.\n"),
        [
            twice("Note", "[^Note]", "[^note] on line 5"),
            (
                "Footnote never shown: nothing points at `[^note]`".to_owned(),
                "[^note]".to_owned(),
                None
            ),
        ]
    );
}

#[test]
fn front_matter_and_a_byte_order_mark_leave_places_in_the_whole_file() {
    let text = "\u{feff}---\ntitle: Post 1\n---\nA @x(1).\n";
    let (sources, document) = read_text(text);
    let (name, _, spelling) = &calls(&sources, &document)[0];
    assert_eq!((name.as_str(), spelling.as_str()), ("x", "@x(1)"));
    let node = all_nodes(&document)
        .into_iter()
        .find(|node| matches!(node.kind, Kind::Call(_)))
        .unwrap();
    assert_eq!(line_column(&sources, node.place), (4, 3));
}

#[test]
fn columns_after_multibyte_writing_count_characters() {
    let text = fixture("spans.md");
    let (sources, document) = read_text(&text);
    let node = all_nodes(&document)
        .into_iter()
        .find(|node| sources.spelling(node.place) == "@image('after-multibyte.jpg')")
        .unwrap();
    assert_eq!(line_column(&sources, node.place), (6, 25));
}

#[test]
fn addresses_are_written_as_an_attribute_carries_them() {
    let html = html_of("[a](/café/?a=1&b='2'#x) ![b](<a b.jpg>)\n");
    assert_eq!(
        html,
        "<p><a href=\"/caf%C3%A9/?a=1&amp;b=&#x27;2&#x27;#x\">a</a> \
         <img src=\"a%20b.jpg\" alt=\"b\" loading=\"lazy\" decoding=\"async\"></p>"
    );
}

#[test]
fn the_writing_a_call_wraps_is_placed_where_it_is_written() {
    let text =
        "> @box(caption=\"a\")\n> A @x(1) inside, [a link](/post-1/).\n>\n> ## Inside\n> @end\n";
    let (sources, document) = read_text(text);
    assert!(document.problems.is_empty(), "{:?}", document.problems);
    let found = calls(&sources, &document);
    assert_eq!(
        found,
        [
            (
                "box".into(),
                Within::Prose,
                text[2..text.len() - 1].to_owned()
            ),
            ("x".into(), Within::Prose, "@x(1)".into()),
        ]
    );
    let call = all_nodes(&document)
        .into_iter()
        .find_map(|node| match &node.kind {
            Kind::Call(call) if call.wraps() => Some(call),
            _ => None,
        })
        .unwrap();
    let head = Place {
        source: call.name.origin.place.unwrap().source,
        span: call.head.unwrap(),
    };
    assert_eq!(sources.spelling(head), "@box(caption=\"a\")");
    let caption = call.named("caption").unwrap().origin.place.unwrap();
    assert_eq!(sources.spelling(caption), "\"a\"");
    let inner = all_nodes(&document)
        .into_iter()
        .find(|node| sources.spelling(node.place) == "@x(1)")
        .unwrap();
    assert_eq!(line_column(&sources, inner.place), (2, 5));
    let link = &links(&document)[0];
    assert_eq!(sources.spelling(link.place), "[a link](/post-1/)");
    let heading = &headings(&document)[0];
    assert_eq!(sources.spelling(heading.place), "## Inside");
}

/// Each reference with no definition, its spelling, and the label offered.
fn undefined(sources: &Sources, document: &Document) -> Vec<(String, String, Option<String>)> {
    document
        .problems
        .iter()
        .map(|problem| match &problem.kind {
            ProblemKind::MissingDefinition { written, nearest } => (
                written.clone(),
                sources.spelling(problem.place.unwrap()).to_owned(),
                nearest.clone(),
            ),
            other => panic!("{other:?}"),
        })
        .collect()
}

#[test]
fn references_with_no_definition_are_told_where_they_are() {
    let text = "A marker.[^nope] An index x[i][j], [plain] brackets and \\[^esc].\n\
                A [link][labl], a [collapsed][] one, ![a picture][pic], `[^code]` and \
                <code>[raw][code] [^raw]</code>.\n\n\
                [label]: /a/\n\n[^note]: A note, pointed at.[^note]\n";
    let (sources, document) = read_text(text);
    let some = |s: &str| Some(s.to_owned());
    assert_eq!(
        undefined(&sources, &document),
        [
            ("[^nope]".into(), "[^nope]".into(), some("[^note]")),
            ("[labl]".into(), "[link][labl]".into(), some("[label]")),
            ("[collapsed]".into(), "[collapsed][]".into(), None),
            ("[pic]".into(), "![a picture][pic]".into(), None),
        ]
    );
    let html = html_of(text);
    assert!(
        html.contains("A marker.[^nope] An index x[i][j], [plain]"),
        "{html}"
    );
}

/// Writing a call wraps is read apart from the page around it, and still
/// finds the definitions written there, as the page finds those inside.
#[test]
fn definitions_are_the_pages_wherever_they_are_written() {
    let text = "Outside, [a link][inner] and a marker.[^inner]\n\n\
                @box()\nInside, [a link][outer], a marker[^outer] and [nothing][none].\n\n\
                [inner]: /inner/\n\n[^inner]: The note written inside.\n@end\n\n\
                [outer]: /outer/\n\n[^outer]: The note written outside.\n";
    let (sources, document) = read_text(text);
    assert_eq!(
        undefined(&sources, &document),
        [("[none]".into(), "[nothing][none]".into(), None)]
    );
    let defined: Vec<(&str, &str, &str)> = document
        .definitions
        .iter()
        .map(|d| {
            (
                d.label.as_str(),
                d.destination.as_str(),
                sources.spelling(d.place),
            )
        })
        .collect();
    assert_eq!(
        defined,
        [
            ("inner", "/inner/", "[inner]: /inner/"),
            ("outer", "/outer/", "[outer]: /outer/"),
        ]
    );
    let html = html_of(text);
    for written in [
        "<p>Outside, <a href=\"/inner/\">a link</a> and a marker.<sup class=\"footnote\"><a href=\"#note-1\" id=\"back-1\">1</a></sup></p>",
        "<p>Inside, <a href=\"/outer/\">a link</a>, a marker<sup class=\"footnote\"><a href=\"#note-2\" id=\"back-2\">2</a></sup> and [nothing][none].</p>",
        "<li id=\"note-1\">\n      <p>The note written inside.",
        "<li id=\"note-2\">\n      <p>The note written outside.",
    ] {
        assert!(html.contains(written), "{written} in {html}");
    }
}

#[test]
fn a_call_that_wraps_writing_is_told_where_its_end_is_missing() {
    let told = |text: &str| -> Vec<(CallFault, String)> {
        let (sources, document) = read_text(text);
        document
            .problems
            .iter()
            .map(|p| match &p.kind {
                ProblemKind::MalformedCall { found } => {
                    (found.clone(), sources.spelling(p.place.unwrap()).to_owned())
                }
                other => panic!("{other:?}"),
            })
            .collect()
    };
    assert_eq!(
        told("Text.\n\n@box('a')\nText.\n"),
        [(CallFault::NoEnd("box".into()), "@box('a')".into())]
    );
    assert_eq!(
        told("Text.\n\n> @end\n"),
        [(CallFault::StrayEnd, "@end".into())]
    );
    // The inner call is told where it is, inside the writing it is in.
    assert_eq!(
        told("> @box()\n> A @x( never closed.\n> @end\n"),
        [(CallFault::Unclosed("@x(".into()), "@x(".into())]
    );
    for quiet in [
        // Told when the page is built: no snippet of that name, and a call
        // that wraps writing in the middle of a sentence.
        "@nothing()\nText.\n@end\n",
        "Text @box() and more.\n@end\n",
        // Code.
        "`@end`\n\n```\n@box()\n```\n\n    @end\n",
        "```\n@box()\n```\n\nText.\n\n@box()\nText.\n@end\n",
        r"\@end",
        // Shown as text, its `@end` with it.
        "\\@box()\nText.\n@end\n",
    ] {
        assert_eq!(told(quiet), [], "{quiet:?}");
    }
}

#[test]
fn calls_nested_past_the_deepest_wrap_nothing() {
    let depth = 200;
    let text = "@box()\n".repeat(depth) + &"@end\n".repeat(depth);
    let (_, document) = read_text(&text);
    let mut deepest = 0;
    let mut node = &document.root;
    while let Some(child) = node.children.first() {
        if matches!(&child.kind, Kind::Call(call) if call.wraps()) {
            deepest += 1;
        }
        node = child;
    }
    assert_eq!(deepest, block::DEEPEST);
}

fn written(text: &str) -> Written {
    let (_, document) = read_text(text);
    write(
        &document,
        &Options {
            snippets: &StandIn,
            back_to_the_text: Some("back to the text"),
            digits: None,
            right_to_left: false,
            highlighter: None,
        },
    )
}

/// The excerpt is the writing before the first more marker on a line of
/// its own at the top of the writing, with its markup. One inside a
/// quotation, a list or code, or among words, or spelled otherwise, is no
/// marker.
#[test]
fn the_excerpt_ends_at_a_more_marker_on_its_own() {
    let excerpt = |text: &str| written(text).excerpt;
    assert_eq!(
        excerpt("One *two*.\n\nThree.\n\n<!--more-->\n\nFour.\n\n<!--more-->\n\nFive.\n")
            .as_deref(),
        Some("<p>One <em>two</em>.</p>\n<p>Three.</p>")
    );
    assert_eq!(
        excerpt("One.\nTwo.\n<!-- more -->\nThree.\n").as_deref(),
        Some("<p>One.\nTwo.</p>")
    );
    let after_html = written("One.\n\n<div>\nTwo.\n</div>\n<!--more-->\n\nThree.\n");
    assert_eq!(
        after_html.excerpt.as_deref(),
        Some("<p>One.</p>\n<div>\nTwo.\n</div>")
    );
    assert!(after_html.excerpt_marked);
    for not_a_marker in [
        "One.\n\n> Two.\n>\n> <!--more-->\n\nThree.\n",
        "One.\n\n```\n<!--more-->\n```\n\nThree.\n",
        "One.\n\n- Two.\n\n  <!--more-->\n\nThree.\n",
        "One.\n\nTwo <!--more--> three.\n",
        "One.\n\n<!-- More -->\n\nThree.\n",
        "One.\n\n<!--more--> Two.\n\nThree.\n",
        "<div class=\"box\">\n\nOne.\n\n<!--more-->\n\nTwo.\n\n</div>\n",
        "<div class=\"box\">\n\nOne.\n\n<div>\n<!--more-->\n\nTwo.\n\n</div></div>\n",
    ] {
        let written = written(not_a_marker);
        assert_eq!(
            written.excerpt.as_deref(),
            Some("<p>One.</p>"),
            "{not_a_marker:?}"
        );
        assert!(!written.excerpt_marked, "{not_a_marker:?}");
    }
    let at_the_top = written("<!--more-->\n\nAll of it.\n");
    assert_eq!(at_the_top.excerpt, None);
    assert!(at_the_top.excerpt_marked);
    assert!(written("One.\n\n<!--more-->\n\nTwo.\n").excerpt_marked);
    // Outside every element the author's HTML holds open, a marker ends
    // the excerpt as anywhere else.
    for (text, excerpt) in [
        (
            "<div class=\"box\">One.</div>\n\n<!--more-->\n\nTwo.\n",
            "<div class=\"box\">One.</div>",
        ),
        (
            "<div class=\"box\">\n\nOne.\n\n</div>\n\n<!--more-->\n\nTwo.\n",
            "<div class=\"box\">\n<p>One.</p>\n</div>",
        ),
        (
            "One <span class=\"box\"> two.\n\n<!--more-->\n\nThree.\n",
            "<p>One <span class=\"box\"> two.</p>",
        ),
        ("<p>One.\n\n<!--more-->\n\nTwo.\n", "<p>One."),
    ] {
        let written = written(text);
        assert_eq!(written.excerpt.as_deref(), Some(excerpt), "{text:?}");
        assert!(written.excerpt_marked, "{text:?}");
    }
    let in_code = written("<div class=\"box\">\n\nOne.\n\n```\n<!--more-->\n```\n\n</div>\n");
    assert_eq!(in_code.excerpt.as_deref(), Some("<p>One.</p>"));
    assert!(!in_code.excerpt_marked);
}

/// A more marker Statac can't act on stops, at the marker: spelled in
/// another case, or standing where it can't end an excerpt. One after the
/// marker that ends the excerpt is warned of. Code shows one, and another
/// tool's marker is a comment.
#[test]
fn a_more_marker_statac_cant_act_on_is_told_at_the_marker() {
    let told = |text: &str| -> Vec<(String, String)> {
        let (sources, document) = read_text(text);
        document
            .problems
            .iter()
            .map(|p| (p.headline(), sources.spelling(p.place.unwrap()).to_owned()))
            .collect()
    };
    let cases = [
        (
            "One.\n\n<!-- More -->\n\nTwo.\n",
            "Unknown marker: `<!-- More -->`",
            "<!-- More -->",
        ),
        (
            "- One.\n- <!--MORE-->\n",
            "Unknown marker: `<!--MORE-->`",
            "<!--MORE-->",
        ),
        (
            "- One.\n\n  <!--more-->\n\n- Two.\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside a list",
            "<!--more-->",
        ),
        (
            "- One.\n- <!--more-->\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside a list",
            "<!--more-->",
        ),
        (
            "> One.\n>\n> <!--more-->\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside a quotation",
            "<!--more-->",
        ),
        (
            "@box()\nOne.\n\n<!--more-->\n\nTwo.\n@end\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside `@box`",
            "<!--more-->",
        ),
        (
            "One.[^a]\n\n[^a]: A.\n\n    <!--more-->\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside a footnote",
            "<!--more-->",
        ),
        (
            "<div>\n<!--more-->\n</div>\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside HTML",
            "<!--more-->",
        ),
        (
            "<div class=\"box\">\n\nOne.\n\n<!--more-->\n\nTwo.\n\n</div>\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside `<div>`",
            "<!--more-->",
        ),
        (
            "<DIV>\n\nOne.\n\n<!--more-->\n\nTwo.\n\n</DIV>\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside `<DIV>`",
            "<!--more-->",
        ),
        (
            "<section>\n<div>\nOne.\n<!--more-->\n\nTwo.\n\n</div>\n</section>\n",
            "Wrong place: `<!--more-->` can't end an excerpt inside `<div>`",
            "<!--more-->",
        ),
        (
            "One <!-- more --> two.\n",
            "Wrong place: `<!-- more -->` ends an excerpt only on a line of its own",
            "<!-- more -->",
        ),
        (
            "<!--more--> One.\n",
            "Wrong place: `<!--more-->` ends an excerpt only on a line of its own",
            "<!--more-->",
        ),
    ];
    for (text, headline, spelling) in cases {
        assert_eq!(
            told(text),
            [(headline.to_owned(), spelling.to_owned())],
            "{text:?}"
        );
    }

    let text = "One.\n\n<!-- more -->\n\nTwo.\n\n<!--more-->\n\nThree.\n";
    let (sources, document) = read_text(text);
    assert_eq!(document.problems.len(), 1);
    let again = &document.problems[0];
    assert_eq!(
        again.headline(),
        "More written twice: the excerpt ends at the first `<!-- more -->`"
    );
    assert_eq!(again.kind.severity(), crate::problem::Severity::Warning);
    assert_eq!(sources.spelling(again.place.unwrap()), "<!--more-->");
    let related = again.related();
    assert_eq!(related[0].label, "First written");
    assert_eq!(sources.spelling(related[0].place.unwrap()), "<!-- more -->");

    for quiet in [
        "One.\n\n<!--more-->\n\nTwo.\n",
        "`<!--MORE-->`\n\n```\n<!--more-->\n<!--more-->\n```\n",
        "<!--break-->\n\nOne <!--break--> two.\n",
        "<div>\nOne.\n</div>\n<!--more-->\n\nTwo.\n",
        "<div class=\"box\">One.</div>\n\n<!--more-->\n\nTwo.\n",
        "<div class=\"box\">\n\nOne.\n\n</div>\n\n<!--more-->\n\nTwo.\n",
        "One <span class=\"box\"> two.\n\n<!--more-->\n\nThree.\n",
        "<p>One.\n\n<!--more-->\n\nTwo.\n",
        "<div class=\"box\">\n\nOne.\n\n```\n<!--more-->\n```\n\n</div>\n",
    ] {
        assert_eq!(told(quiet), [], "{quiet:?}");
    }
}

/// With no marker, the excerpt is the first paragraph holding words: not a
/// picture alone, not the writing a call wraps, not a tight list's items.
/// The author's own `<p>` stands in where there is no paragraph of
/// Markdown at all.
#[test]
fn the_excerpt_and_the_opening_are_the_first_paragraph_with_words() {
    let cases = [
        (
            "## Title\n\nThe first *paragraph*.\n\nThe second.\n",
            "<p>The first <em>paragraph</em>.</p>",
            "The first paragraph.",
        ),
        ("![A picture](a.jpg)\n\nWords.\n", "<p>Words.</p>", "Words."),
        (
            "@box()\nInside.\n@end\n\nWords.\n",
            "<p>Words.</p>",
            "Words.",
        ),
        (
            "> Quoted\n> words.\n",
            "<p>Quoted\nwords.</p>",
            "Quoted words.",
        ),
        ("- Tight.\n- List.\n\nWords.\n", "<p>Words.</p>", "Words."),
        (
            "<pre>code</pre>\n\n<p></p>\n\n<p class=\"lede\">Up<br>the &mdash; page.</p>\n\n<p>More.</p>\n",
            "<p>Up<br>the &mdash; page.</p>",
            "Up the \u{2014} page.",
        ),
        (
            "<p>Written in HTML.</p>\n\nWritten in Markdown.\n",
            "<p>Written in Markdown.</p>",
            "Written in Markdown.",
        ),
    ];
    for (text, excerpt, opening) in cases {
        let written = written(text);
        assert_eq!(written.excerpt.as_deref(), Some(excerpt), "{text:?}");
        assert_eq!(written.opening.as_deref(), Some(opening), "{text:?}");
        assert!(!written.excerpt_marked);
    }
    let none = written("- Only.\n- Items.\n\n```\ncode\n```\n");
    assert_eq!((none.excerpt, none.opening), (None, None));
}

/// Neither the excerpt nor the opening holds a marker pointing at a note,
/// whether it ends a sentence, stands between two words or sits in a
/// heading; the page keeps every one.
#[test]
fn the_excerpt_and_the_opening_leave_the_notes_markers_out() {
    let marked = written(
        "# Up[^a]\n\nThe page [^a] is long.[^b]\n\nDown.[^a]\n\n<!--more-->\n\n\
         More.[^b]\n\n[^a]: A.\n\n[^b]: B.\n",
    );
    assert_eq!(
        marked.excerpt.as_deref(),
        Some("<h1>Up</h1>\n<p>The page is long.</p>\n<p>Down.</p>")
    );
    assert_eq!(marked.opening.as_deref(), Some("The page is long."));
    assert_eq!(marked.html.matches("<sup class=\"footnote\">").count(), 5);
    let opening = written("Up the page.[^a]\n\nDown.\n\n[^a]: A.\n");
    assert_eq!(opening.excerpt.as_deref(), Some("<p>Up the page.</p>"));
    assert_eq!(opening.words, 5);
}

/// What the author wrote stays, even where it is the very HTML of a
/// footnote's marker.
#[test]
fn an_authors_html_like_a_marker_is_kept_in_the_excerpt() {
    let html = "<sup class=\"footnote\"><a href=\"#note-1\" id=\"back-1\">1</a></sup>";
    let written = written(&format!("Up {html} the page.[^a]\n\n[^a]: A.\n"));
    assert_eq!(
        written.excerpt.as_deref(),
        Some(format!("<p>Up {html} the page.</p>").as_str())
    );
}

/// Words are counted as a reader reads them: code blocks, the notes'
/// markers and their ways back are no words, nor is a lone dash; in
/// Chinese and Japanese each character is one.
#[test]
fn words_are_counted_as_a_reader_reads_them() {
    let count = |text: &str| written(text).words;
    assert_eq!(count("Up *and* over & back - don't\n"), 5);
    assert_eq!(
        count("One.\n\n```\nnot these\n```\n\n    nor these\n\nTwo.\n"),
        2
    );
    assert_eq!(count("One[^a] two.\n\n[^a]: Three.\n"), 3);
    assert_eq!(count("<pre>not</pre>\n\n<preview>kept</preview>\n"), 1);
    assert_eq!(count("\u{5c71}\u{3092}\u{767b}\u{308b} and\n"), 5);
}

/// The author's HTML is read as a browser reads it: a `>` inside a quoted
/// attribute ends no tag, nor does one inside a comment, `<pre` inside an
/// attribute opens no code, and a `<` that opens no tag is text.
#[test]
fn the_words_and_the_opening_skip_tags_as_a_browser_does() {
    let cases = [
        ("One <abbr title=\"a > b c d e\">two</abbr> three.\n", 3),
        ("One <abbr title='a > b c d e'>two</abbr> three.\n", 3),
        ("One <abbr title=abcde>two</abbr> three.\n", 3),
        ("One <abbr title=\"<pre>\">two</abbr> three.\n", 3),
    ];
    for (text, words) in cases {
        let written = written(text);
        assert_eq!(written.words, words, "{text:?}");
        assert_eq!(
            written.opening.as_deref(),
            Some("One two three."),
            "{text:?}"
        );
    }
    let cases = [
        (
            "<p title=\"a > b\">One <!-- c > d --> two.</p>\n",
            "One two.",
        ),
        (
            "<div title=\"<p>c d</p>\"><p>One two.</p></div>\n",
            "One two.",
        ),
        ("<p>One < two.</p>\n", "One < two."),
    ];
    for (html, opening) in cases {
        let written = as_written(html);
        assert_eq!(written.words, 2, "{html:?}");
        assert_eq!(written.opening.as_deref(), Some(opening), "{html:?}");
    }
}

/// What a browser doesn't show is no words, in the count or the opening:
/// a script, a style, a title in the writing and what stands in for
/// what the browser can do. A text field shows its text.
#[test]
fn the_words_and_the_opening_leave_out_what_a_browser_does_not_show() {
    let cases = [
        (
            "One <script>let a = b;</script>two three.\n",
            "One two three.",
        ),
        (
            "One <style>p { color: red }</style>two three.\n",
            "One two three.",
        ),
        ("One <title>a b</title>two three.\n", "One two three."),
        ("One <noscript>a b</noscript>two three.\n", "One two three."),
        ("One <iframe>a b</iframe>two three.\n", "One two three."),
        ("One <textarea>two</textarea> three.\n", "One two three."),
        (
            "<script>\nlet a = b;\n</script>\n\nOne two three.\n",
            "One two three.",
        ),
    ];
    for (text, opening) in cases {
        let written = written(text);
        assert_eq!(written.words, 3, "{text:?}");
        assert_eq!(written.opening.as_deref(), Some(opening), "{text:?}");
    }
    let html = "<style>p { color: red }</style><p>One <script>a b</script>two.</p>\n";
    let written = as_written(html);
    assert_eq!(written.words, 2);
    assert_eq!(written.opening.as_deref(), Some("One two."));
}

#[test]
fn the_words_of_html_are_parted_where_a_reader_sees_them_apart() {
    assert_eq!(words_of("<p>a</p><p>b</p>"), "a b");
    assert_eq!(words_of("a<br>b <em>c</em>d"), "a b cd");
    assert_eq!(words_of("<p>a</p>\n<p>b</p>"), "a\nb");
    assert_eq!(
        words_of("cr&ecirc;pe &amp; caf&#233; &mdash; &notaname; &#x41;"),
        "cr\u{ea}pe & caf\u{e9} \u{2014} &notaname; A"
    );
    // A number is read as a browser reads it.
    assert_eq!(
        words_of("&#0; &#xD800; &#x110000; &#99999999999; &#150; &#x81; &#; &#12a;"),
        "\u{fffd} \u{fffd} \u{fffd} \u{fffd} \u{2013} \u{81} &#; &#12a;"
    );
}

#[test]
fn the_lines_of_html_break_where_a_block_ends_and_nowhere_else() {
    for (html, lines) in [
        ("<p>a</p><p>b</p>", "a\nb"),
        ("<p>a</p>\n\n<p>b</p>\n", "a\nb"),
        ("<p>one\ntwo  three</p>", "one two three"),
        ("a<br>b <em>c</em>d", "a\nb cd"),
        (
            "<ul>\n<li>one</li>\n<li>two\n<ul><li>in</li></ul></li>\n</ul>",
            "one\ntwo\nin",
        ),
        ("<table><tr><td>a</td><td>b</td></tr></table>", "a\nb"),
        ("<p>a <script>x</script>b</p>", "a b"),
        ("<p>a\u{2029}b</p>", "a\nb"),
        ("", ""),
    ] {
        assert_eq!(lines_of(html), lines, "{html:?}");
    }
}

/// A page written in HTML has its excerpt and opening read out of its
/// markup as written.
#[test]
fn a_page_written_in_html_has_an_excerpt_too() {
    let marked = as_written("<p>One.</p>\n<!-- more -->\n<p>Two.</p>\n");
    assert_eq!(marked.excerpt.as_deref(), Some("<p>One.</p>"));
    assert!(marked.excerpt_marked);
    assert_eq!(marked.opening.as_deref(), Some("One."));
    assert_eq!(marked.words, 2);
    let unmarked = as_written("<div><p class=\"a\">One &amp; two.</p></div>\n");
    assert_eq!(unmarked.excerpt.as_deref(), Some("<p>One &amp; two.</p>"));
    assert_eq!(unmarked.opening.as_deref(), Some("One & two."));
}

/// Each problem's headline and where it is placed, by its spelling.
fn told_at(text: &str) -> Vec<(String, String)> {
    let (sources, document) = read_text(text);
    document
        .problems
        .iter()
        .map(|problem| {
            let at = sources.spelling(problem.place.unwrap()).to_owned();
            (problem.headline(), at)
        })
        .collect()
}

/// Another tool's calls and tags, with and without space inside, are told
/// at the exact text wherever they stand among the words: in a paragraph,
/// a list item, a quotation, a heading, two on one line, and split by the
/// parser into words and HTML.
#[test]
fn another_tools_call_among_the_words_is_told_where_written() {
    let call = |written: &str| {
        (
            format!("Published as text: `{written}` is another tool's call"),
            written.to_owned(),
        )
    };
    let text = "{{< figure src=\"photo-1.jpg\" >}}\n\n\
                - An item {{%note%}}.\n\n\
                > A quotation {% include note.html %}.\n\n\
                ## A {{<figure>}} heading {{% note %}}\n\n\
                Two {%raw%} and {{< b >}} on one line.\n\n\
                {{ <youtube id=\"abcdefghijk\" /> }} and {{\t<ui.button_1 label=\"Name 1\"/>}}\n";
    let told = told_at(text);
    assert_eq!(
        told,
        [
            call("{{< figure src=\"photo-1.jpg\" >}}"),
            call("{{%note%}}"),
            call("{% include note.html %}"),
            call("{{<figure>}}"),
            call("{{% note %}}"),
            call("{%raw%}"),
            call("{{< b >}}"),
            call("{{ <youtube id=\"abcdefghijk\" /> }}"),
            call("{{\t<ui.button_1 label=\"Name 1\"/>}}"),
        ]
    );
    assert_eq!(told_at(&text.replace('\n', "\r\n")), told);
    assert!(
        html_of(text).contains("<h2>A {{<figure>}} heading {{% note %}}</h2>"),
        "{}",
        html_of(text)
    );
}

/// A line holding nothing but another tool's mark is told at the mark: on
/// its own, under a paragraph's last line, in a quotation, with and
/// without space inside. Among other words on its line it is words.
#[test]
fn another_tools_mark_on_a_line_of_its_own_is_told_where_written() {
    let mark = |written: &str| {
        (
            format!("Published as text: `{written}` is another tool's mark"),
            written.to_owned(),
        )
    };
    let text = "{:toc}\n\nA paragraph.\n{: .box}\n\n> A quotation.\n> {:.box}\n\n\
                Words {: .box} and more.\n";
    let told = told_at(text);
    assert_eq!(told, [mark("{:toc}"), mark("{: .box}"), mark("{:.box}")]);
    assert_eq!(told_at(&text.replace('\n', "\r\n")), told);
}

/// Another tool's call or mark written as code, in raw HTML's own code or
/// a comment, or after a backslash, is never told; nor are doubled braces
/// with neither bracket nor percent sign, or with a bracket opening no
/// tag, Statac's own call, or braces closing a heading.
#[test]
fn another_tools_call_kept_as_code_or_escaped_is_never_told() {
    let text = "A code span: `{{< figure >}}` and `{: .box}`.\n\n\
                ```\n{{< figure >}}\n{% raw %}\n{: .box}\n```\n\n    {{% note %}}\n\n\
                <pre>\n{{< figure >}}\n</pre>\n\n\
                A comment <!-- {{< figure >}} --> and <code>{% raw %}</code>.\n\n\
                <!--\n{: .box}\n-->\n\n\
                Escaped: \\{{< figure >}}, \\{% raw %} and\n\\{: .box}\n\n\
                Braces alone: {{ title }} and {{ page.title | upcase }}.\n\n\
                Not a tag: {{ <3 }}, {{ < name }} and {{ a < b }}.\n\n\
                Statac's own: @figure(src=\"photo-1.jpg\") and \\@figure(src=\"{{< x >}}\").\n\n\
                ## A heading {#id}\n\n## A heading {.class}\n\n## A heading {% raw %}\n";
    for text in [text.to_owned(), text.replace('\n', "\r\n")] {
        let (_, document) = read_text(&text);
        let told: Vec<_> = (document.problems.iter())
            .filter(|problem| problem.kind.code() == crate::problem::Code::PublishedAsText)
            .map(|problem| problem.headline())
            .collect();
        assert!(told.is_empty(), "{told:?}");
    }
}

/// A row's cells past the table's columns are told at what they hold,
/// without the bars at their ends, wherever the table stands; the parser
/// parts the cells, so an escaped bar parts none and a bar in a code span
/// parts it as anywhere in a row. Empty cells past the columns, and a short
/// row, lose nothing and are never told.
#[test]
fn cells_past_a_tables_columns_are_told_at_what_they_hold() {
    let cells = |rows: &str| told_at(&format!("| A | B |\n|---|---|\n{rows}"));
    let one = |cell: &str, columns: &str| {
        (
            format!("Cell never shown: `{cell}` is past the table's {columns}"),
            cell.to_owned(),
        )
    };
    assert_eq!(cells("| 1 | 2 | Extra |\n"), [one("Extra", "2 columns")]);
    assert_eq!(cells("| 1 | 2 | Extra\n"), [one("Extra", "2 columns")]);
    assert_eq!(
        cells("1 | 2 |   Extra   |  \n"),
        [one("Extra", "2 columns")]
    );
    assert_eq!(
        cells("| 1 | 2 | Extra | More |\n"),
        [(
            "Cells never shown: 2 cells are past the table's 2 columns".to_owned(),
            "Extra | More".to_owned()
        )]
    );
    assert_eq!(cells("| 1 | 2 | | Extra |\n"), [one("Extra", "2 columns")]);
    assert_eq!(cells("| 1 \\| 2 | `\\|` \\| @x(a=\"|\") $a|b$ |\n"), []);
    assert_eq!(cells("| 1 | 2 | \\| |\n"), [one("\\|", "2 columns")]);
    assert_eq!(cells("| 1 | `2|3` |\n"), [one("3`", "2 columns")]);
    assert_eq!(cells("| 1 | 2 | |\n| 1 | 2 | | |\n| 1 |\n| 1\n"), []);
    assert_eq!(
        told_at("| A |\n|---|\n| 1 | Extra |\n"),
        [one("Extra", "1 column")]
    );
    let quoted = "> | A | B |\n> |---|---|\n> | 1 | 2 | Extra |\n";
    assert_eq!(told_at(quoted), [one("Extra", "2 columns")]);
    let listed = "- An item:\n\n  | A | B |\n  |---|---|\n  | 1 | 2 | Extra |\n";
    assert_eq!(told_at(listed), [one("Extra", "2 columns")]);
    let wrapped = "@box()\n| A | B |\n|---|---|\n| @x() | $y$ | Extra |\n@end\n";
    assert_eq!(told_at(wrapped), [one("Extra", "2 columns")]);
    let crlf = "| A | B |\r\n|---|---|\r\n| 1 | 2 | Extra |\r\n";
    assert_eq!(told_at(crlf), [one("Extra", "2 columns")]);
}

/// The fixture's row with a cell the table has no column for is told, and
/// the table is written as it always was.
#[test]
fn the_tables_fixtures_extra_cell_is_told() {
    assert_eq!(
        told_at(&fixture("documents/tables.md")),
        [(
            "Cell never shown: `Extra` is past the table's 2 columns".to_owned(),
            "Extra".to_owned()
        )]
    );
}

/// The writing of a site published under `/blog`, read and as HTML.
fn under_a_path(text: &str) -> (Sources, Document, String) {
    let mut sources = Sources::default();
    let id = sources.add("content/post-1.md", text.to_owned());
    let document = read(&sources, id, Span::new(0, text.len()), &wraps, "/blog");
    let options = Options {
        snippets: &StandIn,
        back_to_the_text: None,
        digits: None,
        right_to_left: false,
        highlighter: None,
    };
    let html = to_html(&document, &options);
    (sources, document, html)
}

#[test]
fn an_address_inside_the_site_is_under_the_sites_path() {
    let cases = [
        (
            "[About](@url(/about/))\n",
            "<p><a href=\"/blog/about/\">About</a></p>",
        ),
        (
            "[About](@url(\"/about/\") \"Title\") [About](@url('/about/'))\n",
            "<p><a href=\"/blog/about/\" title=\"Title\">About</a> \
             <a href=\"/blog/about/\">About</a></p>",
        ),
        (
            "[Team](@url(/about/?a=1&b=2#team))\n",
            "<p><a href=\"/blog/about/?a=1&amp;b=2#team\">Team</a></p>",
        ),
        (
            "![Alt](@url(/photo-1.jpg))\n",
            "<p><img src=\"/blog/photo-1.jpg\" alt=\"Alt\" loading=\"lazy\" decoding=\"async\"></p>",
        ),
        (
            "[About][about] and [about].\n\n[about]: @url(/about/)\n",
            "<p><a href=\"/blog/about/\">About</a> and <a href=\"/blog/about/\">about</a>.</p>",
        ),
        (
            "[Outside][about]\n\n@box()\n[Inside][about]\n\n[about]: @url(/about/)\n@end\n",
            "<p><a href=\"/blog/about/\">Outside</a></p>\n<call box=alone>\n\
             <p><a href=\"/blog/about/\">Inside</a></p>\n</call>",
        ),
        (
            "A <a href=\"@url(/about/)\">link</a> in HTML.\n",
            "<p>A <a href=\"/blog/about/\">link</a> in HTML.</p>",
        ),
        (
            "<nav>\n  <a href=\"@url(/caf\u{e9}/)\">@url(/a&b/)</a>\n</nav>\n",
            "<nav>\n  <a href=\"/blog/caf%C3%A9/\">/blog/a&amp;b/</a>\n</nav>",
        ),
        (
            "It is at @url(/about/), as text.\n",
            "<p>It is at /blog/about/, as text.</p>",
        ),
        ("@url(/about/)\n", "<p>/blog/about/</p>"),
    ];
    for (text, html) in cases {
        let (_, document, written) = under_a_path(text);
        assert_eq!(written, html, "{text:?}");
        assert!(document.problems.is_empty(), "{:?}", document.problems);
    }
    assert_eq!(
        html_of("[About](@url(/about/)) <a href=\"@url(/about/)\">@url(/about/)</a>\n"),
        "<p><a href=\"/about/\">About</a> <a href=\"/about/\">/about/</a></p>"
    );
}

#[test]
fn an_address_inside_the_site_is_text_where_a_call_is() {
    let cases = [
        (
            "Shown: \\@url(/about/) and `@url(/about/)`.\n",
            "<p>Shown: @url(/about/) and <code>@url(/about/)</code>.</p>",
        ),
        (
            "```\n@url(/about/)\n```\n",
            "<pre class=\"code\"><code>@url(/about/)</code></pre>",
        ),
        (
            "<pre>@url(/about/)</pre>\n\n<script>go(\"@url(/about/)\")</script>\n",
            "<pre>@url(/about/)</pre>\n<script>go(\"@url(/about/)\")</script>",
        ),
        (
            "<style>a { b: \"@url(/about/)\" }</style>\n",
            "<style>a { b: \"@url(/about/)\" }</style>",
        ),
        (
            "A <code>@url(/about/)</code> <!-- @url(/about/) --> word.\n",
            "<p>A <code>@url(/about/)</code> word.</p>",
        ),
        (
            "![At @url(/about/)](photo-1.jpg)\n",
            "<p><img src=\"photo-1.jpg\" alt=\"At @url(/about/)\" loading=\"lazy\" decoding=\"async\"></p>",
        ),
    ];
    for (text, html) in cases {
        let (_, document, written) = under_a_path(text);
        assert_eq!(written, html, "{text:?}");
        assert!(document.in_site.is_empty(), "{text:?}");
        assert!(document.problems.is_empty(), "{:?}", document.problems);
    }
}

#[test]
fn an_address_after_a_comment_is_under_the_sites_path_however_the_comment_ends() {
    let cases = [
        (
            "<div>\n<!-- x --!>\n<a href=\"@url(/about/)\">about</a>\n</div>\n",
            "<a href=\"/blog/about/\">about</a>",
        ),
        (
            "<div>\n<!---> <a href=\"@url(/about/)\">about</a>\n</div>\n",
            "<a href=\"/blog/about/\">about</a>",
        ),
        (
            "Text <!--> then <a href=\"@url(/about/)\">about</a> and \"quotes\".\n",
            "<p>Text then <a href=\"/blog/about/\">about</a> and &quot;quotes&quot;.</p>",
        ),
    ];
    for (text, html) in cases {
        let (_, document, written) = under_a_path(text);
        assert!(written.contains(html), "{text:?} wrote\n{written}");
        assert_eq!(document.in_site.len(), 1, "{text:?}");
        assert!(document.problems.is_empty(), "{:?}", document.problems);
        // Words shown as typed are left out of typography.
        let as_typed = all_nodes(&document)
            .into_iter()
            .any(|n| matches!(n.kind, Kind::Literal(_)));
        assert!(!as_typed, "{text:?}");
    }
}

#[test]
fn an_address_after_a_comment_opened_in_text_is_under_the_sites_path() {
    let cases = [
        "<div>\n<textarea><!-- </textarea>\n<a href=\"@url(/about/)\">about</a>\n</div>\n",
        "<div>\n<title><!-- </title>\n<a href=\"@url(/about/)\">about</a>\n</div>\n",
        "<div>\n<style><!-- </style>\n<a href=\"@url(/about/)\">about</a>\n</div>\n",
        "<div>\n<TEXTAREA><!-- </TextArea\n><a href=\"@url(/about/)\">about</a>\n</div>\n",
    ];
    for text in cases {
        let (_, document, written) = under_a_path(text);
        assert!(
            written.contains("<a href=\"/blog/about/\">about</a>"),
            "{text:?} wrote\n{written}"
        );
        assert_eq!(document.in_site.len(), 1, "{text:?}");
        assert!(document.problems.is_empty(), "{:?}", document.problems);
    }
}

#[test]
fn a_comment_after_an_element_holding_text_is_a_comment() {
    let text = "<div>\n<textarea>a</textarea><!-- @url(/a/) </textarea>\n\
                <a href=\"@url(/b/)\">b</a> -->\n<a href=\"@url(/about/)\">about</a>\n</div>\n";
    let (_, document, written) = under_a_path(text);
    assert!(
        written.contains("<a href=\"/blog/about/\">about</a>"),
        "wrote\n{written}"
    );
    assert!(!written.contains("/b/"), "wrote\n{written}");
    assert_eq!(document.in_site.len(), 1);
    assert!(document.problems.is_empty(), "{:?}", document.problems);
}

#[test]
fn each_address_inside_the_site_is_kept_where_it_is_written() {
    let text = "[About](@url(/about/)) ![Alt](@url('/photo-1.jpg')) @url(/a/) [b][b] [b]\n\n\
                <a href=\"@url(/c/)\">c</a>\n\n@box()\n[d](@url(/d/))\n@end\n\n[b]: @url(/b/)\n";
    let (sources, document, _) = under_a_path(text);
    let kept: Vec<(&str, &str)> = document
        .in_site
        .iter()
        .map(|stands| (stands.address.as_str(), sources.spelling(stands.place)))
        .collect();
    assert_eq!(
        kept,
        [
            ("/blog/about/", "@url(/about/)"),
            ("/blog/photo-1.jpg", "@url('/photo-1.jpg')"),
            ("/blog/a/", "@url(/a/)"),
            ("/blog/c/", "@url(/c/)"),
            ("/blog/d/", "@url(/d/)"),
            ("/blog/b/", "@url(/b/)"),
        ]
    );
    assert!(
        !all_nodes(&document)
            .iter()
            .any(|node| matches!(&node.kind, Kind::Call(call) if call.name.value == "url"))
    );
}

#[test]
fn an_address_inside_the_site_handed_anything_else_is_told() {
    let cases = [
        ("See @url(about/).\n", "@url(about/)"),
        ("See @url().\n", "@url()"),
        ("See @url(/a/, /b/).\n", "@url(/a/, /b/)"),
        ("See @url(to=/about/).\n", "@url(to=/about/)"),
        ("See @url(//example.org/).\n", "@url(//example.org/)"),
        ("See @url(/\\example.org/).\n", "@url(/\\example.org/)"),
        (
            "See @url(https://example.org/).\n",
            "@url(https://example.org/)",
        ),
        ("See @url(3).\n", "@url(3)"),
        ("[About](@url(about/))\n", "@url(about/)"),
        ("![Alt](@url(photo-1.jpg))\n", "@url(photo-1.jpg)"),
        ("[a][a] [a]\n\n[a]: @url(about/)\n", "@url(about/)"),
        ("<a href=\"@url(about/)\">a</a>\n", "@url(about/)"),
    ];
    for (text, spelling) in cases {
        let (sources, document, html) = under_a_path(text);
        let told: Vec<(&ProblemKind, &str)> = document
            .problems
            .iter()
            .map(|problem| (&problem.kind, sources.spelling(problem.place.unwrap())))
            .collect();
        let found = CallFault::NotOneAddress;
        assert_eq!(
            told,
            [(&ProblemKind::MalformedCall { found }, spelling)],
            "{text:?}"
        );
        assert!(document.in_site.is_empty(), "{text:?}");
        assert!(html.contains(spelling), "{spelling} in {html}");
    }
    for quiet in ["`@url(about/)`\n", "Shown: \\@url(about/).\n"] {
        let (_, document, _) = under_a_path(quiet);
        assert!(document.problems.is_empty(), "{quiet:?}");
    }
    // It wraps no writing, so an `@end` below it closes nothing.
    let (sources, document, _) = under_a_path("@url(/about/)\n\n@end\n");
    let told: Vec<(&ProblemKind, &str)> = document
        .problems
        .iter()
        .map(|problem| (&problem.kind, sources.spelling(problem.place.unwrap())))
        .collect();
    let found = CallFault::StrayEnd;
    assert_eq!(told, [(&ProblemKind::MalformedCall { found }, "@end")]);
}

/// A line of 200,000 characters, a `$` or an `@` every ten, once took
/// seconds to read: each had its line's bounds found afresh.
#[test]
fn one_long_line_thick_with_dollars_and_ats_is_read_in_good_time() {
    let began = std::time::Instant::now();
    for unit in ["$a$ x y z ", "@x(1) y z ", "$ a @ b $ "] {
        read_text(&unit.repeat(20_000));
    }
    assert!(
        began.elapsed() < std::time::Duration::from_secs(10),
        "read in {:?}",
        began.elapsed()
    );
}

#[test]
fn a_call_wrapping_writing_is_a_level_of_nesting() {
    let text = |quotations: usize| {
        format!(
            "{}{}words\n{}",
            "@box()\n".repeat(3),
            "> ".repeat(quotations),
            "@end\n".repeat(3)
        )
    };
    let (_, document) = read_text(&text(197));
    assert!(document.problems.is_empty(), "{:?}", document.problems);
    let (sources, document) = read_text(&text(198));
    let [problem] = document.problems.as_slice() else {
        panic!("{:?}", document.problems);
    };
    assert_eq!(problem.kind, ProblemKind::NestedTooDeep);
    let place = problem.place.expect("a place");
    assert_eq!(
        sources.line_number(place.source, place.span.start as usize),
        1
    );
}

/// Each tag of raw HTML told left unmatched: what it leaves so, the tag
/// as written where it is placed, and its line.
fn unmatched_at(text: &str) -> Vec<(Unmatched, String, usize)> {
    let (sources, document) = read_text(text);
    (document.problems.into_iter())
        .filter_map(|problem| {
            let ProblemKind::UnmatchedTag { unmatched } = problem.kind else {
                return None;
            };
            let place = problem.place.unwrap();
            let line = sources.line_number(place.source, place.span.start as usize);
            Some((unmatched, sources.spelling(place).to_owned(), line))
        })
        .collect()
}

/// Raw HTML is followed through the elements it opens and closes, as a
/// browser builds the page round it, and a tag that reaches past the
/// writing is told at the tag, on its first line: an element or a comment
/// left open, or an end tag closing nothing the writing opened. What a
/// browser closes without harm is never told, nor is what is no markup.
#[test]
fn html_that_reaches_past_the_writing_is_told_at_its_tag() {
    let open =
        |name: &str, at: &str, line| (Unmatched::NeverClosed(name.into()), at.to_owned(), line);
    let stray = |name: &str, line| {
        let at = format!("</{name}>");
        (Unmatched::ClosesNothing(name.into()), at, line)
    };
    let comment = |line| (Unmatched::CommentNeverClosed, "<!--".to_owned(), line);
    let cases = [
        // An element left open, a stray end tag, and an element left open
        // that a browser closes as its container closes.
        (
            "A paragraph.\n\n<div class=\"footnotes\"><h4>Notes</h4>\n\n1. A note.\n",
            vec![open("div", "<div class=\"footnotes\">", 3)],
        ),
        ("One.\n\nTwo.\n\n</div>\n\nThree.\n", vec![stray("div", 5)]),
        (
            "<TABLE CLASS=\"box\">\n<TR><TD>1</TD><TD>2</TD></TR>\n\
             <TR><TD>3</TD><TD>4</TD></TR>\n\nAfter.\n",
            vec![open("TABLE", "<TABLE CLASS=\"box\">", 1)],
        ),
        (
            "<figure>\n<div class=\"table-wrapper\">\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n\
             <figcaption>A table.</figcaption>\n</figure>\n",
            vec![],
        ),
        // Names in any case.
        ("<DIV>\nOne.\n</div>\n", vec![]),
        ("<Section>\nOne.\n", vec![open("Section", "<Section>", 1)]),
        // Elements that hold nothing, and those a browser closes by itself.
        (
            "A picture: <img src=\"photo-1.jpg\"><br>\n\n<hr>\n<input type=\"checkbox\">\n",
            vec![],
        ),
        (
            "<ul>\n<li>One\n<li>Two\n</ul>\n\n<table>\n<tr><td>1\n<tr><td>2\n</table>\n\n\
             <dl><dt>Term<dd>Definition</dl>\n\n<p>Words.\n",
            vec![],
        ),
        ("One.</li> Two.</td> Three.</tr> Four.</dd>\n", vec![]),
        ("One.</p>\n\nTwo.</br> Three.</img>\n", vec![]),
        // An end tag closes what is opened inside its element.
        (
            "<section>\n<div>\n<span>A note in a box.\n</section>\n",
            vec![],
        ),
        ("<pre>\n<span>One\n</pre>\n", vec![]),
        // But not past a table open inside it.
        (
            "<div><table><tr><td>1</div>\n",
            vec![open("div", "<div>", 1), open("table", "<table>", 1)],
        ),
        // A formatting element closed so is opened again round what follows
        // until its own end tag; a later link closes a link.
        ("<div><b>Bold\n</div>\n", vec![open("b", "<b>", 1)]),
        ("<div><b>Bold\n</div>\n\nMore.</b>\n", vec![]),
        (
            "<p><a href=\"/one/\">One</p>\n<p><a href=\"/two/\">Two</a></p>\n",
            vec![],
        ),
        ("<table><tr><td><b>Bold</td></tr></table>\n", vec![]),
        (
            "<em>A caption.\n\nThe next paragraph.\n",
            vec![open("em", "<em>", 1)],
        ),
        // Markdown's blocks close what is opened inside them, and an end tag
        // after still matches; a block element in a paragraph closes it.
        (
            "A sentence <span>with words.\n\n- An item <span>in a list.\n- Another.\n\n\
             > A quotation <span>quoted.\n\n| A |\n|---|\n| <span>1 |\n\n## A <span>heading\n",
            vec![],
        ),
        ("<intro>The opening.\n\nMore.\n\n</intro>\n", vec![]),
        ("Words <div>in a box.\n", vec![open("div", "<div>", 1)]),
        // A comment left open, comments holding tags, and one that closes
        // as it opens.
        ("One.\n\n<!-- To come.\n\nTwo.\n", vec![comment(3)]),
        ("<!-- <div> </span> -->\n\nOne.\n", vec![]),
        ("<!-->\n\n<div>One.</div>\n", vec![]),
        // A comment or text element left open takes in the end tags after
        // it, so is told alone.
        ("<div>\n<!-- </div>\n", vec![comment(2)]),
        (
            "<div>\n<script>\nlet a = 1;\n",
            vec![open("script", "<script>", 2)],
        ),
        (
            "A <title> in the words.\n",
            vec![open("title", "<title>", 1)],
        ),
        // What a text element holds is no markup.
        (
            "<script>if (a<b) { f('</div>'); }</script>\n\n<style>p > a { }</style>\n\n\
             <textarea><div></textarea>\n",
            vec![],
        ),
        // A tag over several lines, told on its first.
        (
            "<div\n  class=\"box\">\nOne.\n",
            vec![open("div", "<div", 1)],
        ),
        ("<div>\nOne.\n</div\n>\n", vec![]),
        // `/>` closes what it opens in SVG, and nowhere else.
        (
            "<svg viewBox=\"0 0 1 1\"><path d=\"M0 0\"/><circle r=\"1\"/></svg>\n",
            vec![],
        ),
        ("<div/>\nOne.\n", vec![open("div", "<div/>", 1)]),
        // Code is no markup, nor is a picture's description.
        (
            "`<div>` and `</span>`\n\n```html\n<div>\n```\n\n    <section>\n",
            vec![],
        ),
        ("![A <span>picture](photo-1.jpg)\n", vec![]),
        // The writing a call wraps, and a footnote's note, are each matched
        // on their own.
        ("@box()\n<div>\n@end\n", vec![open("div", "<div>", 2)]),
        ("<div>\n\n@box()\nWords.\n@end\n\n</div>\n", vec![]),
        (
            "<div>\n\n@box()\n</div>\n@end\n",
            vec![open("div", "<div>", 1), stray("div", 4)],
        ),
        (
            "Words.[^1]\n\n[^1]: <div>A note.\n\n</div>\n",
            vec![open("div", "<div>", 3), stray("div", 5)],
        ),
    ];
    for (text, told) in cases {
        let mut wanted = told.clone();
        wanted.sort_by_key(|(_, _, line)| *line);
        assert_eq!(unmatched_at(text), wanted, "{text:?}");
        let crlf = text.replace('\n', "\r\n");
        assert_eq!(unmatched_at(&crlf), wanted, "{crlf:?}");
    }
}

/// Writes code as a site's own file would: `reads` says whether the file
/// can read `body`, and `stops` that it stops, leaving the block to be
/// written without it. Keeps each `body` it was handed.
struct OwnCode {
    reads: bool,
    stops: bool,
    handed: std::cell::RefCell<Vec<String>>,
}

impl Snippets for OwnCode {
    fn html(&self, _: &Call, _: Place, _: Placement, _: Option<&str>) -> String {
        String::new()
    }

    fn writes(&self, piece: Piece) -> bool {
        piece == Piece::Code
    }

    fn reads(&self, _: Piece, _: &str) -> bool {
        self.reads
    }

    fn piece(&self, _: Piece, values: Vec<(&'static str, PieceValue)>, _: Place) -> Option<String> {
        for (name, value) in values {
            if let ("body", PieceValue::Html(body)) = (name, value) {
                self.handed.borrow_mut().push(body);
            }
        }
        (!self.stops).then(|| "<pre>own</pre>".to_owned())
    }
}

#[test]
fn code_is_coloured_for_a_file_of_the_sites_only_where_the_file_can_read_it() {
    let highlighter = Highlighter::new("ocean").unwrap();
    let coloured = highlighter.html("yaml", "a: 1");
    assert!(coloured.contains("<span"), "{coloured}");
    let written =
        format!("<pre class=\"code\"><code class=\"language-yaml\">{coloured}</code></pre>");
    let cases = [
        (true, false, coloured.as_ref(), "<pre>own</pre>"),
        (false, false, "", "<pre>own</pre>"),
        (true, true, coloured.as_ref(), written.as_str()),
        (false, true, "", written.as_str()),
    ];
    for (reads, stops, handed, html) in cases {
        let snippets = OwnCode {
            reads,
            stops,
            handed: std::cell::RefCell::default(),
        };
        let (_, document) = read_text("```yaml\na: 1\n```\n");
        let built = to_html(
            &document,
            &Options {
                snippets: &snippets,
                back_to_the_text: None,
                digits: None,
                right_to_left: false,
                highlighter: Some(&highlighter),
            },
        );
        assert_eq!(built, html, "reads {reads}, stops {stops}");
        assert_eq!(
            *snippets.handed.borrow(),
            [handed],
            "reads {reads}, stops {stops}"
        );
    }
}
