use super::Goes;
use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;

fn site() -> TempSite {
    TempSite::site(&[
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/blog/\nfields.isbn: required text\n\
             tagline: Notes\nbook:\n  author: Author 1\ntags.pages: true\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        ("templates/post.html", "{{ page.content }}\n"),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-02\ntags: [Rust]\nisbn: 0-000-00000-0\n---\n",
        ),
        (
            "content/posts/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-01-03\ntags: [rust, Go]\nisbn: 0-000-00000-1\n---\n",
        ),
    ])
}

/// What is said of the character after the cursor, written `|`, in
/// `text` as the text of the site's file at `path`, once the site is
/// read: the hover's opening line and each paragraph after, and where
/// the docs say more; and the text it is said of.
fn said(site: &TempSite, path: &str, text: &str) -> Option<(Vec<String>, Option<String>, String)> {
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
    let offset = text.find('|').expect("a cursor");
    let text = text.replacen('|', "", 1);
    let file = site.root.join(path);
    let hover = super::hover(&site.root, &file, &text, offset, checking.renderer())?;
    let mut lines = vec![hover.head];
    lines.extend(hover.body);
    Some((lines, hover.more, text[hover.range].to_owned()))
}

fn settings(site: &TempSite, text: &str) -> Option<(Vec<String>, Option<String>, String)> {
    said(site, "statac.yaml", text)
}

fn post(site: &TempSite, text: &str) -> Option<(Vec<String>, Option<String>, String)> {
    said(
        site,
        "content/posts/post-1.md",
        &format!("---\n{text}\n---\nA post.\n"),
    )
}

#[test]
fn a_setting_says_what_it_does() {
    let site = site();
    let (lines, more, over) = settings(&site, "title: Site 1\ncode_s|tyle: gruvbox\n").unwrap();
    assert_eq!(over, "code_style");
    assert_eq!(
        lines,
        [
            "`code_style`, a setting",
            "The colours of code in the content: `ocean`, `gruvbox`, `solarized`, \
             `catppuccin` or `one-half`, or `none` to colour no code. Unset, it is `ocean`.",
        ]
    );
    assert_eq!(
        more.unwrap(),
        "https://statac.dev/settings/#writing-and-code"
    );
    let (lines, more, _) = settings(&site, "fields.book.au|thor: text\n").unwrap();
    assert_eq!(lines[0], "`fields.book.author`, a setting");
    assert!(lines[1].starts_with("The kind of value a custom field holds"));
    assert_eq!(more.unwrap(), "https://statac.dev/settings/#fields");
    let (lines, more, _) = settings(&site, "pages.lay|out: post\n").unwrap();
    assert_eq!(
        lines,
        [
            "`pages.layout`, a cascade",
            "Cascades `layout` to every page.",
            "The template the page is built with, by its name in `templates/`. On a page \
             with `each:`, the template for the pages it makes.",
        ]
    );
    assert_eq!(
        more.unwrap(),
        "https://statac.dev/front-matter/#handing-down"
    );
    let (lines, ..) = settings(&site, "pages.is|bn: 0-000-00000-0\n").unwrap();
    assert_eq!(
        lines,
        [
            "`pages.isbn`, a cascade",
            "Cascades `isbn` to every page.",
            "Templates show it as `page.isbn`.",
            "Declared in `statac.yaml` as `required text`.",
        ]
    );
}

#[test]
fn a_value_of_the_sites_own_says_how_templates_show_it() {
    let site = site();
    let text = "title: Site 1\ntag|line: Notes\nbook:\n  author: Author 1\n";
    let (lines, more, _) = settings(&site, text).unwrap();
    assert_eq!(
        lines,
        [
            "`tagline`, a custom setting",
            "Templates show it as `site.tagline`."
        ]
    );
    assert_eq!(more.unwrap(), "https://statac.dev/settings/#your-own");
    let nested = "tagline: Notes\nbook:\n  auth|or: Author 1\n";
    let (lines, ..) = settings(&site, nested).unwrap();
    assert_eq!(lines[0], "`book.author`, a custom setting");
    assert_eq!(lines[1], "Templates show it as `site.book.author`.");
}

#[test]
fn a_quietened_kind_says_what_it_is() {
    let site = site();
    let (lines, more, over) = settings(&site, "quiet: [dates-disagree, brok|en-link]\n").unwrap();
    assert_eq!(over, "broken-link");
    assert_eq!(lines[0], "`broken-link`, a warning");
    assert_eq!(
        lines[1],
        "A link, or a page's `redirect_to`, points at a URL on this site, but nothing is \
         published at that URL. A reader who follows it lands on a missing page."
    );
    assert_eq!(more.unwrap(), "https://statac.dev/e/broken-link/");
}

#[test]
fn a_field_says_what_it_does() {
    let site = site();
    let (lines, more, over) = post(&site, "ti|tle: Post 1\ndate: 2026-01-02").unwrap();
    assert_eq!(over, "title");
    assert_eq!(
        lines,
        [
            "`title`, a field",
            "The page's title. Without it, the page's opening `# heading` is its title.",
        ]
    );
    assert_eq!(
        more.unwrap(),
        "https://statac.dev/front-matter/#page-fields"
    );
    let (lines, more, _) = post(&site, "title: Post 1\nis|bn: 0-000-00000-0").unwrap();
    assert_eq!(
        lines,
        [
            "`isbn`, a custom field",
            "Templates show it as `page.isbn`.",
            "Declared in `statac.yaml` as `required text`.",
        ]
    );
    assert_eq!(more.unwrap(), "https://statac.dev/front-matter/#your-own");
    let (lines, ..) = post(&site, "title: Post 1\ngree|ting: Hello").unwrap();
    assert_eq!(
        lines,
        [
            "`greeting`, a custom field",
            "Templates show it as `page.greeting`."
        ]
    );
    let index = "---\ntitle: Posts\npages.lay|out: post\n---\n";
    let (lines, ..) = said(&site, "content/posts/index.md", index).unwrap();
    assert_eq!(
        lines[1],
        "Cascades `layout` to every page in this folder and the folders inside it."
    );
}

#[test]
fn a_value_says_what_the_site_holds_of_it() {
    let site = site();
    let (lines, more, over) = post(&site, "title: Post 1\ntags: [Ru|st]").unwrap();
    assert_eq!(over, "Rust");
    assert_eq!(
        lines,
        [
            "`Rust`, a tag on 2 pages",
            "Its list page is `/blog/tags/rust/`."
        ]
    );
    assert_eq!(more, None);
    let (lines, ..) = post(&site, "title: Post 1\ntags:\n  - G|o").unwrap();
    assert_eq!(lines[0], "`Go`, a tag on 1 page");
    let (lines, ..) = post(&site, "title: Post 1\nlayout: po|st").unwrap();
    assert_eq!(
        lines,
        ["`post`, a template", "Its file is `templates/post.html`."]
    );
}

#[test]
fn nothing_is_said_of_what_statac_knows_nothing_of() {
    let site = site();
    let cases = [
        ("statac.yaml", "title: Si|te 1\n"),
        ("statac.yaml", "title: Site 1 # a com|ment\n"),
        ("statac.yaml", "unkn|own: 1\n"),
        ("statac.yaml", "fr.ti|tle: Site 1\n"),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\n---\nA po|st.\n",
        ),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\nlayout: miss|ing\n---\n",
        ),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\nsl|ug: a\n---\n",
        ),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\npages.lay|out: post\n---\n",
        ),
    ];
    for (path, text) in cases {
        assert_eq!(said(&site, path, text), None, "{path}: {text:?}");
    }
}

#[test]
fn a_setting_for_one_language_says_what_it_is() {
    let site = TempSite::site(&[
        (
            "statac.yaml",
            "title: Site 1\nlanguage: [en, fr]\nfr.title: Site 2\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/en/about.md", "---\ntitle: About\n---\n"),
        ("content/fr/about.md", "---\ntitle: About\n---\n"),
    ]);
    let text = "title: Site 1\nlanguage: [en, fr]\nfr.ti|tle: Site 2\n";
    let (lines, more, _) = settings(&site, text).unwrap();
    assert_eq!(
        lines,
        [
            "`fr.title`, a setting for one language",
            "`title` for the pages in `fr`.",
            "The site's name, which templates show as `site.title`. Feeds need it.",
        ]
    );
    assert_eq!(
        more.unwrap(),
        "https://statac.dev/dates-and-languages/#settings-for-one-language"
    );
}

fn writing_site() -> TempSite {
    TempSite::site(&[
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/blog/\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "snippets/figure.html",
            "{# takes photo, alt #}\n{% set shown = page.image(photo) %}\
             {{ shown.alt }}{{ alt if alt is defined }}\n",
        ),
        ("snippets/note.html", "<aside>{{ body }}</aside>\n"),
        (
            "content/about.md",
            "---\ntitle: About\nredirect_from: [/old-about/]\n---\n## The team\n",
        ),
        (
            "content/notes/note-1.md",
            "---\ntitle: Note 1\n---\nA note.\n",
        ),
        (
            "content/posts/post-1/index.md",
            "---\ntitle: Post 1\n---\n## First\n",
        ),
        ("content/posts/post-1/photo-1.jpg", "not read"),
        ("content/posts/post-1/notes.txt", "A file.\n"),
    ])
}

/// What is said of the character after the cursor, written `|`, in
/// `writing` as the writing of `content/posts/post-1/index.md`.
fn in_writing(site: &TempSite, writing: &str) -> Option<(Vec<String>, Option<String>, String)> {
    let text = format!("---\ntitle: Post 1\n---\n## First\n\n{writing}\n");
    said(site, "content/posts/post-1/index.md", &text)
}

#[test]
fn a_link_says_what_it_leads_to() {
    let site = writing_site();
    let about = [
        "`/blog/about/`, a page",
        "Its title is `About`.",
        "Its file is `content/about.md`.",
    ];
    let (lines, more, over) = in_writing(&site, "Read [about us](/blog/ab|out/).").unwrap();
    assert_eq!(lines, about);
    assert_eq!(over, "[about us](/blog/about/)");
    assert_eq!(more, None);
    let (lines, _, over) = in_writing(&site, "[about]: /blog/ab|out/").unwrap();
    assert_eq!(lines, about);
    assert_eq!(over, "[about]: /blog/about/");
    let (lines, _, over) = in_writing(&site, "Read [about us](@url(/ab|out/)).").unwrap();
    assert_eq!(over, "@url(/about/)");
    assert_eq!(lines[0], "`/about/`, a page", "as written");
    assert_eq!(lines[1..], about[1..]);
    let (lines, ..) = in_writing(&site, "[A note](../../no|tes/note-1/)").unwrap();
    assert_eq!(lines[0], "`../../notes/note-1/`, a page");
    assert_eq!(lines[1], "Its title is `Note 1`.");
    let (lines, ..) = in_writing(&site, "[The team](/blog/about/#the-te|am)").unwrap();
    assert_eq!(
        lines,
        [
            "`/blog/about/#the-team`, a heading",
            "Its words are `The team`.",
            "It is on the page `About`, in `content/about.md`.",
        ]
    );
    let (lines, ..) = in_writing(&site, "[Up](#fi|rst)").unwrap();
    assert_eq!(lines, ["`#first`, a heading", "Its words are `First`."]);
}

#[test]
fn a_call_says_what_its_snippet_is_and_a_name_the_file_it_names() {
    let site = writing_site();
    let (lines, _, over) = in_writing(&site, "@fig|ure(\"photo-1.jpg\", alt=\"A\")").unwrap();
    assert_eq!(over, "@figure(\"photo-1.jpg\", alt=\"A\")");
    assert_eq!(
        lines,
        [
            "`figure(photo, alt)`, a snippet",
            "Its file is `snippets/figure.html`."
        ]
    );
    let (lines, _, over) = in_writing(&site, "@figure(\"pho|to-1.jpg\", alt=\"A\")").unwrap();
    assert_eq!(over, "\"photo-1.jpg\"");
    assert_eq!(
        lines,
        [
            "`photo-1.jpg`, an image beside the page",
            "Its file is `content/posts/post-1/photo-1.jpg`.",
        ]
    );
    let (lines, _, over) = in_writing(&site, "![A photo](pho|to-1.jpg)").unwrap();
    assert_eq!(over, "![A photo](photo-1.jpg)");
    assert_eq!(lines[0], "`photo-1.jpg`, an image beside the page");
    let (lines, _, over) = in_writing(&site, "@no|te()\nA note.\n@end").unwrap();
    assert_eq!(over, "@note()");
    assert_eq!(
        lines,
        [
            "`note()`, a snippet",
            "Its file is `snippets/note.html`.",
            "It wraps the content below the call, up to `@end`.",
        ]
    );
}

#[test]
fn nothing_is_said_where_the_writing_leads_nowhere_statac_knows() {
    let site = writing_site();
    let cases = [
        "Some wo|rds.",
        "`[About](/blog/ab|out/)`",
        "[Another site](https://example.org/ot|her/)",
        "[Gone](/blog/mis|sing/)",
        "[About](/ab|out/)",
        "@figure(\"pho|to-2.jpg\")",
        "@fig|ures(\"photo-1.jpg\")",
        "@note()\nA no|te.\n@end",
    ];
    for writing in cases {
        assert_eq!(in_writing(&site, writing), None, "{writing:?}");
    }
}

fn template_site() -> TempSite {
    TempSite::site(&[
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/blog/\n",
        ),
        (
            "templates/base.html",
            "<title>{% block title %}{% endblock %}</title>\n{% block main %}{% endblock %}\n",
        ),
        (
            "templates/page.html",
            "{% extends \"base\" %}{% block main %}{{ page.content }}\
             {% for post in site.posts %}{{ card(post) }}{% endfor %}{% endblock %}\n",
        ),
        ("snippets/card.html", "{# takes post #}\n{{ post.title }}\n"),
        ("content/about.md", "---\ntitle: About\n---\nA page.\n"),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-02\n---\nA post.\n",
        ),
    ])
}

#[test]
fn a_name_or_field_in_a_template_says_what_it_holds() {
    let site = template_site();
    let page = "templates/page.html";
    let cases = [
        ("{{ pa|ge.title }}", "`page` holds a page", "page"),
        ("{{ page.ti|tle }}", "`page.title` holds text", "title"),
        ("{{ page.da|te.year }}", "`page.date` holds a date", "date"),
        (
            "{{ page.date.ye|ar }}",
            "`page.date.year` holds a number",
            "year",
        ),
        ("{{ site.po|sts }}", "`site.posts` holds pages", "posts"),
        (
            "{% set first = site.posts[0] %}{% set shown = fir|st %}",
            "`first` holds a page",
            "first",
        ),
        (
            "{% for post in site.posts %}{{ po|st.title }}{% endfor %}",
            "`post` holds a page",
            "post",
        ),
        (
            "{% for post in site.posts %}{% if lo|op.first %}{% endif %}{% endfor %}",
            "`loop` holds fields",
            "loop",
        ),
    ];
    for (text, head, over) in cases {
        let (lines, more, said_of) = said(&site, page, text).unwrap();
        assert_eq!(lines, [head], "{text:?}");
        assert_eq!((more, said_of.as_str()), (None, over), "{text:?}");
    }
    let (lines, ..) = said(&site, "snippets/card.html", "{{ po|st.title }}").unwrap();
    assert_eq!(lines, ["`post` holds a page"], "as its calls hand it");
}

#[test]
fn a_template_says_what_it_calls_and_names() {
    let site = template_site();
    let page = "templates/page.html";
    let (lines, _, over) = said(&site, page, "{{ ca|rd(page) }}").unwrap();
    assert_eq!(over, "card");
    assert_eq!(
        lines,
        [
            "`card(post)`, a snippet",
            "Its file is `snippets/card.html`."
        ]
    );
    let (lines, _, over) = said(&site, page, "<a href=\"{{ url(\"/ab|out/\") }}\">").unwrap();
    assert_eq!(over, "\"/about/\"");
    assert_eq!(
        lines,
        [
            "`/about/`, a page",
            "Its title is `About`.",
            "Its file is `content/about.md`."
        ]
    );
    let (lines, _, over) = said(&site, page, "{% extends \"ba|se\" %}").unwrap();
    assert_eq!(over, "\"base\"");
    assert_eq!(
        lines,
        ["`base`, a template", "Its file is `templates/base.html`."]
    );
}

#[test]
fn nothing_is_said_in_a_template_of_what_isnt_read_there() {
    let site = template_site();
    let cases = [
        "<p>pa|ge</p>",
        "{# {{ pa|ge }} #}",
        "{% raw %}{{ pa|ge }}{% endraw %}",
        "{{ page.title | up|per }}",
        "{% i|f page.title %}{% endif %}",
        "{% if page.title is def|ined %}{% endif %}",
        "{{ ur|l('/about/') }}",
        "{{ url(\"/mis|sing/\") }}",
        "{{ page.title.up|per() }}",
        "{{ nothi|ng }}",
        "{% block ma|in %}{% endblock %}",
    ];
    for text in cases {
        assert_eq!(said(&site, "templates/page.html", text), None, "{text:?}");
    }
}

/// Where what the character after the cursor, written `|`, in `text` as
/// the text of the site's file at `path` names is written, once the site
/// is read: the path of its file, and the text it is at where it is at a
/// place in it.
fn goes(site: &TempSite, path: &str, text: &str) -> Option<String> {
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
    let offset = text.find('|').expect("a cursor");
    let text = text.replacen('|', "", 1);
    let file = site.root.join(path);
    let renderer = checking.renderer();
    let hover = super::hover(&site.root, &file, &text, offset, renderer)?;
    let sources = renderer?.sources();
    Some(match hover.to? {
        Goes::File(path) => path,
        Goes::At(place) => format!(
            "{} at `{}`",
            sources
                .path(place.source)
                .to_string_lossy()
                .replace('\\', "/"),
            sources.spelling(place)
        ),
    })
}

#[test]
fn a_link_call_or_name_in_the_writing_goes_to_what_it_names() {
    let site = writing_site();
    let page = "content/posts/post-1/index.md";
    let goes = |writing: &str| {
        let text = format!("---\ntitle: Post 1\n---\n## First\n\n{writing}\n");
        goes(&site, page, &text)
    };
    let cases = [
        ("[About](/blog/ab|out/)", Some("content/about.md")),
        ("[About](/blog/old-ab|out/)", Some("content/about.md")),
        (
            "[The team](/blog/about/#the-t|eam)",
            Some("content/about.md at `## The team`"),
        ),
        (
            "[First](#fi|rst)",
            Some("content/posts/post-1/index.md at `## First`"),
        ),
        (
            "[A note](../../no|tes/note-1/)",
            Some("content/notes/note-1.md"),
        ),
        (
            "[A note]\n\n[a note]: /blog/notes/no|te-1/",
            Some("content/notes/note-1.md"),
        ),
        ("@url(/abo|ut/)", Some("content/about.md")),
        (
            "@fig|ure(photo-1.jpg, alt=\"\")",
            Some("snippets/figure.html"),
        ),
        (
            "@figure(photo-1.j|pg, alt=\"\")",
            Some("content/posts/post-1/photo-1.jpg"),
        ),
        (
            "![A photo](photo-1|.jpg)",
            Some("content/posts/post-1/photo-1.jpg"),
        ),
        ("@no|te()\nA note.\n@end", Some("snippets/note.html")),
        ("[Elsewhere](https://example.com/|)", None),
        ("[Nowhere](/blog/nowh|ere/)", None),
        ("Plain wo|rds.", None),
    ];
    for (writing, wanted) in cases {
        assert_eq!(goes(writing).as_deref(), wanted, "{writing}");
    }
}

#[test]
fn a_front_matter_value_or_field_goes_to_what_it_names() {
    let site = site();
    let post = |text: &str| {
        let text = format!("---\n{text}\n---\nA post.\n");
        goes(&site, "content/posts/post-1.md", &text)
    };
    assert_eq!(
        post("layout: po|st").as_deref(),
        Some("templates/post.html")
    );
    assert_eq!(
        post("is|bn: 0-000-00000-0").as_deref(),
        Some("statac.yaml at `required text`")
    );
    assert_eq!(post("ti|tle: Post 1"), None);
    // The tag's page is one Statac makes, written in no file.
    assert_eq!(post("tags: [Ru|st]"), None);
    let settings = goes(&site, "statac.yaml", "pages.is|bn: 0-000-00000-0\n");
    assert_eq!(settings.as_deref(), Some("statac.yaml at `required text`"));
}

#[test]
fn a_tag_with_a_page_of_the_authors_own_goes_to_it() {
    let site = TempSite::site(&[
        ("statac.yaml", "title: Site 1\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "content/tags/rust.md",
            "---\ntitle: Rust\n---\nAbout Rust.\n",
        ),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-02\ntags: [rust]\n---\n",
        ),
    ]);
    let text = "---\ntitle: Post 1\ndate: 2026-01-02\ntags: [ru|st]\n---\n";
    let goes = goes(&site, "content/posts/post-1.md", text);
    assert_eq!(goes.as_deref(), Some("content/tags/rust.md"));
}

#[test]
fn a_template_goes_to_what_it_calls_extends_and_names() {
    let site = template_site();
    let page = "templates/page.html";
    let cases = [
        ("{% extends \"ba|se\" %}", Some("templates/base.html")),
        ("{{ ca|rd(page) }}", Some("snippets/card.html")),
        ("{{ url(\"/abo|ut/\") }}", Some("content/about.md")),
        ("{{ page.ti|tle }}", None),
    ];
    for (text, wanted) in cases {
        assert_eq!(goes(&site, page, text).as_deref(), wanted, "{text}");
    }
}
