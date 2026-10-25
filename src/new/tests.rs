use super::*;
use crate::testing::Scratch;

use std::path::Path;

/// A folder of its own for each test, named `label`, in the system's
/// temporary folder, with nothing there yet.
fn scratch(label: &str) -> Scratch {
    Scratch::named("new", label)
}

fn locale(lc_all: Option<&str>, lc_messages: Option<&str>, lang: Option<&str>) -> Locale {
    Locale {
        lc_all: lc_all.map(str::to_owned),
        lc_messages: lc_messages.map(str::to_owned),
        lang: lang.map(str::to_owned),
    }
}

/// What `statac new` says making a site in `folder` with `typed` read as
/// the answers, a person at the terminal where `terminal` says, and the
/// exit code.
fn run_with(
    folder: &Path,
    typed: &str,
    terminal: bool,
    yes: bool,
    locale: Locale,
    timezone: Option<&str>,
) -> (String, u8) {
    let mut reader = typed.as_bytes();
    let machine = Machine {
        input: Some(Input {
            reader: &mut reader,
            terminal,
        }),
        locale,
        timezone: timezone.map(str::to_owned),
    };
    let options = Options {
        folder: Some(folder.to_path_buf()),
        yes,
    };
    let mut out = Vec::new();
    let code = run(&options, machine, &mut out, false);
    (String::from_utf8(out).unwrap(), code)
}

fn asked(folder: &Path, typed: &str) -> (String, u8) {
    run_with(
        folder,
        typed,
        true,
        false,
        Locale::default(),
        Some("Europe/London"),
    )
}

fn read(folder: &Path, path: &str) -> String {
    std::fs::read_to_string(platform::join(folder, path)).unwrap()
}

/// `statac.yaml` as written, with its fresh `feed_key` left out.
fn settings_without_key(folder: &Path) -> String {
    let text = read(folder, "statac.yaml");
    let key = text
        .lines()
        .find(|line| line.starts_with("feed_key: "))
        .unwrap();
    text.replace(&format!("{key}\n"), "")
}

/// The key a written `statac.yaml` gives feeds.
fn feed_key(folder: &Path) -> String {
    let text = read(folder, "statac.yaml");
    let key = text
        .lines()
        .find_map(|line| line.strip_prefix("feed_key: "));
    key.unwrap().to_owned()
}

fn answers(languages: &[&str], address: Option<&str>) -> Answers {
    Answers {
        title: "Site 1".into(),
        address: address.map(str::to_owned),
        languages: languages.iter().map(|tag| (*tag).to_owned()).collect(),
        date_format: "2025-12-31".into(),
    }
}

/// Every file and folder under `folder`, by its path in it.
fn listed(folder: &Path) -> Vec<String> {
    fn walk(root: &Path, at: &Path, into: &mut Vec<String>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let rel = platform::slashed(path.strip_prefix(root).unwrap()).unwrap();
            into.push(rel);
            if std::fs::symlink_metadata(&path).unwrap().is_dir() {
                walk(root, &path, into);
            }
        }
    }
    let mut paths = Vec::new();
    walk(folder, folder, &mut paths);
    paths.sort();
    paths
}

#[test]
fn every_default_is_taken_by_enter() {
    let folder = scratch("my-site");
    let (said, code) = asked(&folder, "\n\n\n\n");
    assert_eq!(code, 0, "{said}");
    let shown = folder.to_string_lossy();
    let key = feed_key(&folder);
    assert_eq!(
        said,
        format!(
            "A new site in `{shown}`.\n\
             Enter takes the answer in brackets.\n\
             \n\
             What is the site called?\n\
             \x20 [My Site] > \n\
             What will its URL be?\n\
             \x20 Enter for none yet.\n\
             \x20 > \n\
             What language is it written in?\n\
             \x20 For more than one, the main one first, as `en, fr`.\n\
             \x20 [en] > \n\
             How should dates look?\n\
             \x20 1  31 December 2025\n\
             \x20 2  December 31, 2025\n\
             \x20 3  2025-12-31\n\
             \x20 Or type an example of your own.\n\
             \x20 [1] > \n\
             Made a new site in `{shown}`: 2 files.\n\
             Times are in Europe/London, as this machine is set.\n\
             \n\
             statac.yaml:\n\
             \x20 title: My Site\n\
             \x20 language: en\n\
             \x20 timezone: Europe/London\n\
             \x20 date_format: 31 December 2025\n\
             \x20 feed_key: {key}\n\
             \n\
             Next: `cd {shown}`, then write a page in `content/`, such as \
             `content/hello.md`, and run `statac serve`.\n"
        )
    );
    assert_eq!(
        read(&folder, "statac.yaml"),
        format!(
            "title: My Site\nlanguage: en\ntimezone: Europe/London\n\
             date_format: 31 December 2025\n\
             # What each post's id in a feed is made from: never change it once a feed is \
             published.\nfeed_key: {key}\n"
        )
    );
    assert_eq!(read(&folder, ".gitignore"), ".statac/\n_site/\n");
    assert_eq!(listed(&folder), [".gitignore", "content", "statac.yaml"]);
}
#[test]
fn each_answer_given_is_taken() {
    let folder = scratch("each-answer");
    let typed = "Site: 1\nhttps://example.org/blog/\nfr, en\n2\n";
    let (said, code) = asked(&folder, typed);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(
            "How should dates look?\n\
             \x20 1  31 décembre 2025\n\
             \x20 2  2025-12-31\n\
             \x20 Or type an example of your own.\n\
             \x20 [1] > \n"
        ),
        "{said}"
    );
    assert_eq!(
        settings_without_key(&folder),
        "title: \"Site: 1\"\nurl: https://example.org/blog\nlanguage: [fr, en]\n\
         timezone: Europe/London\ndate_format: 2025-12-31\n\
         # What each post's id in a feed is made from: never change it once a feed is \
         published.\n"
    );
    assert!(said.contains(
        "\n\nStatac has some translations for `fr` only in English: translate them in \
         `languages/fr.yaml`.\n\nNext: "
    ));
    assert!(!said.contains("for `en`"));
    assert_eq!(
        listed(&folder),
        [
            ".gitignore",
            "content",
            "languages",
            "languages/fr.yaml",
            "statac.yaml"
        ]
    );
    assert_eq!(read(&folder, "languages/fr.yaml"), language_file());
}

#[test]
fn an_example_of_ones_own_is_taken_where_it_reads() {
    let folder = scratch("own-example");
    let (said, code) = asked(&folder, "\n\n\nWed 31 Dec 2025\n");
    assert_eq!(code, 0, "{said}");
    let settings = read(&folder, "statac.yaml");
    assert!(
        settings.contains("\ndate_format: Wed 31 Dec 2025\n#"),
        "{settings}"
    );
}

#[test]
fn each_unusable_answer_is_told_and_asked_again() {
    let folder = scratch("unusable");
    let typed = "\nexample\nhttps://example.org/\nenglish\nen, EN\nen\n1 January 2020\n3\n";
    let (said, code) = asked(&folder, typed);
    assert_eq!(code, 0, "{said}");
    let address = "What will its URL be?\n\x20 Enter for none yet.\n";
    let language = "What language is it written in?\n\
                    \x20 For more than one, the main one first, as `en, fr`.\n";
    let dates = "How should dates look?\n\
                 \x20 1  31 December 2025\n\
                 \x20 2  December 31, 2025\n\
                 \x20 3  2025-12-31\n\
                 \x20 Or type an example of your own.\n";
    let told = [
        format!(
            "{address}\x20 > \x20 `example` isn't a full URL. Write it as \
             `https://example.org`.\n\x20 > \n{language}"
        ),
        format!(
            "{language}\x20 [en] > \x20 `english` isn't a language's code. Write it as `en`, \
             or `en-GB`.\n\x20 [en] > \x20 `EN` is `en` again. Name each language once.\n\
             \x20 [en] > \n{dates}"
        ),
        format!(
            "{dates}\x20 [1] > \x20 Unusable date format: 31 December 2025 is in December, not \
             `January`\n\x20 [1] > \n"
        ),
    ];
    for told in told {
        assert!(said.contains(&told), "{told}\n\nin\n\n{said}");
    }
    assert_eq!(
        settings_without_key(&folder),
        "title: Unusable\nurl: https://example.org\nlanguage: en\ntimezone: Europe/London\n\
         date_format: 2025-12-31\n\
         # What each post's id in a feed is made from: never change it once a feed is \
         published.\n"
    );
}

#[test]
fn yes_takes_every_default_without_asking() {
    let folder = scratch("yes");
    let (said, code) = run_with(
        &folder,
        "Typed\n",
        true,
        true,
        Locale::default(),
        Some("Europe/London"),
    );
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Made a new site in `"), "{said}");
    assert!(read(&folder, "statac.yaml").starts_with("title: \"Yes\"\n"));
}

/// What `statac new` says with `typed` read where no person is there to
/// answer, and the exit code.
fn piped(folder: &Path, typed: &str) -> (String, u8) {
    run_with(
        folder,
        typed,
        false,
        false,
        Locale::default(),
        Some("Europe/London"),
    )
}

#[test]
fn with_no_terminal_each_line_read_answers_the_next_question_and_none_is_shown() {
    let folder = scratch("piped");
    let (said, code) = piped(&folder, "Site 1\nhttps://example.org/\nen-GB\n3\n");
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Made a new site in `"), "{said}");
    assert!(!said.contains('?') && !said.contains('>'), "{said}");
    assert!(said.contains("\nstatac.yaml:\n  title: Site 1\n  url: https://example.org\n"));
    assert!(settings_without_key(&folder).starts_with(
        "title: Site 1\nurl: https://example.org\nlanguage: en-GB\ntimezone: Europe/London\n\
         date_format: 2025-12-31\n"
    ));
}

#[test]
fn with_no_terminal_an_empty_line_and_each_question_after_the_last_line_take_the_default() {
    let folder = scratch("piped-some");
    let (said, code) = piped(&folder, "\nhttps://example.org\n");
    assert_eq!(code, 0, "{said}");
    assert!(!said.contains('?'), "{said}");
    assert!(settings_without_key(&folder).starts_with(
        "title: Piped Some\nurl: https://example.org\nlanguage: en\ntimezone: Europe/London\n\
         date_format: 31 December 2025\n"
    ));
}

#[test]
fn with_no_terminal_and_nothing_to_read_every_default_is_taken() {
    let folder = scratch("piped-nothing");
    let (said, code) = piped(&folder, "");
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Made a new site in `"), "{said}");
    assert!(!said.contains('?'), "{said}");
    assert!(settings_without_key(&folder).starts_with(
        "title: Piped Nothing\nlanguage: en\ntimezone: Europe/London\n\
         date_format: 31 December 2025\n"
    ));
}

#[test]
fn with_no_terminal_an_unusable_answer_stops_with_what_a_person_is_told() {
    for (label, typed, told) in [
        (
            "piped-address",
            "Site 1\nexample\nhttps://example.org\n",
            "`example` isn't a full URL. Write it as `https://example.org`.\n",
        ),
        (
            "piped-language",
            "Site 1\n\nenglish\nen\n",
            "`english` isn't a language's code. Write it as `en`, or `en-GB`.\n",
        ),
        (
            "piped-dates",
            "Site 1\n\n\n1 January 2020\n",
            "Unusable date format: 31 December 2025 is in December, not `January`\n",
        ),
    ] {
        let folder = scratch(label);
        let (said, code) = piped(&folder, typed);
        assert_eq!(code, 1, "{label}");
        assert_eq!(said, told, "{label}");
        assert!(!folder.exists(), "{label}");
    }
}

#[test]
fn yes_reads_nothing_of_what_is_there_to_read() {
    for terminal in [true, false] {
        let folder = scratch("yes-unread");
        let mut reader = "Typed\n".as_bytes();
        let machine = Machine {
            input: Some(Input {
                reader: &mut reader,
                terminal,
            }),
            locale: Locale::default(),
            timezone: Some("Europe/London".into()),
        };
        let options = Options {
            folder: Some(folder.clone()),
            yes: true,
        };
        let mut out = Vec::new();
        assert_eq!(run(&options, machine, &mut out, false), 0);
        assert_eq!(reader, b"Typed\n");
        assert!(read(&folder, "statac.yaml").starts_with("title: Yes Unread\n"));
    }
}

#[test]
fn the_language_comes_from_the_locale_in_its_order() {
    let cases = [
        (
            locale(Some("fr_FR.UTF-8"), Some("de_DE"), Some("es_ES")),
            "fr-FR",
        ),
        (
            locale(None, Some("de_DE.UTF-8@euro"), Some("es_ES")),
            "de-DE",
        ),
        (locale(None, None, Some("en_GB.UTF-8")), "en-GB"),
        (locale(Some("C"), None, Some("ja_JP.UTF-8")), "ja-JP"),
        (locale(Some("C.UTF-8"), Some("POSIX"), None), "en"),
        (locale(None, None, Some("C")), "en"),
        (locale(None, None, None), "en"),
        (locale(Some(""), None, Some("ko")), "ko"),
    ];
    for (at, (locale, language)) in cases.into_iter().enumerate() {
        assert_eq!(
            locale.language().unwrap_or_else(|| "en".into()),
            language,
            "{locale:?}"
        );
        let folder = scratch(&format!("locale-{at}"));
        let (said, code) = run_with(&folder, "", false, true, locale, Some("Europe/London"));
        assert_eq!(code, 0, "{said}");
        let written = read(&folder, "statac.yaml");
        assert!(
            written.contains(&format!("\nlanguage: {language}\n")),
            "{written}"
        );
    }
}

#[test]
fn a_machine_with_no_time_zone_is_told_to_name_one() {
    let folder = scratch("no-zone");
    let (said, code) = run_with(&folder, "", false, true, Locale::default(), None);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(
            ": 2 files.\nThis machine doesn't say where it is: add `timezone: \
             Europe/London`, with your own, to `statac.yaml`.\n\nstatac.yaml:\n"
        ),
        "{said}"
    );
    assert!(!read(&folder, "statac.yaml").contains("timezone"));
}

#[test]
fn a_language_with_no_words_offers_dates_in_figures_and_says_where_words_go() {
    let folder = scratch("no-words");
    let (said, code) = asked(&folder, "\n\nsw\n\n");
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(
            "  [en] > \n\
             Statac has no translations for `sw`: write its months and days in \
             `languages/sw.yaml`.\n\
             More: https://statac.dev/dates-and-languages/#languages\n\
             \n\
             How should dates look?\n\
             \x20 1  2025-12-31\n\
             \x20 Or type an example of your own.\n\
             \x20 [1] > \n"
        ),
        "{said}"
    );
    assert!(read(&folder, "statac.yaml").contains("\ndate_format: 2025-12-31\n"));
    assert!(!said.contains("translate them"), "{said}");
    let folder = scratch("no-words-yes");
    let (said, code) = run_with(
        &folder,
        "",
        false,
        true,
        locale(None, None, Some("sw_KE.UTF-8")),
        Some("Africa/Nairobi"),
    );
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with(
            "Statac has no translations for `sw-KE`: write its months and days in \
             `languages/sw-KE.yaml`.\n\
             More: https://statac.dev/dates-and-languages/#languages\n\
             \n\
             Made a new site in `"
        ),
        "{said}"
    );
}

#[test]
fn every_example_offered_reads_in_its_language() {
    for tag in [
        "ar", "de", "en", "en-GB", "es", "fa", "fr", "ja", "ko", "sw",
    ] {
        let words = date_words(tag);
        for (number, example) in dates_offered(tag) {
            let read = Format::for_the_site(&example, &words);
            assert!(read.is_ok(), "{tag} {number} `{example}`: {read:?}");
        }
    }
    assert_eq!(dates_offered("ar")[0].1, "31 ديسمبر 2025");
    assert_eq!(dates_offered("fa")[0].1, "۳۱ دسامبر ۲۰۲۵");
}

#[test]
fn the_first_example_offered_is_the_languages_own_way() {
    for (tag, own) in [
        ("de", "31. Dezember 2025"),
        ("es", "31 de diciembre de 2025"),
        ("ja", "2025年12月31日"),
        ("ko", "2025년 12월 31일"),
        ("fr", "31 décembre 2025"),
    ] {
        assert_eq!(dates_offered(tag)[0].1, own, "{tag}");
    }
}

#[test]
fn the_name_offered_is_the_folders_read_as_words() {
    assert_eq!(as_words("my-site"), "My Site");
    assert_eq!(as_words("notes_2026"), "Notes 2026");
    assert_eq!(as_words("2026"), "Site");
    assert_eq!(as_words("..."), "Site");
    assert_eq!(
        Defaults::of("sites/my-site", &Locale::default()).title,
        "My Site"
    );
}

#[test]
fn a_name_needing_quotes_reads_back_as_written() {
    for title in [
        "Site: 1",
        "1984",
        "yes",
        "#1",
        "\"Quoted\"",
        "A \\ B",
        " Spaced ",
        "null",
    ] {
        let mut sources = Sources::default();
        let text = format!("title: {}\n", yaml::write::text(title));
        let source = sources.add(config::FILE_NAME, text.clone());
        let (settings, problems) = config::read_source(&sources, source);
        assert!(problems.is_empty(), "{text}: {problems:?}");
        assert_eq!(settings.text("title").unwrap().value, title, "{text}");
    }
    assert_eq!(yaml::write::text("My Site"), "My Site");
    assert_eq!(yaml::write::text("Site: 1"), "\"Site: 1\"");
}

#[test]
fn the_settings_written_read_back_with_no_problem_for_every_combination() {
    for languages in [&["en-GB"][..], &["ar", "en"], &["sw"]] {
        for address in [None, Some("https://example.org")] {
            for timezone in [None, Some("Europe/London")] {
                let answers = answers(languages, address);
                let text = settings(&answers, timezone);
                let mut sources = Sources::default();
                let source = sources.add(config::FILE_NAME, text.clone());
                let (read, problems) = config::read_source(&sources, source);
                assert!(problems.is_empty(), "{text}: {problems:?}");
                assert_eq!(read.text("title").unwrap().value, "Site 1");
                let tags: Vec<&str> = read.languages().iter().map(|tag| tag.value).collect();
                assert_eq!(tags, languages, "{text}");
                assert_eq!(read.text("url").map(|url| url.value), address);
                assert_eq!(read.text("timezone").map(|zone| zone.value), timezone);
                assert!(read.text("feed_key").is_some());
                let names: Vec<&str> = (text.lines())
                    .filter_map(|line| line.split_once(':'))
                    .map(|(name, _)| name)
                    .filter(|name| !name.starts_with('#'))
                    .collect();
                let written = ["title", "url", "language", "timezone", "date_format"];
                for name in &names {
                    assert!(written.contains(name) || *name == "feed_key", "{text}");
                }
            }
        }
    }
}

#[test]
fn no_setting_is_written_at_statacs_own_value() {
    let text = settings(&answers(&["en"], None), None);
    for line in text.lines().filter(|line| !line.starts_with('#')) {
        let (name, value) = line.split_once(": ").unwrap();
        assert_ne!(
            preset(name)
                .map(|preset| yaml::write::text(&preset))
                .as_deref(),
            Some(value)
        );
    }
}

#[test]
fn two_sites_have_different_feed_keys() {
    let (one, two) = (scratch("key-1"), scratch("key-2"));
    let answers = answers(&["en"], None);
    make(Maker::New, &one, &answers, None).unwrap();
    make(Maker::New, &two, &answers, None).unwrap();
    assert_ne!(feed_key(&one), feed_key(&two));
}

#[test]
fn a_folder_that_isnt_there_is_made_with_the_folders_above_it() {
    let outer = scratch("outer");
    let folder = outer.join("sites").join("site-1");
    let answers = answers(&["en"], None);
    let made = make(Maker::New, &folder, &answers, None).unwrap();
    assert_eq!(made.files, 2);
    assert!(folder.join("statac.yaml").is_file());
    assert!(folder.join("content").is_dir());
}

/// A folder holding `names`, each an empty file.
fn holding(label: &str, names: &[&str]) -> Scratch {
    let folder = scratch(label);
    std::fs::create_dir_all(&folder).unwrap();
    for name in names {
        std::fs::write(folder.join(name), "").unwrap();
    }
    folder
}

fn refused(folder: &Path) -> String {
    let before = listed(folder);
    let (said, code) = run_with(folder, "", false, true, Locale::default(), None);
    assert_eq!(code, 1, "{said}");
    assert_eq!(listed(folder), before, "{said}");
    said
}

#[test]
fn a_folder_holding_anything_is_refused_naming_the_first_four() {
    let folder = holding("one-name", &["a.txt"]);
    let shown = folder.to_string_lossy();
    assert_eq!(
        refused(&folder),
        format!(
            "error  Folder not empty: `{shown}` holds `a.txt`\n\
             \x20 Give `statac new` an empty folder, or the name of a new one.\n\
             \x20 More: https://statac.dev/e/folder-not-empty/\n"
        )
    );
    let folder = holding("four-names", &["d.txt", "c.txt", "b.txt", "a.txt"]);
    assert!(refused(&folder).contains("holds `a.txt`, `b.txt`, `c.txt` and `d.txt`\n"));
    let names = [
        "g.txt", "f.txt", "e.txt", "d.txt", "c.txt", "b.txt", "a.txt",
    ];
    let folder = holding("seven-names", &names);
    assert!(refused(&folder).contains("holds `a.txt`, `b.txt`, `c.txt`, `d.txt` and 3 more\n"));
    let folder = holding("hidden", &[".hidden"]);
    assert!(refused(&folder).contains("holds `.hidden`\n"));
}

#[test]
fn a_folder_is_a_site_by_its_settings_spelt_as_statac_spells_them_or_by_its_writing() {
    let mark = |folder: &Scratch| site_mark(folder).unwrap();
    let settings = holding("mark-settings", &["a.txt", "statac.yaml"]);
    assert_eq!(mark(&settings), Some(SiteMark::Settings));
    // Found by that name too where the disk ignores case, but not the file.
    let capitals = holding("mark-capitals", &["Statac.yaml"]);
    assert_eq!(mark(&capitals), None);
    let writing = holding("mark-writing", &["Statac.yaml"]);
    std::fs::create_dir(writing.join("content")).unwrap();
    assert_eq!(mark(&writing), Some(SiteMark::Content));
    assert_eq!(mark(&holding("mark-none", &["a.txt"])), None);
}

#[test]
fn a_site_is_refused_by_its_settings_or_its_writing() {
    let folder = holding("settings", &["statac.yaml", "a.txt"]);
    let shown = folder.to_string_lossy().into_owned();
    assert_eq!(
        refused(&folder),
        format!(
            "error  Already a site: `{shown}` has a `statac.yaml`\n\
             \x20 More: https://statac.dev/e/already-a-site/\n"
        )
    );
    let folder = scratch("writing");
    std::fs::create_dir_all(folder.join("content")).unwrap();
    let shown = folder.to_string_lossy().into_owned();
    assert!(refused(&folder).starts_with(&format!(
        "error  Already a site: `{shown}` has a `content/` folder\n"
    )));
}

#[test]
fn a_file_in_place_of_the_folder_is_refused() {
    let folder = scratch("a-file");
    std::fs::write(&folder, "").unwrap();
    let (said, code) = run_with(&folder, "", false, true, Locale::default(), None);
    assert_eq!(code, 1);
    assert!(
        said.contains(": a file is in the way of a folder\n"),
        "{said}"
    );
    assert_eq!(std::fs::read(&folder).unwrap(), b"");
}

#[cfg(unix)]
#[test]
fn a_link_in_place_of_the_folder_or_inside_it_is_refused() {
    use std::os::unix::fs::symlink;
    let target = scratch("link-target");
    std::fs::create_dir_all(&target).unwrap();
    let folder = scratch("link");
    symlink(&target, &folder).unwrap();
    let (said, code) = run_with(&folder, "", false, true, Locale::default(), None);
    assert_eq!(code, 1);
    assert!(said.contains(": it is a link\n"), "{said}");
    assert!(listed(&target).is_empty());
    let folder = scratch("link-inside");
    std::fs::create_dir_all(&folder).unwrap();
    symlink(&target, folder.join("content-1")).unwrap();
    assert!(refused(&folder).contains("holds `content-1`\n"));
    assert!(listed(&target).is_empty());
}

#[test]
fn a_write_that_fails_part_way_leaves_the_folder_as_found() {
    let answers = answers(&["fr", "en"], None);
    let empty = scratch("fails-empty");
    std::fs::create_dir_all(&empty).unwrap();
    let missing_in = scratch("fails-missing");
    let missing = missing_in.join("site-1");
    for folder in [&empty, &missing] {
        for failing in [0, 1, 2] {
            let mut written = 0;
            let mut write = |root: &Path, path: &str, bytes: &[u8]| {
                if written == failing {
                    return Err(io::Error::from(io::ErrorKind::StorageFull));
                }
                written += 1;
                platform::make_file(root, path, bytes)
            };
            let problem = make_with(Maker::New, folder, &answers, None, &mut write).unwrap_err();
            assert!(
                matches!(&problem.kind, Kind::CantWrite { reason, .. } if *reason == Reason::DiskFull),
                "{problem:?}"
            );
            assert_eq!(written, failing);
            match folder == &*empty {
                true => assert!(listed(folder).is_empty(), "{failing}"),
                false => assert!(!folder.parent().unwrap().exists(), "{failing}"),
            }
        }
    }
}

#[test]
fn a_refused_address_is_told_under_the_answer_and_asked_again_without_the_question() {
    let folder = scratch("address-again");
    let (said, code) = asked(&folder, "\nexample\nhttps://example.org\n\n");
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(
            "What will its URL be?\n\
             \x20 Enter for none yet.\n\
             \x20 > \x20 `example` isn't a full URL. Write it as \
             `https://example.org`.\n\
             \x20 > \n\
             What language is it written in?\n"
        ),
        "{said}"
    );
    assert_eq!(said.matches("What will its URL be?").count(), 1);
}

#[test]
fn a_date_typed_as_an_example_of_ones_own_is_taken_at_the_last_answer() {
    let folder = scratch("own-date-shown");
    let (said, code) = asked(&folder, "\n\n\nWed 31 Dec 2025\n");
    assert_eq!(code, 0, "{said}");
    assert!(
        said.contains(
            "\x20 Or type an example of your own.\n\
             \x20 [1] > \nMade a new site in `"
        ),
        "{said}"
    );
    assert!(read(&folder, "statac.yaml").contains("\ndate_format: Wed 31 Dec 2025\n"));
}

/// What `statac new` says with `colour` as given, taking every default.
fn asked_with_colour(folder: &Path, colour: bool) -> String {
    let mut reader = "\n\n\n\n".as_bytes();
    let machine = Machine {
        input: Some(Input {
            reader: &mut reader,
            terminal: true,
        }),
        locale: Locale::default(),
        timezone: Some("Europe/London".into()),
    };
    let options = Options {
        folder: Some(folder.to_path_buf()),
        yes: false,
    };
    let mut out = Vec::new();
    let code = run(&options, machine, &mut out, colour);
    assert_eq!(code, 0);
    String::from_utf8(out).unwrap()
}

#[test]
fn colour_picks_out_the_question_the_hints_the_default_and_the_mark() {
    let said = asked_with_colour(&scratch("coloured"), true);
    assert!(
        said.contains("\n\x1b[1mWhat is the site called?\x1b[0m\n"),
        "{said:?}"
    );
    assert!(
        said.contains("\n  \x1b[2mEnter for none yet.\x1b[0m\n"),
        "{said:?}"
    );
    assert!(
        said.contains("\n  \x1b[2m1  31 December 2025\x1b[0m\n"),
        "{said:?}"
    );
    assert!(said.contains("  \x1b[2m[en]\x1b[0m \x1b[38;"), "{said:?}");
    assert!(said.contains(">\x1b[0m \n"), "{said:?}");
    // The address has no default to show.
    assert!(said.contains("\n  \x1b[38;"), "{said:?}");
}

#[test]
fn nothing_is_coloured_where_colour_is_off() {
    let said = asked_with_colour(&scratch("uncoloured"), false);
    assert!(!said.contains('\x1b'), "{said:?}");
    assert!(said.contains("\n  [Uncoloured] > \n"), "{said:?}");
}

/// The explanation of words a language lacks shows the lines a new site's
/// file for a language holds, as they are written there.
#[test]
fn the_example_of_missing_words_is_a_new_sites_language_file() {
    let shown = crate::language::tests::example(crate::problem::Code::MissingWords);
    let written = language_file();
    let (comment, lines) = written.split_once('\n').unwrap();
    assert!(comment.starts_with("# "), "{comment}");
    assert_eq!(shown, lines);
}
