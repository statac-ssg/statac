use super::{fixes, writes};
use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::testing::TempSite;

/// Every fix for the problems in the site at `files`, once it is checked:
/// the problem's name and file, the text the fix replaces, and the fix's
/// title and text.
fn fixed(files: &[(&str, &str)]) -> Vec<String> {
    let site = TempSite::site(files);
    let mut checking = Checking::new();
    let outcome = checking.check(&Options {
        mode: Mode::Check,
        site: site.root.clone(),
        out: None,
        today: None,
        drafts: false,
        url: None,
    });
    let sources = outcome.sources();
    let base = outcome.settings().map(|settings| settings.base());
    let mut found = Vec::new();
    for problem in &outcome.problems {
        for fix in fixes(problem, sources, &base.clone().unwrap_or_default()) {
            found.push(format!(
                "{} in {}: `{}` as `{}`, offered as {}",
                problem.kind.name(),
                sources.path(fix.place.source).to_string_lossy(),
                sources.spelling(fix.place),
                fix.text,
                fix.title,
            ));
        }
    }
    found.sort();
    found
}

#[test]
fn a_word_spelled_near_one_statac_knows_is_written_as_that_word() {
    let found = fixed(&[
        (
            "statac.yaml",
            "title: Site 1\ntags.permalnk: /topics/{slug}/\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "templates/other.html",
            "{{ page.titel }} {{ page.title | uppr }}\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummery: A post.\naliases: [/old/]\n---\nA post.\n",
        ),
    ]);
    assert_eq!(
        found,
        [
            "other-spelling in content/post-1.md: `aliases` as `redirect_from`, offered as \
             Write it as `redirect_from`",
            "read-as-your-own in content/post-1.md: `summery` as `summary`, offered as \
             Write it as `summary`",
            "unknown-word in statac.yaml: `tags.permalnk` as `tags.permalink`, offered as \
             Write it as `tags.permalink`",
            "unknown-word in templates/other.html: `page.titel` as `page.title`, offered as \
             Write it as `page.title`",
            "unknown-word in templates/other.html: `uppr` as `upper`, offered as \
             Write it as `upper`",
        ]
    );
}

#[test]
fn a_key_is_not_respelt_as_one_its_mapping_has() {
    let found = fixed(&[
        (
            "statac.yaml",
            "title: Site 1\ncode_styl: ocean\ncode_style: ocean\n\
             tags.permalnk: /a/{slug}/\n# A note.\n\ntags.permalink: /b/{slug}/\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummery: A post.\ntaggs: [a]\ntags: [a]\n\
             aliases: [/old/]\nredirect_from: [/older/]\n---\nA post.\n",
        ),
    ]);
    assert_eq!(
        found,
        [
            "read-as-your-own in content/post-1.md: `summery` as `summary`, offered as \
          Write it as `summary`"
        ]
    );
}

#[test]
fn a_value_read_two_ways_is_written_as_each_reading() {
    let found = fixed(&[
        ("statac.yaml", "title: Site 1\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        ("templates/other.html", "{% extends \"pag.html\" %}\n"),
        (
            "content/post-1.md",
            "---\ntitle: 3.10\ndate: 3/4/2026\nlayout: pag\n---\nA post.\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\nlayout: \"pag.html\"\n---\nA post.\n",
        ),
    ]);
    assert_eq!(
        found,
        [
            "missing-template in content/post-1.md: `pag` as `page`, offered as \
             Write it as `page`",
            "missing-template in content/post-2.md: `pag` as `page`, offered as \
             Write it as `page`",
            "missing-template in templates/other.html: `pag` as `page`, offered as \
             Write it as `page`",
            "needs-quotes in content/post-1.md: `3.10` as `\"3.10\"`, offered as \
             Write it as `\"3.10\"`",
            "unclear-date in content/post-1.md: `3/4/2026` as `2026-03-04`, offered as \
             Write it as `2026-03-04`",
            "unclear-date in content/post-1.md: `3/4/2026` as `2026-04-03`, offered as \
             Write it as `2026-04-03`",
        ]
    );
}

#[test]
fn a_broken_link_written_whole_is_written_as_the_address_suggested() {
    let found = fixed(&[
        (
            "statac.yaml",
            "title: Site 1\nurl: https://example.org/blog/\n",
        ),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "templates/other.html",
            "<a href=\"{{ url('/post-l/') }}\">1</a> <a href=\"/blog/post-l/\">2</a>\n\
             {{ page.content }}\n",
        ),
        (
            "content/about.md",
            "---\ntitle: About\nlayout: other\n---\nA page.\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n\
             [1](/blog/abot/) [2](@url(/abuot/)) [3](/about/) [4][label] [5](abot/)\n\n\
             [label]: /blog/about-us/\n",
        ),
    ]);
    assert_eq!(
        found,
        [
            "broken-link in content/post-1.md: `/about/` as `@url(/about/)`, offered as \
             Write it as `@url(/about/)`",
            "broken-link in content/post-1.md: `/blog/abot/` as `/blog/about/`, offered as \
             Write it as `/blog/about/`",
            "broken-link in content/post-1.md: `/blog/about-us/` as `/blog/about/`, offered as \
             Write it as `/blog/about/`",
            "broken-link in content/post-1.md: `@url(/abuot/)` as `@url(/about/)`, offered as \
             Write it as `@url(/about/)`",
            "broken-link in templates/other.html: `/blog/post-l/` as `/blog/post-1/`, offered \
             as Write it as `/blog/post-1/`",
            "broken-link in templates/other.html: `{{ url('/post-l/') }}` as \
             `{{ url(\"/post-1/\") }}`, offered as Write it as `{{ url(\"/post-1/\") }}`",
        ]
    );
}

#[test]
fn a_page_sent_on_to_an_address_nothing_publishes_is_sent_to_the_one_suggested() {
    let found = fixed(&[
        ("statac.yaml", "title: Site 1\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/about.md", "---\ntitle: About\n---\nA page.\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nredirect_to: /abot/\n---\n",
        ),
    ]);
    assert_eq!(
        found,
        [
            "broken-link in content/post-1.md: `/abot/` as `/about/`, offered as \
          Write it as `/about/`"
        ]
    );
}

#[test]
fn a_link_is_written_whole_only_where_it_is_handed_one_address_as_written() {
    let cases = [
        ("/about/", "", Some("/about/")),
        ("@url(/about/)", "/blog", Some("/blog/about/")),
        ("@url(\"/a b/\")", "", Some("/a b/")),
        ("{{ url(\"/about/\") }}", "/blog", Some("/blog/about/")),
        ("{{- url('/about/') -}}", "", Some("/about/")),
        ("{{ url(page.address) }}", "", None),
        ("{{ url(\"/a/\" ~ name) }}", "", None),
        ("{{ url(\"/a/\") | upper }}", "", None),
        ("@url(\"/a\\\"b/\")", "", None),
    ];
    for (at, base, wanted) in cases {
        assert_eq!(writes(at, base).as_deref(), wanted, "{at}");
    }
}
