use super::*;

fn words(text: &str) -> Vec<&str> {
    spans(text).map(|span| &text[span]).collect()
}

#[test]
fn words_are_found_as_unicode_finds_them() {
    assert_eq!(
        words("Don't panic: the Pi 5's GPIO, café-au-lait 2026!"),
        [
            "Don't", "panic", "the", "Pi", "5", "s", "GPIO", "café", "au", "lait", "2026"
        ]
    );
}

#[test]
fn japanese_is_split_into_words_by_dictionary() {
    let found = words("日本語のブログを書く");
    assert!(
        found.contains(&"日本語") && found.contains(&"ブログ"),
        "{found:?}"
    );
    assert!(!found.contains(&"ログ"), "{found:?}");
}

#[test]
fn a_word_is_folded_to_small_letters_without_its_accents() {
    assert_eq!(fold("Ｃafé"), "cafe");
    assert_eq!(fold("NAÏVE"), "naive");
    assert_eq!(fold("Ελληνικά"), "ελληνικα");
    // The kana's voicing changes the word, so it stays.
    assert_eq!(fold("ガイド"), "ガイド");
}

#[test]
fn a_word_is_cut_to_its_stem_by_its_language() {
    let english = Stemmer::of(Some("en-GB"));
    for word in ["running", "runs", "run"] {
        assert_eq!(english.term(word), "run", "{word}");
    }
    assert_eq!(Stemmer::of(None).term("Photos"), "photo");
    let french = Stemmer::of(Some("fr"));
    assert_eq!(french.term("chanteuses"), french.term("chanteuse"));
    assert_eq!(Stemmer::of(Some("ja")).term("ブログ"), "ブログ");
}

/// A site at a folder of its own, read, with its files as `files` gives
/// them beside a page template that shows the writing.
fn site(name: &str, files: &[(&str, &str)]) -> (Renderer, std::path::PathBuf) {
    let root = std::env::temp_dir().join(format!("statac-search-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let base = [
        ("statac.yaml", "title: Site\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        ("templates/home.html", "{{ page.content }}\n"),
        ("content/index.md", "---\ntitle: Home\n---\nWelcome.\n"),
    ];
    for (path, text) in base.iter().chain(files) {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let (renderer, _) = crate::build::renderer(&root, None);
    (renderer, root)
}

fn page(renderer: &Renderer, path: &str) -> PageId {
    let mut pages = renderer.site().pages.iter();
    pages
        .find(|(_, page)| page.path == path)
        .map(|(id, _)| id)
        .unwrap()
}

const ZFS: &str = "\
---
title: ZFS on the Pi
tags: [storage]
---
Opening about disks.

## Setting up

Install zfs-dkms on the Pi.

![A Pi with a drive](https://a.example/pi.jpg)

## Speed

It runs fast.
";

#[test]
fn a_page_is_cut_at_each_heading_with_what_it_shows() {
    let raw =
        "---\ntitle: Raw\n---\n<p>Before.</p>\n<h2 id=\"part\">Part two</h2>\n<p>Inside.</p>\n";
    let hidden = "---\ntitle: Hidden\nsearch: false\n---\nzfs\n";
    let (renderer, root) = site(
        "cut",
        &[
            ("content/zfs.md", ZFS),
            ("content/raw.html", raw),
            ("content/hidden.md", hidden),
        ],
    );
    let records = records(&renderer);
    let _ = std::fs::remove_dir_all(root);
    let of = |path: &str| {
        let page = page(&renderer, path);
        let held = records.iter().filter(|record| record.page == page);
        held.map(|r| (r.heading.clone(), r.text.as_str(), r.described.clone()))
            .collect::<Vec<_>>()
    };
    let heading = |id: &str, words: &str| Some((id.to_owned(), words.to_owned()));
    assert_eq!(
        of("zfs.md"),
        [
            (None, "Opening about disks.", vec![]),
            (
                heading("setting-up", "Setting up"),
                "Install zfs-dkms on the Pi.",
                vec!["A Pi with a drive".to_owned()]
            ),
            (heading("speed", "Speed"), "It runs fast.", vec![]),
        ]
    );
    assert_eq!(
        of("raw.html"),
        [
            (None, "Before.", vec![]),
            (heading("part", "Part two"), "Inside.", vec![])
        ]
    );
    assert!(of("hidden.md").is_empty());
}

#[test]
fn a_page_named_for_the_words_comes_first_and_every_word_counts_before_any() {
    let other = "---\ntitle: Notes\n---\nZFS came up once, and speed twice: speed.\n";
    let (renderer, root) = site(
        "rank",
        &[("content/zfs.md", ZFS), ("content/notes.md", other)],
    );
    let index = Index::of(renderer.site(), records(&renderer));
    let _ = std::fs::remove_dir_all(root);
    let first = |words: &str| {
        let found = index.search(words);
        let record = &index.records[found[0].record];
        (
            record.page,
            record.heading.as_ref().map(|(id, _)| id.clone()),
        )
    };
    let zfs = page(&renderer, "zfs.md");
    assert_eq!(first("zfs"), (zfs, None));
    assert_eq!(first("zfs speed"), (zfs, Some("speed".to_owned())));
    assert_eq!(first("drive"), (zfs, Some("setting-up".to_owned())));
    assert!(index.search("nowhere").is_empty());
    // Other forms of a word, and the word without its accents.
    assert_eq!(first("ZFS speeds"), (zfs, Some("speed".to_owned())));
    assert_eq!(first("Drives"), (zfs, Some("setting-up".to_owned())));
}

#[test]
fn each_page_is_searched_in_its_own_languages_forms() {
    let (renderer, root) = site(
        "languages",
        &[
            ("statac.yaml", "title: Site\nlanguage: fr\n"),
            ("content/index.md", "---\ntitle: Accueil\n---\nBienvenue.\n"),
            (
                "content/chant.md",
                "---\ntitle: Chant\n---\nLes chanteuses du café.\n",
            ),
        ],
    );
    let index = Index::of(renderer.site(), records(&renderer));
    let _ = std::fs::remove_dir_all(root);
    let chant = page(&renderer, "chant.md");
    for words in ["chanteuse", "cafe", "CAFÉ"] {
        let found = index.search(words);
        assert_eq!(index.records[found[0].record].page, chant, "{words}");
    }
}

#[test]
fn an_excerpt_is_cut_at_words_around_the_first_word_asked_for() {
    let asked: std::collections::HashSet<String> = ["zfs".to_owned()].into();
    let text = format!(
        "{} ZFS is here. {}",
        "Before ".repeat(20),
        "after ".repeat(40)
    );
    let stemmer = Stemmer::of(None);
    let pieces = excerpt(&text, &asked, &stemmer);
    assert_eq!(pieces.first().unwrap().0, "\u{2026}");
    assert_eq!(pieces.last().unwrap().0, "\u{2026}");
    assert!(pieces.contains(&("ZFS".to_owned(), true)));
    let shown: String = pieces.iter().map(|(text, _)| text.as_str()).collect();
    assert!(shown.len() < 220, "{shown}");
    assert_eq!(
        excerpt("Nothing asked.", &asked, &stemmer),
        [("Nothing asked.".to_owned(), false)]
    );
    // A form of the word asked for is marked as it is written.
    let plain = excerpt("Plain zfs's here.", &asked, &stemmer);
    assert!(plain.contains(&("zfs's".to_owned(), true)), "{plain:?}");
}

#[test]
fn the_command_shows_each_page_found_once_at_its_best_part() {
    let other = "---\ntitle: Notes\n---\nZFS came up once.\n";
    let (_, root) = site(
        "command",
        &[("content/zfs.md", ZFS), ("content/notes.md", other)],
    );
    let said = |words: &str| {
        let options = Options {
            site: root.clone(),
            words: words.to_owned(),
            today: None,
            drafts: false,
        };
        let outcome = run(&options);
        let report = crate::report::Report::new(outcome.sources().unwrap(), &root, false);
        report.found(words, &outcome)
    };
    let zfs = said("zfs");
    let fast = said("fast pi");
    let some = said("fast nowhere");
    let none = said("nowhere");
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(
        zfs,
        "\
2 pages hold `zfs`, best first:

  ZFS on the Pi
  /zfs/
  Opening about disks.

  Notes
  /notes/
  ZFS came up once.
"
    );
    assert_eq!(
        fast,
        "\
1 page holds `fast pi`:

  ZFS on the Pi \u{203a} Speed
  /zfs/#speed
  It runs fast.
"
    );
    assert!(
        some.starts_with("No page holds every word of `fast nowhere`; 1 holds some:\n"),
        "{some}"
    );
    assert_eq!(none, "No page holds `nowhere`.\n");
}
