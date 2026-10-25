use super::*;
use crate::problem::{Expected, Found, Word, WordList};
use crate::testing::TempSite;
use crate::yaml::{Node, Value as Yaml};

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

/// The words Statac's own file for `tag` gives, read over `onto`.
fn statacs_file(tag: &str, onto: Words) -> (Words, Vec<Problem>) {
    let (_, text) = BUNDLED.iter().find(|(known, _)| *known == tag).unwrap();
    let mut sources = Sources::default();
    let id = sources.add(format!("languages/{tag}.yaml"), (*text).to_owned());
    read_statacs(&sources, id, onto)
}

/// The tag Statac's own file for `tag` is read over, if any.
fn bundled_parent(tag: &str) -> Option<&'static str> {
    parents::inherited(tag)
        .skip(1)
        .find_map(bundled)
        .map(|(known, _)| known)
}

#[test]
fn every_bundled_language_reads_whole() {
    for (tag, _) in BUNDLED {
        let onto = bundled_parent(tag).map_or_else(Words::default, |parent| {
            (*bundled(parent).unwrap().1).clone()
        });
        let (words, problems) = statacs_file(tag, onto);
        assert!(problems.is_empty(), "{tag}: {problems:?}");
        assert_eq!(words, *bundled(tag).unwrap().1, "{tag}");
        assert!(words.months.is_some() && words.weekdays.is_some(), "{tag}");
        assert!(words.quotes.is_some() && words.am_pm.is_some(), "{tag}");
        assert!(words.dates.is_some(), "{tag}");
    }
    // A region is read over the tag it inherits from, which comes first.
    assert_eq!(bundled_parent("en-GB"), Some("en-001"));
    assert_eq!(bundled_parent("en-BE"), Some("en-150"));
    assert_eq!(bundled_parent("en-CA"), Some("en"));
    assert_eq!(bundled_parent("es-MX"), Some("es-419"));
    assert_eq!(bundled_parent("es-419"), Some("es"));
    assert_eq!(bundled_parent("en"), None);
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

/// The CLDR's patterns for the dates of the language `tag`.
pub(crate) fn patterns(tag: &str) -> Patterns {
    let calendar = calendar(tag);
    let [full, long, short, figures] = texts(
        under(&calendar, &["dateFormats"]),
        ["full", "long", "medium", "short"],
    );
    let [time] = texts(under(&calendar, &["timeFormats"]), ["short"]);
    let [date_and_time] = texts(under(&calendar, &["dateTimeFormats"]), ["medium"]);
    let available = under(&calendar, &["dateTimeFormats", "availableFormats"]);
    let [day, month_and_year, month_short_and_year, year] =
        texts(available, ["MMMd", "yMMMM", "yMMM", "y"]);
    Patterns {
        full,
        long,
        short,
        figures,
        day,
        time,
        date_and_time,
        month_and_year,
        month_short_and_year,
        year,
    }
}

/// The words for dates of the language `tag` as the CLDR has them, the
/// digits it writes figures in and its patterns. A month alone is left
/// out where it is written as it is in a date, and so are Western
/// digits, as a file leaves them out.
fn the_cldrs_dates(tag: &str) -> Words {
    let calendar = calendar(tag);
    let [system] = texts(
        &main(tag, "cldr-numbers-full", "numbers", &["numbers"]),
        ["defaultNumberingSystem"],
    );
    let systems = cldr("cldr-core/supplemental/numberingSystems.json");
    let [digits] = texts(
        under(&systems, &["supplemental", "numberingSystems", &system]),
        ["_digits"],
    );
    let months = |form, width| texts(under(&calendar, &["months", form, width]), MONTHS);
    let weekdays = |width| texts(under(&calendar, &["days", "format", width]), WEEKDAYS);
    let mut words = Words {
        months: Some(months("format", "wide")),
        months_short: Some(months("format", "abbreviated")),
        weekdays: Some(weekdays("wide")),
        weekdays_short: Some(weekdays("abbreviated")),
        am_pm: Some(texts(
            under(&calendar, &["dayPeriods", "format", "abbreviated"]),
            ["am", "pm"],
        )),
        digits: (system != "latn").then(|| characters(&digits)),
        dates: Some(patterns(tag)),
        ..Words::default()
    };
    let alone = months("stand-alone", "wide");
    let short_alone = months("stand-alone", "abbreviated");
    words.months_alone = (words.months.as_ref() != Some(&alone)).then_some(alone);
    words.months_short_alone =
        (words.months_short.as_ref() != Some(&short_alone)).then_some(short_alone);
    words
}

/// The lines a language's file holds and the words they give, as the
/// Unicode CLDR has them. A line is left out where it would say what
/// Statac takes a file without it to mean: a month alone written as it is
/// in a date, Western digits, lines running left to right.
fn the_cldrs(tag: &str) -> (Vec<&'static str>, Words) {
    let [order] = texts(
        &main(tag, "cldr-misc-full", "layout", &["layout", "orientation"]),
        ["characterOrder"],
    );
    let mut words = Words {
        quotes: Some(quotes(tag)),
        ..the_cldrs_dates(tag)
    };
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
        ("dates", true),
    ];
    let held = lines.into_iter().filter(|(_, held)| *held);
    (held.map(|(line, _)| line).collect(), words)
}

/// Statac's words for every language but English are the Unicode CLDR's
/// and nothing else: each language's file holds the lines the CLDR's own
/// data gives the language, with its values, and no other line. A
/// region's file is held to the CLDR as English's regions are.
#[test]
fn every_bundled_line_outside_english_is_the_cldrs() {
    let others: Vec<_> = (BUNDLED.iter())
        .filter(|(tag, _)| !tag.contains('-') && *tag != ENGLISH)
        .collect();
    assert_eq!(others.len(), 7);
    for (tag, text) in others {
        let (lines, words) = the_cldrs(tag);
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
        let (read, problems) = read_statacs(&sources, id, Words::default());
        assert!(problems.is_empty(), "{tag}: {problems:?}");
        assert_eq!(read, words, "{tag}");
    }
}

/// A language's months, weekdays, words for morning and afternoon, digits
/// and patterns, as `words` holds them.
fn dated(words: &Words) -> Words {
    Words {
        months: words.months.clone(),
        months_alone: words.months_alone.clone(),
        months_short: words.months_short.clone(),
        months_short_alone: words.months_short_alone.clone(),
        weekdays: words.weekdays.clone(),
        weekdays_short: words.weekdays_short.clone(),
        am_pm: words.am_pm.clone(),
        digits: words.digits,
        dates: words.dates.clone(),
        ..Words::default()
    }
}

/// The tags the CLDR's calendars here are for in the languages Statac has
/// words for: each language, and each region written otherwise than the
/// tag it inherits from.
fn tags_held() -> Vec<String> {
    let folder = format!(
        "{}/tests/cldr/cldr-dates-full/main",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut tags: Vec<String> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|tag| tag.split('-').next().and_then(bundled).is_some())
        .collect();
    tags.sort();
    tags
}

/// Every language's words for dates are the Unicode CLDR's, region by
/// region: Statac has a file for the language and for each region the
/// CLDR writes otherwise than the tag it inherits from, holding only the
/// lines that differ, with the CLDR's values.
#[test]
fn every_language_and_its_regions_write_dates_as_the_cldr_does() {
    let held = tags_held();
    let files: Vec<&str> = BUNDLED.iter().map(|(tag, _)| *tag).collect();
    assert_eq!(files, held);
    for tag in &held {
        let words = bundled(tag).unwrap().1;
        assert_eq!(dated(&words), the_cldrs_dates(tag), "{tag}");
        let Some(parent) = bundled_parent(tag) else {
            continue;
        };
        let (own, theirs) = (the_cldrs_dates(tag), the_cldrs_dates(parent));
        let differ = |line: &'static str, differs: bool| differs.then_some(line);
        let mut lines: Vec<&str> = [
            differ("months", own.months != theirs.months),
            differ("months_alone", own.months_alone != theirs.months_alone),
            differ("months_short", own.months_short != theirs.months_short),
            differ(
                "months_short_alone",
                own.months_short_alone != theirs.months_short_alone,
            ),
            differ("weekdays", own.weekdays != theirs.weekdays),
            differ(
                "weekdays_short",
                own.weekdays_short != theirs.weekdays_short,
            ),
            differ("am_pm", own.am_pm != theirs.am_pm),
            differ("digits", own.digits != theirs.digits),
        ]
        .into_iter()
        .flatten()
        .collect();
        let (own, theirs) = (own.dates.unwrap(), theirs.dates.unwrap());
        let patterns: Vec<&str> = [
            differ("full", own.full != theirs.full),
            differ("long", own.long != theirs.long),
            differ("short", own.short != theirs.short),
            differ("figures", own.figures != theirs.figures),
            differ("day", own.day != theirs.day),
            differ("time", own.time != theirs.time),
            differ("date_and_time", own.date_and_time != theirs.date_and_time),
            differ(
                "month_and_year",
                own.month_and_year != theirs.month_and_year,
            ),
            differ(
                "month_short_and_year",
                own.month_short_and_year != theirs.month_short_and_year,
            ),
            differ("year", own.year != theirs.year),
        ]
        .into_iter()
        .flatten()
        .collect();
        if !patterns.is_empty() {
            lines.push("dates");
        }
        assert!(!lines.is_empty(), "{tag} is written as {parent} is");
        let (_, text) = BUNDLED.iter().find(|(known, _)| known == tag).unwrap();
        let mut sources = Sources::default();
        let id = sources.add(format!("languages/{tag}.yaml"), (*text).to_owned());
        let Yaml::Mapping(entries) = crate::yaml::read(&sources, id).unwrap().value else {
            panic!("{tag}");
        };
        let keys = |entries: &[crate::yaml::Entry]| -> Vec<String> {
            (entries.iter())
                .map(|entry| crate::content::words::key_name(&entry.key))
                .collect()
        };
        assert_eq!(keys(&entries), lines, "{tag}");
        if let Some(dates) = entries.last().filter(|_| !patterns.is_empty()) {
            let Yaml::Mapping(written) = &dates.value.value else {
                panic!("{tag}");
            };
            assert_eq!(keys(written), patterns, "{tag}");
        }
    }
}

/// The broader form of its language each region is written as is the
/// Unicode CLDR's, in every language Statac has words for.
#[test]
fn the_broader_form_of_each_region_is_the_cldrs() {
    let parents = cldr("cldr-core/supplemental/parentLocales.json");
    let Yaml::Mapping(parents) =
        &under(&parents, &["supplemental", "parentLocales", "parentLocale"]).value
    else {
        panic!("parent locales");
    };
    let mut the_cldrs: Vec<(String, String)> = (parents.iter())
        .map(|entry| {
            let Yaml::Scalar(parent) = &entry.value.value else {
                panic!("{entry:?}");
            };
            (crate::content::words::key_name(&entry.key), parent.clone())
        })
        .collect();
    the_cldrs.retain(|(tag, parent)| {
        tag.split('-').next().and_then(bundled).is_some() && parent != "und"
    });
    let mut statacs: Vec<(String, String)> = (parents::PARENTS.iter())
        .flat_map(|(parent, regions)| {
            regions
                .iter()
                .map(|region| ((*region).to_owned(), (*parent).to_owned()))
        })
        .collect();
    the_cldrs.sort();
    statacs.sort();
    assert_eq!(statacs, the_cldrs);
    let inherited: Vec<&str> = parents::inherited("en-GB-oxendict").collect();
    assert_eq!(inherited, ["en-GB-oxendict", "en-GB", "en-001", "en"]);
    let inherited: Vec<&str> = parents::inherited("EN-at").collect();
    assert_eq!(inherited, ["EN-at", "en-150", "en-001", "en"]);
    let inherited: Vec<&str> = parents::inherited("es-MX").collect();
    assert_eq!(inherited, ["es-MX", "es-419", "es"]);
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
        // How dates are written is Statac's own, which no site's file writes.
        (
            "dates:\n  long: d MMMM y\n",
            Kind::UnknownWord {
                word: Word::Setting,
                written: "dates".into(),
                nearest: None,
            },
            "dates",
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
    let cases: [(&str, Pairs, Whence, Pairs); 17] = [
        ("fr", &[], Whence::Statac("fr"), &[]),
        ("en-US", &[], Whence::Statac("en"), &[]),
        ("fr-SN", &[], Whence::Statac("fr"), &[]),
        ("fr-CA", &[], Whence::Statac("fr-CA"), &[]),
        // A region is looked for as the broader form of its language the
        // CLDR writes it as.
        ("en-GB", &[], Whence::Statac("en-GB"), &[]),
        ("en-NZ", &[], Whence::Statac("en-001"), &[]),
        ("en-AT", &[], Whence::Statac("en-150"), &[]),
        ("en-GB-oxendict", &[], Whence::Statac("en-GB"), &[]),
        ("es-MX", &[], Whence::Statac("es-MX"), &[]),
        ("es-CU", &[], Whence::Statac("es-419"), &[]),
        // The site's own file wins over Statac's for a narrower tag.
        (
            "en-GB",
            &[("languages/en.yaml", own)],
            Whence::Site("languages/en.yaml".into()),
            &[],
        ),
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

    // A file for a region adds to Statac's words for its language, or for
    // the region where Statac has its own.
    let (regional, _) = language(
        "fr-SN",
        &[("languages/fr-SN.yaml", "am_pm: [a.m., p.m.]\n")],
    );
    assert_eq!(regional.words.am_pm.as_ref().unwrap()[0], "a.m.");
    assert_eq!(regional.words.months, french.months);
    assert_eq!(regional.words.dates, french.dates);
    let canadian = bundled("fr-CA").unwrap().1;
    let (regional, _) = language("fr-CA", &[("languages/fr-CA.yaml", "am_pm:\n")]);
    assert_eq!(regional.words.am_pm, None);
    assert_eq!(regional.words.months_short, canadian.months_short);
    assert_eq!(regional.words.dates, canadian.dates);

    // A file for the language adds to Statac's words for the region.
    let (english, _) = language("en-AU", &[("languages/en.yaml", text)]);
    assert_eq!(
        english.words.months_short,
        bundled("en-AU").unwrap().1.months_short
    );
    assert_eq!(english.words.dates, bundled("en-AU").unwrap().1.dates);
    assert_eq!(english.words.am_pm, None);

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

/// The script of each tag the CLDR gives a likely script for, and
/// whether the CLDR has that script written right to left.
fn the_cldrs_directions() -> (Vec<(String, bool)>, Vec<String>) {
    let scripts = cldr("cldr-core/scriptMetadata.json");
    let Yaml::Mapping(scripts) = &under(&scripts, &["scriptMetadata"]).value else {
        panic!("scripts");
    };
    let right_to_left: Vec<String> = scripts
        .iter()
        .filter(|entry| texts(&entry.value, ["rtl"]) == ["YES"])
        .map(|entry| crate::content::words::key_name(&entry.key))
        .collect();
    let likely = cldr("cldr-core/supplemental/likelySubtags.json");
    let Yaml::Mapping(likely) = &under(&likely, &["supplemental", "likelySubtags"]).value else {
        panic!("likely subtags");
    };
    let tags = likely
        .iter()
        .map(|entry| {
            let Yaml::Scalar(full) = &entry.value.value else {
                panic!("{entry:?}");
            };
            let script = full.split('-').nth(1).expect("a script");
            let tag = crate::content::words::key_name(&entry.key);
            (tag, right_to_left.iter().any(|known| known == script))
        })
        // `und` is no language: the CLDR's guess from a script or a region alone.
        .filter(|(tag, _)| tag.split('-').next() != Some("und"))
        .collect();
    (tags, right_to_left)
}

/// Which way a language is written is the Unicode CLDR's and nothing
/// else: every tag it gives a script for runs the way that script does,
/// and no script, language or region is listed that it doesn't give.
#[test]
fn the_direction_of_every_language_is_the_cldrs() {
    let (tags, scripts) = the_cldrs_directions();
    assert!(tags.len() > 7000, "{}", tags.len());
    for (tag, right_to_left) in &tags {
        let direction = Direction::of(tag);
        assert_eq!(direction == Direction::RightToLeft, *right_to_left, "{tag}");
    }
    assert_eq!(direction::SCRIPTS, scripts);
    let listed = |tag: &&str| tags.iter().any(|(known, right)| known == tag && *right);
    assert!(direction::LANGUAGES.iter().all(listed));
    let known = |tag: &&str| tags.iter().any(|(known, _)| known == tag);
    assert!(direction::REGIONS.iter().all(known));
}

#[test]
fn a_tag_is_read_by_its_script_then_its_region_then_its_language() {
    let cases = [
        ("he", Direction::RightToLeft),
        ("HE-il", Direction::RightToLeft),
        ("ur-PK", Direction::RightToLeft),
        ("pa", Direction::LeftToRight),
        ("pa-PK", Direction::RightToLeft),
        ("pa-Arab", Direction::RightToLeft),
        ("pa-Guru-PK", Direction::LeftToRight),
        ("ug-KZ", Direction::LeftToRight),
        ("ar-Latn", Direction::LeftToRight),
        ("sw", Direction::LeftToRight),
        ("", Direction::LeftToRight),
    ];
    for (tag, direction) in cases {
        assert_eq!(Direction::of(tag), direction, "{tag}");
    }
}

#[test]
fn a_language_statac_has_no_words_for_runs_the_way_its_script_does() {
    let (hebrew, problems) = language("he", &[]);
    assert_eq!(problems, [("unknown-language", "he".to_owned())]);
    assert_eq!(*hebrew.words, Words::wordless("he"));
    assert_eq!(hebrew.words.direction, Direction::RightToLeft);

    // Its own file keeps the direction, and says another where it writes one.
    let (own, problems) = language("he-IL", &[("languages/he.yaml", "quotes: ” ” ’ ’\n")]);
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(own.words.direction, Direction::RightToLeft);
    let (turned, _) = language("he", &[("languages/he.yaml", "direction: ltr\n")]);
    assert_eq!(turned.words.direction, Direction::LeftToRight);
}
