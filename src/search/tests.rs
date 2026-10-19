use super::*;
use crate::site::Id;

/// The site's language at `at` in its order, the main one first.
fn language(at: usize) -> LanguageId {
    LanguageId::from_index(at)
}

/// The index of the pages `renderer` read in the site's language at `at`.
fn index_of(renderer: &Renderer, at: usize) -> Index {
    Index::of(
        renderer.site(),
        language(at),
        records(renderer, language(at)),
    )
}

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
    let records = records(&renderer, language(0));
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
    let other = "---\ntitle: Notes\n---\nZFS came up once, and speed twice: speed. Running.\n";
    let (renderer, root) = site(
        "rank",
        &[("content/zfs.md", ZFS), ("content/notes.md", other)],
    );
    let index = index_of(&renderer, 0);
    let _ = std::fs::remove_dir_all(root);
    let first = |words: &str| {
        let found = index.search(&Asked::of(words, false));
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
    assert!(index.search(&Asked::of("nowhere", false)).is_empty());
    // Another form of a word the site writes finds the others.
    let running = index.search(&Asked::of("running", false));
    let records = running.iter().map(|found| &index.records[found.record]);
    let speed = |record: &&Record| record.heading.as_ref().is_some_and(|(id, _)| id == "speed");
    assert!(
        records
            .clone()
            .any(|record| record.page == zfs && speed(&record))
    );
    assert_eq!(running.len(), 2);
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
            ("content/voix.md", "---\ntitle: Voix\n---\nUne chanteuse.\n"),
        ],
    );
    let index = index_of(&renderer, 0);
    let _ = std::fs::remove_dir_all(root);
    let chant = page(&renderer, "chant.md");
    for words in ["cafe", "CAFÉ"] {
        let found = index.search(&Asked::of(words, false));
        assert_eq!(index.records[found[0].record].page, chant, "{words}");
    }
    // A word the site writes somewhere finds its other forms; one it
    // never writes finds nothing, as a reader's browser holds no rules.
    assert_eq!(index.search(&Asked::of("chanteuse", false)).len(), 2);
    assert_eq!(index.search(&Asked::of("chanteuses", false)).len(), 2);
    assert!(index.search(&Asked::of("chanter", false)).is_empty());
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
    let marked = |word: &str| asked.contains(&stemmer.term(word));
    let pieces = excerpt(&text, marked);
    assert_eq!(pieces.first().unwrap().0, "\u{2026}");
    assert_eq!(pieces.last().unwrap().0, "\u{2026}");
    assert!(pieces.contains(&("ZFS".to_owned(), true)));
    let shown: String = pieces.iter().map(|(text, _)| text.as_str()).collect();
    assert!(shown.len() < 220, "{shown}");
    assert_eq!(
        excerpt("Nothing asked.", marked),
        [("Nothing asked.".to_owned(), false)]
    );
    // A form of the word asked for is marked as it is written.
    let plain = excerpt("Plain zfs's here.", marked);
    assert!(plain.contains(&("zfs's".to_owned(), true)), "{plain:?}");
}

#[test]
fn the_command_shows_each_page_found_once_at_its_best_part() {
    let other = "---\ntitle: Notes\n---\nZFS came up once.\n";
    let (_, root) = site(
        "command",
        &[("content/zfs.md", ZFS), ("content/notes.md", other)],
    );
    // A site in one language reads the same with its language asked for.
    for language in [None, Some("en")] {
        let said = |words: &str| said(&root, words, language).unwrap();
        let zfs = said("zfs");
        let fast = said("fast pi");
        let some = said("fast nowhere");
        let none = said("nowhere");
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
",
            "{language:?}"
        );
        assert_eq!(
            fast,
            "\
1 page holds `fast pi`:

  ZFS on the Pi \u{203a} Speed
  /zfs/#speed
  It runs fast.
",
            "{language:?}"
        );
        assert!(
            some.starts_with("No page holds every word of `fast nowhere`; 1 holds some:\n"),
            "{some}"
        );
        assert_eq!(none, "No page holds `nowhere`.\n", "{language:?}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// What `statac search` says of the site at `root` for `words`, of the
/// one language `language` where it is given; or the mistake it tells.
fn said(root: &std::path::Path, words: &str, language: Option<&str>) -> Result<String, String> {
    let options = Options {
        site: root.to_owned(),
        words: words.to_owned(),
        today: None,
        drafts: false,
        language: language.map(str::to_owned),
    };
    let outcome = run(&options);
    let report = crate::report::Report::new(outcome.sources().unwrap(), root, false);
    assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
    match &outcome.mistake {
        Some(mistake) => Err(report.problems(std::slice::from_ref(mistake))),
        None => Ok(report.found(words, &outcome)),
    }
}

/// A site in English and French, each with pages holding `photo`, and
/// English pages holding `running` and `runs`, and French pages too.
fn in_two_languages(name: &str) -> (Renderer, std::path::PathBuf) {
    site(
        name,
        &[
            ("statac.yaml", "title: Site\nlanguage: [en, fr]\n"),
            (
                "content/fr/index.md",
                "---\ntitle: Accueil\n---\nBienvenue.\n",
            ),
            ("content/photo.md", "---\ntitle: Photo\n---\nA photo.\n"),
            ("content/run-1.md", "---\ntitle: Post 1\n---\nRunning.\n"),
            ("content/run-2.md", "---\ntitle: Post 2\n---\nRuns.\n"),
            (
                "content/fr/photo.md",
                "---\ntitle: Photo\n---\nUne photo.\n",
            ),
            ("content/fr/run-1.md", "---\ntitle: Post 1\n---\nRunning.\n"),
            ("content/fr/run-2.md", "---\ntitle: Post 2\n---\nRuns.\n"),
        ],
    )
}

#[test]
fn each_language_is_searched_apart_its_words_cut_by_its_own_rules() {
    let (renderer, root) = in_two_languages("apart");
    let _ = std::fs::remove_dir_all(root);
    let site = renderer.site();
    let found = |at: usize, words: &str| {
        let index = index_of(&renderer, at);
        let found = index.search(&Asked::of(words, false)).into_iter();
        let mut paths: Vec<&str> = found
            .map(|found| {
                site.pages
                    .get(index.records[found.record].page)
                    .path
                    .as_str()
            })
            .collect();
        paths.sort_unstable();
        paths
    };
    let cases: &[(usize, &str, &[&str])] = &[
        // A word both languages write finds each language's pages in its
        // own index only.
        (0, "photo", &["photo.md"]),
        (1, "photo", &["fr/photo.md"]),
        // English cuts `running` to `run`, as it does `runs`; French cuts
        // only `runs`.
        (0, "runs", &["run-1.md", "run-2.md"]),
        (1, "runs", &["fr/run-2.md"]),
        (1, "running", &["fr/run-1.md"]),
        (0, "bienvenue", &[]),
        (1, "bienvenue", &["fr/index.md"]),
    ];
    for &(at, words, paths) in cases {
        assert_eq!(found(at, words), paths, "{at} {words}");
    }
    // Each index counts only its own language's records.
    for at in [0, 1] {
        assert_eq!(index_of(&renderer, at).records.len(), 4, "{at}");
    }
}

#[test]
fn the_command_shows_what_each_language_found_under_its_code() {
    let (_, root) = in_two_languages("command-languages");
    let cases: &[(&str, Option<&str>, Result<&str, &str>)] = &[
        (
            "photo",
            None,
            Ok("\
en
1 page holds `photo`:

  Photo
  /photo/
  A photo.

fr
1 page holds `photo`:

  Photo
  /fr/photo/
  Une photo.
"),
        ),
        // A language that finds nothing is left out.
        (
            "bienvenue",
            None,
            Ok("\
fr
1 page holds `bienvenue`:

  Accueil
  /fr/
  Bienvenue.
"),
        ),
        ("nowhere", None, Ok("No page holds `nowhere`.\n")),
        // One language asked for reads as a site in one does.
        (
            "photo",
            Some("fr"),
            Ok("\
1 page holds `photo`:

  Photo
  /fr/photo/
  Une photo.
"),
        ),
        ("bienvenue", Some("en"), Ok("No page holds `bienvenue`.\n")),
        (
            "photo",
            Some("de"),
            Err("\
error  Wrong kind of value: `--language` is `en` or `fr`, not `de`
  More: https://statac.dev/e/wrong-kind-of-value/
"),
        ),
        (
            "photo",
            Some("FR"),
            Err("\
error  Wrong kind of value: `--language` is `en` or `fr`, not `FR`
  More: https://statac.dev/e/wrong-kind-of-value/
"),
        ),
    ];
    let said: Vec<Result<String, String>> = (cases.iter())
        .map(|&(words, language, _)| said(&root, words, language))
        .collect();
    let _ = std::fs::remove_dir_all(&root);
    for (&(words, language, wanted), said) in cases.iter().zip(&said) {
        let said = said.as_ref().map(String::as_str).map_err(String::as_str);
        assert_eq!(said, wanted, "{words} {language:?}");
    }
}

#[test]
fn a_long_word_list_is_cut_into_pieces_each_word_in_the_one_its_first_word_says() {
    // Words that pack poorly, each on a page of its own.
    let posts: Vec<(String, String)> = (0..120u32)
        .map(|n| {
            let word: String = (0..8)
                .map(|i| char::from(b'a' + ((n * 7 + i * 13 + n * i) % 26) as u8))
                .collect();
            (
                format!("content/post-{n}.md"),
                format!("---\ntitle: P{n}\n---\n{word} w{n}x\n"),
            )
        })
        .collect();
    let files: Vec<(&str, &str)> = posts
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let (renderer, root) = site("pieces", &files);
    let index = index_of(&renderer, 0);
    let _ = std::fs::remove_dir_all(root);
    let pieces = files::pieces(&index, 0, 200);
    assert!(pieces.len() > 3, "{}", pieces.len());
    assert_eq!(pieces[0].0, "");
    let firsts: Vec<&str> = pieces.iter().map(|(first, _)| first.as_str()).collect();
    assert!(
        firsts
            .windows(2)
            .all(|pair| in_utf16(pair[0], pair[1]).is_lt())
    );
    let unpacked: Vec<String> = (pieces.iter())
        .map(|(_, file)| {
            let bytes = miniz_oxide::inflate::decompress_to_vec_zlib(&file.bytes).unwrap();
            String::from_utf8(bytes).unwrap()
        })
        .collect();
    for word in index.words() {
        let at = firsts
            .iter()
            .rposition(|first| in_utf16(first, word).is_le())
            .unwrap();
        let quoted = format!("\"{word}\"");
        let holding: Vec<usize> = (unpacked.iter().enumerate())
            .filter(|(_, json)| json.contains(&quoted))
            .map(|(at, _)| at)
            .collect();
        assert_eq!(holding, [at], "{word}");
    }
    // Whole where it is small enough.
    assert_eq!(files::pieces(&index, usize::MAX, 200).len(), 1);
}
