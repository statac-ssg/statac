use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::problem::{Expected, Found, Word, WordList};
use crate::yaml::{Node, Value as Yaml};

/// A site written to a fresh folder, removed when dropped.
pub(crate) struct TempSite {
    pub(crate) root: PathBuf,
}

impl TempSite {
    pub(crate) fn new(files: &[(&str, &str)]) -> TempSite {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "statac-language-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        TempSite { root }
    }
}

impl Drop for TempSite {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// Reads `text` as a language file: its words, and each problem's kind
/// and the text it is at.
fn file(text: &str) -> (Words, Vec<(Kind, String)>) {
    let mut sources = Sources::default();
    let id = sources.add("languages/xx.yaml", text.to_owned());
    let (words, problems) = read(&sources, id);
    let problems = problems
        .into_iter()
        .map(|p| (p.kind, sources.spelling(p.place.unwrap()).to_owned()))
        .collect();
    (words, problems)
}

/// The language a site with `files` names by `tag`, and each problem's
/// name and the text it is at.
fn language(tag: &str, files: &[(&str, &str)]) -> (Language, Vec<(&'static str, String)>) {
    let mut all = vec![("statac.yaml", format!("language: {tag}\n"))];
    all.extend(files.iter().map(|(path, text)| (*path, (*text).to_owned())));
    let all: Vec<(&str, &str)> = all.iter().map(|(p, t)| (*p, t.as_str())).collect();
    let site = TempSite::new(&all);
    let mut sources = Sources::default();
    let (settings, _) = crate::config::read(&site.root, &mut sources);
    let mut problems = Vec::new();
    let languages = Languages::find(&site.root, &mut problems);
    let language = languages.language(settings.text("language"), &mut sources, &mut problems);
    let problems = problems
        .iter()
        .map(|p| {
            let at = p.place.map(|place| sources.spelling(place).to_owned());
            (p.kind.name(), at.unwrap_or_default())
        })
        .collect();
    (language, problems)
}

#[test]
fn every_bundled_language_reads_whole() {
    for (tag, text) in BUNDLED {
        let mut sources = Sources::default();
        let id = sources.add(format!("languages/{tag}.yaml"), (*text).to_owned());
        let (words, problems) = read(&sources, id);
        assert!(problems.is_empty(), "{tag}: {problems:?}");
        assert!(words.months.is_some() && words.weekdays.is_some(), "{tag}");
        assert!(words.quotes.is_some() && words.am_pm.is_some(), "{tag}");
    }
    let english = english();
    assert_eq!(english.months.as_ref().unwrap()[11], "December");
    assert_eq!(english.days.as_ref().unwrap()[0], "1st");
    assert_eq!(english.callout("warning"), Some("Warning"));
    let persian = bundled("fa").unwrap().1;
    assert_eq!(persian.digits.unwrap()[3], '۳');
    assert_eq!(persian.direction, Direction::RightToLeft);
    let french = bundled("FR").unwrap().1;
    assert_eq!(french.months.as_ref().unwrap()[7], "août");
}

/// A file of the Unicode CLDR's JSON release, kept as it is published.
pub(crate) fn cldr(path: &str) -> Node {
    let path = format!("{}/tests/cldr/{path}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"));
    let mut sources = Sources::default();
    let id = sources.add(path.clone(), text);
    crate::yaml::read_json(&sources, id).unwrap_or_else(|problem| panic!("{path}: {problem:?}"))
}

/// The value reached from `node` through each of `keys` in turn.
pub(crate) fn under<'a>(node: &'a Node, keys: &[&str]) -> &'a Node {
    keys.iter().fold(node, |node, key| {
        node.get(key)
            .unwrap_or_else(|| panic!("no {key} in {keys:?}"))
    })
}

/// The text under each of `names`, in their order.
pub(crate) fn texts<const N: usize>(node: &Node, names: [&str; N]) -> [String; N] {
    names.map(|name| match &under(node, &[name]).value {
        Yaml::Scalar(text) => text.clone(),
        other => panic!("{name}: {other:?}"),
    })
}

/// The ten digits of a numbering system, or of one of the four marks of
/// `quotes`, each one character.
fn characters<const N: usize>(text: &str) -> [char; N] {
    let characters: Vec<char> = text.chars().collect();
    characters.try_into().expect("counted")
}

const MONTHS: [&str; 12] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12",
];
const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// What a file of the CLDR's for the language `tag` holds under `keys`.
pub(crate) fn main(tag: &str, package: &str, file: &str, keys: &[&str]) -> Node {
    let whole = cldr(&format!("{package}/main/{tag}/{file}.json"));
    under(under(&whole, &["main", tag]), keys).clone()
}

/// The CLDR's Gregorian calendar for the language `tag`.
pub(crate) fn calendar(tag: &str) -> Node {
    let keys = ["dates", "calendars", "gregorian"];
    main(tag, "cldr-dates-full", "ca-gregorian", &keys)
}

/// The CLDR's quotation marks for the language `tag`.
fn quotes(tag: &str) -> Quotes {
    let delimiters = main(tag, "cldr-misc-full", "delimiters", &["delimiters"]);
    let [open, close, inner_open, inner_close] = texts(
        &delimiters,
        [
            "quotationStart",
            "quotationEnd",
            "alternateQuotationStart",
            "alternateQuotationEnd",
        ],
    )
    .map(|mark| characters::<1>(&mark)[0]);
    Quotes {
        open,
        close,
        inner_open,
        inner_close,
    }
}

/// The last example in YAML of the explanation of `code`.
pub(crate) fn example(code: crate::problem::Code) -> &'static str {
    let (_, example) = code
        .explanation()
        .rsplit_once("```yaml\n")
        .expect("an example");
    example.split_once("```").expect("a closed example").0
}

/// The lines a language's file holds and the words they give, as the
/// Unicode CLDR has them. A line is left out where it would say what
/// Statac takes a file without it to mean: a month alone written as it is
/// in a date, Western digits, lines running left to right.
fn the_cldrs(tag: &str, systems: &Node) -> (Vec<&'static str>, Words) {
    let calendar = calendar(tag);
    let months = |form, width| texts(under(&calendar, &["months", form, width]), MONTHS);
    let weekdays = |width| texts(under(&calendar, &["days", "format", width]), WEEKDAYS);
    let [order] = texts(
        &main(tag, "cldr-misc-full", "layout", &["layout", "orientation"]),
        ["characterOrder"],
    );
    let [system] = texts(
        &main(tag, "cldr-numbers-full", "numbers", &["numbers"]),
        ["defaultNumberingSystem"],
    );
    let [digits] = texts(
        under(systems, &["supplemental", "numberingSystems", &system]),
        ["_digits"],
    );

    let mut words = Words {
        months: Some(months("format", "wide")),
        months_short: Some(months("format", "abbreviated")),
        weekdays: Some(weekdays("wide")),
        weekdays_short: Some(weekdays("abbreviated")),
        am_pm: Some(texts(
            under(&calendar, &["dayPeriods", "format", "abbreviated"]),
            ["am", "pm"],
        )),
        quotes: Some(quotes(tag)),
        ..Words::default()
    };
    let alone = months("stand-alone", "wide");
    let short_alone = months("stand-alone", "abbreviated");
    words.months_alone = (words.months.as_ref() != Some(&alone)).then_some(alone);
    words.months_short_alone =
        (words.months_short.as_ref() != Some(&short_alone)).then_some(short_alone);
    words.digits = (system != "latn").then(|| characters(&digits));
    words.direction = match order.as_str() {
        "right-to-left" => Direction::RightToLeft,
        "left-to-right" => Direction::LeftToRight,
        other => panic!("{tag}: {other}"),
    };
    let lines = [
        ("months", true),
        ("months_alone", words.months_alone.is_some()),
        ("months_short", true),
        ("months_short_alone", words.months_short_alone.is_some()),
        ("weekdays", true),
        ("weekdays_short", true),
        ("am_pm", true),
        ("digits", words.digits.is_some()),
        ("quotes", true),
        ("direction", words.direction == Direction::RightToLeft),
    ];
    let held = lines.into_iter().filter(|(_, held)| *held);
    (held.map(|(line, _)| line).collect(), words)
}

/// Statac's words for every language but English are the Unicode CLDR's
/// and nothing else: each file holds the lines the CLDR's own data gives
/// the language, with its values, and no other line.
#[test]
fn every_bundled_line_outside_english_is_the_cldrs() {
    let systems = cldr("cldr-core/supplemental/numberingSystems.json");
    let others: Vec<_> = BUNDLED.iter().filter(|(tag, _)| *tag != ENGLISH).collect();
    assert_eq!(others.len(), 7);
    for (tag, text) in others {
        let (lines, words) = the_cldrs(tag, &systems);
        let mut sources = Sources::default();
        let id = sources.add(format!("languages/{tag}.yaml"), (*text).to_owned());
        let Yaml::Mapping(entries) = crate::yaml::read(&sources, id).unwrap().value else {
            panic!("{tag}");
        };
        let written: Vec<String> = entries
            .iter()
            .map(|entry| crate::content::words::key_name(&entry.key))
            .collect();
        assert_eq!(written, lines, "{tag}");
        let (read, problems) = read(&sources, id);
        assert!(problems.is_empty(), "{tag}: {problems:?}");
        assert_eq!(read, words, "{tag}");
    }
}

/// The explanation of a language Statac has no words for shows a file of
/// Swahili's, and Statac has none of its own for it: the months, the
/// weekdays and the quotation marks shown are the Unicode CLDR's, and no
/// other line is shown.
#[test]
fn the_example_of_a_language_file_is_the_cldrs() {
    assert!(bundled("sw").is_none());
    let shown = example(crate::problem::Code::UnknownLanguage);
    let (read, problems) = file(shown);
    assert!(problems.is_empty(), "{problems:?}");
    let calendar = calendar("sw");
    let the_cldrs = Words {
        months: Some(texts(
            under(&calendar, &["months", "format", "wide"]),
            MONTHS,
        )),
        weekdays: Some(texts(
            under(&calendar, &["days", "format", "wide"]),
            WEEKDAYS,
        )),
        quotes: Some(quotes("sw")),
        ..Words::default()
    };
    assert_eq!(read, the_cldrs);
}

#[test]
fn a_language_file_is_read_into_its_words() {
    let (words, problems) = file(
        "months: [janvier, février, mars, avril, mai, juin, juillet, août,\n  \
         septembre, octobre, novembre, décembre]\n\
         days: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,\n  \
         19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31]\n\
         digits: [۰, ۱, ۲, ۳, ۴, ۵, ۶, ۷, ۸, ۹]\n\
         quotes: « » “ ”\n\
         no_break_after: «\n\
         inside_quotes: \"\\u202F\"\n\
         direction: rtl\n\
         back_to_the_text: back to the text\n\
         callouts: [Remarque, Astuce, Important, Avertissement, Attention]\n",
    );
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(words.callout("tip"), Some("Astuce"));
    assert_eq!(words.callout("aside"), None);
    assert_eq!(words.months.unwrap()[1], "février");
    assert_eq!(words.days.unwrap()[30], "31");
    assert_eq!(words.digits.unwrap()[9], '۹');
    assert_eq!(
        words.quotes,
        Some(Quotes {
            open: '«',
            close: '»',
            inner_open: '“',
            inner_close: '”',
        })
    );
    assert_eq!(words.no_break_after, ['«']);
    assert_eq!(words.inside_quotes, Some('\u{202f}'));
    assert_eq!(words.direction, Direction::RightToLeft);
    assert_eq!(words.back_to_the_text.as_deref(), Some("back to the text"));
    assert!(words.weekdays.is_none());
    let (words, problems) = file("");
    assert!(problems.is_empty());
    assert_eq!(words, Words::default());
}

/// Each refusal of a language file's value, and the text it is at.
#[test]
fn a_value_that_cant_be_read_is_refused_at_its_text() {
    let wrong = |key: &str, expected, found, respelt: Option<&str>| Kind::WrongKind {
        field: key.into(),
        expected,
        found,
        respelt: respelt.map(Into::into),
    };
    let cases = [
        (
            "weekdays: [Mon, Tue]\n",
            wrong(
                "weekdays",
                Expected::Words(WordList::Weekdays),
                Found::Items(2),
                None,
            ),
            "[Mon, Tue]",
        ),
        (
            "am_pm: am, pm\n",
            wrong(
                "am_pm",
                Expected::Words(WordList::AmPm),
                Found::Items(1),
                Some("[am, pm]"),
            ),
            "am, pm",
        ),
        (
            "months: [a, b, c, d, e, f, g, h, i, j, k, B]\n",
            wrong(
                "months",
                Expected::Words(WordList::Months),
                Found::Twice("B".into()),
                None,
            ),
            "B",
        ),
        (
            "digits: [0, 1, 2, 3, 4, 5, 6, 7, 8, 99]\n",
            wrong(
                "digits",
                Expected::OneCharacter,
                Found::Written("99".into()),
                None,
            ),
            "99",
        ),
        (
            "digits: [0, 1, 2, 3, 4, 5, 6, 7, 8, 1]\n",
            wrong(
                "digits",
                Expected::Words(WordList::Digits),
                Found::Twice("1".into()),
                None,
            ),
            "1",
        ),
        (
            "quotes: « »\n",
            wrong(
                "quotes",
                Expected::Words(WordList::Quotes),
                Found::Items(2),
                None,
            ),
            "« »",
        ),
        (
            "callouts: [Note, Tip]\n",
            wrong(
                "callouts",
                Expected::Words(WordList::Callouts),
                Found::Items(2),
                None,
            ),
            "[Note, Tip]",
        ),
        (
            "no_break_before: \"; :: !\"\n",
            wrong(
                "no_break_before",
                Expected::OneCharacter,
                Found::Written("::".into()),
                None,
            ),
            "::",
        ),
        (
            "inside_quotes: x\n",
            wrong(
                "inside_quotes",
                Expected::Space,
                Found::Written("x".into()),
                None,
            ),
            "x",
        ),
        (
            "direction: RTL\n",
            wrong(
                "direction",
                Expected::OneOf(vec!["ltr".into(), "rtl".into()]),
                Found::Written("RTL".into()),
                None,
            ),
            "RTL",
        ),
        (
            "monts: [a]\n",
            Kind::UnknownWord {
                word: Word::Setting,
                written: "monts".into(),
                nearest: Some("months".into()),
            },
            "monts",
        ),
        (
            "- months",
            wrong("languages/xx.yaml", Expected::Fields, Found::List, None),
            "- months",
        ),
    ];
    for (text, kind, at) in cases {
        let (_, problems) = file(text);
        assert_eq!(problems, [(kind, at.to_owned())], "{text}");
    }
    let (words, _) = file("months: [a]\nweekdays: [Mon, Tue, Wed, Thu, Fri, Sat, Sun]\n");
    assert!(words.months.is_none());
    assert!(words.weekdays.is_some());
}

#[test]
fn a_site_names_no_language_and_is_written_in_english() {
    let site = TempSite::new(&[("statac.yaml", "title: Site\n")]);
    let mut sources = Sources::default();
    let (settings, _) = crate::config::read(&site.root, &mut sources);
    let mut problems = Vec::new();
    let languages = Languages::find(&site.root, &mut problems);
    let language = languages.language(settings.text("language"), &mut sources, &mut problems);
    assert!(problems.is_empty());
    assert_eq!(language.whence, Whence::Statac("en"));
    assert_eq!(language.words, english());
}

/// Files by path and text, or problems by name and the text they are at.
type Pairs<'a> = &'a [(&'a str, &'a str)];

#[test]
fn a_tag_is_looked_for_whole_and_then_broader() {
    let own = "months: [a, b, c, d, e, f, g, h, i, j, k, l]\n";
    let cases: [(&str, Pairs, Whence, Pairs); 8] = [
        ("fr", &[], Whence::Statac("fr"), &[]),
        ("en-US", &[], Whence::Statac("en"), &[]),
        (
            "en-US",
            &[("languages/en-GB.yaml", own)],
            Whence::Statac("en"),
            &[],
        ),
        (
            "fr-CA",
            &[("languages/fr.yaml", own)],
            Whence::Site("languages/fr.yaml".into()),
            &[],
        ),
        (
            "fr-CA",
            &[("languages/fr-ca.yaml", own)],
            Whence::Site("languages/fr-ca.yaml".into()),
            &[],
        ),
        (
            "fr",
            &[("languages/fr.yaml", own)],
            Whence::Site("languages/fr.yaml".into()),
            &[],
        ),
        (
            "zh-Hant-TW",
            &[("languages/zh-Hant.yaml", own)],
            Whence::Site("languages/zh-Hant.yaml".into()),
            &[],
        ),
        ("sw", &[], Whence::Nothing, &[("unknown-language", "sw")]),
    ];
    for (tag, files, whence, said) in cases {
        let (language, problems) = language(tag, files);
        assert_eq!(language.whence, whence, "{tag} {files:?}");
        let said: Vec<(&str, String)> = said.iter().map(|(n, at)| (*n, (*at).to_owned())).collect();
        assert_eq!(problems, said, "{tag} {files:?}");
    }
}

#[test]
fn a_sites_own_file_adds_to_statacs_words_key_by_key() {
    let french = bundled("fr").unwrap().1;
    let text = "quotes: „ “ ‚ ‘\nam_pm:\nback_to_the_text: retour au texte\n";
    let (own, problems) = language("fr", &[("languages/fr.yaml", text)]);
    assert!(problems.is_empty(), "{problems:?}");
    let words = &own.words;
    assert_eq!(words.quotes.map(|quotes| quotes.open), Some('„'));
    assert_eq!(words.back_to_the_text.as_deref(), Some("retour au texte"));
    // Written with nothing after its colon, a key gives nothing in place
    // of Statac's.
    assert_eq!(words.am_pm, None);
    assert_eq!(words.months, french.months);
    assert_eq!(words.weekdays_short, french.weekdays_short);
    assert_eq!(words.months_short, french.months_short);
    assert_eq!(own.whence, Whence::Site("languages/fr.yaml".into()));

    // A file for a region adds to Statac's words for its language.
    let (regional, _) = language(
        "fr-CA",
        &[("languages/fr-CA.yaml", "am_pm: [a.m., p.m.]\n")],
    );
    assert_eq!(regional.words.am_pm.as_ref().unwrap()[0], "a.m.");
    assert_eq!(regional.words.months, french.months);

    // A key whose value can't be read keeps Statac's.
    let (unread, problems) = language("fr", &[("languages/fr.yaml", "months: [janvier]\n")]);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(unread.words.months, french.months);
}

#[test]
fn a_file_for_a_language_statac_has_no_words_for_is_the_language_whole() {
    let (language, problems) = language("sw", &[("languages/sw.yaml", "quotes: “ ” ‘ ’\n")]);
    assert!(problems.is_empty(), "{problems:?}");
    assert!(language.words.quotes.is_some());
    assert_eq!(
        *language.words,
        Words {
            quotes: language.words.quotes,
            ..Words::default()
        }
    );
}

#[test]
fn a_regional_tag_reads_its_languages_words_and_says_nothing() {
    let mut sources = Sources::default();
    let id = sources.add("statac.yaml", "language: en-US\n".into());
    let (settings, _) = crate::config::read_source(&sources, id);
    let mut problems = Vec::new();
    let language =
        Languages::default().language(settings.text("language"), &mut sources, &mut problems);
    assert_eq!(language.tag.as_deref(), Some("en-US"));
    assert_eq!(language.whence, Whence::Statac("en"));
    assert!(problems.is_empty(), "{problems:?}");
}
