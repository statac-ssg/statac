//! A site in several languages: each page built in its own language's
//! words and settings, and `site` answering in it.

use super::*;
use crate::problem::ShownWords;
use crate::site::LanguageId;

/// What a page says of its language, its site and its date.
const SAYS: (&str, &str) = (
    "templates/page.html",
    "{{ page.language }}|{{ page.direction }}|{{ site.language }}|{{ site.title }}|\
     {{ site.languages | join(\",\") }}|{{ page.date }}|\
     {{ page.content }}",
);

const POST: &str = "---\ndate: 2026-10-04\n---\n\"Words\" and a note.[^1]\n\n[^1]: A note.\n";

const DATED: &str = "---\ndate: 2026-10-04\n---\n";

/// The words `page`'s language lacks for what its writing shows, by the
/// language's file, and its HTML.
fn wordless(built: &Built, page: &str) -> (Vec<(String, ShownWords)>, String) {
    let rendered = built.render(page);
    let lacking = rendered
        .problems
        .iter()
        .map(|problem| match &problem.kind {
            Kind::MissingWords {
                file,
                words,
                pages: 1,
                ..
            } => (file.clone(), *words),
            other => panic!("{other:?}"),
        })
        .collect();
    (lacking, rendered.html.expect("built"))
}

fn two_languages(settings: &str) -> Built {
    site(&[
        ("statac.yaml", settings),
        ("content/post-1.md", POST),
        ("content/en/post-1.md", POST),
        SAYS,
    ])
}

#[test]
fn each_page_is_built_in_its_own_languages_words_and_settings() {
    let built = two_languages(
        "language: [ar, en]\ntitle: Site 1\nen.title: Site 2\ntimezone: UTC\npages.feed: false\n",
    );
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let (lacking, arabic) = wordless(&built, "post-1.md");
    assert_eq!(
        lacking,
        [("languages/ar.yaml".into(), ShownWords::BackToTheText)]
    );
    let english = built.html("en/post-1.md");
    let said = |html: &str| html.splitn(7, '|').take(6).collect::<Vec<_>>().join("|");
    assert_eq!(said(&arabic), "ar|rtl|ar|Site 1|ar,en|4 أكتوبر 2026");
    assert_eq!(said(&english), "en|ltr|en|Site 2|ar,en|4 October 2026");
    assert!(
        arabic.contains("<p>\u{201d}Words\u{201c} and a note."),
        "{arabic}"
    );
    assert!(
        english.contains("<p>\u{201c}Words\u{201d} and a note."),
        "{english}"
    );
    // Statac has no words for a note's way back in Arabic, and its arrow
    // points the way Arabic is read.
    assert!(arabic.contains("class=\"back\">\u{21aa}</a>"), "{arabic}");
    assert!(
        english.contains("class=\"back\" aria-label=\"back to the text\">\u{21a9}</a>"),
        "{english}"
    );
}

#[test]
fn a_page_records_the_words_and_settings_of_its_own_language_alone() {
    let built = two_languages(
        "language: [ar, en]\ntitle: Site 1\nen.title: Site 2\ntimezone: UTC\npages.feed: false\n",
    );
    let (ar, en) = (LanguageId::from_index(0), LanguageId::from_index(1));
    let reads = built.render("en/post-1.md").reads;
    let keys = reads.keys();
    assert!(keys.contains(&Key::Language(en)));
    assert!(!keys.contains(&Key::Language(ar)));
    let setting = |name: &str| Key::Setting(built.renderer.name_id(name));
    assert!(keys.contains(&setting("en.title")));
    assert!(keys.contains(&setting("en.date_format")));
    // English writes its own title, so the title for every language is
    // no part of it.
    assert!(!keys.contains(&setting("title")));
    let reads = built.render("post-1.md").reads;
    assert!(reads.keys().contains(&Key::Language(ar)));
    assert!(!reads.keys().contains(&Key::Language(en)));
    assert!(reads.keys().contains(&setting("title")));
}

#[test]
fn a_date_format_for_one_language_writes_its_pages_dates() {
    let built = site(&[
        (
            "statac.yaml",
            "language: [ar, en]\ntimezone: UTC\npages.feed: false\n\
             en.date_format: Wed, Dec 31, 2025\n",
        ),
        ("content/post-1.md", DATED),
        ("content/en/post-1.md", DATED),
        ("templates/page.html", "{{ page.date }}"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "4 أكتوبر 2026");
    assert_eq!(built.html("en/post-1.md"), "Sun, Oct 4, 2026");
}

#[test]
fn a_date_format_is_read_in_the_words_it_uses_and_written_in_the_pages() {
    let built = site(&[
        (
            "statac.yaml",
            "language: [en, fr]\ntimezone: UTC\npages.feed: false\n\
             date_format: 31 December 2025\n",
        ),
        ("content/post-1.md", DATED),
        ("content/fr/post-1.md", DATED),
        (
            "templates/page.html",
            "{{ page.date }}|{{ page.date | date(\"December 2025\") }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "4 October 2026|October 2026");
    assert_eq!(built.html("fr/post-1.md"), "4 octobre 2026|octobre 2026");
    // Read in the main language's words, it depends on them too.
    let reads = built.render("fr/post-1.md").reads;
    assert!(
        reads
            .keys()
            .contains(&Key::Language(LanguageId::from_index(0)))
    );
}

#[test]
fn a_date_format_neither_language_can_read_is_told_once() {
    let built = site(&[
        (
            "statac.yaml",
            "language: [en, fr]\ntimezone: UTC\npages.feed: false\ndate_format: soon\n",
        ),
        ("content/post-1.md", POST),
        ("content/fr/post-1.md", POST),
        ("templates/page.html", "{{ page.date }}"),
    ]);
    let told: Vec<_> = built
        .told(&built.problems)
        .into_iter()
        .map(|(name, _, at)| (name, at))
        .collect();
    assert_eq!(told, [("unusable-date-format", "soon".to_owned())]);
}

#[test]
fn a_date_format_in_a_language_other_than_the_main_one_writes_the_main_ones_pages() {
    let built = site(&[
        (
            "statac.yaml",
            "language: [ar, en]\ntimezone: UTC\npages.feed: false\n\
             date_format: 31 December 2025\n",
        ),
        ("content/post-1.md", DATED),
        ("content/en/post-1.md", DATED),
        (
            "templates/page.html",
            "{{ page.date }}|{{ page.date | date(\"December 2025\") }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "4 أكتوبر 2026|أكتوبر 2026");
    assert_eq!(built.html("en/post-1.md"), "4 October 2026|October 2026");
    // Read in English's words, the Arabic page depends on them too.
    let reads = built.render("post-1.md").reads;
    assert!(
        reads
            .keys()
            .contains(&Key::Language(LanguageId::from_index(1)))
    );
    let reads = built.render("en/post-1.md").reads;
    assert!(
        !reads
            .keys()
            .contains(&Key::Language(LanguageId::from_index(0)))
    );
}

#[test]
fn a_date_format_no_language_reads_offers_its_own_languages_word_or_a_figure() {
    let refused = |format: &str| {
        let settings = format!(
            "language: [ar, en]\ntimezone: UTC\npages.feed: false\ndate_format: {format}\n"
        );
        let built = site(&[
            ("statac.yaml", &settings),
            ("content/post-1.md", POST),
            ("content/en/post-1.md", POST),
            ("templates/page.html", "{{ page.date }}"),
        ]);
        assert_eq!(built.problems.len(), 1, "{:?}", built.problems);
        let Kind::UnusableDateFormat { fault } = &built.problems[0].kind else {
            panic!("{:?}", built.problems);
        };
        (**fault).clone()
    };
    // Near an English word, it is told in English.
    assert_eq!(
        refused("31 Decembr 2025"),
        crate::problem::FormatFault::NotInReference {
            written: "Decembr".into(),
            reference: "Wednesday 31 December 2025, 3.04pm".into(),
            corrected: Some("31 December 2025".into()),
        }
    );
    // In words no declared language has, the month is offered in figures.
    let crate::problem::FormatFault::Missing { months, .. } = refused("31 grudnia 2025") else {
        panic!("not missing a month");
    };
    assert_eq!(months, ["12"]);
}

#[test]
fn a_footnotes_number_is_shown_in_the_digits_of_the_pages_language() {
    let built =
        two_languages("language: [fa, en]\ntitle: Site 1\ntimezone: UTC\npages.feed: false\n");
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let (_, persian) = wordless(&built, "post-1.md");
    assert!(
        persian.contains(
            "<sup class=\"footnote\"><a href=\"#note-1\" id=\"back-1\">\u{6f1}</a></sup>"
        ),
        "{persian}"
    );
    assert!(persian.contains("<li id=\"note-1\">"), "{persian}");
    assert!(persian.contains("<a href=\"#back-1\""), "{persian}");
    let english = built.html("en/post-1.md");
    assert!(
        english.contains("<a href=\"#note-1\" id=\"back-1\">1</a>"),
        "{english}"
    );
}

#[test]
fn a_sites_own_footnote_is_handed_the_number_plain_and_as_shown() {
    let built = site(&[
        (
            "statac.yaml",
            "language: [fa, en]\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/post-1.md", POST),
        ("content/en/post-1.md", POST),
        (
            "snippets/markdown/footnote.html",
            "[{{ number }} {{ shown }} {{ id }} {{ note }}]",
        ),
        (
            "snippets/markdown/footnotes.html",
            "{% for note in notes %}[{{ note.number }} {{ note.shown }} {{ note.id }} \
             {{ note.back }}]{% endfor %}",
        ),
        ("templates/page.html", "{{ page.content }}"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "<p>\u{ab}Words\u{bb} and a note.[1 \u{6f1} back-1 note-1]</p>\n\
         [1 \u{6f1} note-1 back-1]"
    );
    assert_eq!(
        built.html("en/post-1.md"),
        "<p>\u{201c}Words\u{201d} and a note.[1 1 back-1 note-1]</p>\n\
         [1 1 note-1 back-1]"
    );
}

#[test]
fn a_footnote_in_a_value_read_as_markdown_is_numbered_in_the_pages_digits() {
    let noted = "---\nsummary: \"A note.[^1]\\n\\n[^1]: Words.\"\n---\nWords.\n";
    let built = site(&[
        ("statac.yaml", "language: [fa, en]\npages.feed: false\n"),
        ("content/post-1.md", noted),
        ("content/en/post-1.md", noted),
        (
            "templates/page.html",
            "{{ page.summary | markdown }}{{ page.content }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let marker = |shown: &str| format!("<a href=\"#note-1\" id=\"back-1\">{shown}</a>");
    let persian = built.html("post-1.md");
    assert!(persian.contains(&marker("\u{6f1}")), "{persian}");
    let english = built.html("en/post-1.md");
    assert!(english.contains(&marker("1")), "{english}");
}

#[test]
fn a_callouts_title_is_in_the_pages_language() {
    let callout = "> [!NOTE]\n> Words.\n";
    let built = site(&[
        ("statac.yaml", "language: [ar, en]\npages.feed: false\n"),
        ("content/post-1.md", callout),
        ("content/en/post-1.md", callout),
        ("snippets/markdown/callout.html", "[{{ title }}]"),
        ("templates/page.html", "{{ page.content }}"),
    ]);
    let (lacking, arabic) = wordless(&built, "post-1.md");
    assert_eq!(
        lacking,
        [("languages/ar.yaml".into(), ShownWords::Callouts)]
    );
    assert_eq!(arabic, "[]");
    assert_eq!(built.html("en/post-1.md"), "[Note]");
}

#[test]
fn words_a_language_lacks_are_told_only_where_the_page_would_show_them() {
    const WRITING: &str = "> [!TIP]\n> Words.[^1]\n\n[^1]: A note.\n";
    let told = |settings: &str, files: &[(&str, &str)]| {
        let mut all = vec![
            ("statac.yaml", settings),
            ("content/post-1.md", WRITING),
            ("templates/page.html", "{{ page.content }}"),
        ];
        all.extend_from_slice(files);
        wordless(&site(&all), "post-1.md").0
    };
    let french = "language: fr-CA\npages.feed: false\n";
    let both = |file: &str| {
        vec![
            (file.to_owned(), ShownWords::BackToTheText),
            (file.to_owned(), ShownWords::Callouts),
        ]
    };
    // With no file of the site's for either, Statac writes both itself: the
    // footnotes with their way back, and a callout with no name.
    let way_back = |file: &str| vec![(file.to_owned(), ShownWords::BackToTheText)];
    assert_eq!(told(french, &[]), way_back("languages/fr-CA.yaml"));
    // The site's own file for the language is where the words go.
    let quotes = ("languages/fr.yaml", "quotes: « » “ ”\n");
    assert_eq!(told(french, &[quotes]), way_back("languages/fr.yaml"));
    // A file of the site's that reads what Statac hands it shows them.
    let reading = [
        ("snippets/markdown/callout.html", "{{ title }}{{ body }}"),
        (
            "snippets/markdown/footnotes.html",
            "{% if label %}{{ notes | length }}{% endif %}",
        ),
    ];
    assert_eq!(told(french, &reading), both("languages/fr-CA.yaml"));
    // One that never reads them has words of its own, or none.
    let own = [
        (
            "snippets/markdown/callout.html",
            "{{ site.data.words[kind] }}{{ body }}",
        ),
        ("snippets/markdown/footnotes.html", "{{ notes | length }}"),
        ("data/words.yaml", "tip: Astuce\n"),
    ];
    assert_eq!(told(french, &own), []);
    // Words the site gives are words enough, and a line written with
    // nothing in it gives none.
    let given = (
        "languages/fr.yaml",
        "back_to_the_text: retour au texte\n\
         callouts: [Note, Astuce, Important, Avertissement, Attention]\n",
    );
    assert_eq!(told(french, &[given]), []);
    assert_eq!(told(french, &[given, reading[0], reading[1]]), []);
    let emptied = ("languages/fr.yaml", "back_to_the_text:\ncallouts:\n");
    assert_eq!(
        told(french, &[emptied, reading[0]]),
        both("languages/fr.yaml")
    );
    // Statac has both in English alone.
    assert_eq!(told("language: en\npages.feed: false\n", &reading), []);
    for tag in ["ar", "de", "es", "fa", "fr", "ja", "ko"] {
        let settings = format!("language: {tag}\npages.feed: false\n");
        let file = format!("languages/{tag}.yaml");
        assert_eq!(told(&settings, &reading), both(&file), "{tag}");
    }
    // A language Statac has no words for is told of in its own file, and
    // one with no words at all is told of itself.
    let portuguese = "language: pt\npages.feed: false\n";
    let marks = ("languages/pt.yaml", "quotes: “ ” ‘ ’\n");
    assert_eq!(told(portuguese, &[marks]), way_back("languages/pt.yaml"));
    assert_eq!(told("language: sw\npages.feed: false\n", &[]), []);
    // Writing with neither a footnote nor a callout shows neither.
    let plain = site(&[
        ("statac.yaml", french),
        ("content/post-1.md", "Words.\n"),
        ("templates/page.html", "{{ page.content }}"),
    ]);
    assert_eq!(wordless(&plain, "post-1.md").0, []);
}

#[test]
fn a_site_title_written_for_some_languages_alone_may_have_no_value() {
    let built = site(&[
        (
            "statac.yaml",
            "language: [ar, en]\nen.title: Site 2\npages.feed: false\n",
        ),
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("content/en/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "{{ site.title or \"none\" }}"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "none");
    assert_eq!(built.html("en/post-1.md"), "Site 2");
}

#[test]
fn digits_writes_figures_in_the_digits_of_the_pages_own_language() {
    const WORDS: &str = "---\ntitle: Post 1\n---\nOne two three.\n";
    let built = site(&[
        (
            "statac.yaml",
            "language: [fa, en]\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/post-1.md", WORDS),
        ("content/en/post-1.md", WORDS),
        (
            "templates/page.html",
            "{{ 12 | digits }}|{{ page.word_count | digits }}|{{ \"Page 3 of 10\" | digits }}|\
             {{ (-1.5) | digits }}|{{ \"1,234.50\" | digits }}|{{ \"\u{6f3} and 4\" | digits }}|\
             {{ \"<b>1</b> & 2\" | safe | digits }}|{{ page.content }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let said = |page: &str| built.html(page).rsplit_once('|').unwrap().0.to_owned();
    assert_eq!(
        said("post-1.md"),
        "\u{6f1}\u{6f2}|\u{6f3}|Page \u{6f3} of \u{6f1}\u{6f0}|-\u{6f1}.\u{6f5}|\
         \u{6f1},\u{6f2}\u{6f3}\u{6f4}.\u{6f5}\u{6f0}|\u{6f3} and \u{6f4}|\u{6f1} &amp; \u{6f2}"
    );
    assert_eq!(
        said("en/post-1.md"),
        "12|3|Page 3 of 10|-1.5|1,234.50|\u{6f3} and 4|1 &amp; 2"
    );
    let (fa, en) = (LanguageId::from_index(0), LanguageId::from_index(1));
    let reads = built.render("en/post-1.md").reads;
    assert!(reads.keys().contains(&Key::Language(en)));
    assert!(!reads.keys().contains(&Key::Language(fa)));
}

#[test]
fn digits_stops_at_a_missing_value_as_any_filter_does() {
    let built = site(&[
        ("statac.yaml", "language: fa\npages.feed: false\n"),
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "{{ page.rank | digits }}"),
    ]);
    assert!(built.render("post-1.md").html.is_none());
}
