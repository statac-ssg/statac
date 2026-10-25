use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;

const BASE: &str = "<title>{% block title %}{% endblock %}</title>\n\
                    {% block main %}{% endblock %}\n{% block foot %}{% endblock %}\n";

fn site() -> TempSite {
    TempSite::site(&[
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/\n\
             fields.book.author: text\nfields.book.year: number\n",
        ),
        ("templates/base.html", BASE),
        (
            "templates/page.html",
            "{% extends \"base\" %}{% block main %}{{ page.content }}\
             {% for post in site.posts %}{{ card(post) }}{% endfor %}{% endblock %}\n",
        ),
        (
            "snippets/figure.html",
            "{# takes photo, alt #}\n{% set shown = page.image(photo) %}\
             {{ shown.alt }}{{ alt if alt is defined }}{{ caption | default('') }}\n",
        ),
        ("snippets/card.html", "{# takes post #}\n{{ post.title }}\n"),
        (
            "snippets/markdown/heading.html",
            "<h{{ level }} id=\"{{ id }}\">{{ body }}</h{{ level }}>\n",
        ),
        ("assets/site.css", "p { margin: 0 }\n"),
        ("assets/menu.js", "let opened = false;\n"),
        (
            "data/people.yaml",
            "person-1:\n  name: Person 1\n  links: [a, b]\n",
        ),
        ("content/about.md", "---\ntitle: About\n---\n## The team\n"),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-02\n---\nA post.\n",
        ),
    ])
}

type Offered = Vec<(String, String, String)>;

/// What is offered at the cursor, written as the last `|`, in `text` as the text of
/// the site's file at `path`, once the site is read: each as its label,
/// what is said beside it and what is written.
fn offered(site: &TempSite, path: &str, text: &str) -> Option<Offered> {
    let mut checking = Checking::new();
    let options = Options {
        mode: Mode::Check,
        site: site.root.clone(),
        out: None,
        today: None,
        drafts: false,
        url: None,
    };
    checking.check(&options);
    let offset = text.rfind('|').expect("a cursor");
    let text = format!("{}{}", &text[..offset], &text[offset + 1..]);
    let file = site.root.join(path);
    let renderer = checking.renderer();
    let completion = super::super::complete(&site.root, &file, &text, offset, renderer, true)?;
    let items = completion.items.into_iter().map(|item| {
        let detail = item.detail.unwrap_or_default();
        (item.label, detail, item.insert)
    });
    Some(items.collect())
}

fn in_page(site: &TempSite, text: &str) -> Option<Offered> {
    offered(site, "templates/page.html", text)
}

fn labels(offered: &Offered) -> Vec<&str> {
    offered.iter().map(|(label, ..)| label.as_str()).collect()
}

fn item<'a>(offered: &'a Offered, label: &str) -> &'a (String, String, String) {
    (offered.iter())
        .find(|(said, ..)| said == label)
        .unwrap_or_else(|| panic!("`{label}` not among {:?}", labels(offered)))
}

fn has(offered: &Offered, label: &str) -> bool {
    offered.iter().any(|(said, ..)| said == label)
}

#[test]
fn a_name_offers_what_the_template_may_read_there() {
    let site = site();
    let names = in_page(&site, "{{ | }}").unwrap();
    assert_eq!(
        item(&names, "page"),
        &("page".into(), "page".into(), "page".into())
    );
    assert!(has(&names, "site"));
    assert!(!has(&names, "loop"), "outside a loop");
    assert_eq!(
        item(&names, "card(post)"),
        &(
            "card(post)".into(),
            "snippets/card.html".into(),
            "card($1)".into()
        )
    );
    assert_eq!(item(&names, "url").2, "url($1)");
    let looping = in_page(&site, "{% for post in site.posts %}{{ po| }}{% endfor %}").unwrap();
    assert_eq!(item(&looping, "post").1, "page");
    assert_eq!(item(&looping, "loop").1, "fields");
    let set = in_page(&site, "{% set greeting = 'Hello' %}{% if gr|").unwrap();
    assert_eq!(item(&set, "greeting").1, "text");
    let setting = in_page(&site, "{% set greeting = pa|").unwrap();
    assert_eq!(item(&setting, "page").1, "page", "what a name is set to");
    let heading = offered(&site, "snippets/markdown/heading.html", "{{ le| }}").unwrap();
    assert_eq!(
        item(&heading, "level").1,
        "number",
        "what a piece is handed"
    );
    let opened = in_page(&site, "{{ ca|rd(post) }}").unwrap();
    assert_eq!(item(&opened, "card(post)").2, "card");
}

#[test]
fn a_field_offers_what_the_value_before_its_dot_holds() {
    let site = site();
    let fields = in_page(&site, "{{ page.| }}").unwrap();
    assert_eq!(item(&fields, "title").1, "text");
    assert_eq!(item(&fields, "date").1, "date");
    assert_eq!(item(&fields, "tags").1, "lists");
    assert_eq!(item(&fields, "image").2, "image($1)");
    assert!(has(&fields, "book"), "a field the site declares");
    let looped = in_page(&site, "{% for post in site.posts %}{{ post.da|").unwrap();
    assert!(has(&looped, "date"), "the loop's page, the tag left open");
    let date = in_page(&site, "{{ page.date.| }}").unwrap();
    assert_eq!(labels(&date), ["day", "month", "weekday", "year"]);
    let book = in_page(&site, "{{ page.book.| }}").unwrap();
    assert_eq!(
        book,
        [
            ("author".into(), "text".into(), "author".into()),
            ("year".into(), "number".into(), "year".into()),
        ]
    );
    let data = in_page(&site, "{{ site.data.| }}").unwrap();
    assert_eq!(item(&data, "people").1, "fields");
    let person = in_page(&site, "{{ site.data.people.person-1.|").unwrap_or_default();
    assert!(
        person.is_empty(),
        "a name no template can write after a dot"
    );
    let people = in_page(&site, "{{ site.data.people.| }}").unwrap();
    assert_eq!(labels(&people), ["person-1"]);
    let snippet = offered(&site, "snippets/card.html", "{{ post.| }}").unwrap();
    assert!(has(&snippet, "title"), "a page, as its calls hand it");
    assert_eq!(
        in_page(&site, "{{ page.title.| }}"),
        None,
        "text has no fields"
    );
}

#[test]
fn filters_tests_and_statements_are_offered_by_name() {
    let site = site();
    let filters = in_page(&site, "{{ page.title | tr|").unwrap();
    assert!(has(&filters, "truncate") && has(&filters, "date") && has(&filters, "upper"));
    let block = in_page(&site, "{% filter |").unwrap();
    assert!(has(&block, "upper"));
    let tests = in_page(&site, "{% if page.summary is not |").unwrap();
    assert!(has(&tests, "defined"));
    assert!(!has(&tests, "=="));
    let statements = in_page(&site, "{% for post in site.posts %}{% if post.title %}{% |").unwrap();
    assert_eq!(
        labels(&statements),
        [
            "autoescape",
            "block",
            "break",
            "call",
            "continue",
            "do",
            "elif",
            "else",
            "endif",
            "extends",
            "filter",
            "for",
            "if",
            "macro",
            "raw",
            "set",
            "with",
        ]
    );
    let closed = in_page(&site, "{% for post in site.posts %}{% endfor %}{% |").unwrap();
    assert!(!has(&closed, "endfor") && !has(&closed, "break"));
}

#[test]
fn what_a_template_names_is_offered_from_the_site() {
    let site = site();
    let extends = in_page(&site, "{% extends \"|").unwrap();
    assert_eq!(
        extends,
        [("base".into(), "templates/base.html".into(), "base".into())]
    );
    let blocks = in_page(
        &site,
        "{% extends \"base\" %}{% block title %}{% endblock %}{% block |",
    )
    .unwrap();
    assert_eq!(labels(&blocks), ["foot", "main"]);
    let addresses = in_page(&site, "<a href=\"{{ url('/ab| }}").unwrap();
    assert_eq!(item(&addresses, "/about/").1, "About");
    let css = in_page(&site, "{{ css(\"|").unwrap();
    assert_eq!(labels(&css), ["site.css"]);
    let js = in_page(&site, "{{ js(\"|").unwrap();
    assert_eq!(labels(&js), ["menu.js"]);
    let headings = in_page(&site, "{{ url(\"/about/#|\") }}").unwrap();
    assert_eq!(labels(&headings), ["the-team"]);
}

#[test]
fn a_snippet_call_offers_the_names_it_has_no_value_for() {
    let site = site();
    let call = in_page(&site, "{{ figure(\"photo-1.jpg\", |").unwrap();
    assert!(has(&call, "page"), "a value may be any name");
    assert_eq!(
        item(&call, "caption"),
        &("caption".into(), "value".into(), "caption=".into())
    );
    assert!(has(&call, "alt"));
    let named = in_page(&site, "{{ figure(\"photo-1.jpg\", alt=\"A\", |").unwrap();
    assert_eq!(
        labels(&named),
        ["caption"],
        "only named values may follow one"
    );
    let first = in_page(&site, "{{ figure(|").unwrap();
    assert!(has(&first, "photo"));
}

#[test]
fn nothing_is_offered_outside_a_tag_or_where_a_name_is_made() {
    let site = site();
    let cases = [
        "<p>pa|",
        "{# {{ pa| #}",
        "{% raw %}{{ pa|",
        "{% for po|",
        "{% set gr|",
        "{% macro ca|",
        "{{ 1|",
        "{{ page.title }}|",
        "{{ url(\"/about/\") ~ \"a|",
    ];
    for text in cases {
        assert_eq!(in_page(&site, text), None, "{text:?}");
    }
    let after_raw = in_page(&site, "{% raw %}{{ x }}{% endraw %}{{ pa|").unwrap();
    assert!(has(&after_raw, "page"));
    let after_comment = in_page(&site, "{# {{ #}{{ pa|").unwrap();
    assert!(has(&after_comment, "page"));
}

#[test]
fn each_of_statacs_names_filters_and_functions_says_what_it_is() {
    use crate::described::Group;
    let site = site();
    let mut checking = Checking::new();
    checking.check(&Options {
        mode: Mode::Check,
        site: site.root.clone(),
        out: None,
        today: None,
        drafts: false,
        url: None,
    });
    let file = site.root.join("templates/page.html");
    let about = |text: &str, label: &str| {
        let offset = text.rfind('|').expect("a cursor");
        let text = format!("{}{}", &text[..offset], &text[offset + 1..]);
        let renderer = checking.renderer();
        let completion = super::super::complete(&site.root, &file, &text, offset, renderer, true);
        let items = completion.expect("something is offered").items;
        let item = items.into_iter().find(|item| item.label == label);
        item.unwrap_or_else(|| panic!("`{label}` offered")).about
    };
    let cases = [
        ("{{ | }}", "page", Group::Handed.of("page")),
        ("{{ | }}", "url", Group::Functions.of("url")),
        ("{{ | }}", "card(post)", None),
        ("{{ page.| }}", "title", Group::Page.of("title")),
        ("{{ page.| }}", "book", None),
        ("{{ site.| }}", "posts", Group::Site.of("posts")),
        ("{{ page.date.| }}", "weekday", Group::Date.of("weekday")),
        ("{{ page.image('a.jpg').| }}", "alt", Group::Image.of("alt")),
        ("{{ page.file('a.pdf').| }}", "url", Group::File.of("url")),
        ("{{ page.card.| }}", "url", Group::Card.of("url")),
        (
            "{% for heading in page.toc %}{{ heading.| }}{% endfor %}",
            "level",
            Group::Heading.of("level"),
        ),
        ("{{ page.title | | }}", "upper", Group::Filters.of("upper")),
        ("{{ page.date | | }}", "moment", Group::Filters.of("moment")),
        (
            "{% if page.title is | %}{% endif %}",
            "defined",
            Group::Tests.of("defined"),
        ),
        (
            "{% for post in site.posts %}{{ loop.| }}{% endfor %}",
            "index",
            Group::Loop.of("index"),
        ),
        (
            "{% for post in site.posts %}{{ po| }}{% endfor %}",
            "post",
            None,
        ),
    ];
    for (text, label, wanted) in cases {
        assert_eq!(about(text, label), wanted, "{text:?}: {label}");
    }
}
