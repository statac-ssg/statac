//! `statac new` writes a site that checks and builds: in one language or
//! two, with an address or none, in languages Statac has words for and one
//! it hasn't, left to right and right to left. Each test makes its sites in
//! the system's temporary folder, with the machine's locale and zone given;
//! none touches the network.

mod scratch;

use scratch::Scratch;
use std::fs;
use std::path::Path;

use statac::build::{self, Mode, Options};
use statac::date::Date;
use statac::new::{self, Input, Locale, Machine};
use statac::report::Report;

const KEY: &str = "0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20";

/// A folder of its own for each test, named `label`, with nothing there.
fn scratch(label: &str) -> Scratch {
    Scratch::new("new-sites", label)
}

fn day() -> Date {
    Date::day_only(2026, 10, 6)
}

/// Makes a site in `folder` with `typed` as the answers, and gives its
/// feed a key of its own so what is built is the same every time.
fn made(folder: &Path, typed: &str) {
    let mut reader = typed.as_bytes();
    let machine = Machine {
        input: Some(Input {
            reader: &mut reader,
            terminal: true,
        }),
        locale: Locale::default(),
        timezone: Some("Europe/London".into()),
    };
    let options = new::Options {
        folder: Some(folder.to_path_buf()),
        yes: false,
    };
    let mut said = Vec::new();
    let code = new::run(&options, machine, &mut said, false);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&said));
    let settings = folder.join("statac.yaml");
    let text = fs::read_to_string(&settings).unwrap();
    let fixed: Vec<String> = text
        .lines()
        .map(|line| match line.starts_with("feed_key: ") {
            true => format!("feed_key: {KEY}"),
            false => line.to_owned(),
        })
        .collect();
    fs::write(&settings, fixed.join("\n") + "\n").unwrap();
}

/// What `mode` finds: the name of each kind of problem, and its headline,
/// in order; and whether anything was published.
fn built(folder: &Path, mode: Mode) -> (Vec<(String, String)>, bool) {
    let out = folder.with_extension("built");
    let outcome = build::run(&Options {
        mode,
        site: folder.to_path_buf(),
        out: Some(out),
        today: Some(day()),
        drafts: false,
        url: None,
    });
    let report = Report::new(outcome.sources(), folder, false);
    let mut found: Vec<(String, String)> = outcome
        .problems
        .iter()
        .map(|problem| {
            let drawn = report.problem(problem);
            let headline = drawn.lines().next().unwrap_or_default().to_owned();
            (problem.kind.code().name().to_owned(), headline)
        })
        .collect();
    found.sort();
    (found, outcome.published.is_some())
}

#[test]
fn every_combination_is_written_checks_and_builds() {
    let languages = [
        ("en-GB", &[][..]),
        ("en-GB, fr", &[]),
        ("ar", &[]),
        ("ar, en", &[]),
        ("sw", &["sw"]),
        ("sw, ar", &["sw"]),
    ];
    let mut at = 0;
    for (typed_languages, without_words) in languages {
        for address in ["https://example.org", ""] {
            at += 1;
            let folder = scratch(&format!("site-{at}"));
            made(
                &folder,
                &format!("Site {at}\n\n{address}\n{typed_languages}\n\n"),
            );
            let mut wanted: Vec<(String, String)> = without_words
                .iter()
                .map(|tag| {
                    let line = format!(
                        "warning  Unknown language: Statac has no translations for `{tag}`"
                    );
                    ("unknown-language".to_owned(), line)
                })
                .collect();
            wanted.sort();
            let combination = format!("{typed_languages} `{address}`");
            let (found, _) = built(&folder, Mode::Check);
            assert_eq!(found, wanted, "check: {combination}");
            let (found, published) = built(&folder, Mode::Build);
            assert_eq!(found, wanted, "build: {combination}");
            assert!(published, "{combination}");
            // A build that publishes no page has no URL to keep.
            assert!(!folder.join("published.txt").exists(), "{combination}");
        }
    }
}

#[test]
fn a_site_in_a_language_lacking_the_words_statac_shows_is_given_them_in_english() {
    for tag in ["ar", "de", "es", "fa", "fr", "fr-CA", "ja", "ko"] {
        let folder = scratch(&format!("words-{tag}"));
        made(
            &folder,
            &format!("Site 1\n\nhttps://example.org\n{tag}\n\n"),
        );
        let file = folder.join(format!("languages/{tag}.yaml"));
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            "# The text Statac adds to your content, in English for you to translate: \
             see https://statac.dev/dates-and-languages/#language-files\n\
             back_to_the_text: back to the text\n\
             callouts: [Note, Tip, Important, Warning, Caution]\n",
            "{tag}"
        );
        fs::create_dir(folder.join("templates")).unwrap();
        for template in ["home", "page"] {
            fs::write(
                folder.join(format!("templates/{template}.html")),
                "<title>{{ page.title }}</title>\n{{ page.content }}",
            )
            .unwrap();
        }
        fs::write(folder.join("content/index.md"), "---\ntitle: Home\n---\n").unwrap();
        fs::write(
            folder.join("content/page-1.md"),
            "---\ntitle: Page 1\n---\nA footnote.[^1]\n\n[^1]: The note.\n",
        )
        .unwrap();
        assert_eq!(built(&folder, Mode::Check).0, [], "{tag}");
        let (found, published) = built(&folder, Mode::Build);
        assert_eq!(found, [], "{tag}");
        assert!(published, "{tag}");
        let page = read_built(&folder, "page-1/index.html");
        assert!(
            page.contains("aria-label=\"back to the text\""),
            "{tag}: {page}"
        );

        // Without the file, the footnote has no translation.
        fs::remove_file(&file).unwrap();
        let line =
            format!("note  Missing translation: the footnotes' back link, on 1 page in `{tag}`");
        let wanted = [("missing-translation".to_owned(), line)];
        assert_eq!(built(&folder, Mode::Check).0, wanted, "{tag}");
    }
}

#[test]
fn a_site_in_english_or_a_language_with_no_words_is_given_no_file_of_words() {
    for (at, typed) in ["en", "en-GB", "sw", "en, sw"].iter().enumerate() {
        let folder = scratch(&format!("no-words-file-{at}"));
        made(
            &folder,
            &format!("Site 1\n\nhttps://example.org\n{typed}\n\n"),
        );
        assert!(!folder.join("languages").exists(), "{typed}");
    }
    let folder = scratch("no-words-file-for-one");
    made(&folder, "Site 1\n\nhttps://example.org\nen, fr\n\n");
    let held: Vec<_> = fs::read_dir(folder.join("languages"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(held, ["fr.yaml"]);
}

#[test]
fn a_site_naming_no_language_is_in_english() {
    let folder = scratch("no-language");
    made(&folder, "Site 1\n\nhttps://example.org\nen\n\n");
    let settings = folder.join("statac.yaml");
    let text = fs::read_to_string(&settings).unwrap();
    let kept: Vec<&str> = text
        .lines()
        .filter(|line| !line.starts_with("language: "))
        .collect();
    assert_eq!(kept.len() + 1, text.lines().count());
    fs::write(&settings, kept.join("\n") + "\n").unwrap();
    fs::create_dir(folder.join("templates")).unwrap();
    for template in ["home", "page"] {
        fs::write(
            folder.join(format!("templates/{template}.html")),
            "{{ site.language }} {{ site.languages }} {{ site.languages.en.url }}",
        )
        .unwrap();
    }
    fs::write(folder.join("content/index.md"), "---\ntitle: Home\n---\n").unwrap();
    let note = [(
        "no-language".to_owned(),
        "note  No language set: Statac reads the site as English".to_owned(),
    )];
    assert_eq!(built(&folder, Mode::Check).0, note);
    let (found, published) = built(&folder, Mode::Build);
    assert_eq!(found, note);
    assert!(published);
    assert_eq!(read_built(&folder, "index.html"), "en en /");
}

/// The built site's file at `path`.
fn read_built(folder: &Path, path: &str) -> String {
    fs::read_to_string(folder.with_extension("built").join(path)).unwrap()
}
