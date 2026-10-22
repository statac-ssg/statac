use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;

fn site() -> TempSite {
    TempSite::site(&[
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/blog/\nfields.isbn: required text\n\
             tagline: Notes\nbook:\n  author: Author 1\n",
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
            "The colours of code in the writing: `ocean`, `gruvbox`, `solarized`, \
             `catppuccin` or `one-half`, or `none` to colour no code. Unset, it is `ocean`.",
        ]
    );
    assert_eq!(
        more.unwrap(),
        "https://statac.dev/settings/#writing-and-code"
    );
    let (lines, more, _) = settings(&site, "fields.book.au|thor: text\n").unwrap();
    assert_eq!(lines[0], "`fields.book.author`, a setting");
    assert!(lines[1].starts_with("The kind of value a field of your own holds"));
    assert_eq!(more.unwrap(), "https://statac.dev/settings/#fields");
    let (lines, more, _) = settings(&site, "pages.lay|out: post\n").unwrap();
    assert_eq!(
        lines,
        [
            "`pages.layout`, handed down",
            "Hands `layout` down to every page.",
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
            "`pages.isbn`, handed down",
            "Hands `isbn` down to every page.",
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
            "`tagline`, a value of your own",
            "Templates show it as `site.tagline`."
        ]
    );
    assert_eq!(more.unwrap(), "https://statac.dev/settings/#your-own");
    let nested = "tagline: Notes\nbook:\n  auth|or: Author 1\n";
    let (lines, ..) = settings(&site, nested).unwrap();
    assert_eq!(lines[0], "`book.author`, a value of your own");
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
        "A link, or a page's `redirect_to`, points at an address on this site, but nothing \
         is published at that address. A reader who follows it lands on a missing page."
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
            "`isbn`, a field of your own",
            "Templates show it as `page.isbn`.",
            "Declared in `statac.yaml` as `required text`.",
        ]
    );
    assert_eq!(more.unwrap(), "https://statac.dev/front-matter/#your-own");
    let (lines, ..) = post(&site, "title: Post 1\ngree|ting: Hello").unwrap();
    assert_eq!(
        lines,
        [
            "`greeting`, a field of your own",
            "Templates show it as `page.greeting`."
        ]
    );
    let index = "---\ntitle: Posts\npages.lay|out: post\n---\n";
    let (lines, ..) = said(&site, "content/posts/index.md", index).unwrap();
    assert_eq!(
        lines[1],
        "Hands `layout` down to every page in this folder and the folders inside it."
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
            "Its page is `/blog/tags/rust/`."
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
        ("templates/page.html", "{{ page.ti|tle }}\n"),
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
