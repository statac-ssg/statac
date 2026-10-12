//! Every sentence Statac says about a problem, and the words problems are
//! drawn and counted with.
//!
//! A headline leads with what happened, as a short label and the specifics,
//! with no full stop. A suggestion is a full sentence, offered only when there
//! is a good one.

use crate::build::{Carried, Published};
use crate::fetch::Refusal;
use crate::markdown;
use crate::platform::{Alike, Unwritable};
use crate::problem::{
    AddressFault, ArgumentFault, Called, CardFileFault, Chooser, Ended, Expected, FontWithThem,
    FormatFault, Found, Held, Hint, Instead, Kind, Lack, ListFault, Maker, Missing, MoreAt, Named,
    NamedIn, Needed, NotDrawn, OptionHint, Own, PictureFault, Piece, PublishedAsText, Reach,
    RecordFault, Relation, Requirer, Root, SetAside, Severity, ShownBy, ShownWords, SiteMark, Sort,
    Stopped, Taken, TakesFault, TemplateFault, Translated, Unheard, Unmatched, Unquietenable,
    Unsupported, Unwritten, Waiting, Word, WordList, WordsFrom, WordsWanted, WrongPlace,
};
use crate::publish::Occupant;
use crate::serve::live::Told;
use crate::serve::watch::Because;
use crate::yaml::{JsonFault, Malformed, Repeated};

const EXPLANATIONS: &str = "https://statac.dev/e/";
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub(crate) fn headline(kind: &Kind) -> String {
    match kind {
        Kind::BrokenLink { target, .. } => {
            format!("Broken link: nothing is published at `{target}`")
        }
        Kind::OtherSpelling { word, written, .. } | Kind::UnknownWord { word, written, .. } => {
            format!("Unknown {}: `{written}`", word_name(*word))
        }
        Kind::AlwaysFalse { name, .. } => format!("Always false: no page has `{name}`"),
        Kind::CantWrite { path, reason } => {
            format!("Can't write `{}`: {reason}", path.display())
        }
        Kind::MalformedYaml { found } => format!("Malformed YAML: {}", malformed(found)),
        Kind::MalformedJson { found } => format!("Malformed JSON: {}", json_fault(found)),
        Kind::DuplicateKey { key, .. } => format!("Duplicate key: `{key}`"),
        Kind::MoreWrittenTwice { written, .. } => {
            format!("More written twice: the excerpt ends at the first `{written}`")
        }
        Kind::AliasLimit { alias, limit } => format!(
            "Alias limit: by `{alias}`, aliases have repeated more than {}",
            match limit {
                Repeated::Values(n) => format!("{} values", grouped(*n)),
                Repeated::TextBytes(n) => format!("{} MB of text", n / 1_000_000),
            }
        ),
        Kind::NestedTooDeep => format!(
            "Nested too deep: the writing here is nested more than {} deep",
            crate::markdown::DEEPEST_NESTING
        ),
        Kind::UnknownYamlTag { tag } => format!("Unknown YAML tag: `{tag}`"),
        Kind::NotData { path } => format!("Not data: `{path}` isn't YAML or JSON"),
        Kind::DataNamedTwice {
            first,
            second,
            name,
        } => format!("Data named twice: `{first}` and `{second}` are both `{name}`"),
        Kind::MergeKey => "Merge key: Statac doesn't copy fields in with `<<`".into(),
        Kind::SameId { id, .. } => format!("Same id: two pages have the id `{id}`"),
        Kind::TwoLists { first, second, .. } => {
            format!("Two lists: this page has both `{first}:` and `{second}:`")
        }
        Kind::UnclosedFrontMatter => "Unclosed front matter: no `---` line ends it".into(),
        Kind::UnknownProblemName { written, .. } => {
            format!("Unknown problem name: `{written}`")
        }
        Kind::CantQuieten { name, because } => match because {
            Unquietenable::Error => format!("Can't quieten: `{name}` stops the build"),
            Unquietenable::LeavesAPageOut => format!("Can't quieten: `{name}` leaves a page out"),
        },
        Kind::UnknownCommand { written, .. } => format!("Unknown command: `{written}`"),
        Kind::MissingCommand => {
            "Missing command: no `build`, `check`, `serve`, `stamp`, `refresh`, `refuse`, `new`, `import` or `explain`".into()
        }
        Kind::UnknownOption { written, hint } => match hint {
            OptionHint::ForOther {
                command, option, ..
            } => format!("Unknown option: `statac {command}` has no `{option}`"),
            OptionHint::ShownAlready => "Unknown option: `statac serve` has no `--drafts`".into(),
            OptionHint::OnlyWithUpdate => format!(
                "Unknown option: `statac new` takes `{}` only with `--update`",
                option_name(written)
            ),
            OptionHint::NotWithUpdate | OptionHint::CardsWithUpdate => format!(
                "Unknown option: `statac new --update` has no `{}`",
                option_name(written)
            ),
            _ => format!("Unknown option: `{written}`"),
        },
        Kind::MissingValue {
            written, option, ..
        } => format!("Missing value: `{written}` needs {}", needs(option)),
        Kind::OptionGivenTwice { option } => format!("Option given twice: `{option}`"),
        Kind::UnexpectedArgument { written, .. } => format!("Unexpected argument: `{written}`"),
        Kind::MissingName => "Missing name: `statac explain` needs the name of a problem".into(),
        Kind::MissingAddress => {
            "Missing address: `statac refuse` needs the address of a webmention or a sender".into()
        }
        Kind::NotAWebAddress { written, .. } => {
            format!(
                "Not a web address: `statac refuse` takes an address starting `https://` or \
                 `http://`, not `{written}`"
            )
        }
        Kind::UnreadableName { written } => {
            format!("Unreadable name: `{written}` holds bytes that aren't UTF-8")
        }
        Kind::UnusableName { written } => {
            format!("Unusable name: `{written}` holds a character that can't be shown")
        }
        Kind::MissingFile { path, .. } => format!("Missing file: there is no `{path}`"),
        Kind::SassStopped { message } => format!("Sass stopped: {}", sentence_part(message)),
        Kind::WorkerOutsideImportMap { written } => {
            format!("Worker outside the import map: `{written}` isn't published under that name")
        }
        Kind::SameAddress {
            first,
            second,
            alike,
            address,
        } => {
            let (first, second) = (named(first), named(second));
            match alike {
                Alike::Exactly => {
                    format!("Same address: {first} and {second} are both published at `{address}`")
                }
                Alike::ByCase => {
                    format!("Same address: {first} and {second} differ only in capitals")
                }
                Alike::ByNormalisation => format!(
                    "Same address: {first} and {second} differ only in how an accent is stored"
                ),
            }
        }
        Kind::LaterPageTaken {
            list,
            number,
            page,
            address,
        } => format!(
            "Same address: {} and {} are both published at `{address}`",
            later(*number, list),
            named(page)
        ),
        Kind::AddressTaken {
            held,
            address,
            taken,
        } => {
            let what = match held {
                Held::Draft => "draft",
                Held::Waiting => "post",
            };
            format!(
                "Address taken: this {what} would be published at `{address}`, as {} is",
                named(taken)
            )
        }
        Kind::UnwritableName { path, why } => unwritable(&visible(path), why),
        Kind::NotStatacsFolder { path, found } => {
            let path = path.display();
            match found {
                Occupant::File => format!("Not Statac's folder: `{path}` is a file"),
                Occupant::Link => format!("Not Statac's folder: `{path}` is a link"),
                Occupant::Entries { files, folders } => {
                    let mut held = Vec::new();
                    if *files > 0 {
                        held.push(how_many(*files, "file", "files"));
                    }
                    if *folders > 0 {
                        held.push(how_many(*folders, "folder", "folders"));
                    }
                    format!(
                        "Not Statac's folder: `{path}` already holds {}",
                        held.join(" and ")
                    )
                }
            }
        }
        Kind::OutputInTheWay { out, folder, whole } => {
            let out = out.display();
            match whole {
                true => {
                    format!("Output folder in the way: `{out}` is the site's `{folder}/` folder")
                }
                false => format!("Output folder in the way: `{out}` is inside `{folder}/`"),
            }
        }
        Kind::CantRead { path, reason } => format!("Can't read `{}`: {reason}", path.display()),
        Kind::CantRemove { path, reason } => {
            format!("Can't remove `{}`: {reason}", path.display())
        }
        Kind::CantListen { port, last, why } => {
            let (ports, one) = match port == last {
                true => (format!("port {port}"), true),
                false => (format!("ports {port} to {last}"), false),
            };
            match (why, one) {
                (Unheard::InUse, true) => {
                    format!("Can't listen on {ports}: another program is using it")
                }
                (Unheard::InUse, false) => {
                    format!("Can't listen on {ports}: other programs are using them")
                }
                (Unheard::System(reason), _) => format!("Can't listen on {ports}: {reason}"),
            }
        }
        Kind::SassWarning { message } => format!("Sass warning: `{message}`"),
        Kind::OutsideSite { reach } => match reach {
            Reach::Load(written) => format!("Outside the site: `{written}` isn't in `assets/`"),
            Reach::CardFile(written) => {
                format!("Outside the site: `{written}` isn't in `templates/og/`")
            }
            Reach::Link {
                path,
                within: Some(folder),
            } => format!("Outside the site: the link `{path}` leads out of `{folder}`"),
            Reach::Link { path, within: None } => {
                format!("Outside the site: the link `{path}` leads out of the site's folder")
            }
        },
        Kind::MalformedCall { found } => format!("Malformed call: {}", malformed_call(found)),
        Kind::MalformedMaths { found } => format!("Malformed maths: {}", malformed_maths(found)),
        Kind::ColonInCall { name } => format!("Colon in call: `{name}:` was read as `{name}=`"),
        Kind::PublishedAsText { written, why } => match why {
            PublishedAsText::Call => {
                format!("Published as text: `{written}` is another tool's call")
            }
            PublishedAsText::Mark => {
                format!("Published as text: `{written}` is another tool's mark")
            }
            PublishedAsText::FrontMatter => {
                format!("Published as text: `{written}` isn't between `---` lines")
            }
        },
        Kind::ControlCharacter { character } => format!(
            "Control character: U+{:04X} is left out of the page",
            *character as u32
        ),
        Kind::ReadAsYourOwn { own, written, .. } => {
            let own = match own {
                Own::Field(_) => "field",
                Own::Setting => "setting",
                Own::Type => "type",
            };
            format!("Read as your own {own}: `{written}`")
        }
        Kind::NestedSetting { key, under, .. } => {
            format!("Nested setting: `{key}` under `{under}`")
        }
        Kind::FieldWrittenTwoWays {
            flat, nested_under, ..
        } => format!("Field written two ways: `{flat}` is also nested under `{nested_under}`"),
        Kind::ReservedName { name } => format!("Reserved name: `{name}` is Statac's own"),
        Kind::SetAside { field, why } => match why {
            SetAside::StatacFills(on) => {
                format!(
                    "Field set aside: Statac fills `{}.{field}` itself",
                    root(*on)
                )
            }
            SetAside::FolderPermalink => {
                "Field set aside: a folder's own page stays at the folder's address".into()
            }
            SetAside::SendsOn => {
                "Field set aside: a page sending readers on publishes only its redirect".into()
            }
            SetAside::NotOnCard { .. } => format!("Field set aside: a card has no `{field}`"),
            SetAside::OwnPicture => {
                "Field set aside: `og_image` names the picture this page is shared with".into()
            }
            SetAside::NothingWritten(Word::Setting) => {
                format!("Setting set aside: `{field}` has nothing after its colon")
            }
            SetAside::NothingWritten(_) => {
                format!("Field set aside: `{field}` has nothing after its colon")
            }
        },
        Kind::WrongKind {
            field,
            expected,
            found,
            ..
        } => {
            // Front matter is no author's text, so it isn't quoted.
            let field = match field.as_ref() {
                FRONT_MATTER => FRONT_MATTER.to_owned(),
                field => format!("`{field}`"),
            };
            format!(
                "Wrong kind of value: {field} is {}, not {}",
                expected_words(expected),
                found_words(found)
            )
        }
        Kind::NamesNothing { written, among, .. } => match among {
            NamedIn::Data(path) => format!("Names nothing: `{path}` has no `{written}`"),
            NamedIn::Folder(folder) => {
                format!("Names nothing: no page in `{folder}` is called `{written}`")
            }
        },
        Kind::NeedsQuotes { written, reading } => {
            format!("Needs quotes: YAML reads `{written}` as {reading}")
        }
        Kind::OneTagNotTwo {
            line, tag, parts, ..
        } => format!(
            "One tag, not {}: `{line}` makes a single tag called `{tag}`",
            in_words(parts.len())
        ),
        Kind::UnclearDate { written, readings } => format!(
            "Unclear date: `{written}` could be {} or {}",
            readings[0].1, readings[1].1
        ),
        Kind::NoSuchDate { missing, .. } => format!(
            "No such date: there is no {}",
            match missing {
                Missing::Day { day, month, year } => {
                    format!("{day} {} in {year}", MONTHS[usize::from(*month) - 1])
                }
                Missing::Month(month) => format!("month {month}"),
                Missing::Time(time) => format!("time `{time}`"),
                Missing::Offset(offset) => format!("offset `{offset}`"),
            }
        ),
        Kind::DatesDisagree { named, written } => {
            format!("Dates disagree: `date` is `{written}`, the file name `{named}`")
        }
        Kind::NotStamped { field, written } => {
            format!("Not stamped: `{field}` is still `{written}`")
        }
        Kind::MissingField { field, .. } => format!("Missing field: `{field}`"),
        Kind::MissingData {
            table, declared, ..
        } => format!("Missing data: there is no `{table}` for `{declared}` to name"),
        Kind::MissingContentFolder {
            folder, declared, ..
        } => format!("Missing folder: there is no `{folder}` for `{declared}` to name"),
        Kind::NoNamesToCheck { path, declared } => format!(
            "No names to check: `{path}` is a list, so `{declared}` can't name one of its items"
        ),
        Kind::WrongPlace { key, why } => match why {
            WrongPlace::EachPagesOwn => format!("Wrong place: `{key}` can't be handed down"),
            WrongPlace::SiteSetting => format!("Wrong place: `{key}` is a site setting"),
            WrongPlace::TopFolder => {
                format!("Wrong place: `{key}` at the top of `content/` reaches every page")
            }
            WrongPlace::NothingInside => {
                format!("Wrong place: `{key}` hands down only from a folder's `index.md`")
            }
            WrongPlace::WholeSite => {
                format!("Wrong place: `{key}` can't be handed down from `statac.yaml`")
            }
            WrongPlace::SiteValue => format!("Wrong place: a page can't set `{key}`"),
            WrongPlace::FolderOnly => {
                format!("Wrong place: `{key}` belongs in a folder's `index.md`")
            }
            WrongPlace::Item => format!("Wrong place: an item can't set `{key}`"),
            WrongPlace::NoCommand(command) => {
                format!("Wrong place: `{key}` is for a site that sets `{command}`")
            }
            WrongPlace::More(at) => match at {
                MoreAt::List => format!("Wrong place: `{key}` can't end an excerpt inside a list"),
                MoreAt::Quotation => {
                    format!("Wrong place: `{key}` can't end an excerpt inside a quotation")
                }
                MoreAt::Call(name) => {
                    format!("Wrong place: `{key}` can't end an excerpt inside `@{name}`")
                }
                MoreAt::Footnote => {
                    format!("Wrong place: `{key}` can't end an excerpt inside a footnote")
                }
                MoreAt::Html => format!("Wrong place: `{key}` can't end an excerpt inside HTML"),
                MoreAt::Element(name) => {
                    format!("Wrong place: `{key}` can't end an excerpt inside `<{name}>`")
                }
                MoreAt::Words => {
                    format!("Wrong place: `{key}` ends an excerpt only on a line of its own")
                }
            },
        },
        Kind::UnusableAddress { fault } => format!("Unusable address: {}", address_fault(fault)),
        Kind::UnusableDateFormat { fault } => {
            format!("Unusable date format: {}", format_fault(fault))
        }
        Kind::NoTimeZone => "No time zone: dates without an offset are read as UTC".into(),
        Kind::HookStopped { hook, ended, .. } => match ended {
            Ended::Code(code) => format!("Hook stopped: `{hook}` ended with exit code {code}"),
            Ended::Cut => format!("Hook stopped: `{hook}` was ended before it finished"),
            Ended::NotStarted(reason) => {
                format!("Hook stopped: can't start a shell for `{hook}`: {reason}")
            }
        },
        Kind::HookNotRun { hooks } => {
            format!("Hook not run: `check` doesn't run {}", quoted_either(hooks))
        }
        Kind::NothingMade { place } => {
            format!("Nothing made: `after_build` left nothing at `{place}`")
        }
        Kind::OutputFolderNamed { folder, .. } => format!(
            "Output folder named: `{folder}` holds the site built last time while \
             `after_build` runs"
        ),
        Kind::UnknownLanguage { tag } => {
            format!("Unknown language: Statac has no words for `{tag}`")
        }
        Kind::LanguageWrittenTwice {
            written,
            first_written,
            ..
        } => match written == first_written {
            true => format!("Language written twice: `{written}`"),
            false => format!(
                "Language written twice: `{first_written}` and `{written}` are one language"
            ),
        },
        Kind::MissingWords {
            tag,
            words,
            pages,
            ..
        } => {
            let pages = how_many(*pages, "page", "pages");
            match words {
                ShownWords::BackToTheText => format!(
                    "Missing words: the footnotes of {pages} in `{tag}` have no words for the \
                     way back"
                ),
                ShownWords::Callouts => {
                    format!("Missing words: the callouts of {pages} in `{tag}` have no names")
                }
            }
        }
        Kind::NoPagesInLanguage { tag } => {
            format!("No pages in `{tag}`: there is no `content/{tag}/` folder")
        }
        Kind::LanguageFolderInPattern {
            pattern, folder, ..
        } => format!("Language folder in pattern: `{pattern}` is read from `{folder}` already"),
        Kind::SameTranslation { tag, by, .. } => match by {
            Translated::Key(key) => {
                format!("Same translation: two pages in `{tag}` have `translation_key: {key}`")
            }
            Translated::Place(path) => {
                format!("Same translation: two pages in `{tag}` stand where `{path}` does")
            }
        },
        Kind::UnmatchedTranslationKey { key, .. } => {
            format!("Unmatched translation key: no page in another language has `{key}`")
        }
        Kind::NotLinkedByPlace { keyed, key } => {
            format!("Not linked by place: `{keyed}` has `translation_key: {key}`")
        }
        Kind::UnknownCodeLanguage { written, .. } => {
            format!("Unknown code language: Statac has no colours for `{written}`")
        }
        Kind::CodeNotColoured { characters, .. } => format!(
            "Code not coloured: a line in this block is {} characters long",
            grouped(*characters)
        ),
        Kind::CodeColoursNotLoaded => {
            "Code colours not loaded: no template asks for `code.css`".into()
        }
        Kind::MalformedTemplate { found } => format!(
            "Malformed template: {}",
            match found {
                TemplateFault::Syntax(detail) => sentence_part(detail),
                TemplateFault::Takes(fault) => takes_fault(fault),
            }
        ),
        Kind::MissingTemplate { path, .. } => format!("Missing template: there is no `{path}`"),
        Kind::NoValue { written, lack, .. } => match lack {
            Lack::OnPage(page) => format!("No value: `{written}` isn't set on {}", named(page)),
            Lack::Unset => format!("No value: nothing is set for `{written}`"),
            Lack::InDataFile(file) => format!("No value: `{written}` isn't set in `{file}`"),
            Lack::InDataFolder(folder) => {
                format!("No value: `{written}` names nothing in `{folder}`")
            }
            Lack::Empty => format!("No value: `{written}` is empty"),
            Lack::InLiteral { map: false } => {
                format!("No value: `{written}` may have no value inside a list")
            }
            Lack::InLiteral { map: true } => {
                format!("No value: `{written}` may have no value inside a map")
            }
            Lack::ByField(by) => {
                let item = match &by.item {
                    Named::Item(item) => format!("`{item}`"),
                    page => named(page),
                };
                format!("No value: {item} has no `{}` for `{}`", by.field, by.filter)
            }
        },
        Kind::NotSupported { what } => match what {
            Unsupported::Statement { statement, .. } => format!("Not supported: `{statement}`"),
            Unsupported::NotYet(name) => format!("Not supported: `{name}` isn't available yet"),
        },
        Kind::WrongArgument { snippet, fault } => format!(
            "Wrong argument: {}",
            match fault {
                ArgumentFault::NoSuchName { given, .. } =>
                    format!("`{snippet}` takes no `{given}`"),
                ArgumentFault::NeedsNames { wanted } if wanted.is_empty() => {
                    format!("`{snippet}` takes no values")
                }
                ArgumentFault::NeedsNames { .. } => format!("`{snippet}` needs each value named"),
                ArgumentFault::TooMany { takes, .. } => format!(
                    "`{snippet}` takes only {} without a name",
                    listed(&quoted(takes))
                ),
                ArgumentFault::Twice(name) => format!("`{snippet}` is given `{name}` twice"),
                ArgumentFault::Supplied(name) => {
                    format!("Statac hands `{snippet}` its `{name}` itself")
                }
                ArgumentFault::NotGiven(name) => format!("`{snippet}` wasn't given `{name}`"),
                ArgumentFault::Needs(needed) => match needed {
                    Needed::Name(sort) => {
                        format!("`{snippet}` needs the name of one {}", sort_name(*sort))
                    }
                    Needed::Names => format!("`{snippet}` needs a list of names"),
                    Needed::Brackets => format!("`{snippet}` needs its brackets"),
                    Needed::Text(option) => format!("`{snippet}` needs `{option}` as text"),
                    Needed::TrueFalse(option) => {
                        format!("`{snippet}` needs `{option}` to be `true` or `false`")
                    }
                    Needed::OnSite(written) => {
                        format!("`{snippet}` needs a file of the site, not `{written}`")
                    }
                    Needed::VideoOn {
                        site,
                        written: Some(written),
                    } => format!("`{snippet}` needs a {site} address or id, not `{written}`"),
                    Needed::VideoOn {
                        site,
                        written: None,
                    } => format!("`{snippet}` needs a {site} address or id"),
                    Needed::Playable { sort, written } => format!(
                        "`{snippet}` needs a {} a browser plays, not `{written}`",
                        sort_name(*sort)
                    ),
                },
            }
        ),
        Kind::TemplateStopped { reason } => format!(
            "Template stopped: {}",
            match reason {
                Stopped::Engine(detail) => sentence_part(detail),
                Stopped::NoSiteAddress => "`absolute` needs the site's `url`".into(),
                Stopped::NoFeedKey(_) => "`feed_id` needs the site's `feed_key`".into(),
                Stopped::NotAPage(function) => format!("`{function}` needs a page"),
                Stopped::NotAnAddress => "`url` needs an address from the top of the site".into(),
                Stopped::NotADate(filter) => format!("`{filter}` needs a date"),
                Stopped::NotAList(filter) => format!("`{filter}` needs a list of pages"),
                Stopped::CallsItself(snippet) => format!("`{snippet}` calls itself"),
                Stopped::ContentInItself => "a page's writing shows itself".into(),
                Stopped::RanOn { template, steps } => format!(
                    "`{template}` ran {} steps without finishing",
                    grouped(steps)
                ),
                Stopped::NotText => "a `!!binary` value holds bytes that aren't text".into(),
                Stopped::CardInWriting => {
                    "`page.card` isn't answered in a page's writing".into()
                }
                Stopped::CardInCard => "`page.card` isn't answered in a card's own text".into(),
            }
        ),
        Kind::PageInSnippet {
            snippet, written, ..
        } => format!("Page in snippet: `{snippet}` shows `{written}`, the page being built"),
        Kind::TextNeverShown {
            extended, block, ..
        } => format!("Text never shown: `{extended}` has no block called `{block}`"),
        Kind::WritingNeverShown { layout, pages } => format!(
            "Writing never shown: {} built by `{layout}` {} writing it never shows",
            how_many(*pages, "page", "pages"),
            if *pages == 1 { "has" } else { "have" }
        ),
        Kind::SpelledTwoWays {
            word,
            spellings,
            chosen,
        } => {
            let each: Vec<String> = spellings
                .iter()
                .map(|(spelling, pages)| {
                    format!("`{spelling}` ({})", how_many(*pages, "page", "pages"))
                })
                .collect();
            let word = word_name(*word);
            let mut label = word.to_owned();
            label[..1].make_ascii_uppercase();
            format!(
                "{label} spelled two ways: {} are one {word}, shown as `{chosen}`",
                listed(&each)
            )
        }
        Kind::DraftLeftOut { path } => format!("Draft left out: `{path}`"),
        Kind::WaitingForItsDay { path, date } => {
            format!("Waiting for its day: `{path}` is dated `{date}`")
        }
        Kind::NumberedFolder { folder } => {
            format!("Numbered folder: `{folder}` is ordered by the number each name opens with")
        }
        Kind::UnorderedPages { names } => format!(
            "Unordered pages: `order` leaves out {}",
            listed(&quoted(names))
        ),
        Kind::MalformedList { fault } => format!(
            "Malformed list: {}",
            match fault {
                ListFault::NotFromThePosts(written) => {
                    format!("a list starts from `site.posts`, not `{written}`")
                }
                ListFault::NothingAfterPipe => "nothing follows `|`".into(),
                ListFault::NotChainable(name) => format!("`{name}` can't choose a list's posts"),
                ListFault::NoBrackets(name) => format!("`{name}` needs brackets"),
                ListFault::Unclosed(opening) => format!("`{opening}` is never closed"),
                ListFault::Values {
                    filter,
                    wants,
                    given,
                } => format!(
                    "`{filter}` takes {}, not {given}",
                    how_many(*wants, "value", "values")
                ),
            }
        ),
        Kind::NotAPost { path, word, name } => {
            format!("Not a post: `{path}` speaks for {}", the_list(*word, name))
        }
        Kind::FeedNotWritten { list, more, why } => {
            let lists = lists(list, *more);
            let need = match more {
                0 => "needs",
                _ => "need",
            };
            match why {
                Unwritten::NoTitle => {
                    format!("Feed not written: {lists} {need} the site's `title`")
                }
                Unwritten::NoTemplate(template) => {
                    format!("Feed not written: {lists} {need} `{template}`")
                }
                Unwritten::PageThere { page, address } => {
                    format!(
                        "Feed not written: {} is published at `{address}`",
                        named(page)
                    )
                }
                Unwritten::Undated => {
                    let has = match more {
                        0 => "has",
                        _ => "have",
                    };
                    format!("Feed not written: {lists} {has} nothing dated")
                }
            }
        }
        Kind::NoFeedKey { .. } => "No feed key: `statac.yaml` has no `feed_key`".into(),
        Kind::NoSiteAddress { waiting } => {
            let what = match waiting {
                Waiting::Page(path) => format!("`{path}` is"),
                Waiting::Feeds { list, more: 0 } => {
                    format!("the feed of {} is", lists(list, 0))
                }
                Waiting::Feeds { list, more } => {
                    format!("the feeds of {} are", lists(list, *more))
                }
                Waiting::Cards => "sharing cards are".into(),
                Waiting::Webmentions => "webmentions are".into(),
            };
            format!("No site address: {what} left out until the site has a `url`")
        }
        Kind::FeedIdChanged { was, .. } => {
            format!("Feed id changed: this post was published at `{was}`")
        }
        Kind::NoLanguage => "No language set: the site is read as English".into(),
        Kind::RobotsBeneathPath { top } => {
            format!("Robots beneath a path: crawlers read only `{top}`")
        }
        Kind::RedirectToItself { written } => {
            format!("Redirect to itself: `{written}` is this page's own address")
        }
        Kind::RedirectedBack { written } => {
            format!("Redirect to itself: `{written}` sends readers back to this page")
        }
        Kind::WritingNotPublished { path } => {
            format!("Writing not published: `{path}` sends readers on")
        }
        Kind::LoopOverEveryPage { written, pages, .. } => {
            let pages = match pages {
                1 => "1 page".to_owned(),
                pages => format!("{} pages", grouped(pages)),
            };
            match written {
                Some(written) => {
                    format!("Loop over every page: `{written}` is gone through on {pages}")
                }
                None => format!(
                    "Loop over every page: {pages} each go through a list of the whole site"
                ),
            }
        }
        Kind::SortedLast {
            field,
            count,
            first,
            ..
        } => {
            let what = match first {
                Named::Item(_) => "items",
                _ => "pages",
            };
            let first = named(first);
            match count {
                1 => format!("Sorted last: {first} has no `{field}`"),
                _ => {
                    let count = grouped(count);
                    format!("Sorted last: {count} {what} have no `{field}`, starting with {first}")
                }
            }
        }
        Kind::CallInHtml { name } => format!("Call in HTML: `@{name}` is built as a snippet"),
        Kind::UntitledPage { page, by, name } => {
            let by = match by {
                Called::Summary => "its `summary`",
                Called::OpeningWords => "its opening words",
                Called::Date => "its date",
                Called::Address => "its address",
            };
            format!("Untitled page: {} is called by {by}, `{name}`", named(page))
        }
        Kind::MissingDefinition { written, .. } => {
            format!("Missing definition: there is no `{written}:` line")
        }
        Kind::MissingAnchor { id, page, .. } => match page {
            Some(page) => format!("Missing anchor: nothing on `{page}` has the id `{id}`"),
            None => format!("Missing anchor: nothing on this page has the id `{id}`"),
        },
        Kind::FootnoteNeverShown { label, .. } => {
            format!("Footnote never shown: nothing points at `[^{label}]`")
        }
        Kind::DefinitionWrittenTwice { label, .. } => {
            format!("Definition written twice: the first `[^{label}]:` line isn't shown")
        }
        Kind::CellNeverShown { cells, columns } => {
            let columns = how_many(*columns, "column", "columns");
            match cells.as_slice() {
                [cell] => format!("Cell never shown: `{cell}` is past the table's {columns}"),
                _ => format!(
                    "Cells never shown: {} are past the table's {columns}",
                    how_many(cells.len(), "cell", "cells")
                ),
            }
        }
        Kind::IdUsedTwice { id, taken } => match taken {
            Taken::Heading(_) => format!("Id used twice: a heading above has the id `{id}`"),
            Taken::Footnote => format!("Id used twice: a footnote has the id `{id}`"),
        },
        Kind::UnmatchedTag { unmatched } => match unmatched {
            Unmatched::NeverClosed(name) => format!("Unmatched tag: `<{name}>` is never closed"),
            Unmatched::CommentNeverClosed => "Unmatched tag: `<!--` is never closed".into(),
            Unmatched::ClosesNothing(name) => {
                format!("Unmatched tag: `</{name}>` closes nothing opened before it")
            }
        },
        Kind::SizeKept {
            written, at, size, ..
        } => format!("Size kept: at `{at}`, `{written}` is drawn at {size}"),
        Kind::UnreadablePicture { path, fault } => match fault {
            PictureFault::CutShort => {
                format!("Unreadable picture: `{path}` is cut short or damaged")
            }
            PictureFault::Unsupported(_) => {
                format!("Unreadable picture: `{path}` is saved in a form Statac doesn't read")
            }
            PictureFault::TooLarge { width, height } => format!(
                "Unreadable picture: `{path}` is {} by {} pixels, too large to open",
                grouped(width),
                grouped(height)
            ),
            PictureFault::CantMake(message) => {
                format!("Unreadable picture: no copy of `{path}` could be made: {message}")
            }
            PictureFault::NotAStill => {
                format!("Unreadable picture: `{path}` isn't a JPEG, a PNG or a still WebP")
            }
        },
        Kind::ColourConverted { path, profile } => match profile {
            Some(profile) => format!(
                "Colour converted: the copies of `{path}` are in sRGB, converted from `{profile}`"
            ),
            None => format!(
                "Colour converted: the copies of `{path}` are in sRGB, converted from its \
                 colour profile"
            ),
        },
        Kind::DescribesNoPicture { picture, .. } => {
            format!("Describes no picture: there is no `{picture}`")
        }
        Kind::CacheNotWritten { path, reason } => format!(
            "Cache not written: can't write `{}`: {reason}",
            path.display()
        ),
        Kind::UndescribedPicture { picture, .. } => {
            format!("Picture with no description: `{picture}`")
        }
        Kind::HeicPicture { path } => {
            format!("HEIC picture: `{path}` is published as it is, with no copies made")
        }
        Kind::LocationPublished { path } => {
            format!("Location published: `{path}` holds where it was taken in its metadata")
        }
        Kind::PicturesOutOfProportion { property } => format!(
            "Pictures out of proportion: `img` has {} `{property}` and no `{}`",
            if property.to_ascii_lowercase().starts_with('i') {
                "an"
            } else {
                "a"
            },
            height_beside(property)
        ),
        Kind::NoStill { host, absent, .. } => match absent {
            false => "No still: no picture has been fetched for this video yet".into(),
            true => format!("No still: {} has no picture for this video", host.site()),
        },
        Kind::MalformedRecord { fault } => format!(
            "Malformed record: {}",
            match fault {
                RecordFault::Opening => "the first line isn't `statac fetched 1`".into(),
                RecordFault::Fields(n) => {
                    format!("a line has {}, not 3", how_many(*n, "field", "fields"))
                }
                RecordFault::Path(path) => format!("`{path}` isn't a path inside `fetched/`"),
                RecordFault::Held(held) => format!("`{held}` is neither a hash nor `absent`"),
                RecordFault::Day(day) => format!("`{day}` isn't a day"),
                RecordFault::Twice(path) => format!("`{path}` has two lines"),
                RecordFault::RefusedOpening => "the first line isn't `statac refused 1`".into(),
                RecordFault::RefusedFields(n) => {
                    format!("a line has {}, not 3", how_many(*n, "field", "fields"))
                }
                RecordFault::Reach(reach) => {
                    format!("`{reach}` is neither `mention` nor `sender`")
                }
                RecordFault::Address(address) => format!("`{address}` isn't a web address"),
            }
        ),
        Kind::TokenInSettings => {
            "Token in settings: `webmention_token` doesn't belong in a file kept with the site"
                .into()
        }
        Kind::WebmentionsOff => "Webmentions off: `webmention()` writes nothing here".into(),
        Kind::MentionOfNoPage { target } => {
            format!("Mention of no page: nothing is published at `{target}`")
        }
        Kind::FolderNotEmpty {
            folder,
            names,
            more,
            ..
        } => {
            let mut held: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
            if *more > 0 {
                held.push(format!("{} more", grouped(*more)));
            }
            format!("Folder not empty: `{folder}` holds {}", listed(&held))
        }
        Kind::CantTellWhatMade {
            folder, several, ..
        } => match several {
            false => format!(
                "Can't tell what made the site: `{folder}` has no settings file Statac knows"
            ),
            true => format!(
                "Can't tell what made the site: `{folder}` has the settings files of more than one tool"
            ),
        },
        Kind::MissingFolder => {
            "Missing folder: `statac import` needs the folder of the site to import".into()
        }
        Kind::AlreadyASite { folder, has, .. } => match has {
            SiteMark::Settings => format!("Already a site: `{folder}` has a `statac.yaml`"),
            SiteMark::Content => format!("Already a site: `{folder}` has a `content/` folder"),
        },
        Kind::NoSiteHere { folder } => {
            format!("No site here: `{folder}` has no `content/` folder and no `statac.yaml`")
        }
        Kind::NewerStarter { files } => match files {
            1 => {
                "Newer starter files: 1 of the files `statac new` wrote has a newer version".into()
            }
            _ => format!(
                "Newer starter files: {} of the files `statac new` wrote have a newer version",
                grouped(*files)
            ),
        },
        Kind::UnusableCardFile { path, fault } => match fault {
            CardFileFault::BackgroundSize { width, height } => format!(
                "Unusable card file: `{path}` is {} by {} pixels, not 1,200 by 630",
                grouped(width),
                grouped(height)
            ),
            CardFileFault::NotAStill => {
                format!("Unusable card file: `{path}` isn't a JPEG, a PNG or a still WebP")
            }
            CardFileFault::Font => {
                format!("Unusable card file: `{path}` doesn't read as a font")
            }
        },
        Kind::CardNotDrawn { why, pages, first } => {
            let first = named(first);
            let (of, starting) = match pages {
                1 => (first, String::new()),
                n => (
                    format!("{} pages", grouped(*n)),
                    format!(", starting with {first}"),
                ),
            };
            let one = *pages == 1;
            match why {
                NotDrawn::NoFont {
                    characters, more, ..
                } => {
                    let needed = match more {
                        0 => characters_listed(characters),
                        more => {
                            let quoted: Vec<String> =
                                characters.iter().map(|c| format!("`{c}`")).collect();
                            format!(
                                "{} and {}",
                                quoted.join(", "),
                                how_many(*more, "other", "others")
                            )
                        }
                    };
                    format!(
                        "Card not drawn: {of} {} {needed}, which no font has{starting}",
                        if one { "needs" } else { "need" }
                    )
                }
                NotDrawn::TooTall {
                    text,
                    size,
                    smallest,
                } => format!(
                    "Card not drawn: the `{text}` of {of} doesn't fit{} at {size} pixels{starting}",
                    if *smallest { ", even" } else { "" }
                ),
                NotDrawn::NoRoom => format!(
                    "Card not drawn: the top and bottom texts of {of} leave no room between \
                     them{starting}"
                ),
            }
        }
        Kind::UnusedCard { name: None } => "Unused card: no layout reads `page.card`".into(),
        Kind::UnusedCard { name: Some(name) } => format!("Unused card: no page picks `{name}`"),
        Kind::ImageMayNotShow { path, holds, pages } => {
            let of = match pages {
                1 => format!("`{path}`"),
                n => format!("the `{path}` of {} pages", grouped(*n)),
            };
            match holds {
                Some(kind) => format!("Sharing image may not show: {of} is {}", a_kind(*kind)),
                None => format!(
                    "Sharing image may not show: {of} is a kind of picture some link previews \
                     don't show"
                ),
            }
        }
    }
}

/// The property that gives a picture its height, in the words `property`
/// gives its width in: `block-size` beside `max-inline-size`.
fn height_beside(property: &str) -> &'static str {
    match property.to_ascii_lowercase().ends_with("inline-size") {
        true => "block-size",
        false => "height",
    }
}

/// A kind of picture, as a file is said to be one.
fn a_kind(kind: crate::pictures::Kind) -> &'static str {
    use crate::pictures::Kind as Picture;
    match kind {
        Picture::Jpeg => "a JPEG",
        Picture::WebP => "a WebP",
        Picture::AnimatedWebP => "an animated WebP",
        Picture::Png => "a PNG",
        Picture::AnimatedPng => "an animated PNG",
        Picture::Gif => "a GIF",
        Picture::Svg => "an SVG",
        Picture::Avif => "an AVIF picture",
        Picture::Heic | Picture::Heif => "a HEIC picture",
    }
}

/// Characters, each quoted: `` `a` and `b` ``.
fn characters_listed(characters: &[char]) -> String {
    let quoted: Vec<String> = characters.iter().map(|c| format!("`{c}`")).collect();
    listed(&quoted)
}

/// A sort of file, as a call needing one names it.
fn sort_name(sort: Sort) -> &'static str {
    match sort {
        Sort::Picture => "picture",
        Sort::File => "file",
        Sort::Video => "video",
        Sort::Recording => "recording",
    }
}

/// What a value was found to be, as the end of a headline.
fn found_words(found: &Found) -> String {
    match found {
        Found::Written(text) => format!("`{text}`"),
        Found::List => "a list".into(),
        Found::Mapping => "`name: value` lines".into(),
        Found::Hidden => "what YAML reads there".into(),
        Found::Items(n) => n.to_string(),
        Found::Twice(word) => format!("`{word}` twice"),
        Found::Nothing => "nothing".into(),
    }
}

/// Lists named by the first and counting the rest: `the tag `Rust` and 2
/// other lists`.
fn lists(first: &(Word, String), more: usize) -> String {
    match more {
        0 => the_list(first.0, &first.1),
        n => format!(
            "{} and {}",
            the_list(first.0, &first.1),
            how_many(n, "other list", "other lists")
        ),
    }
}

/// Something published, or an item, as a sentence names it:
/// `` `content/post-1.md` ``, `the page for the tag `Rust``, `page 2 of
/// `/blog/``, `item 2`.
pub(crate) fn named(named: &Named) -> String {
    match named {
        Named::File(path) => format!("`{path}`"),
        Named::ListPage(Word::Folder, path) if path.is_empty() => "the site's home page".into(),
        Named::ListPage(word, name) => format!("the page for {}", the_list(*word, name)),
        Named::Later { number, list } => later(*number as usize, list),
        Named::Feed(feed) => format!(
            "the feed of {} at `{}`",
            the_list(feed.word, &feed.name),
            feed.address
        ),
        Named::Item(item) => item.clone(),
        Named::MadeAfter(place) => format!("what `after_build` makes at `{place}`"),
    }
}

/// A list's page after its first, by where the first stands.
fn later(number: usize, list: &str) -> String {
    format!("page {number} of `{list}`")
}

/// A list a page speaks for, as the end of a sentence: `the tag `Rust``.
fn the_list(word: Word, name: &str) -> String {
    match (word, name) {
        (Word::Folder, "") => "the site's own list".into(),
        (Word::Folder, path) => format!("the folder `content/{path}/`"),
        (Word::Page, path) => format!("the list on `{path}`"),
        (word, name) => format!("the {} `{name}`", word_name(word)),
    }
}

/// What `--drafts` brought into the run: drafts and posts waiting for their
/// day, built as if published.
pub(crate) fn included(drafts: usize, waiting: usize) -> String {
    let mut what = Vec::new();
    if drafts > 0 {
        what.push(how_many(drafts, "draft", "drafts"));
    }
    if waiting > 0 {
        what.push(how_many(
            waiting,
            "post waiting for its day",
            "posts waiting for their day",
        ));
    }
    format!("Included {} as if published.", listed(&what))
}

/// What `serve` says as it opens of the drafts and posts waiting for
/// their day that it shows and a build leaves out.
pub(crate) fn showing(drafts: usize, waiting: usize) -> String {
    let mut what = Vec::new();
    if drafts > 0 {
        what.push(how_many(drafts, "draft", "drafts"));
    }
    if waiting > 0 {
        what.push(how_many(
            waiting,
            "post waiting for its day",
            "posts waiting for their day",
        ));
    }
    format!(
        "Showing {}, which `statac build` leaves out.",
        listed(&what)
    )
}

/// What the reader of a page's front matter calls the field it reads.
pub(crate) const FRONT_MATTER: &str = "front matter";

/// Why a date format can't be read, as the end of a sentence.
fn format_fault(fault: &FormatFault) -> String {
    match fault {
        FormatFault::Empty { .. } => "it is empty".into(),
        FormatFault::Twice { second, piece, .. } => {
            format!("`{second}` is a second {}", piece_name(*piece))
        }
        FormatFault::NotInReference {
            written, reference, ..
        } => format!("`{written}` isn't in `{reference}`"),
        FormatFault::LoneHour { written, am, pm } => {
            format!("`{written}` is an hour with no {am} or {pm}")
        }
        FormatFault::MinuteWithoutHour { written, .. } => {
            format!("`{written}` is a minute with no hour")
        }
        FormatFault::NoPattern {
            example, reference, ..
        } => format!("nothing in `{example}` is in `{reference}`"),
        FormatFault::OnlyATime { example, .. } => format!("`{example}` is a time with no date"),
        FormatFault::OnlyAWeekday { example, .. } => {
            format!("`{example}` is a weekday with no date")
        }
        FormatFault::Missing {
            example, pieces, ..
        } => {
            let names: Vec<String> = pieces.iter().map(|p| piece_name(*p).to_owned()).collect();
            format!("`{example}` has no {}", either_of(&names))
        }
        FormatFault::TwoDigitYear { example } => {
            format!("`{example}` writes the year in two digits")
        }
        FormatFault::OtherWeekday {
            written,
            day,
            weekday,
            ..
        } => format!("{day} is a {weekday}, not `{written}`"),
        FormatFault::OtherMonth {
            written,
            day,
            month,
            ..
        } => format!("{day} is in {month}, not `{written}`"),
        FormatFault::NoWords { list, from } => match from {
            WordsFrom::Site(path) => format!("`{path}` has no `{}`", list_key(*list)),
            WordsFrom::Statac(tag) | WordsFrom::Nowhere(tag) => {
                format!("Statac has no `{}` for `{tag}`", list_key(*list))
            }
        },
    }
}

fn piece_name(piece: Piece) -> &'static str {
    match piece {
        Piece::Weekday => "weekday",
        Piece::Day => "day",
        Piece::Month => "month",
        Piece::Year => "year",
        Piece::Hour => "hour",
        Piece::Minute => "minute",
        Piece::AmPm => "morning or afternoon",
    }
}

fn list_key(list: WordsWanted) -> &'static str {
    match list {
        WordsWanted::Months => "months",
        WordsWanted::MonthsShort => "months_short",
        WordsWanted::Weekdays => "weekdays",
        WordsWanted::WeekdaysShort => "weekdays_short",
        WordsWanted::Days => "days",
        WordsWanted::AmPm => "am_pm",
    }
}

/// What a list holds, and the other way to write a date without it.
fn list_and_instead(list: WordsWanted) -> (&'static str, &'static str) {
    match list {
        WordsWanted::Months => ("the 12 months", "write the month in figures"),
        WordsWanted::MonthsShort => ("the 12 short months", "write the month in full"),
        WordsWanted::Weekdays => ("the 7 weekdays, Monday first,", "leave the weekday out"),
        WordsWanted::WeekdaysShort => (
            "the 7 short weekdays, Monday first,",
            "leave the weekday out",
        ),
        WordsWanted::Days => ("the 31 days", "write the day as a number"),
        WordsWanted::AmPm => (
            "the words for morning and afternoon",
            "write the time as `15:04`",
        ),
    }
}

/// `a`, `a or b`, `a, b or c`.
fn either_of(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
    }
}

/// Spellings offered, each quoted: `a`, `a or b`, `a, b or c`.
fn quoted_either(items: &[String]) -> String {
    let quoted: Vec<String> = items.iter().map(|i| format!("`{i}`")).collect();
    either_of(&quoted)
}

fn format_suggestion(fault: &FormatFault) -> Option<String> {
    let write = |reference: &str, examples: &[String]| {
        format!(
            "Write `{reference}` the way dates should look, such as {}.",
            quoted_either(examples)
        )
    };
    match fault {
        FormatFault::Empty {
            reference,
            examples,
        }
        | FormatFault::NoPattern {
            reference,
            examples,
            ..
        } => Some(write(reference, examples)),
        FormatFault::Twice { first, piece, .. } => Some(format!(
            "`{first}` is already the {}, so keep one of them.",
            piece_name(*piece)
        )),
        FormatFault::NotInReference { corrected, .. } => {
            corrected.as_ref().map(|c| format!("Did you mean `{c}`?"))
        }
        FormatFault::LoneHour { pm, .. } => Some(format!(
            "Write the time as `15:04`, or add `{pm}`, such as `3.04{pm}`."
        )),
        FormatFault::MinuteWithoutHour { examples, .. } => Some(format!(
            "Add the hour before it, such as {}.",
            quoted_either(examples)
        )),
        FormatFault::OnlyATime { with_date, .. } => {
            Some(format!("Write the date with it, such as `{with_date}`."))
        }
        FormatFault::OnlyAWeekday { with_date, .. } => with_date
            .as_ref()
            .map(|with_date| format!("Write the date with it, such as `{with_date}`.")),
        FormatFault::Missing { pieces, months, .. } => Some(match pieces.as_slice() {
            [Piece::Month] => format!("Write the month as {}.", quoted_either(months)),
            _ => {
                let names: Vec<String> = pieces.iter().map(|p| piece_name(*p).to_owned()).collect();
                format!(
                    "Add the {}, which every date on the site is written with.",
                    listed(&names)
                )
            }
        }),
        FormatFault::TwoDigitYear { .. } => {
            Some("Write the year in full, `2025`, so no date is read in the wrong century.".into())
        }
        FormatFault::OtherWeekday { corrected, .. } | FormatFault::OtherMonth { corrected, .. } => {
            Some(format!("Did you mean `{corrected}`?"))
        }
        FormatFault::NoWords { list, from } => {
            let (holds, instead) = list_and_instead(*list);
            Some(match from {
                WordsFrom::Site(path) => format!(
                    "List {holds} under `{}:` in `{path}`, or {instead}.",
                    list_key(*list)
                ),
                WordsFrom::Nowhere(tag) => format!(
                    "List {holds} under `{}:` in `languages/{tag}.yaml`, or {instead}.",
                    list_key(*list)
                ),
                WordsFrom::Statac(_) => {
                    let mut chars = instead.chars();
                    let first = chars.next().map(|c| c.to_uppercase().to_string());
                    format!("{}{}.", first.unwrap_or_default(), chars.as_str())
                }
            })
        }
    }
}

/// A `{# takes #}` line's fault, as the end of a sentence.
fn takes_fault(fault: &TakesFault) -> String {
    match fault {
        TakesFault::Unclosed => "the `takes` line has no `#}`".into(),
        TakesFault::NamesNothing => "the `takes` line names nothing".into(),
        TakesFault::Missing => "a name is missing from the `takes` line".into(),
        TakesFault::NotAlone(name) => format!("`{name}` must be the only name on its line"),
        TakesFault::NotAName(piece) => format!("`{piece}` isn't a name a snippet can read"),
        TakesFault::Twice(name) => format!("`{name}` is named twice"),
        TakesFault::TooMany => "the `takes` line names more than 2".into(),
        TakesFault::NeverRead(name) => format!("the snippet never reads `{name}`"),
        TakesFault::Supplied(name) => format!("Statac hands every snippet `{name}` itself"),
    }
}

/// Each item in backticks.
fn quoted(items: &[String]) -> Vec<String> {
    items.iter().map(|item| format!("`{item}`")).collect()
}

fn malformed_call(found: &markdown::Malformed) -> String {
    match found {
        markdown::Malformed::Unclosed(opening) => format!("`{opening}` is never closed"),
        markdown::Malformed::UnclosedQuote(quote) => format!("`{quote}` is never closed"),
        markdown::Malformed::TextAfterQuote => "text after the closing quote".into(),
        markdown::Malformed::Name(name) => format!("a snippet can't be called `{name}`"),
        markdown::Malformed::NoEnd(name) => format!("`@{name}` has no `@end`"),
        markdown::Malformed::StrayEnd => "`@end` closes nothing".into(),
        markdown::Malformed::WrapsNothing(name) => {
            format!("`@{name}` isn't on a line of its own")
        }
        markdown::Malformed::NotOneAddress => {
            "`@url` takes one address from the top of the site".into()
        }
    }
}

fn malformed_maths(found: &markdown::MathsFault) -> String {
    use markdown::MathsFault;
    match found {
        MathsFault::UnknownCommand(command) => format!("Statac doesn't know `{command}`"),
        MathsFault::UnknownEnvironment(name) => {
            format!("Statac doesn't know the environment `{name}`")
        }
        MathsFault::UnknownColour(name) => format!("Statac doesn't know the colour `{name}`"),
        MathsFault::Unclosed {
            opening: Some(opening),
            ..
        } => format!("`{opening}` is never closed"),
        MathsFault::Unclosed {
            opening: None,
            closing,
        } => format!("`{closing}` is missing"),
        MathsFault::ClosesNothing(written) => format!("`{written}` closes nothing"),
        MathsFault::Mismatched { written, expected } => format!(
            "`{written}` closes `{}`",
            expected.replacen("\\end", "\\begin", 1)
        ),
        MathsFault::MissingBefore(next) => format!("something is missing before `{next}`"),
        MathsFault::Twice(bound) => format!("two `{bound}` on one thing"),
        MathsFault::NoSuchLabel(label) => format!("no equation is labelled `{label}`"),
        MathsFault::Unreadable(written) => format!("Statac can't read `{written}`"),
    }
}

/// What a group closed by `closing` opens with, as written.
fn opener_of(closing: &str) -> String {
    match closing {
        "}" => "{".into(),
        "]" => "[".into(),
        _ if closing.starts_with("\\right") => "\\left".into(),
        _ => closing.replacen("\\end", "\\begin", 1),
    }
}

fn unwritable(path: &str, why: &Unwritable) -> String {
    match why {
        Unwritable::Device(device) => {
            format!("Unwritable name: `{path}` uses `{device}`, a device name on Windows")
        }
        Unwritable::Character(c) => format!("Unwritable name: Windows refuses `{c}` in `{path}`"),
        Unwritable::Control => format!("Unwritable name: `{path}` holds a control character"),
        Unwritable::TrailingDot => {
            format!("Unwritable name: Windows drops the dot ending a name in `{path}`")
        }
        Unwritable::TrailingSpace => {
            format!("Unwritable name: Windows drops the space ending a name in `{path}`")
        }
        Unwritable::NotText => format!("Unwritable name: `{path}` isn't text"),
    }
}

/// `1 file`, `2 files`.
fn how_many(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", grouped(n), if n == 1 { one } else { many })
}

/// Another program's sentence, to follow a colon: no capital to open it,
/// no full stop to close it.
fn sentence_part(text: &str) -> String {
    let text = text.trim().trim_end_matches('.');
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// A name with its control characters spelled out, since a terminal would
/// act on them rather than show them.
fn visible(text: &str) -> String {
    text.chars()
        .map(|c| match c.is_control() {
            true => c.escape_unicode().to_string(),
            false => c.to_string(),
        })
        .collect()
}

/// What an option's value is, as a phrase and as a placeholder.
fn needs(option: &str) -> &'static str {
    match option {
        "--site" | "--out" => "a folder",
        "--today" => "a date",
        "--url" => "an address",
        "--port" => "a number",
        "--watch" => "a way",
        _ => "a value",
    }
}

fn placeholder(option: &str) -> String {
    let needs = needs(option);
    let named = needs
        .strip_prefix("an ")
        .or_else(|| needs.strip_prefix("a "));
    format!("<{}>", named.unwrap_or(needs))
}

/// `a`, `a and b`, `a, b and c`.
pub(crate) fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn word_name(word: Word) -> &'static str {
    match word {
        Word::Field => "field",
        Word::Setting => "setting",
        Word::FieldKind => "kind",
        Word::AddressPiece => "address piece",
        Word::File => "file",
        Word::TimeZone => "time zone",
        Word::Name => "name",
        Word::Filter => "filter",
        Word::Test => "test",
        Word::Snippet => "snippet",
        Word::Tag => "tag",
        Word::Series => "series",
        Word::Folder => "folder",
        Word::Page => "page",
        Word::Table => "table",
        Word::Marker => "marker",
        Word::Place => "place",
        Word::Card => "card",
        Word::Stamp => "stamp",
    }
}

fn root(on: Root) -> &'static str {
    match on {
        Root::Page => "page",
        Root::Site => "site",
    }
}

fn expected_words(expected: &Expected) -> String {
    match expected {
        Expected::Text => "text".into(),
        Expected::TrueFalse => "`true` or `false`".into(),
        Expected::Number => "a number".into(),
        Expected::WholeNumber { from: 0 } => "a whole number".into(),
        Expected::WholeNumber { from } => format!("a whole number from {from}"),
        Expected::List => "a list".into(),
        Expected::Date => "a date".into(),
        Expected::When => "a date, a year and month, or a year".into(),
        Expected::Address => "an address, such as `https://example.org` or `/about/`".into(),
        Expected::SiteAddress => "a full address, such as `https://example.org`".into(),
        Expected::OneOf(words) => {
            let quoted: Vec<String> = words.iter().map(|w| format!("`{w}`")).collect();
            match quoted.as_slice() {
                [one] => one.clone(),
                [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
                [] => "nothing".into(),
            }
        }
        Expected::LanguageCode => "a language code, such as `en-GB`".into(),
        Expected::TrueFalseOrAddress => "`true`, `false` or an address".into(),
        Expected::Fields => "`name: value` lines".into(),
        Expected::ListAddress => "an address ending in `{slug}/`, such as `/topics/{slug}/`".into(),
        Expected::LaterAddress => {
            "an address opening with `{list}` and holding `{n}`, such as `{list}page/{n}/`".into()
        }
        Expected::Id => "six letters and digits, such as `k3m9x2`".into(),
        Expected::FieldKind => "a kind, such as `text` or `one of a, b`".into(),
        Expected::Words(list) => match list {
            WordList::Months => "12 months, January first".into(),
            WordList::Weekdays => "7 weekdays, Monday first".into(),
            WordList::Days => "31 days, the first first".into(),
            WordList::AmPm => "2 words, the morning's first".into(),
            WordList::Digits => "10 digits, zero first".into(),
            WordList::Quotes => "4 marks, the outer pair then the inner pair".into(),
            WordList::Callouts => "5 names, for note, tip, important, warning and caution".into(),
        },
        Expected::OneCharacter => "one character each".into(),
        Expected::Space => "one space, such as `\"\\u00A0\"`".into(),
        Expected::Table => "a table of items".into(),
        Expected::Between { from, to } => format!("a whole number from {from} to {to}"),
        Expected::RoundedBetween { from, to } => format!("a number from {from} to {to}"),
        Expected::FontFile => "a font file ending `.ttf`, `.otf` or `.ttc`".into(),
        Expected::BuiltPlace => {
            "a file or a folder in the built site, such as `search/` or `feed.json`".into()
        }
        Expected::SitePlace => {
            "a file or a folder in the site, such as `templates/` or `assets/site.css`".into()
        }
        Expected::CardSize => "a size from 1 to 630, such as `36` or `up to 75`".into(),
        Expected::Colour => "a colour such as `#F2E9D3` or `#FFF`, or a template".into(),
        Expected::BuiltColour => "a colour such as `#F2E9D3` or `#FFF`".into(),
        Expected::OwnPicture => "a picture of the site's".into(),
        Expected::FaceName => "a face's file name, such as `0123456789abcdef.jpg`".into(),
        Expected::TableName => "`site.posts` or a table in `data/`, such as `data.people`".into(),
    }
}

/// Why an address can't be written, as the end of a sentence.
pub(crate) fn address_fault(fault: &AddressFault) -> String {
    match fault {
        AddressFault::UnknownPiece { written, .. } => format!("there is no piece `{written}`"),
        AddressFault::Unclosed => "`{` is never closed".into(),
        AddressFault::Stray => "`}` closes nothing".into(),
        AddressFault::Character(c) => format!("`{c}` can't be in a name"),
        AddressFault::Dots(dots) => format!("`{dots}` can't be a name"),
        AddressFault::TrailingSpaceOrDot(' ') => "a name can't end with a space".into(),
        AddressFault::TrailingSpaceOrDot(c) => format!("a name can't end with `{c}`"),
        AddressFault::Device(name) => format!("`{name}` is kept for a device on Windows"),
        AddressFault::Percent => "`%` can't be in a name".into(),
        AddressFault::Hash => "`#` can't be in a name".into(),
        AddressFault::Control => "a control character can't be in a name".into(),
        AddressFault::NoDate { piece } => format!("this page has no date for `{piece}`"),
        AddressFault::FillsNothing { pattern } => {
            format!("`{pattern}` puts every page at one address")
        }
        AddressFault::ItemLacks {
            item: named,
            field,
            piece,
        }
        | AddressFault::PostLacks {
            post: named,
            field,
            piece,
        } => format!("`{named}` has no `{field}` for `{piece}`"),
        AddressFault::FeedAtFolder { .. } => "a feed can't be published at a folder".into(),
        AddressFault::SlashBesidePage { .. } => "a name beside the page can't hold `/`".into(),
    }
}

fn json_fault(found: &JsonFault) -> String {
    match found {
        JsonFault::UnclosedString => "`\"` is never closed".into(),
        JsonFault::UnclosedBracket(open) => format!("`{open}` is never closed"),
        JsonFault::MismatchedBracket { open, close } => format!("`{open}` is closed by `{close}`"),
        JsonFault::UnknownEscape(escape) => format!("unknown escape `{escape}`"),
        JsonFault::HalfCharacter(escape) => {
            format!("`{escape}` is half of a character written in two escapes")
        }
        JsonFault::ControlCharacter(c) => format!(
            "U+{:04X}, a control character JSON allows in text only as an escape",
            u32::from(*c)
        ),
        JsonFault::NotANumber(written) => format!("`{written}` isn't a number as JSON writes one"),
        JsonFault::TrailingComma => "a comma with nothing after it".into(),
        JsonFault::TooDeep => "nested too deeply".into(),
        JsonFault::EndsEarly => "ends too soon".into(),
        JsonFault::Unexpected(text) => format!("unexpected `{text}`"),
    }
}

fn malformed(found: &Malformed) -> String {
    match found {
        Malformed::UnclosedQuote(quote) => format!("`{quote}` is never closed"),
        Malformed::ShallowLine { .. } => {
            "a line of a text in quotes isn't indented far enough".into()
        }
        Malformed::UnclosedBracket(open) => format!("`{open}` is never closed"),
        Malformed::MismatchedBracket { open, close } => {
            format!("`{open}` is closed by `{close}`")
        }
        Malformed::UnknownEscape(escape) => format!("unknown escape `{escape}`"),
        Malformed::TextAfterQuote => "text after the closing quote".into(),
        Malformed::Indentation => "unexpected indentation".into(),
        Malformed::Tab => "a tab where YAML needs spaces".into(),
        Malformed::UnknownAnchor(alias) => format!("`{alias}` refers to no anchor"),
        Malformed::SecondDocument => "more than one document".into(),
        Malformed::TooDeep => "nested too deeply".into(),
        Malformed::EndsEarly => "ends too soon".into(),
        Malformed::ControlCharacter(c) => {
            format!(
                "U+{:04X}, a control character YAML allows nowhere",
                *c as u32
            )
        }
        Malformed::Unexpected(text) => format!("unexpected `{text}`"),
        Malformed::NotBase64 => "the `!!binary` value isn't base64".into(),
    }
}

/// What to do about a control character typed where it can't stand.
fn retype(c: char) -> String {
    match c {
        '\u{80}'..='\u{9f}' => {
            "This is usually a character such as \u{2019} saved in an old encoding: type it again."
                .into()
        }
        _ => "Delete it, or type the character you meant.".into(),
    }
}

fn closing(open: char) -> char {
    match open {
        '[' => ']',
        '{' => '}',
        other => other,
    }
}

/// A count of a few in words, as a sentence says it: `two`, `three`.
fn in_words(n: usize) -> String {
    const WORDS: [&str; 8] = [
        "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    match n.checked_sub(2).and_then(|at| WORDS.get(at)) {
        Some(word) => (*word).to_owned(),
        None => grouped(n),
    }
}

/// `10000` as `10,000`.
fn grouped(n: impl std::fmt::Display) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, digit) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

pub(crate) fn suggestion(kind: &Kind) -> Option<String> {
    match kind {
        Kind::BrokenLink {
            suggestion, inside, ..
        } => suggestion.as_ref().map(|s| match inside {
            true => format!("To link inside the site, write `{s}`."),
            false => format!("Did you mean `{s}`?"),
        }),
        Kind::OtherSpelling {
            word: Word::Marker,
            written,
            ..
        } if written == "+++" => Some("Write the front matter as YAML between `---` lines.".into()),
        Kind::OtherSpelling { ours, .. } => Some(format!("Statac calls this `{ours}`.")),
        Kind::CantWrite { .. } => None,
        Kind::MalformedYaml { found } => match found {
            Malformed::UnclosedQuote(quote) => {
                Some(format!("End the value with another `{quote}`."))
            }
            Malformed::ShallowLine { spaces, .. } => Some(format!(
                "Start each later line of the text with at least {spaces} {}.",
                if *spaces == 1 { "space" } else { "spaces" }
            )),
            Malformed::UnclosedBracket(open) | Malformed::MismatchedBracket { open, .. } => {
                Some(format!("Close it with `{}`.", closing(*open)))
            }
            Malformed::UnknownEscape(_) => {
                Some(r"Write `\\` for a backslash, or use single quotes.".into())
            }
            Malformed::TextAfterQuote => Some("Put the whole value inside the quotes.".into()),
            Malformed::Tab => Some("Use spaces instead.".into()),
            Malformed::ControlCharacter(c) => Some(retype(*c)),
            Malformed::NotBase64 => {
                Some("Write it in base64, or take off `!!binary` to keep it as text.".into())
            }
            Malformed::UnknownAnchor(alias) => Some(format!(
                "Mark the value it repeats with `&{}`, earlier in the file.",
                alias.trim_start_matches('*')
            )),
            _ => None,
        },
        Kind::DuplicateKey { .. }
        | Kind::AliasLimit { .. }
        | Kind::NestedTooDeep
        | Kind::UnknownYamlTag { .. } => None,
        Kind::MoreWrittenTwice { .. } => Some("Delete the later one.".into()),
        Kind::MalformedJson { found } => match found {
            JsonFault::UnclosedString => Some("End the text with another `\"`.".into()),
            JsonFault::UnclosedBracket(open) => Some(format!("Close it with `{}`.", closing(*open))),
            JsonFault::MismatchedBracket { open, .. } => {
                Some(format!("Close it with `{}`.", closing(*open)))
            }
            JsonFault::UnknownEscape(_) => Some("Write `\\\\` for a backslash.".into()),
            JsonFault::ControlCharacter(c) => {
                Some(format!("Write it as `\\u{:04x}`.", u32::from(*c)))
            }
            JsonFault::TrailingComma => Some("Take the comma out.".into()),
            JsonFault::HalfCharacter(_)
            | JsonFault::NotANumber(_)
            | JsonFault::TooDeep
            | JsonFault::EndsEarly
            | JsonFault::Unexpected(_) => None,
        },
        Kind::NotData { .. } => {
            Some("Save it as `.yaml` or `.json`, or move it out of `data/`.".into())
        }
        Kind::DataNamedTwice { .. } => Some("Rename one of them.".into()),
        Kind::MergeKey => Some("Write the fields out in full.".into()),
        Kind::SameId { .. } => Some("Give each page its own id.".into()),
        Kind::TwoLists { .. } => Some("Move one of them to a page of its own.".into()),
        Kind::UnclosedFrontMatter => Some("Add a `---` line after the last field.".into()),
        Kind::UnknownProblemName { suggestion, .. }
        | Kind::UnexpectedArgument { suggestion, .. } => {
            suggestion.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::CantQuieten { .. } => Some("Take it out of `quiet:`.".into()),
        Kind::UnknownCommand { suggestion, .. } => Some(match suggestion {
            Some(s) => format!("Did you mean `statac {s}`?"),
            None => "Run `statac --help` to see the commands.".into(),
        }),
        Kind::MissingCommand => Some("Run `statac --help` to see the commands.".into()),
        Kind::UnknownOption { written, hint } => Some(match hint {
            OptionHint::Similar(s) => format!("Did you mean `{s}`?"),
            OptionHint::ForOther { option, takers, .. } => {
                let takers: Vec<_> = takers.iter().map(|c| format!("`statac {c}`")).collect();
                format!("`{option}` is for {}.", listed(&takers))
            }
            OptionHint::ShownAlready => "`statac serve` shows drafts and posts waiting for their \
                                         day already. `--no-drafts` leaves them out."
                .into(),
            OptionHint::Takes { command, options } => {
                let options: Vec<_> = options.iter().map(|o| format!("`{o}`")).collect();
                match options.len() {
                    0 => format!("`statac {command}` takes no options."),
                    _ => format!("`statac {command}` takes {}.", listed(&options)),
                }
            }
            OptionHint::None => "Run `statac --help` to see the options.".into(),
            OptionHint::OnlyWithUpdate => format!(
                "Add `--update`, or leave `{}` out.",
                option_name(written)
            ),
            OptionHint::NotWithUpdate => format!(
                "`--update` asks nothing, so leave `{}` out.",
                option_name(written)
            ),
            OptionHint::CardsWithUpdate => "Run `statac new --cards` by itself.".into(),
        }),
        Kind::MissingValue { option, value, .. } => Some(match value {
            Some(value) => format!("Write it as `{option}={value}`."),
            None => format!("Write it as `{option}={}`.", placeholder(option)),
        }),
        Kind::OptionGivenTwice { .. } => Some("Keep one of them.".into()),
        Kind::MissingName => Some("The name is the last part of a problem's `More:` link.".into()),
        Kind::MissingAddress => Some(
            "Give the address after the command, as `statac refuse https://a.example/post-1/`."
                .into(),
        ),
        Kind::NotAWebAddress { respelt, .. } => Some(match respelt {
            Some(address) => format!("Did you mean `{address}`?"),
            None => "Give the whole address, as the browser shows it.".into(),
        }),
        Kind::UnreadableName { .. } => None,
        Kind::UnusableName { .. } => Some("Name the folder without it.".into()),
        Kind::MissingFile { hint, .. } => hint.as_ref().map(|hint| match hint {
            Hint::Closest(name) => format!("Did you mean `{name}`?"),
            Hint::InStatic(address) => {
                format!("Link it as `{address}`, where `static/` publishes it under its own name.")
            }
        }),
        Kind::WorkerOutsideImportMap { .. } => Some(
            "Keep the worker and the files it imports in `static/`, which publishes them under \
             their own names."
                .into(),
        ),
        Kind::SameAddress { alike, .. } => Some(match alike {
            Alike::Exactly => "Rename one of them.".into(),
            Alike::ByCase | Alike::ByNormalisation => {
                "Most disks hold them as one file, so rename one of them.".into()
            }
        }),
        Kind::LaterPageTaken { page, .. } => Some(format!(
            "Give {} another address, or put later pages elsewhere with `later_pages` \
             in `statac.yaml`.",
            named(page)
        )),
        Kind::AddressTaken { held, .. } => Some(match held {
            Held::Draft => "Give one of them another `permalink` before it is published.".into(),
            Held::Waiting => "Give one of them another `permalink` before its day comes.".into(),
        }),
        Kind::NotStatacsFolder { .. } => Some(
            "Statac replaces only folders it built, so build into a new folder or empty this \
             one first."
                .into(),
        ),
        Kind::OutputInTheWay { .. } => {
            Some("Build into a folder Statac doesn't read, such as `--out=_site`.".into())
        }
        Kind::OutsideSite { reach } => Some(match reach {
            Reach::Load(_) => "Copy the file into `assets/` and load it from there.".into(),
            Reach::CardFile(_) => {
                "Copy the file into `templates/og/` and name it from there.".into()
            }
            Reach::Link {
                within: Some(folder),
                ..
            } => format!("Put the file itself in `{folder}` instead of a link to it."),
            Reach::Link { within: None, .. } => {
                "Put the file itself in the site instead of a link to it.".into()
            }
        }),
        Kind::SassStopped { .. }
        | Kind::SassWarning { .. }
        | Kind::UnwritableName { .. }
        | Kind::CantRead { .. }
        | Kind::CantRemove { .. } => None,
        Kind::CantListen { .. } => Some("Choose another port with `--port=<number>`.".into()),
        Kind::MalformedCall { found } => Some(match found {
            markdown::Malformed::Unclosed(_) => "Close it with `)`.".into(),
            markdown::Malformed::UnclosedQuote(quote) => {
                format!("End the value with another `{quote}` on the same line.")
            }
            markdown::Malformed::TextAfterQuote => "Put the whole value inside the quotes.".into(),
            markdown::Malformed::Name(_) => {
                r"Name the snippet with the letters a to z, digits and `_`, or write `\@` to show this as text.".into()
            }
            markdown::Malformed::NoEnd(_) => {
                "Put `@end` on a line of its own where the writing it wraps ends.".into()
            }
            markdown::Malformed::StrayEnd => {
                "Take the line out: only a call to a snippet that shows `body` wraps the writing up to an `@end`.".into()
            }
            markdown::Malformed::WrapsNothing(_) => {
                "Write the call on a line of its own, the writing it wraps below it, and `@end` on a line after that.".into()
            }
            markdown::Malformed::NotOneAddress => "Write it as `@url(/about/)`.".into(),
        }),
        Kind::MalformedMaths { found } => {
            use markdown::MathsFault;
            match found {
                MathsFault::Unclosed {
                    opening: Some(_),
                    closing,
                } => Some(format!("Close it with `{closing}`.")),
                MathsFault::ClosesNothing(written) => Some(format!(
                    "Take it out, or open it with `{}` where it starts.",
                    opener_of(written)
                )),
                MathsFault::Mismatched { expected, .. } => {
                    Some(format!("Did you mean `{expected}`?"))
                }
                MathsFault::MissingBefore(_) => Some(
                    r"Give the command before it everything it takes, as in `\frac{1}{2}`.".into(),
                ),
                MathsFault::Twice(bound) => {
                    let example = match bound {
                        '_' => "x_{i_j}",
                        _ => "x^{a^b}",
                    };
                    Some(format!(
                        "Put braces round what goes together, as in `{example}`."
                    ))
                }
                MathsFault::NoSuchLabel(label) => Some(format!(
                    "Write `\\label{{{label}}}` in the equation it points to."
                )),
                MathsFault::UnknownCommand(_)
                | MathsFault::UnknownEnvironment(_)
                | MathsFault::UnknownColour(_)
                | MathsFault::Unclosed { opening: None, .. }
                | MathsFault::Unreadable(_) => None,
            }
        }
        Kind::ColonInCall { name } => Some(format!("Write `{name}=` instead.")),
        Kind::PublishedAsText { why, .. } => Some(
            match why {
                PublishedAsText::Call => {
                    "Call a snippet in its place, written `@name(...)`, or take it out."
                }
                PublishedAsText::Mark => "Take it out, or write the HTML it stood for.",
                PublishedAsText::FrontMatter => {
                    "Put a `---` line above the first field and another below the last."
                }
            }
            .into(),
        ),
        Kind::ControlCharacter { character } => Some(retype(*character)),
        Kind::UnknownWord { nearest, .. } => {
            nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::AlwaysFalse { meant, .. } => Some(format!("Did you mean `{meant}`?")),
        Kind::ReadAsYourOwn {
            own,
            written,
            nearest,
        } => Some(match own {
            Own::Field(name) => format!(
                "Did you mean `{nearest}`? To keep `{written}`, declare it in `statac.yaml` as \
                 `fields.{name}: text`."
            ),
            Own::Setting | Own::Type => format!("Did you mean `{nearest}`?"),
        }),
        Kind::NestedSetting { flat, .. } => Some(format!("Write it flat: `{flat}`.")),
        Kind::FieldWrittenTwoWays {
            on,
            flat,
            nested_under,
            ..
        } => Some(format!(
            "Nest it under `{nested_under}`, so templates read it as `{}.{flat}`.",
            root(*on)
        )),
        Kind::ReservedName { name } => {
            let last = name.rsplit('.').next().unwrap_or(name);
            Some(format!("Give it a name of your own, such as `my_{last}`."))
        }
        Kind::SetAside { field, why, .. } => match why {
            SetAside::StatacFills(_) if field == "slug" || field == "url" => {
                Some("Write `permalink:` to choose the page's address.".into())
            }
            SetAside::StatacFills(_) => {
                Some(format!("Give it a name of your own, such as `my_{field}`."))
            }
            SetAside::FolderPermalink => Some("Rename the folder to move its page.".into()),
            SetAside::SendsOn => {
                Some("Take it out, or take out `redirect_to` to publish the page.".into())
            }
            SetAside::NotOnCard { nearest: Some(name) } => Some(format!("Did you mean `{name}`?")),
            SetAside::NotOnCard { nearest: None } => Some(
                "A card reads `background`, `font` and `padding`, and `text`, `place`, `size` \
                 and `colour` for each text."
                    .into(),
            ),
            SetAside::OwnPicture => {
                Some("Take out `card`, or take out `og_image` to have the card drawn.".into())
            }
            SetAside::NothingWritten(_) => Some("Give it a value, or take the line out.".into()),
        },
        Kind::WrongKind { respelt, .. } => respelt.as_ref().map(|s| format!("Write it as `{s}`.")),
        Kind::NamesNothing { nearest, .. }
        | Kind::MissingData { nearest, .. }
        | Kind::MissingContentFolder { nearest, .. } => {
            nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::NoNamesToCheck { .. } => Some(
            "Give each item a name with its fields beneath, or declare the field as `text`.".into(),
        ),
        Kind::NeedsQuotes { written, .. } => Some(format!(
            "Write it as `\"{}\"` to keep it as written.",
            written.replace('\\', "\\\\").replace('"', "\\\"")
        )),
        Kind::OneTagNotTwo { key, parts, .. } => Some(format!(
            "For {} tags, write `{key}: [{}]`.",
            in_words(parts.len()),
            parts.join(", ")
        )),
        Kind::UnclearDate { readings, .. } => Some(format!(
            "Write it as `{}` or `{}`.",
            readings[0].0, readings[1].0
        )),
        Kind::NoSuchDate { .. } => None,
        Kind::DatesDisagree { .. } => {
            Some("Change one so they match; Statac uses the `date` line.".into())
        }
        Kind::NotStamped { .. } => Some(
            "Run `statac stamp` to write the date in its place, or `statac serve`, which \
             writes it on every save."
                .into(),
        ),
        Kind::MissingField { field, required_by } => Some(match required_by {
            Requirer::EveryPage => {
                format!("Add `{field}:`, which `statac.yaml` requires on every page.")
            }
            Requirer::Type(name) => {
                format!("Add `{field}:`, which `statac.yaml` requires on pages of type `{name}`.")
            }
            Requirer::Each => {
                format!("Add a `{field}:` pattern, such as `/people/{{key}}/`, to give each item its address.")
            }
            Requirer::EachPost => format!(
                "Add a `{field}:` pattern, such as `/cards/{{name}}/`, to give the page made for each post its address."
            ),
            Requirer::Card => format!("Add `{field}:`, which every card needs."),
            Requirer::CardText => format!("Add `{field}:`, which each text on a card needs."),
            Requirer::Mention => format!("Add `{field}:`, which every mention Statac keeps has."),
        }),
        Kind::WrongPlace { key, why } => match why {
            WrongPlace::EachPagesOwn => {
                let bare = key.strip_prefix("pages.").unwrap_or(key);
                Some(format!("Write `{bare}:` on each page instead."))
            }
            WrongPlace::SiteSetting | WrongPlace::TopFolder => {
                Some("Write it in `statac.yaml` instead.".into())
            }
            WrongPlace::NothingInside | WrongPlace::FolderOnly => {
                Some("Move it to the folder's `index.md`.".into())
            }
            WrongPlace::WholeSite => {
                let bare = key.strip_prefix("pages.").unwrap_or(key);
                Some(format!("Write `{bare}:` in a folder's `index.md` instead."))
            }
            WrongPlace::SiteValue => {
                let bare = key.strip_prefix("site.").unwrap_or(key);
                Some(format!("Write `{bare}:` in `statac.yaml` instead."))
            }
            WrongPlace::Item => Some("Write it on a page in `content/` instead.".into()),
            WrongPlace::NoCommand(command) => {
                Some(format!("Add the `{command}:` line it describes, or take it out."))
            }
            WrongPlace::More(MoreAt::Element(name)) => {
                Some(format!("Move it outside the `<{name}>`."))
            }
            WrongPlace::More(_) => {
                Some("Move it to a line of its own between two paragraphs.".into())
            }
        },
        Kind::UnusableAddress { fault } => match fault {
            AddressFault::UnknownPiece { nearest, .. } => {
                nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
            }
            AddressFault::Unclosed => Some("Close it with `}`.".into()),
            AddressFault::Character('\\') => Some("Separate folders with `/`.".into()),
            AddressFault::Character(_) | AddressFault::Control | AddressFault::Stray => {
                Some("Leave it out of the address.".into())
            }
            AddressFault::Percent => Some("Write the character it stands for instead.".into()),
            AddressFault::Hash => Some("Leave out the `#` and what follows it.".into()),
            AddressFault::TrailingSpaceOrDot(_) => {
                Some("Take it off: Windows drops it, and links would miss the page.".into())
            }
            AddressFault::Dots(_) | AddressFault::Device(_) => None,
            AddressFault::NoDate { .. } => Some(
                "Add a `date:` line, or start the file name with a date such as `2026-10-04-`."
                    .into(),
            ),
            AddressFault::FillsNothing { .. } => {
                Some("Add a piece such as `{slug}` so each page has its own.".into())
            }
            AddressFault::ItemLacks { field, piece, .. } if field == "key" => Some(format!(
                "Give the table's items names, or take `{piece}` out of the pattern."
            )),
            AddressFault::ItemLacks { field, piece, .. } => Some(format!(
                "Add `{field}:` to the item, or take `{piece}` out of the pattern."
            )),
            AddressFault::PostLacks { field, piece, .. } => Some(format!(
                "Add `{field}:` to the post, or take `{piece}` out of the pattern."
            )),
            AddressFault::FeedAtFolder { nearest } | AddressFault::SlashBesidePage { nearest } => {
                Some(format!("Did you mean `{nearest}`?"))
            }
        },
        Kind::UnusableDateFormat { fault } => format_suggestion(fault),
        Kind::HookStopped {
            ended: Ended::Code(code),
            ..
        } if crate::platform::command_not_found(*code) => Some(format!(
            "The shell ends with {code} when it can't find a command. Is the command \
             installed, and on the path Statac was run with?"
        )),
        Kind::HookStopped { .. } | Kind::HookNotRun { .. } | Kind::NothingMade { .. } => None,
        Kind::OutputFolderNamed { rewritten, .. } => Some(format!("Write it as `{rewritten}`.")),
        Kind::NoTimeZone => Some(
            "Add a line such as `timezone: Europe/London` to `statac.yaml`, naming the zone \
             your dates are written in."
                .into(),
        ),
        Kind::UnknownLanguage { tag } => Some(format!(
            "Write its months, weekdays and quotation marks in `languages/{tag}.yaml`."
        )),
        Kind::LanguageWrittenTwice { .. } => None,
        Kind::MissingWords { file, words, .. } => Some(format!(
            "Add them to `{file}`, translating the English line `{}`.",
            match words {
                ShownWords::BackToTheText => "back_to_the_text: back to the text",
                ShownWords::Callouts => "callouts: [Note, Tip, Important, Warning, Caution]",
            }
        )),
        Kind::NoPagesInLanguage { tag } => Some(format!(
            "Put the pages written in `{tag}` in `content/{tag}/`."
        )),
        Kind::LanguageFolderInPattern { without, .. } => {
            Some(format!("Write it as `{without}`."))
        }
        Kind::SameTranslation { tag, by, .. } => Some(match by {
            Translated::Key(_) => format!("Give each page in `{tag}` its own key."),
            Translated::Place(_) => "Rename one of them, or give one a `translation_key`.".into(),
        }),
        Kind::UnmatchedTranslationKey { nearest, .. } => {
            nearest.as_ref().map(|key| format!("Did you mean `{key}`?"))
        }
        Kind::NotLinkedByPlace { key, .. } => Some(format!(
            "To link the two, give this page `translation_key: {key}` too."
        )),
        Kind::UnknownCodeLanguage { nearest, .. } => {
            nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::CodeNotColoured { block, .. } => block
            .then(|| "Break the line up, or take the language off the block's first line.".into()),
        Kind::CodeColoursNotLoaded => {
            Some("Add `{{ css(\"code.css\") }}` inside a template's `<head>`.".into())
        }
        Kind::MalformedTemplate { found } => match found {
            TemplateFault::Syntax(_) => None,
            TemplateFault::Takes(fault) => Some(match fault {
                TakesFault::Unclosed | TakesFault::Missing | TakesFault::NamesNothing => {
                    "Write it on one line, as `{# takes photo, alt #}`.".into()
                }
                TakesFault::NotAlone(name) => {
                    format!("Write `{{# takes {name} #}}`, and have calls name anything else.")
                }
                TakesFault::NotAName(_) => {
                    "Write a name as the snippet reads it, in letters, digits and `_`.".into()
                }
                TakesFault::Twice(_) => "Keep one of them.".into(),
                TakesFault::TooMany => {
                    "Have calls name the rest, as `caption=...`.".into()
                }
                TakesFault::NeverRead(_) | TakesFault::Supplied(_) => {
                    "Take it out of the line.".into()
                }
            }),
        },
        Kind::MissingTemplate {
            chosen, nearest, ..
        } => match (nearest, chosen) {
            (Some(nearest), _) => Some(format!("Did you mean `{nearest}`?")),
            (None, Chooser::Default) => {
                Some("Add the template, or choose another with `layout:`.".into())
            }
            (None, Chooser::Written) => None,
        },
        Kind::NoValue { instead, .. } => match instead {
            Instead::Handed(name) => Some(format!("Did you mean `{name}`?")),
            Instead::Default(written) => Some(format!(
                "Write `{written}` to show nothing where it isn't set."
            )),
            Instead::Selected(written) => Some(format!(
                "Write `{written}` to leave out the items without it."
            )),
            Instead::Headline(written) => Some(format!(
                "A page Statac makes has no title: write `{written}` to name it."
            )),
            Instead::Nothing => None,
        },
        Kind::NotSupported { what } => match what {
            Unsupported::Statement { statement, calls } => match (statement.as_str(), &calls[..]) {
                (_, []) => Some("Call the snippet by its name instead.".into()),
                ("from", calls) => Some(format!(
                    "Take this line out, and call {}.",
                    listed(&quoted(calls))
                )),
                (_, calls) => Some(format!(
                    "Call the snippet instead: {}.",
                    listed(&quoted(calls))
                )),
            },
            Unsupported::NotYet(_) => None,
        },
        Kind::WrongArgument { snippet, fault } => match fault {
            ArgumentFault::NoSuchName { nearest, .. } => {
                nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
            }
            ArgumentFault::NeedsNames { wanted } if wanted.is_empty() => {
                Some("Take the values out of the call.".into())
            }
            ArgumentFault::NeedsNames { wanted } => {
                let named: Vec<String> = wanted.iter().map(|w| format!("{w}=...")).collect();
                Some(format!("Write it as `{snippet}({})`.", named.join(", ")))
            }
            ArgumentFault::TooMany { others, .. } => match others.first() {
                Some(first) => Some(format!("Name the rest, as `{first}=...`.")),
                None => Some("Take the rest out of the call.".into()),
            },
            ArgumentFault::Twice(_) => Some("Keep one of them.".into()),
            ArgumentFault::Supplied(_) => Some("Take it out of the call.".into()),
            ArgumentFault::NotGiven(name) => {
                Some(format!("Write it as `{snippet}({name}=...)`."))
            }
            ArgumentFault::Needs(needed) => match needed {
                Needed::Name(sort) => {
                    let example = match sort {
                        Sort::Picture => "photo-1.jpg",
                        Sort::File => "notes-1.txt",
                        Sort::Video => "video-1.mp4",
                        Sort::Recording => "recording-1.mp3",
                    };
                    Some(format!("Write it as `{snippet}(\"{example}\")`."))
                }
                Needed::Names => Some(format!(
                    "Write it as `{snippet}([\"photo-1.jpg\", \"photo-2.jpg\"])`, or \
                     `{snippet}()` for every photograph beside the page."
                )),
                Needed::Brackets => Some(format!("Write it as `{snippet}()`.")),
                Needed::Text(option) if option == "sizes" => Some(
                    "Write it as a browser reads it: `sizes=\"(max-width: 40rem) 100vw, 40rem\"`."
                        .into(),
                ),
                Needed::Text(_) | Needed::TrueFalse(_) | Needed::OnSite(_) => None,
                Needed::VideoOn { site, .. } => Some(format!(
                    "Write the address {site} gives for the video, such as `{}`.",
                    match *site {
                        "Vimeo" => "https://vimeo.com/123456789",
                        _ => "https://www.youtube.com/watch?v=abcdefghijk",
                    }
                )),
                Needed::Playable {
                    sort: Sort::Recording,
                    ..
                } => Some("Save it as `.mp3`, `.m4a`, `.ogg`, `.wav` or `.flac`.".into()),
                Needed::Playable { .. } => Some("Save it as `.mp4`, `.webm` or `.ogv`.".into()),
            },
        },
        Kind::TemplateStopped { reason } => match reason {
            Stopped::NoSiteAddress => Some("Add `url:` to `statac.yaml`.".into()),
            Stopped::NoFeedKey(key) => Some(feed_key_line(key)),
            Stopped::NotAPage(function) => Some(format!("Write it as `{function}(post)`.")),
            Stopped::NotAnAddress => Some("Write it as `url(\"/about/\")`.".into()),
            Stopped::NotAList(filter) => Some(format!("Write it as `site.posts | {filter}`.")),
            Stopped::CallsItself(_) => Some("Take the call out of one of them.".into()),
            Stopped::RanOn { .. } => Some(
                "Look for a loop that never ends, or a loop inside a loop over long lists.".into(),
            ),
            Stopped::Engine(_)
            | Stopped::NotADate(_)
            | Stopped::ContentInItself
            | Stopped::CardInCard => None,
            Stopped::CardInWriting => Some("Read `page.card` in a layout.".into()),
            Stopped::NotText => {
                Some("Show something else here: a page can only show text.".into())
            }
        },
        Kind::PageInSnippet { handed, .. } => Some(format!("Did you mean `{handed}`?")),
        Kind::TextNeverShown { nearest, .. } => {
            nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::WritingNeverShown { .. } => {
            Some("Add `{{ page.content }}` where the writing belongs.".into())
        }
        Kind::SpelledTwoWays { .. } => {
            Some("Use one spelling on every page to choose how it's shown.".into())
        }
        Kind::DraftLeftOut { .. } | Kind::WaitingForItsDay { .. } => {
            Some("Run with `--drafts` to build it as if published.".into())
        }
        Kind::NumberedFolder { folder } => Some(format!(
            "If the numbers are part of the names, write `numbered: false` in `{folder}index.md`."
        )),
        Kind::UnorderedPages { .. } => {
            Some("They follow in their usual order; name them in `order` to place them.".into())
        }
        Kind::MalformedList { fault } => match fault {
            ListFault::NotFromThePosts(_) => {
                Some("Start it with `site.posts`, as `site.posts | tagged(\"Rust\")`.".into())
            }
            ListFault::NothingAfterPipe => Some("Name a filter after it, or take it out.".into()),
            ListFault::NotChainable(_) => {
                Some("A list chooses its posts with `tagged`, `in` and `where`.".into())
            }
            ListFault::NoBrackets(name) => Some(format!("Write it as `{name}(...)`.")),
            ListFault::Unclosed('(') => Some("Close it with `)`.".into()),
            ListFault::Unclosed(quote) => Some(format!("End the value with another `{quote}`.")),
            ListFault::Values { filter, .. } => Some(match filter.as_str() {
                "where" => "Write it as `where(\"field\", \"value\")`.".into(),
                _ => format!("Write it as `{filter}(\"...\")`."),
            }),
        },
        Kind::NotAPost { .. } => Some(
            "It stands at the list's address, so no list holds it; give it a `permalink` \
             of its own to list it as a post."
                .into(),
        ),
        Kind::FeedNotWritten { more, why, .. } => Some(match why {
            Unwritten::NoTitle => "Add `title:` to `statac.yaml`; a feed is named by it.".into(),
            Unwritten::NoTemplate(_) => {
                let them = match more {
                    0 => "it",
                    _ => "them",
                };
                format!(
                    "Add the template to publish {them}, or write `pages.feed: false` in \
                     `statac.yaml` for no feeds."
                )
            }
            Unwritten::PageThere { .. } => {
                "The page is published there instead; give the feed another address with \
                 `feed:` on the list's page."
                    .into()
            }
            Unwritten::Undated => {
                "Give a page in it a `date:`, or take the `feed:` line out.".into()
            }
        }),
        Kind::NoFeedKey { key } => Some(feed_key_line(key)),
        Kind::NoSiteAddress { .. } => {
            Some("Add the site's address to `statac.yaml`, as `url: https://example.org/`.".into())
        }
        Kind::FeedIdChanged { id, .. } => Some(format!(
            "Add `id: {id}` to its front matter to keep it the same post in feed readers."
        )),
        Kind::NoLanguage => Some("Add the site's language to `statac.yaml`, as `language: en`.".into()),
        Kind::RobotsBeneathPath { .. } => Some(
            "Copy its lines into the `robots.txt` at the top of the host to have them read."
                .into(),
        ),
        Kind::RedirectToItself { .. } => {
            Some("Point `redirect_to` at the address readers should reach.".into())
        }
        Kind::RedirectedBack { .. } => None,
        Kind::WritingNotPublished { .. } => Some(
            "Move it to where readers are sent, or take out `redirect_to` to publish it.".into(),
        ),
        Kind::SortedLast { instead, .. } => instead
            .as_ref()
            .map(|written| format!("Write `{written}` to leave out the items without it.")),
        Kind::LoopOverEveryPage { instead, .. } => instead
            .as_ref()
            .map(|written| format!("Write `{written}` to choose the pages once for the site.")),
        Kind::CallInHtml { name } => Some(format!("To show it as text, write `\\@{name}`.")),
        Kind::UntitledPage { page, .. } => Some(match page {
            Named::ListPage(..) => "Add `title:` to `statac.yaml`; the home page is called by it.",
            _ => "Give it a `title`, or open its writing with a `# heading`. A folder whose pages go without titles says `pages.untitled: true`.",
        }
        .into()),
        Kind::MissingDefinition { nearest, .. } => {
            nearest.as_ref().map(|label| format!("Did you mean `{label}`?"))
        }
        Kind::MissingAnchor { suggestion, .. } => {
            suggestion.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::FootnoteNeverShown { nearest, .. } => nearest
            .as_ref()
            .map(|marker| format!("Is it the note for `[^{marker}]`?")),
        Kind::DefinitionWrittenTwice { .. } => {
            Some("Give one of them another name, or take one out.".into())
        }
        Kind::CellNeverShown { cells, .. } => Some(
            match cells.len() {
                1 => "Add a column to the table's first two lines, or take the cell out.",
                _ => "Add columns to the table's first two lines, or take the cells out.",
            }
            .into(),
        ),
        Kind::IdUsedTwice { .. } => Some("Give this heading an id of its own.".into()),
        Kind::UnmatchedTag { unmatched } => Some(match unmatched {
            Unmatched::NeverClosed(name) => format!("Close it with `</{name}>` where it should end."),
            Unmatched::CommentNeverClosed => "Close it with `-->`.".into(),
            Unmatched::ClosesNothing(_) => "Take it out, or open the element above it.".into(),
        }),
        Kind::SizeKept { field, size, .. } => Some(format!(
            "Write `{field}: {size}`; only a text at `left`, `centre` or `right` is fitted."
        )),
        Kind::UnreadablePicture { path, fault } => match fault {
            PictureFault::CutShort => Some("Copy or export the file again.".into()),
            PictureFault::NotAStill => {
                Some("Delete it, then run `statac refresh` to fetch one.".into())
            }
            _ if path.starts_with("content/") => Some(format!(
                "To publish it as the file it is, write `optimise: false` in `{path}.yaml`."
            )),
            _ => None,
        },
        Kind::DescribesNoPicture { nearest, .. } => {
            nearest.as_ref().map(|n| format!("Did you mean `{n}`?"))
        }
        Kind::ColourConverted { .. } | Kind::CacheNotWritten { .. } => None,
        Kind::UndescribedPicture { shown, .. } => Some(match shown {
            ShownBy::Markdown {
                described_beside: true,
            } => "Describe it in the brackets, or mark it decorative.".into(),
            ShownBy::Markdown {
                described_beside: false,
            } => "Describe it in the brackets.".into(),
            ShownBy::Html => {
                "Give the `<img>` an `alt`, or `alt=\"\"` where it is decoration.".into()
            }
            ShownBy::Template {
                beside: Some(beside),
            } => format!("Describe it with `alt:` in `{beside}`, or mark it decorative."),
            ShownBy::Template { beside: None } => {
                "Describe it where it is shown, or mark it decorative.".into()
            }
        }),
        Kind::HeicPicture { .. } => {
            Some("Export it as a JPEG: most browsers can't show a HEIC picture.".into())
        }
        Kind::LocationPublished { .. } => Some(
            "Publish a copy without it, or add `location-published` to `quiet:` in \
             `statac.yaml`."
                .into(),
        ),
        Kind::PicturesOutOfProportion { property } => Some(format!(
            "Add `{}: auto` to the rule.",
            height_beside(property)
        )),
        Kind::NoStill { absent, .. } => Some(
            match absent {
                false => "Run `statac refresh` to fetch one.",
                true => "Show a picture of your own in its place.",
            }
            .into(),
        ),
        Kind::MalformedRecord { fault } => match fault {
            RecordFault::Opening => Some("Write `statac fetched 1` as its first line.".into()),
            RecordFault::Fields(_) => Some(
                "Write the file's path, its hash or `absent`, and the day, parted by tabs.".into(),
            ),
            RecordFault::Path(_) => None,
            RecordFault::Held(_) => Some(
                "Write the first 16 hex digits of the file's BLAKE3 hash, or `absent`.".into(),
            ),
            RecordFault::Day(_) => Some("Write the day as `2026-10-06`.".into()),
            RecordFault::Twice(_) => Some("Keep one of them.".into()),
            RecordFault::RefusedOpening => {
                Some("Write `statac refused 1` as its first line.".into())
            }
            RecordFault::RefusedFields(_) => Some(
                "Write `mention` or `sender`, the address, and the day, parted by tabs.".into(),
            ),
            RecordFault::Reach(_) => Some("Write `mention` or `sender`.".into()),
            RecordFault::Address(_) => {
                Some("Write the address in full, starting `https://` or `http://`.".into())
            }
        },
        Kind::TokenInSettings => Some(
            "Remove the line, add `webmentions: true`, and put the token in \
             `STATAC_WEBMENTION_TOKEN` or in the site's `.statac/token`."
                .into(),
        ),
        Kind::WebmentionsOff => {
            Some("Add `webmentions: true` to `statac.yaml`, or remove the call.".into())
        }
        Kind::MentionOfNoPage { .. } => {
            Some("If the page moved, add the old address to its `redirect_from`.".into())
        }
        Kind::FolderNotEmpty { by, .. } => Some(match by {
            Maker::New => "Give `statac new` an empty folder, or the name of a new one.".into(),
            Maker::Import => {
                "Give `statac import` an empty folder, or the name of a new one.".into()
            }
        }),
        Kind::AlreadyASite { by, has, .. } => match (by, has) {
            (Maker::New, SiteMark::Settings) => Some(
                "To bring its starter files up to date, run `statac new --update`.".into(),
            ),
            (Maker::New, SiteMark::Content) | (Maker::Import, _) => None,
        },
        Kind::NoSiteHere { .. } => {
            Some("Run it in a site's folder, or make one with `statac new`.".into())
        }
        Kind::NewerStarter { .. } => Some(
            "Run `statac new --update`: it replaces those you never changed and shows what is \
             new in the rest."
                .into(),
        ),
        Kind::CantTellWhatMade { tools, .. } => {
            let options: Vec<String> = tools.iter().map(|tool| format!("`--from={tool}`")).collect();
            let named = match options.split_last() {
                Some((last, [])) => last.clone(),
                Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
                None => "`--from`".into(),
            };
            Some(format!("Say what made it with {named}."))
        }
        Kind::MissingFolder => Some("Name it after the command: `statac import <folder>`.".into()),
        Kind::UnusableCardFile { fault, .. } => Some(match fault {
            CardFileFault::BackgroundSize { .. } => {
                "Save the background 1,200 pixels wide and 630 tall."
            }
            CardFileFault::NotAStill => "Save the background as a PNG or a JPEG.",
            CardFileFault::Font => "Name a TrueType or OpenType font file.",
        }
        .into()),
        Kind::CardNotDrawn { why, .. } => Some(match why {
            NotDrawn::NoFont {
                characters,
                more,
                font,
            } => {
                let them = if characters.len() + more == 1 { "it" } else { "them" };
                match font.as_deref() {
                    Some(FontWithThem::Here(font)) => {
                        format!("Add `{font}`, which has {them}, to the card's `font`.")
                    }
                    Some(FontWithThem::Brought(font)) => format!(
                        "Run `statac new --update --missing` to write `{}/{font}`, which has \
                         {them}, then add `{font}` to the card's `font`.",
                        crate::cards::FOLDER
                    ),
                    None => format!("Add a font that has {them} to the card's `font`."),
                }
            }
            NotDrawn::TooTall { .. } => {
                "Give the page fewer words there, or the middle of the card more room.".into()
            }
            NotDrawn::NoRoom => "Make the texts at the top and bottom smaller or shorter.".into(),
        }),
        Kind::UnusedCard { name: None } => {
            Some(
                "Write the card's `<meta>` lines in a layout, or remove the card's settings and \
                 `templates/og/`."
                    .into(),
            )
        }
        Kind::UnusedCard { name: Some(name) } => Some(format!(
            "Write `card: {name}` on the pages it is for, or remove the card's settings."
        )),
        Kind::ImageMayNotShow { .. } => Some("For a picture more link previews show, save it as a PNG.".into()),
    }
}

/// The line that gives a site its feed key, and what it is for.
fn feed_key_line(key: &str) -> String {
    format!(
        "Add `feed_key: {key}` to `statac.yaml` and never change it: feed readers know \
         each post by an id made from it."
    )
}

/// Statac's own name, where it is shown as a badge.
pub(crate) const NAME: &str = "Statac";

pub(crate) fn explanation_link(name: &str) -> String {
    format!("{EXPLANATIONS}{name}/")
}

pub(crate) fn relation(relation: &Relation) -> String {
    match relation {
        Relation::FirstWritten => "First written".into(),
        Relation::Shown => "Shown instead".into(),
        Relation::Nested => "Nested here".into(),
        Relation::Building(page) => while_building(page.as_ref()),
        Relation::CalledFrom => "Called here".into(),
        Relation::ReadAsMarkdown => "Read as Markdown here".into(),
    }
}

/// The first page whose build met a problem, to open its line: a page of
/// the author's is named by the file that follows.
pub(crate) fn while_building(page: Option<&Named>) -> String {
    match page {
        None => "While building".into(),
        Some(page) => format!("While building {}", named(page)),
    }
}

/// What heads the other pages whose builds met the same problem.
pub(crate) fn while_building_more(pages: usize) -> String {
    format!("And while building {} more:", grouped(pages))
}

/// What closes the other pages whose builds met a problem, where `build`
/// names only the first of them.
pub(crate) fn and_more(pages: usize) -> String {
    let them = if pages == 1 { "it" } else { "them" };
    format!(
        "and {} more. Run `statac check` to see {them}.",
        grouped(pages)
    )
}

pub(crate) fn severity(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}

fn counted(n: usize, severity: Severity) -> String {
    let noun = self::severity(severity);
    match n {
        1 => format!("1 {noun}"),
        _ => format!("{} {noun}s", grouped(n)),
    }
}

/// Where a problem is, above its line.
pub(crate) fn location(path: &str, line: usize) -> String {
    format!("{path}, line {line}")
}

/// After the carets under a place that runs over more than one line.
pub(crate) fn runs_to(line: usize) -> String {
    format!("runs to line {line}")
}

pub(crate) fn more(link: &str) -> String {
    format!("More: {link}")
}

/// One severity's count: `3 warnings: 2 broken-link, 1 duplicate-key`, with
/// the quietened said apart, and no full stop.
pub(crate) fn count(severity: Severity, kinds: &[(&str, usize)], quietened: usize) -> String {
    let shown: usize = kinds.iter().map(|(_, n)| n).sum();
    if shown == 0 {
        return match quietened {
            1 => format!("{}, quietened", counted(1, severity)),
            _ => format!("{}, all quietened", counted(quietened, severity)),
        };
    }
    let kinds: Vec<_> = kinds
        .iter()
        .map(|(name, n)| format!("{} {name}", grouped(n)))
        .collect();
    let mut line = format!("{}: {}", counted(shown, severity), kinds.join(", "));
    if quietened > 0 {
        line.push_str(&format!(", and {} quietened", grouped(quietened)));
    }
    line
}

/// `build`'s pointer at `check`, for the problems it counted but didn't draw.
pub(crate) fn see_them(shown: usize) -> &'static str {
    match shown {
        1 => "Run `statac check` to see it.",
        _ => "Run `statac check` to see them.",
    }
}

pub(crate) const NO_PROBLEMS: &str = "No problems.";

/// Above the JavaScript `check` says the site carries.
pub(crate) const JAVASCRIPT: &str = "JavaScript the site carries:";

/// One kind of JavaScript the built site carries, and how many pages carry it.
/// Where a browser may not run it, that is said after.
pub(crate) fn javascript(carried: &Carried, sure: bool, pages: usize) -> String {
    let mut on = match pages {
        0 => "on no page".to_owned(),
        n => format!("on {}", how_many(n, "page", "pages")),
    };
    if !sure {
        on.push_str(", where a browser may not run it");
    }
    match carried {
        Carried::Published { address, from } => format!("`{address}` from `{from}`, {on}"),
        Carried::Loaded(address) => format!("`{address}`, {on}"),
        Carried::Written(file) => format!("Written in `{file}`, {on}"),
        Carried::Embedded(address) if address.is_empty() => {
            format!("An embed with no address, which may run its own, {on}")
        }
        Carried::Embedded(address) => format!("`{address}` embedded, which may run its own, {on}"),
    }
}

/// `build`'s last line, for a site written to `out`.
pub(crate) fn wrote(published: &Published, out: &str, seconds: f64) -> String {
    let mut what = vec![how_many(published.pages, "page", "pages")];
    if published.redirects > 0 {
        what.push(how_many(published.redirects, "redirect", "redirects"));
    }
    if published.feeds > 0 {
        what.push(how_many(published.feeds, "feed", "feeds"));
    }
    if published.files > 0 {
        what.push(how_many(published.files, "other file", "other files"));
    }
    format!(
        "Wrote {} to `{out}` in {seconds:.2} seconds.",
        listed(&what)
    )
}

/// What `build` says of the copies of photographs it made.
pub(crate) fn made_copies(copies: usize, photographs: usize) -> String {
    format!(
        "Made {} of {}.",
        how_many(copies, "copy", "copies"),
        how_many(photographs, "photograph", "photographs")
    )
}

/// What `build` says of the copies it kept that no page shows any more.
pub(crate) fn removed_copies(copies: usize) -> String {
    let copies = how_many(copies, "copy", "copies");
    format!("Removed {copies} no page shows any more from `.statac`.")
}

/// How far a build has got with the copies of photographs it is making.
pub(crate) fn making_copies(done: usize, of: usize) -> String {
    format!(
        "Making copies of photographs: {} of {}.",
        grouped(done),
        grouped(of)
    )
}

/// What `build` says of the sharing cards it drew.
pub(crate) fn drew_cards(cards: usize) -> String {
    format!("Drew {}.", how_many(cards, "card", "cards"))
}

/// What `build` says of the cards it kept that no page is shared with any
/// more.
pub(crate) fn removed_cards(cards: usize) -> String {
    let cards = how_many(cards, "card", "cards");
    format!("Removed {cards} no page is shared with any more from `.statac`.")
}

/// How far a build has got with the cards it is drawing.
pub(crate) fn drawing_cards(done: usize, of: usize) -> String {
    format!("Drawing cards: {} of {}.", grouped(done), grouped(of))
}

/// How many of the last lines a command printed are shown.
const LINES_SHOWN: usize = 40;

/// What one of the site's own commands printed before it stopped, as a
/// heading and the lines under it, the last of them where there are many;
/// none for a kind that carries no such thing.
pub(crate) fn printed(kind: &Kind) -> Option<(String, Vec<&str>)> {
    let Kind::HookStopped { said, ended, .. } = kind else {
        return None;
    };
    if matches!(ended, Ended::NotStarted(_)) {
        return None;
    }
    let lines: Vec<&str> = said.trim_end().lines().collect();
    if said.trim().is_empty() {
        return Some(("It printed nothing.".into(), Vec::new()));
    }
    let heading = match lines.len() {
        all if all > LINES_SHOWN => format!(
            "The last {LINES_SHOWN} of the {} lines it printed:",
            grouped(all)
        ),
        _ => "What it printed:".into(),
    };
    let from = lines.len().saturating_sub(LINES_SHOWN);
    Some((heading, lines[from..].to_vec()))
}

/// What `build` says on a terminal while a command of the site's own
/// runs.
pub(crate) fn running_hook(hook: &str) -> String {
    format!("Running `{hook}`.")
}

/// What `build` says of a command of the site's own that it ran.
pub(crate) fn ran_hook(hook: &str) -> String {
    format!("Ran `{hook}`.")
}

/// What `build` says of `after_build` once it has run: how many files it
/// added to the built site, wrote over and removed from it.
pub(crate) fn ran_after(added: usize, changed: usize, removed: usize) -> String {
    let mut what = vec![format!("added {}", how_many(added, "file", "files"))];
    if changed > 0 {
        what.push(format!("changed {}", grouped(changed)));
    }
    if removed > 0 {
        what.push(format!("removed {}", grouped(removed)));
    }
    format!("Ran `after_build`, which {}.", listed(&what))
}

/// What `serve` says as it opens of a `before_build` it ran.
pub(crate) const HOOK_RAN_ONCE: &str = "Ran `before_build` once, before the first build.";

/// What `serve` says as it opens of a `before_build` it ran, where the
/// site names files it is run again for.
pub(crate) const HOOK_RUNS_AGAIN: &str =
    "Ran `before_build`: it runs again when a file `before_build_watches` names changes.";

/// What `serve` says as it opens of an `after_build` it ran.
pub(crate) const AFTER_RAN_ONCE: &str =
    "`after_build` ran once: what it made stays as it is until serve is started again.";

/// What `serve` says as it opens of an `after_build` it hasn't run, the
/// first build having stopped.
pub(crate) const AFTER_WAITS: &str =
    "`after_build` runs once, after the first build with no errors.";

/// What `serve` says as it opens where the site has something to fetch.
pub(crate) const FETCHING: &str =
    "Fetching what's new from other sites. `--offline` turns this off.";

/// What `serve` says as it opens where the site has something to fetch
/// and it was asked for `--offline`.
pub(crate) const OFFLINE: &str = "Offline: nothing is fetched from other sites.";

/// What `serve` says where fetching ended in a fault of Statac's own.
pub(crate) const FETCHING_STOPPED: &str = "Fetching stopped: Statac met a fault of its own.";

/// What `build` says, after Rust's report of a panic, of a fault of
/// Statac's own.
pub(crate) const BUILD_STOPPED: &str = "Build stopped: Statac met a fault of its own.";

/// What `check` says, after Rust's report of a panic, of a fault of
/// Statac's own.
pub(crate) const CHECK_STOPPED: &str = "Check stopped: Statac met a fault of its own.";

/// What `build` says after a fault of Statac's own that came before the
/// site was put in place.
pub(crate) const WROTE_NOTHING: &str = "Wrote nothing.";

/// What any command says last after a fault of Statac's own.
pub(crate) const REPORT_FAULT: &str =
    "Report it at https://github.com/statac-ssg/statac/issues, with the lines above.";

/// What `build` says of a `before_build` it didn't run.
pub(crate) const HOOK_PASSED_OVER: &str = "Didn't run `before_build`: `statac.yaml` has an error.";

/// What `stamp` and `serve` say of a date written over a stamp.
pub(crate) fn stamped(file: &str, field: &str, written: &str) -> String {
    format!("Stamped `{file}`: `{field}` is `{written}`.")
}

/// What `stamp` and `serve` say of an `id:` line written into a post.
pub(crate) fn wrote_id(file: &str) -> String {
    format!("Wrote an `id` into `{file}`.")
}

/// What `stamp` and `serve` say where `write_feed_ids` is on and no build
/// has yet kept which posts feed readers know.
pub(crate) const IDS_NOT_WRITTEN: &str =
    "Wrote no feed ids: they start once `statac build` has published the site.";

/// Why a post isn't given an `id:` line: nowhere in its front matter
/// would the line be read as a field of the page.
pub(crate) const NO_LINE_FOR_AN_ID: &str = "its front matter has no place for an `id:` line";

/// What `stamp` says where no stamp waits.
pub(crate) const NOTHING_TO_STAMP: &str = "Nothing to stamp.";

/// What `stamp` and `serve` say of stamps they left as they were.
pub(crate) const NOT_STAMPED: &str = "Didn't stamp: `statac.yaml` has an error.";

/// Why a file of the author's isn't written into: it no longer holds what
/// was read.
pub(crate) const CHANGED_SINCE_READ: &str = "it changed while Statac was reading the site";

/// Why a file of the author's isn't written into: writing it would part
/// it from its other name.
pub(crate) const NAMED_TWICE: &str = "the same file has another name somewhere";

/// Why a fetch of `address` brought nothing back, to follow a colon: the
/// same words whatever was being fetched.
pub(crate) fn refusal(address: &str, refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotAnAddress | Refusal::Scheme { .. } | Refusal::NoHost | Refusal::Credentials => {
            format!("`{address}` isn't a web address")
        }
        Refusal::UnknownHost { host } => format!("nothing answers to the name `{host}`"),
        Refusal::PrivateAddress { host, address } => {
            format!("`{host}` is at `{address}`, which isn't on the open web")
        }
        Refusal::Missing { .. } => "the server has nothing at that address".into(),
        Refusal::Status { status } => format!("the server answered {status}"),
        Refusal::Redirects { limit } => format!("the address redirected more than {limit} times"),
        Refusal::NoRedirectTarget { .. } => "the server redirected without saying where".into(),
        Refusal::TooLarge { limit, .. } => {
            format!("the answer is larger than {} MB", limit / 1_000_000)
        }
        // As long as a fetch is given, which `fetch` keeps to itself.
        Refusal::TimedOut => "no answer came within 30 seconds".into(),
        Refusal::Tls { host } => format!("a secure connection to `{host}` couldn't be made"),
        Refusal::Failed { reason } => crate::problem::system_words(reason),
    }
}

/// Why an answer fetched as a picture wasn't kept.
pub(crate) const NOT_A_PICTURE: &str = "the answer isn't a picture";

/// Why a video site's answer naming its picture led to none.
pub(crate) const NO_PICTURE_NAMED: &str = "the video site named no picture";

/// What `refresh` says of a file it fetched, by its path from the site's
/// folder.
pub(crate) fn fetched_file(file: &str) -> String {
    format!("Fetched `{file}`.")
}

/// What `refresh` says where a video site answered it has no still.
pub(crate) fn no_still_there(site: &str, id: &str) -> String {
    format!("No still: {site} has no picture for `{id}`.")
}

/// What `refresh` says of a still it couldn't fetch, and why.
pub(crate) fn cant_fetch_still(id: &str, reason: &str) -> String {
    format!("Can't fetch the still for `{id}`: {reason}.")
}

/// `refresh`'s last line, where some still was missing: what it fetched,
/// what was there already, and what it couldn't fetch, each where there
/// is any.
pub(crate) fn refreshed(fetched: usize, here: usize, failed: usize) -> String {
    let mut parts = Vec::new();
    if fetched > 0 {
        parts.push(format!("Fetched {}.", how_many(fetched, "still", "stills")));
    }
    if here > 0 {
        parts.push(format!("{} already here.", grouped(here)));
    }
    if failed > 0 {
        parts.push(format!("{} couldn't be fetched.", grouped(failed)));
    }
    match parts.is_empty() {
        true => "Fetched no stills.".into(),
        false => parts.join(" "),
    }
}

/// What `refresh` says after its last line where the site stops with its
/// drafts and posts waiting for their day, and builds without them.
pub(crate) fn drafts_not_refreshed(errors: usize) -> String {
    let them = if errors == 1 { "it" } else { "them" };
    let errors = how_many(errors, "error", "errors");
    format!(
        "Drafts weren't looked at: the site has {errors} with {them}. Run `statac check --drafts` to see {them}."
    )
}

/// `refresh`'s last line, where no still was missing.
pub(crate) const NOTHING_TO_FETCH: &str = "Nothing to fetch.";

/// What `refresh` says where the site uses webmentions and the relay's
/// token wasn't given.
pub(crate) const NO_TOKEN: &str =
    "Webmentions weren't asked for: no token is in `STATAC_WEBMENTION_TOKEN` or `.statac/token`.";

/// What is said of a `.statac/token` that is there and isn't used, and
/// why.
pub(crate) fn token_unused(reason: &str) -> String {
    format!("`.statac/token` wasn't used: {reason}.")
}

pub(crate) const TOKEN_FILE_EMPTY: &str = "it is empty";
pub(crate) const TOKEN_FILE_LINES: &str = "it holds more than one line";

/// Why a `.statac/token` reached through the link `rel` isn't used.
pub(crate) fn token_file_linked(rel: &str) -> String {
    match rel {
        ".statac/token" => "it is a link".into(),
        _ => format!("`{rel}` is a link"),
    }
}

/// What `refresh` says where the relay couldn't be asked, and why.
pub(crate) fn cant_ask(reason: &str) -> String {
    format!("Can't ask for webmentions: {reason}.")
}

/// Why the relay couldn't be asked, where it refused the token.
pub(crate) const TOKEN_REFUSED: &str = "the relay didn't accept the token";

/// Why the relay couldn't be asked, where its answer isn't the feed it
/// sends.
pub(crate) const UNREADABLE_ANSWER: &str = "the relay's answer couldn't be read";

/// What `refresh` says where it stopped asking the relay with more to come.
pub(crate) fn asked_enough(asks: usize) -> String {
    format!(
        "Stopped asking for webmentions after {asks} asks: run `statac refresh` again for the rest."
    )
}

/// What `refresh` says of a mention of a kind Statac doesn't read.
pub(crate) fn not_read(id: u64, property: &str) -> String {
    format!("Webmention `{id}` not kept: Statac doesn't read `{property}`.")
}

/// What `refresh` says of a mention the relay gave no kind for.
pub(crate) fn no_kind_said(id: u64) -> String {
    format!("Webmention `{id}` not kept: it doesn't say what kind it is.")
}

/// What `refresh` says of a mention naming no page of its sender's.
pub(crate) fn no_address_named(id: u64) -> String {
    format!("Webmention `{id}` not kept: it names no address.")
}

/// What `refresh` says of a mention whose sender's page isn't on the web.
pub(crate) fn not_a_web_address(id: u64, url: &str) -> String {
    format!("Webmention `{id}` not kept: `{url}` isn't a web address.")
}

/// What `refresh` says of a mention of an address elsewhere.
pub(crate) fn not_on_site(id: u64, target: &str) -> String {
    format!("Webmention `{id}` not kept: `{target}` isn't on this site.")
}

/// What `refuse` says first where it turned one mention away, with how
/// many it removed.
pub(crate) fn refused_one(address: &str, removed: usize) -> String {
    let removed = how_many(removed, "webmention", "webmentions");
    format!("Refused `{address}`: {removed} removed.")
}

/// What `refuse` says first where it turned a sender away, with how many
/// mentions it removed.
pub(crate) fn refused_sender(address: &str, removed: usize) -> String {
    let removed = how_many(removed, "webmention", "webmentions");
    format!("Refused everything from `{address}`: {removed} removed.")
}

/// What `refuse` says of each file it removed, by its path from the
/// site's folder.
pub(crate) fn removed_file(file: &str) -> String {
    format!("Removed `{file}`.")
}

/// What `refuse` says of an address its list holds, where nothing kept is
/// from it.
pub(crate) fn refused_already(address: &str) -> String {
    format!("`{address}` is refused already.")
}

/// What `refuse` says where nothing kept is from the address.
pub(crate) fn nothing_refused(address: &str) -> String {
    format!("Nothing refused: no webmention kept here is from `{address}`.")
}

/// What `refuse` says after [`nothing_refused`] of a kept mention's
/// address close to the one given.
pub(crate) fn did_you_mean(address: &str) -> String {
    format!("Did you mean `{address}`?")
}

/// What `refresh` says of a mention `statac refuse` turned away.
pub(crate) fn refused_here(id: u64, address: &str) -> String {
    format!("Webmention `{id}` not kept: `{address}` is refused.")
}

/// What `refresh` says of a mention kept without its sender's address.
pub(crate) fn sender_not_kept(id: u64, address: &str) -> String {
    format!(
        "Webmention `{id}`: `{address}` isn't a web address, so the sender's own address isn't kept."
    )
}

/// What `refresh` says of a mention kept without its sender's face.
pub(crate) fn cant_fetch_face(id: u64, reason: &str) -> String {
    format!("Can't fetch the face for `{id}`: {reason}.")
}

/// Why a face fetched wasn't kept: its pixels would take too much to read.
pub(crate) fn too_large_to_open(width: u32, height: u32) -> String {
    format!(
        "the picture is {} by {} pixels, too large to open",
        grouped(width),
        grouped(height)
    )
}

/// Why a face fetched wasn't kept: no copy of it could be made.
pub(crate) const NOT_MADE_AGAIN: &str = "the picture couldn't be made again";

/// `refresh`'s line for webmentions, where the relay answered: what it
/// kept, the faces it fetched, and what it didn't keep, each where there
/// is any.
pub(crate) fn mentions_refreshed(fetched: usize, faces: usize, not_kept: usize) -> String {
    let mut parts = Vec::new();
    match (fetched, faces) {
        (0, 0) => {}
        (fetched, 0) => parts.push(format!(
            "Fetched {}.",
            how_many(fetched, "webmention", "webmentions")
        )),
        (fetched, faces) => parts.push(format!(
            "Fetched {} and {}.",
            how_many(fetched, "webmention", "webmentions"),
            how_many(faces, "face", "faces")
        )),
    }
    if not_kept > 0 {
        parts.push(format!("{} not kept.", grouped(not_kept)));
    }
    match parts.is_empty() {
        true => "No new webmentions.".into(),
        false => parts.join(" "),
    }
}

/// The part of [`fetched_ages`] for webmentions: how many, and the age of
/// the newest the record dates, in days before the build's.
pub(crate) fn mentions_age(mentions: usize, newest: Option<i64>) -> String {
    let counted = how_many(mentions, "webmention", "webmentions");
    match newest {
        None => format!("{counted}, with no date"),
        Some(days) if mentions == 1 => format!("{counted}, {}", days_ago(days)),
        Some(days) => format!("{counted}, the newest {}", days_ago(days)),
    }
}

/// How long before the build's day something was fetched.
fn days_ago(days: i64) -> String {
    match days {
        ..=0 => "today".to_owned(),
        1 => "yesterday".to_owned(),
        days => format!("{} days ago", grouped(days.unsigned_abs() as usize)),
    }
}

/// What `build` says of what `fetched/` holds, a part for each kind of
/// thing fetched.
pub(crate) fn fetched_ages(parts: &[String]) -> String {
    format!("Fetched: {}.", parts.join("; "))
}

/// The part of [`fetched_ages`] for the stills: how many, the age of the
/// oldest the record dates, in days before the build's, and how many it
/// doesn't date.
pub(crate) fn stills_age(stills: usize, oldest: Option<i64>, undated: usize) -> String {
    let counted = how_many(stills, "still", "stills");
    let age = days_ago;
    match (oldest, undated) {
        (None, _) => format!("{counted}, with no date"),
        (Some(days), 0) if stills == 1 => format!("{counted}, {}", age(days)),
        (Some(days), 0) => format!("{counted}, the oldest {}", age(days)),
        (Some(days), undated) => format!(
            "{counted}, the oldest {}; {} with no date",
            age(days),
            grouped(undated)
        ),
    }
}

/// `build`'s last line, where errors stopped it.
pub(crate) fn wrote_nothing(errors: usize) -> String {
    let errors = how_many(errors, "error", "errors");
    format!("Wrote nothing: {errors} stopped the build.")
}

/// Where `serve` answers, and the folder it answers from. The address
/// comes last and bare, so a terminal makes a link of exactly it.
pub(crate) fn serving(folder: &str, address: &str) -> String {
    format!("Serving `{folder}` at {address}")
}

/// What `serve` says where the port asked for was another program's.
pub(crate) fn port_moved(asked: u16, port: u16) -> String {
    format!("Port {asked} is in use: serving on port {port} instead.")
}

/// What `serve` says where it polls the site's folder, and why where the
/// author didn't ask it to.
pub(crate) fn polling(every: std::time::Duration, because: Option<&Because>) -> String {
    let polling = format!("Watching by polling every {} ms", every.as_millis());
    match because {
        None => format!("{polling}."),
        Some(Because::Remote(kind)) => {
            format!("{polling}: the folder is on a `{kind}` disk, which may not report changes.")
        }
        Some(Because::NoReports(reason)) => {
            format!("{polling}: the system didn't start reporting changes, {reason}.")
        }
    }
}

/// How `serve` is ended, said once it is ready.
pub(crate) const STOP_SERVING: &str = "Press Ctrl+C to stop.";

/// What `serve` says as it ends because it can no longer tell when a file
/// changes.
pub(crate) const WATCHING_ENDED: &str = "Stopped: the site's folder is no longer being watched.";

/// The files of the site a round of `serve` answers to, as its line
/// opens. With none named, the system lost count of them.
pub(crate) fn files_changed(files: &[String]) -> String {
    match files {
        [] => "Files changed.".into(),
        [one] => format!("`{one}` changed."),
        [one, other] => format!("`{one}` and `{other}` changed."),
        [one, rest @ ..] => format!("`{one}` and {} other files changed.", grouped(rest.len())),
    }
}

/// What a round of `serve` built.
pub(crate) fn built_round(built: usize, of: usize, seconds: f64) -> String {
    format!(
        "Built {} of {} in {seconds:.2} seconds.",
        grouped(built),
        how_many(of, "file", "files")
    )
}

/// A round of `serve` that errors stopped.
pub(crate) fn stopped_round(errors: usize) -> String {
    format!("{} stopped the build.", how_many(errors, "error", "errors"))
}

/// What a round of `serve` asked of the pages open in a browser.
pub(crate) fn told(told: &Told) -> String {
    match told {
        Told::Nothing => "No built file changed.".into(),
        Told::Reload => "Browser told to reload.".into(),
        Told::Styles(styles) => format!(
            "Browser told to swap {}.",
            how_many(styles.len(), "stylesheet", "stylesheets")
        ),
    }
}

/// After a round of `serve` that stopped, in the terminal.
pub(crate) const BROWSER_KEEPS: &str = "The browser keeps the pages as last built.";

/// After a round of `serve` that stopped, on the page.
pub(crate) const PAGE_KEPT: &str = "This page is as last built.";

/// The panel's line on a page `serve` answers, for a site built at
/// `time` with this many problems of each severity shown.
pub(crate) fn panel_built(time: &str, errors: usize, warnings: usize, notes: usize) -> String {
    let mut counts = Vec::new();
    for (n, one, many) in [
        (errors, "error", "errors"),
        (warnings, "warning", "warnings"),
        (notes, "note", "notes"),
    ] {
        if n > 0 {
            counts.push(how_many(n, one, many));
        }
    }
    match counts.is_empty() {
        true => format!("Built at {time}. {NO_PROBLEMS}"),
        false => format!("Built at {time}. {}.", counts.join(", ")),
    }
}

/// The panel's line for a build that stopped at `time`. `kept` is whether
/// an earlier build's pages are still answered.
pub(crate) fn panel_stopped(time: &str, errors: usize, kept: bool) -> String {
    let stopped = format!(
        "{} stopped the build at {time}.",
        how_many(errors, "error", "errors")
    );
    match kept {
        true => format!("{stopped} {PAGE_KEPT}"),
        false => stopped,
    }
}

/// What opens the panel's line where a build stopped, ahead of the first
/// error's own words, and what closes it where there are more.
pub(crate) const PANEL_LINE_STOPPED: &str = "Build stopped.";
pub(crate) fn panel_more_errors(more: usize) -> String {
    format!("And {}.", how_many(more, "more error", "more errors"))
}

/// After the panel's line, on a page a build leaves out.
pub(crate) const PANEL_DRAFT: &str = "Draft.";
pub(crate) fn panel_waiting(day: &str) -> String {
    format!("Waiting for {day}.")
}

/// The panel's button, closed and open, and what the panel is called.
pub(crate) const PANEL_SHOW: &str = "Show details";
pub(crate) const PANEL_HIDE: &str = "Hide details";
pub(crate) const PANEL_NAME: &str = NAME;

/// The headings in the panel.
pub(crate) const PANEL_STOPPED: &str = "What stopped the build";
pub(crate) const PANEL_PAGE: &str = "This page";
pub(crate) const PANEL_SITE: &str = "The site";
pub(crate) const PANEL_WRITTEN: &str = "Written into your files";

/// What the panel says of the page it is on.
pub(crate) const PANEL_IS_DRAFT: &str = "A draft: `statac build` leaves it out.";
pub(crate) fn panel_is_waiting(day: &str) -> String {
    format!("Waiting for its day, {day}: `statac build` leaves it out until then.")
}
pub(crate) const PANEL_PAGE_CLEAR: &str = "No problems on this page.";

/// The panel's account of the page it is on: what builds it and which
/// lists hold it.
pub(crate) const PANEL_FROM: &str = "Where this page comes from";
pub(crate) const PANEL_FILE: &str = "File";
pub(crate) const PANEL_ADDRESS: &str = "Address";
pub(crate) const PANEL_LAYOUT: &str = "Layout";
pub(crate) const PANEL_LISTS: &str = "Lists";
pub(crate) const PANEL_FEEDS: &str = "Feeds";
pub(crate) const PANEL_NO_FILE: &str = "None: Statac made this page.";
pub(crate) const PANEL_NO_LAYOUT: &str = "None.";
pub(crate) const PANEL_IN_NO_LIST: &str = "None.";
pub(crate) const PANEL_IN_NO_FEED: &str = "None.";

/// The template building a page, and what chose it: where its name is
/// written, on the page or handed down to it, or that nothing names one.
pub(crate) fn panel_layout(name: &str, written: Option<&str>, handed_down: bool) -> String {
    let template = format!("`{name}`, the file `templates/{name}.html`");
    match (written, handed_down) {
        (Some(at), false) => format!("{template}, named at {at}."),
        (Some(at), true) => format!("{template}, handed down from {at}."),
        (None, _) => format!("{template}, as nothing names one."),
    }
}

/// A list a page is in, and where it comes among the list's posts.
pub(crate) fn panel_member(word: Word, name: &str, at: usize, of: usize) -> String {
    let list = the_list(word, name);
    let list = list.strip_prefix("the ").unwrap_or(&list);
    format!("The {list}: {} of {}.", grouped(at), grouped(of))
}

/// The panel's table of every name a template can write on the page.
pub(crate) const PANEL_SHOWS: &str = "What a template can show here";
pub(crate) const PANEL_SHOWS_HOW: &str =
    "Every name a template can write on this page, and what each holds here.";
pub(crate) const PANEL_COLUMNS: [&str; 4] = ["Write", "Holds", "On this page", "Written at"];
pub(crate) const PANEL_NO_VALUE: &str = "no value";
pub(crate) const PANEL_FIELDS_BELOW: &str = "the fields beneath";
pub(crate) const PANEL_A_CALL: &str = "called, not shown";

/// What heads one group of the table: whose names they are, how many, and
/// how many hold nothing on this page.
pub(crate) fn panel_group(of: &str, names: usize, empty: usize) -> String {
    let names = how_many(names, "name", "names");
    match empty {
        0 => format!("`{of}`: {names}"),
        _ => format!("`{of}`: {names}, {} with no value", grouped(empty)),
    }
}

/// A list a name holds, counted in what it holds.
pub(crate) fn panel_count(count: usize, one: &str, many: &str) -> String {
    how_many(count, one, many)
}

/// After the fields of a value with more than the panel shows.
pub(crate) fn panel_more_fields(more: usize) -> String {
    format!("and {}", how_many(more, "more field", "more fields"))
}

/// The names a value answers in turn, each as it follows the value's own.
pub(crate) fn panel_has(names: &[String]) -> String {
    let names: Vec<String> = names.iter().map(|name| format!(".{name}")).collect();
    format!("Also {}", names.join(" "))
}

/// Above the panel's links to the pages a build leaves out.
pub(crate) const PANEL_DRAFTS: &str = "Drafts:";
pub(crate) const PANEL_WAITING: &str = "Waiting for their day:";

/// The problems a round of `serve` no longer has, counted by kind.
pub(crate) fn gone(kinds: &[(&str, usize)]) -> String {
    let kinds: Vec<_> = kinds
        .iter()
        .map(|(name, n)| format!("{} {name}", grouped(n)))
        .collect();
    format!("Gone since the last build: {}.", kinds.join(", "))
}

/// The page `serve` answers with where the site has nothing at an address
/// and no `404.html` of its own.
pub(crate) const NOT_FOUND_TITLE: &str = "Not found";
pub(crate) const NOT_FOUND: &str = "Nothing is published at this address.";

/// Before the link to the top of a site published under a path, on the
/// same page.
pub(crate) const PUBLISHED_UNDER: &str = "The site is published under";

/// `serve`'s answer to an address that could name a file outside the site.
pub(crate) const ADDRESS_REFUSED: &str = "This address can't name a file of the site.";

/// `serve`'s answer to a request that names another machine, or comes
/// from another site's page.
pub(crate) fn answers_only_at(address: &str) -> String {
    format!("statac serve answers only at {address}")
}

/// `serve`'s answer to a request that isn't for reading.
pub(crate) const READING_ONLY: &str = "Only GET and HEAD are answered.";

/// What `statac new` says before its questions.
pub(crate) fn new_site_in(folder: &str) -> String {
    format!("A new site in `{folder}`.")
}

/// Said once under the line above, so that the brackets in the questions are
/// understood.
pub(crate) const ASK_ENTER: &str = "Enter takes the answer in brackets.";

pub(crate) const ASK_TITLE: &str = "What is the site called?";
pub(crate) const ASK_ADDRESS: &str = "What will its address be?";
pub(crate) const ASK_ADDRESS_HINT: &str = "Enter for none yet.";
pub(crate) const ASK_OLD_ADDRESS: &str = "What is the site's address?";
pub(crate) const ASK_OLD_ADDRESS_HINT: &str = "The old site's settings don't say.";
pub(crate) const ASK_LANGUAGE: &str = "What language is it written in?";
pub(crate) const ASK_LANGUAGE_HINT: &str = "For more than one, the main one first, as `en, fr`.";
pub(crate) const ASK_DATES: &str = "How should dates look?";
pub(crate) const ASK_DATES_OWN: &str = "Or type an example of your own.";

/// One example offered for dates, by its number.
pub(crate) fn ask_date_choice(number: u8, example: &str) -> String {
    format!("{number}  {example}")
}

/// The answer Enter gives, as the line the answer is typed on shows it.
pub(crate) fn ask_default(default: &str) -> String {
    format!("[{default}]")
}

/// Marks where the answer is typed.
pub(crate) const ASK_MARK: &str = ">";

pub(crate) fn not_a_site_address(written: &str) -> String {
    format!("`{written}` isn't a web address. Write it whole, as `https://example.org`.")
}

pub(crate) fn not_a_language_code(written: &str) -> String {
    format!("`{written}` isn't a language's code. Write it as `en`, or `en-GB`.")
}

pub(crate) fn language_given_twice(written: &str, first: &str) -> String {
    format!("`{written}` is `{first}` again. Name each language once.")
}

pub(crate) fn no_words_for(tag: &str) -> String {
    format!("Statac has no words for `{tag}`: write its months and days in `languages/{tag}.yaml`.")
}

/// Where the docs describe a language's own words.
pub(crate) const LANGUAGE_FILES: &str = "https://statac.dev/dates-and-languages/#languages";

pub(crate) fn made_site(folder: &str, files: usize) -> String {
    format!(
        "Made a new site in `{folder}`: {}.",
        how_many(files, "file", "files")
    )
}

pub(crate) fn times_in(zone: &str) -> String {
    format!("Times are in {zone}, as this machine is set.")
}

pub(crate) const NO_ZONE_HERE: &str = "This machine doesn't say where it is: add \
     `timezone: Europe/London`, with your own, to `statac.yaml`.";

/// Where a new site's words in `tag` wait in English: the site's own, and
/// Statac's where `file` is the site's file holding them.
pub(crate) fn translate_words(tag: &str, file: Option<&str>) -> String {
    match file {
        Some(file) => format!(
            "The site's own words are in English in `data/words.yaml`, under `{tag}`, and \
             Statac's in `{file}`. Translate them there."
        ),
        None => format!(
            "The site's own words are in English in `data/words.yaml`, under `{tag}`. \
             Translate them there."
        ),
    }
}

/// The last thing `statac new` says, with the way to the site's folder
/// where it isn't the folder Statac was run in.
pub(crate) fn next_steps(folder: Option<&str>) -> String {
    match folder {
        Some(folder) => format!("Next: `cd {folder}`, then `statac serve`."),
        None => "Next: `statac serve`.".into(),
    }
}

/// Why `statac new --update` can't bring a folder up to date.
pub(crate) const NOT_A_SITE: &str = "it has no `statac.yaml`, so it isn't a site";

pub(crate) fn updated_file(path: &str) -> String {
    format!("Updated `{path}`.")
}

pub(crate) fn yours_with_new(path: &str) -> String {
    format!(
        "`{path}` is yours and wasn't touched. The starter's file has \
         changed since the version nearest to yours:"
    )
}

/// What `statac new --update` says of a file of the author's that isn't
/// text, where the starter's file there has changed in some release.
pub(crate) fn yours_not_text(path: &str) -> String {
    format!("`{path}` is yours; the starter has a newer one.")
}

/// What `statac new --cards` says of a file the site has where one of the
/// sharing card's would go.
pub(crate) fn yours_untouched(path: &str) -> String {
    format!("`{path}` is yours and wasn't touched.")
}

/// The head of a run of changed lines, by its first line's number.
pub(crate) fn changed_at(line: usize) -> String {
    format!("@@ line {line}")
}

/// Shown after a changed line that is the last and has no line break.
pub(crate) const NO_LINE_BREAK: &str = "(no line break at the end)";

pub(crate) fn added_words(names: &[&str], tag: &str) -> String {
    let named: Vec<String> = names.iter().map(|name| format!("`{name}`")).collect();
    format!(
        "Added {} to `data/words.yaml` under `{tag}`: {}.",
        how_many(names.len(), "word", "words"),
        named.join(", ")
    )
}

pub(crate) const NO_WORDS_FILE: &str = "No words were added: `data/words.yaml` isn't here.";

pub(crate) const WORDS_UNREADABLE: &str =
    "No words were added: Statac can't read `data/words.yaml`. Run `statac check` to see why.";

pub(crate) fn no_words_under(tag: &str) -> String {
    format!("No words were added under `{tag}`: `data/words.yaml` has no words under it.")
}

pub(crate) fn words_not_one_to_a_line(tag: &str) -> String {
    format!(
        "No words were added under `{tag}`: its words in `data/words.yaml` aren't one to a line."
    )
}

/// The starter files not in a site, the first few by name and the rest
/// counted.
pub(crate) fn not_here(named: &[&str], more: usize) -> String {
    let listed: Vec<String> = named.iter().map(|path| format!("`{path}`")).collect();
    let all = named.len() + more;
    let mut list = listed.join(", ");
    if more > 0 {
        list = format!("{list} and {} more", grouped(more));
    }
    match all {
        1 => format!(
            "1 starter file isn't here: {list}. To write it, run `statac new --update --missing`."
        ),
        _ => format!(
            "{} starter files aren't here: {list}. To write them, run \
             `statac new --update --missing`.",
            grouped(all)
        ),
    }
}

pub(crate) fn wrote_file(path: &str) -> String {
    format!("Wrote `{path}`.")
}

/// What `statac new --update` says of a starter file at or behind a
/// link, `link` being the link's own path.
pub(crate) fn left_for_a_link(path: &str, link: &str) -> String {
    match path == link {
        true => format!("`{path}` was left alone: it is a link, which Statac never goes through."),
        false => {
            format!("`{path}` was left alone: `{link}` is a link, which Statac never goes through.")
        }
    }
}

/// The last line of `statac new --update`, by how many files it updated
/// and how many of the author's have something new; none where both are
/// none.
pub(crate) fn updated_files(updated: usize, new: usize) -> Option<String> {
    let updated = match updated {
        0 => None,
        n => Some(format!("Updated {}.", how_many(n, "file", "files"))),
    };
    let new = match new {
        0 => None,
        1 => Some("1 of yours has something new, shown above.".to_owned()),
        n => Some(format!(
            "{} of yours have something new, shown above.",
            grouped(n)
        )),
    };
    match (updated, new) {
        (None, None) => None,
        (Some(one), None) | (None, Some(one)) => Some(one),
        (Some(updated), Some(new)) => Some(format!("{updated} {new}")),
    }
}

pub(crate) const NOTHING_TO_UPDATE: &str =
    "Nothing to update: every starter file here is the newest, or yours with nothing new.";

/// An option as written, without the value given it.
fn option_name(written: &str) -> &str {
    written.split('=').next().unwrap_or(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::MathsFault;

    fn kinds() -> Vec<Kind> {
        vec![
            Kind::BrokenLink {
                target: "/about-us/".into(),
                suggestion: Some("/about/".into()),
                inside: false,
            },
            Kind::OtherSpelling {
                word: Word::Field,
                written: "template".into(),
                ours: "layout".into(),
            },
            Kind::CantWrite {
                path: "out/index.html".into(),
                reason: "a folder already has that name".into(),
            },
        ]
        .into_iter()
        .chain(yaml().into_iter().map(|(kind, _, _)| kind))
        .chain(others().into_iter().map(|(kind, _, _)| kind))
        .chain(files().into_iter().map(|(kind, _, _)| kind))
        .chain(content().into_iter().map(|(kind, _, _)| kind))
        .chain(dates().into_iter().map(|(kind, _, _)| kind))
        .chain(templates().into_iter().map(|(kind, _, _)| kind))
        .chain(lists().into_iter().map(|(kind, _, _)| kind))
        .chain(data().into_iter().map(|(kind, _, _)| kind))
        .chain(feeds().into_iter().map(|(kind, _, _)| kind))
        .chain(links().into_iter().map(|(kind, _, _)| kind))
        .chain(cards().into_iter().map(|(kind, _, _)| kind))
        .chain(fetched().into_iter().map(|(kind, _, _)| kind))
        .collect()
    }

    fn fetched() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        use crate::fetched::stills::Host;
        let no_still = |host, absent| Kind::NoStill {
            host,
            id: "abcdefghijk".into(),
            absent,
        };
        let record = |fault| Kind::MalformedRecord { fault };
        vec![
            (
                no_still(Host::YouTube, false),
                "No still: no picture has been fetched for this video yet",
                Some("Run `statac refresh` to fetch one."),
            ),
            (
                no_still(Host::YouTube, true),
                "No still: YouTube has no picture for this video",
                Some("Show a picture of your own in its place."),
            ),
            (
                no_still(Host::Vimeo, true),
                "No still: Vimeo has no picture for this video",
                Some("Show a picture of your own in its place."),
            ),
            (
                record(RecordFault::Opening),
                "Malformed record: the first line isn't `statac fetched 1`",
                Some("Write `statac fetched 1` as its first line."),
            ),
            (
                record(RecordFault::Fields(2)),
                "Malformed record: a line has 2 fields, not 3",
                Some("Write the file's path, its hash or `absent`, and the day, parted by tabs."),
            ),
            (
                record(RecordFault::Fields(1)),
                "Malformed record: a line has 1 field, not 3",
                Some("Write the file's path, its hash or `absent`, and the day, parted by tabs."),
            ),
            (
                record(RecordFault::Path("../record".into())),
                "Malformed record: `../record` isn't a path inside `fetched/`",
                None,
            ),
            (
                record(RecordFault::Held("0123".into())),
                "Malformed record: `0123` is neither a hash nor `absent`",
                Some("Write the first 16 hex digits of the file's BLAKE3 hash, or `absent`."),
            ),
            (
                record(RecordFault::Day("6 October".into())),
                "Malformed record: `6 October` isn't a day",
                Some("Write the day as `2026-10-06`."),
            ),
            (
                record(RecordFault::Twice("stills/youtube-abcdefghijk.jpg".into())),
                "Malformed record: `stills/youtube-abcdefghijk.jpg` has two lines",
                Some("Keep one of them."),
            ),
            (
                record(RecordFault::RefusedOpening),
                "Malformed record: the first line isn't `statac refused 1`",
                Some("Write `statac refused 1` as its first line."),
            ),
            (
                record(RecordFault::RefusedFields(2)),
                "Malformed record: a line has 2 fields, not 3",
                Some("Write `mention` or `sender`, the address, and the day, parted by tabs."),
            ),
            (
                record(RecordFault::Reach("site".into())),
                "Malformed record: `site` is neither `mention` nor `sender`",
                Some("Write `mention` or `sender`."),
            ),
            (
                record(RecordFault::Address("a.example".into())),
                "Malformed record: `a.example` isn't a web address",
                Some("Write the address in full, starting `https://` or `http://`."),
            ),
            (
                Kind::UnreadablePicture {
                    path: "fetched/stills/youtube-abcdefghijk.jpg".into(),
                    fault: PictureFault::NotAStill,
                },
                "Unreadable picture: `fetched/stills/youtube-abcdefghijk.jpg` isn't a JPEG, a \
                 PNG or a still WebP",
                Some("Delete it, then run `statac refresh` to fetch one."),
            ),
            (
                Kind::TokenInSettings,
                "Token in settings: `webmention_token` doesn't belong in a file kept with the site",
                Some(
                    "Remove the line, add `webmentions: true`, and put the token in \
                     `STATAC_WEBMENTION_TOKEN` or in the site's `.statac/token`.",
                ),
            ),
            (
                Kind::WebmentionsOff,
                "Webmentions off: `webmention()` writes nothing here",
                Some("Add `webmentions: true` to `statac.yaml`, or remove the call."),
            ),
            (
                Kind::MentionOfNoPage {
                    target: "https://example.org/post-1/".into(),
                },
                "Mention of no page: nothing is published at `https://example.org/post-1/`",
                Some("If the page moved, add the old address to its `redirect_from`."),
            ),
            (
                Kind::NoSiteAddress {
                    waiting: Waiting::Webmentions,
                },
                "No site address: webmentions are left out until the site has a `url`",
                Some("Add the site's address to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::WrongKind {
                    field: "face".into(),
                    expected: Expected::FaceName,
                    found: Found::Written("\"../record\"".into()),
                    respelt: None,
                },
                "Wrong kind of value: `face` is a face's file name, such as \
                 `0123456789abcdef.jpg`, not `\"../record\"`",
                None,
            ),
            (
                Kind::MissingField {
                    field: "target".into(),
                    required_by: Requirer::Mention,
                },
                "Missing field: `target`",
                Some("Add `target:`, which every mention Statac keeps has."),
            ),
        ]
    }

    #[test]
    fn what_refresh_says_of_webmentions_reads_as_written() {
        assert_eq!(
            NO_TOKEN,
            "Webmentions weren't asked for: no token is in `STATAC_WEBMENTION_TOKEN` or \
             `.statac/token`."
        );
        assert_eq!(
            token_unused(TOKEN_FILE_LINES),
            "`.statac/token` wasn't used: it holds more than one line."
        );
        assert_eq!(
            token_unused(&token_file_linked(".statac")),
            "`.statac/token` wasn't used: `.statac` is a link."
        );
        assert_eq!(
            cant_ask(TOKEN_REFUSED),
            "Can't ask for webmentions: the relay didn't accept the token."
        );
        assert_eq!(
            cant_ask(UNREADABLE_ANSWER),
            "Can't ask for webmentions: the relay's answer couldn't be read."
        );
        assert_eq!(
            not_read(1, "follow-of"),
            "Webmention `1` not kept: Statac doesn't read `follow-of`."
        );
        assert_eq!(
            not_a_web_address(1, "javascript:alert(1)"),
            "Webmention `1` not kept: `javascript:alert(1)` isn't a web address."
        );
        assert_eq!(
            not_on_site(1, "https://example.com/"),
            "Webmention `1` not kept: `https://example.com/` isn't on this site."
        );
        assert_eq!(
            sender_not_kept(1, "ftp://a.example/"),
            "Webmention `1`: `ftp://a.example/` isn't a web address, so the sender's own address \
             isn't kept."
        );
        assert_eq!(
            cant_fetch_face(1, NOT_A_PICTURE),
            "Can't fetch the face for `1`: the answer isn't a picture."
        );
        assert_eq!(
            mentions_refreshed(3, 2, 1),
            "Fetched 3 webmentions and 2 faces. 1 not kept."
        );
        assert_eq!(mentions_refreshed(1, 0, 0), "Fetched 1 webmention.");
        assert_eq!(
            mentions_refreshed(1, 1, 0),
            "Fetched 1 webmention and 1 face."
        );
        assert_eq!(mentions_refreshed(0, 0, 2), "2 not kept.");
        assert_eq!(mentions_refreshed(0, 0, 0), "No new webmentions.");
    }

    #[test]
    fn the_age_of_the_webmentions_is_said_of_the_newest() {
        let line = |parts: &[String]| fetched_ages(parts);
        assert_eq!(
            line(&[stills_age(87, Some(6), 0), mentions_age(12, Some(0))]),
            "Fetched: 87 stills, the oldest 6 days ago; 12 webmentions, the newest today."
        );
        assert_eq!(
            line(&[mentions_age(1, Some(1))]),
            "Fetched: 1 webmention, yesterday."
        );
        assert_eq!(
            line(&[mentions_age(3, None)]),
            "Fetched: 3 webmentions, with no date."
        );
        assert_eq!(
            line(&[mentions_age(2, Some(40))]),
            "Fetched: 2 webmentions, the newest 40 days ago."
        );
    }

    #[test]
    fn fetched_problems_read_as_written() {
        for (kind, line, suggested) in fetched() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    #[test]
    fn every_refusal_of_a_fetch_reads_as_written() {
        let address = "https://i.ytimg.com/vi/abcdefghijk/maxresdefault.jpg";
        let host = || "i.ytimg.com".to_owned();
        let cases = [
            (
                Refusal::NotAnAddress,
                format!("`{address}` isn't a web address"),
            ),
            (
                Refusal::Scheme {
                    scheme: "ftp".into(),
                },
                format!("`{address}` isn't a web address"),
            ),
            (Refusal::NoHost, format!("`{address}` isn't a web address")),
            (
                Refusal::Credentials,
                format!("`{address}` isn't a web address"),
            ),
            (
                Refusal::UnknownHost { host: host() },
                "nothing answers to the name `i.ytimg.com`".into(),
            ),
            (
                Refusal::PrivateAddress {
                    host: host(),
                    address: std::net::IpAddr::from([10, 0, 0, 1]),
                },
                "`i.ytimg.com` is at `10.0.0.1`, which isn't on the open web".into(),
            ),
            (
                Refusal::Missing { status: 404 },
                "the server has nothing at that address".into(),
            ),
            (
                Refusal::Status { status: 503 },
                "the server answered 503".into(),
            ),
            (
                Refusal::Redirects { limit: 5 },
                "the address redirected more than 5 times".into(),
            ),
            (
                Refusal::NoRedirectTarget {
                    host: host(),
                    location: None,
                },
                "the server redirected without saying where".into(),
            ),
            (
                Refusal::TooLarge {
                    limit: 20_000_000,
                    content_length: Some(30_000_000),
                },
                "the answer is larger than 20 MB".into(),
            ),
            (Refusal::TimedOut, "no answer came within 30 seconds".into()),
            (
                Refusal::Tls { host: host() },
                "a secure connection to `i.ytimg.com` couldn't be made".into(),
            ),
            (
                Refusal::Failed {
                    reason: "Connection reset by peer (os error 104)".into(),
                },
                "connection reset by peer".into(),
            ),
        ];
        for (refused, words) in cases {
            assert_eq!(refusal(address, &refused), words, "{refused:?}");
        }
        assert_eq!(NOT_A_PICTURE, "the answer isn't a picture");
        assert_eq!(NO_PICTURE_NAMED, "the video site named no picture");
    }

    #[test]
    fn what_refresh_fetched_is_counted_rightly() {
        assert_eq!(
            refreshed(2, 85, 1),
            "Fetched 2 stills. 85 already here. 1 couldn't be fetched."
        );
        assert_eq!(refreshed(1, 0, 0), "Fetched 1 still.");
        assert_eq!(refreshed(0, 1, 0), "1 already here.");
        assert_eq!(refreshed(0, 0, 0), "Fetched no stills.");
        assert_eq!(
            drafts_not_refreshed(1),
            "Drafts weren't looked at: the site has 1 error with it. Run `statac check --drafts` to see it."
        );
        assert_eq!(
            drafts_not_refreshed(2),
            "Drafts weren't looked at: the site has 2 errors with them. Run `statac check --drafts` to see them."
        );
        assert_eq!(and_more(1), "and 1 more. Run `statac check` to see it.");
        assert_eq!(
            and_more(1200),
            "and 1,200 more. Run `statac check` to see them."
        );
    }

    #[test]
    fn the_age_of_what_was_fetched_is_said_in_days() {
        let line = |stills, oldest, undated| fetched_ages(&[stills_age(stills, oldest, undated)]);
        assert_eq!(
            line(87, Some(6), 0),
            "Fetched: 87 stills, the oldest 6 days ago."
        );
        assert_eq!(line(1, Some(0), 0), "Fetched: 1 still, today.");
        assert_eq!(line(1, Some(1), 0), "Fetched: 1 still, yesterday.");
        assert_eq!(
            line(87, Some(6), 2),
            "Fetched: 87 stills, the oldest 6 days ago; 2 with no date."
        );
        assert_eq!(line(2, None, 2), "Fetched: 2 stills, with no date.");
        // Another part joins after a semicolon.
        assert_eq!(
            fetched_ages(&["1 still, today".into(), "a part to come".into()]),
            "Fetched: 1 still, today; a part to come."
        );
    }

    fn cards() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let wrong = |field: &str, expected, written: &str, respelt: Option<&str>| Kind::WrongKind {
            field: field.into(),
            expected,
            found: Found::Written(written.into()),
            respelt: respelt.map(Into::into),
        };
        vec![
            (
                Kind::MissingField {
                    field: "background".into(),
                    required_by: Requirer::Card,
                },
                "Missing field: `background`",
                Some("Add `background:`, which every card needs."),
            ),
            (
                Kind::MissingField {
                    field: "title.size".into(),
                    required_by: Requirer::CardText,
                },
                "Missing field: `title.size`",
                Some("Add `title.size:`, which each text on a card needs."),
            ),
            (
                Kind::UnknownWord {
                    word: Word::Place,
                    written: "top lft".into(),
                    nearest: Some("top left".into()),
                },
                "Unknown place: `top lft`",
                Some("Did you mean `top left`?"),
            ),
            (
                Kind::OtherSpelling {
                    word: Word::Place,
                    written: "center".into(),
                    ours: "centre".into(),
                },
                "Unknown place: `center`",
                Some("Statac calls this `centre`."),
            ),
            (
                Kind::OtherSpelling {
                    word: Word::Place,
                    written: "Top Left".into(),
                    ours: "top left".into(),
                },
                "Unknown place: `Top Left`",
                Some("Statac calls this `top left`."),
            ),
            (
                Kind::OtherSpelling {
                    word: Word::Field,
                    written: "title.color".into(),
                    ours: "title.colour".into(),
                },
                "Unknown field: `title.color`",
                Some("Statac calls this `title.colour`."),
            ),
            (
                wrong("title.size", Expected::CardSize, "big", None),
                "Wrong kind of value: `title.size` is a size from 1 to 630, such as `36` or \
                 `up to 75`, not `big`",
                None,
            ),
            (
                wrong(
                    "title.size",
                    Expected::CardSize,
                    "Up to 75",
                    Some("up to 75"),
                ),
                "Wrong kind of value: `title.size` is a size from 1 to 630, such as `36` or \
                 `up to 75`, not `Up to 75`",
                Some("Write it as `up to 75`."),
            ),
            (
                wrong("title.colour", Expected::Colour, "red", None),
                "Wrong kind of value: `title.colour` is a colour such as `#F2E9D3` or `#FFF`, \
                 or a template, not `red`",
                None,
            ),
            (
                wrong("font", Expected::FontFile, "text.woff", None),
                "Wrong kind of value: `font` is a font file ending `.ttf`, `.otf` or `.ttc`, \
                 not `text.woff`",
                None,
            ),
            (
                Kind::NeedsQuotes {
                    written: "#F2E9D3".into(),
                    reading: "a comment".into(),
                },
                "Needs quotes: YAML reads `#F2E9D3` as a comment",
                Some("Write it as `\"#F2E9D3\"` to keep it as written."),
            ),
            (
                Kind::OutsideSite {
                    reach: Reach::CardFile("../background.png".into()),
                },
                "Outside the site: `../background.png` isn't in `templates/og/`",
                Some("Copy the file into `templates/og/` and name it from there."),
            ),
            (
                Kind::MissingFile {
                    path: "templates/og/backgrund.png".into(),
                    hint: Some(Hint::Closest("background.png".into())),
                },
                "Missing file: there is no `templates/og/backgrund.png`",
                Some("Did you mean `background.png`?"),
            ),
            (
                Kind::NestedSetting {
                    key: "text".into(),
                    under: "title".into(),
                    flat: "title.text: x".into(),
                },
                "Nested setting: `text` under `title`",
                Some("Write it flat: `title.text: x`."),
            ),
            (
                wrong(
                    "padding",
                    Expected::RoundedBetween { from: 0, to: 314 },
                    "400",
                    None,
                ),
                "Wrong kind of value: `padding` is a number from 0 to 314, not `400`",
                None,
            ),
            (
                Kind::WrongKind {
                    field: "padding".into(),
                    expected: Expected::RoundedBetween { from: 0, to: 314 },
                    found: Found::List,
                    respelt: None,
                },
                "Wrong kind of value: `padding` is a number from 0 to 314, not a list",
                None,
            ),
            (
                Kind::SetAside {
                    field: "backgroud".into(),
                    why: SetAside::NotOnCard {
                        nearest: Some("background".into()),
                    },
                },
                "Field set aside: a card has no `backgroud`",
                Some("Did you mean `background`?"),
            ),
            (
                Kind::SetAside {
                    field: "title.shadow".into(),
                    why: SetAside::NotOnCard { nearest: None },
                },
                "Field set aside: a card has no `title.shadow`",
                Some(
                    "A card reads `background`, `font` and `padding`, and `text`, `place`, \
                     `size` and `colour` for each text.",
                ),
            ),
            (
                Kind::SizeKept {
                    field: "footer.size".into(),
                    written: "up to 36".into(),
                    at: "bottom".into(),
                    size: 36,
                },
                "Size kept: at `bottom`, `up to 36` is drawn at 36",
                Some(
                    "Write `footer.size: 36`; only a text at `left`, `centre` or `right` is fitted.",
                ),
            ),
            (
                Kind::UnknownWord {
                    word: Word::Card,
                    written: "wid".into(),
                    nearest: Some("wide".into()),
                },
                "Unknown card: `wid`",
                Some("Did you mean `wide`?"),
            ),
            (
                Kind::SetAside {
                    field: "card".into(),
                    why: SetAside::OwnPicture,
                },
                "Field set aside: `og_image` names the picture this page is shared with",
                Some("Take out `card`, or take out `og_image` to have the card drawn."),
            ),
            (
                Kind::WrongKind {
                    field: "og_image".into(),
                    expected: Expected::OwnPicture,
                    found: Found::Written("https://example.org/photo-1.jpg".into()),
                    respelt: None,
                },
                "Wrong kind of value: `og_image` is a picture of the site's, not \
                 `https://example.org/photo-1.jpg`",
                None,
            ),
            (
                Kind::WrongKind {
                    field: "title.colour".into(),
                    expected: Expected::BuiltColour,
                    found: Found::Nothing,
                    respelt: None,
                },
                "Wrong kind of value: `title.colour` is a colour such as `#F2E9D3` or `#FFF`, \
                 not nothing",
                None,
            ),
            (
                Kind::TemplateStopped {
                    reason: Stopped::CardInWriting,
                },
                "Template stopped: `page.card` isn't answered in a page's writing",
                Some("Read `page.card` in a layout."),
            ),
            (
                Kind::TemplateStopped {
                    reason: Stopped::CardInCard,
                },
                "Template stopped: `page.card` isn't answered in a card's own text",
                None,
            ),
        ]
    }

    #[test]
    fn card_problems_read_as_written() {
        for (kind, line, suggested) in cards() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    fn links() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        vec![
            (
                Kind::MissingDefinition {
                    written: "[^2]".into(),
                    nearest: Some("[^1]".into()),
                },
                "Missing definition: there is no `[^2]:` line",
                Some("Did you mean `[^1]`?"),
            ),
            (
                Kind::MissingDefinition {
                    written: "[rust]".into(),
                    nearest: None,
                },
                "Missing definition: there is no `[rust]:` line",
                None,
            ),
            (
                Kind::MissingAnchor {
                    id: "rust".into(),
                    page: Some("/posts/post-1/".into()),
                    suggestion: Some("#rust-1".into()),
                },
                "Missing anchor: nothing on `/posts/post-1/` has the id `rust`",
                Some("Did you mean `#rust-1`?"),
            ),
            (
                Kind::MissingAnchor {
                    id: "note-2".into(),
                    page: None,
                    suggestion: None,
                },
                "Missing anchor: nothing on this page has the id `note-2`",
                None,
            ),
            (
                Kind::FootnoteNeverShown {
                    label: "sources".into(),
                    nearest: Some("source".into()),
                },
                "Footnote never shown: nothing points at `[^sources]`",
                Some("Is it the note for `[^source]`?"),
            ),
            (
                Kind::FootnoteNeverShown {
                    label: "1".into(),
                    nearest: None,
                },
                "Footnote never shown: nothing points at `[^1]`",
                None,
            ),
            (
                Kind::DefinitionWrittenTwice {
                    label: "n".into(),
                    shown: None,
                },
                "Definition written twice: the first `[^n]:` line isn't shown",
                Some("Give one of them another name, or take one out."),
            ),
            (
                Kind::CellNeverShown {
                    cells: vec!["Extra".into()],
                    columns: 2,
                },
                "Cell never shown: `Extra` is past the table's 2 columns",
                Some("Add a column to the table's first two lines, or take the cell out."),
            ),
            (
                Kind::CellNeverShown {
                    cells: vec!["Extra".into(), "More".into()],
                    columns: 2,
                },
                "Cells never shown: 2 cells are past the table's 2 columns",
                Some("Add columns to the table's first two lines, or take the cells out."),
            ),
            (
                Kind::CellNeverShown {
                    cells: vec!["Extra".into()],
                    columns: 1,
                },
                "Cell never shown: `Extra` is past the table's 1 column",
                Some("Add a column to the table's first two lines, or take the cell out."),
            ),
            (
                Kind::IdUsedTwice {
                    id: "rust".into(),
                    taken: Taken::Heading(crate::source::Place {
                        source: crate::source::SourceId::from_index(0),
                        span: crate::source::Span::new(0, 1),
                    }),
                },
                "Id used twice: a heading above has the id `rust`",
                Some("Give this heading an id of its own."),
            ),
            (
                Kind::IdUsedTwice {
                    id: "note-1".into(),
                    taken: Taken::Footnote,
                },
                "Id used twice: a footnote has the id `note-1`",
                Some("Give this heading an id of its own."),
            ),
            (
                Kind::UnmatchedTag {
                    unmatched: Unmatched::NeverClosed("div".into()),
                },
                "Unmatched tag: `<div>` is never closed",
                Some("Close it with `</div>` where it should end."),
            ),
            (
                Kind::UnmatchedTag {
                    unmatched: Unmatched::NeverClosed("TABLE".into()),
                },
                "Unmatched tag: `<TABLE>` is never closed",
                Some("Close it with `</TABLE>` where it should end."),
            ),
            (
                Kind::UnmatchedTag {
                    unmatched: Unmatched::CommentNeverClosed,
                },
                "Unmatched tag: `<!--` is never closed",
                Some("Close it with `-->`."),
            ),
            (
                Kind::UnmatchedTag {
                    unmatched: Unmatched::ClosesNothing("div".into()),
                },
                "Unmatched tag: `</div>` closes nothing opened before it",
                Some("Take it out, or open the element above it."),
            ),
            (
                Kind::BrokenLink {
                    target: "/blog/post-1.md".into(),
                    suggestion: Some("/blog/post-1/".into()),
                    inside: false,
                },
                "Broken link: nothing is published at `/blog/post-1.md`",
                Some("Did you mean `/blog/post-1/`?"),
            ),
            (
                Kind::BrokenLink {
                    target: "/blog/about-us/".into(),
                    suggestion: Some("@url(/about/)".into()),
                    inside: false,
                },
                "Broken link: nothing is published at `/blog/about-us/`",
                Some("Did you mean `@url(/about/)`?"),
            ),
            (
                Kind::BrokenLink {
                    target: "/about/".into(),
                    suggestion: Some("@url(/about/)".into()),
                    inside: true,
                },
                "Broken link: nothing is published at `/about/`",
                Some("To link inside the site, write `@url(/about/)`."),
            ),
            (
                Kind::BrokenLink {
                    target: "/about/".into(),
                    suggestion: Some(r#"{{ url("/about/") }}"#.into()),
                    inside: true,
                },
                "Broken link: nothing is published at `/about/`",
                Some(r#"To link inside the site, write `{{ url("/about/") }}`."#),
            ),
        ]
    }

    #[test]
    fn link_problems_read_as_written() {
        for (kind, line, suggested) in links() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    fn feeds() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let site = (Word::Folder, String::new());
        let not_written = |list, more, why| Kind::FeedNotWritten { list, more, why };
        vec![
            (
                not_written(
                    site.clone(),
                    0,
                    Unwritten::NoTemplate("templates/feed.xml".into()),
                ),
                "Feed not written: the site's own list needs `templates/feed.xml`",
                Some(
                    "Add the template to publish it, or write `pages.feed: false` in \
                     `statac.yaml` for no feeds.",
                ),
            ),
            (
                not_written(
                    (Word::Folder, "posts".into()),
                    2,
                    Unwritten::NoTemplate("templates/feed.xml".into()),
                ),
                "Feed not written: the folder `content/posts/` and 2 other lists need \
                 `templates/feed.xml`",
                Some(
                    "Add the template to publish them, or write `pages.feed: false` in \
                     `statac.yaml` for no feeds.",
                ),
            ),
            (
                not_written((Word::Tag, "Rust".into()), 0, Unwritten::NoTitle),
                "Feed not written: the tag `Rust` needs the site's `title`",
                Some("Add `title:` to `statac.yaml`; a feed is named by it."),
            ),
            (
                not_written(
                    (Word::Page, "content/list-1.md".into()),
                    0,
                    Unwritten::PageThere {
                        page: "content/feed.md".into(),
                        address: "/list-1/feed.xml".into(),
                    },
                ),
                "Feed not written: `content/feed.md` is published at `/list-1/feed.xml`",
                Some(
                    "The page is published there instead; give the feed another address \
                     with `feed:` on the list's page.",
                ),
            ),
            (
                not_written((Word::Folder, "notes".into()), 0, Unwritten::Undated),
                "Feed not written: the folder `content/notes/` has nothing dated",
                Some("Give a page in it a `date:`, or take the `feed:` line out."),
            ),
            (
                Kind::NoFeedKey { key: "key".into() },
                "No feed key: `statac.yaml` has no `feed_key`",
                Some(
                    "Add `feed_key: key` to `statac.yaml` and never change it: feed readers \
                     know each post by an id made from it.",
                ),
            ),
            (
                Kind::NoLanguage,
                "No language set: the site is read as English",
                Some("Add the site's language to `statac.yaml`, as `language: en`."),
            ),
            (
                Kind::NoSiteAddress {
                    waiting: Waiting::Page("content/sitemap.md".into()),
                },
                "No site address: `content/sitemap.md` is left out until the site has a `url`",
                Some("Add the site's address to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::NoSiteAddress {
                    waiting: Waiting::Feeds {
                        list: site.clone(),
                        more: 0,
                    },
                },
                "No site address: the feed of the site's own list is left out until the site \
                 has a `url`",
                Some("Add the site's address to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::NoSiteAddress {
                    waiting: Waiting::Feeds {
                        list: (Word::Series, "Series 1".into()),
                        more: 1,
                    },
                },
                "No site address: the feeds of the series `Series 1` and 1 other list are left \
                 out until the site has a `url`",
                Some("Add the site's address to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::FeedIdChanged {
                    was: "/posts/post-1/".into(),
                    id: "fc370633-88f0-8092-9f96-7749dbfff0eb".into(),
                },
                "Feed id changed: this post was published at `/posts/post-1/`",
                Some(
                    "Add `id: fc370633-88f0-8092-9f96-7749dbfff0eb` to its front matter to \
                     keep it the same post in feed readers.",
                ),
            ),
            (
                Kind::RobotsBeneathPath {
                    top: "https://example.org/robots.txt".into(),
                },
                "Robots beneath a path: crawlers read only `https://example.org/robots.txt`",
                Some(
                    "Copy its lines into the `robots.txt` at the top of the host to have them \
                     read.",
                ),
            ),
            (
                Kind::UnusableAddress {
                    fault: AddressFault::FeedAtFolder {
                        nearest: "/feeds/feed.xml".into(),
                    },
                },
                "Unusable address: a feed can't be published at a folder",
                Some("Did you mean `/feeds/feed.xml`?"),
            ),
            (
                Kind::UnusableAddress {
                    fault: AddressFault::SlashBesidePage {
                        nearest: "/posts/feeds/atom.xml".into(),
                    },
                },
                "Unusable address: a name beside the page can't hold `/`",
                Some("Did you mean `/posts/feeds/atom.xml`?"),
            ),
            (
                Kind::TemplateStopped {
                    reason: Stopped::NoFeedKey("key".into()),
                },
                "Template stopped: `feed_id` needs the site's `feed_key`",
                Some(
                    "Add `feed_key: key` to `statac.yaml` and never change it: feed readers \
                     know each post by an id made from it.",
                ),
            ),
        ]
    }

    #[test]
    fn feed_problems_read_as_written() {
        for (kind, line, suggested) in feeds() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    fn lists() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let malformed = |fault| Kind::MalformedList { fault };
        let mut sources = crate::source::Sources::default();
        let first = crate::source::Place {
            source: sources.add("content/post-1.md", "One.\n\n<!--more-->\n".into()),
            span: crate::source::Span::new(6, 17),
        };
        vec![
            (
                Kind::SpelledTwoWays {
                    word: Word::Tag,
                    spellings: vec![("Rust".into(), 12), ("rust".into(), 2)],
                    chosen: "Rust".into(),
                },
                "Tag spelled two ways: `Rust` (12 pages) and `rust` (2 pages) are one tag, shown as `Rust`",
                Some("Use one spelling on every page to choose how it's shown."),
            ),
            (
                Kind::SpelledTwoWays {
                    word: Word::Series,
                    spellings: vec![
                        ("Series 1".into(), 2),
                        ("series 1".into(), 1),
                        ("SERIES 1".into(), 1),
                    ],
                    chosen: "Series 1".into(),
                },
                "Series spelled two ways: `Series 1` (2 pages), `series 1` (1 page) and \
                 `SERIES 1` (1 page) are one series, shown as `Series 1`",
                Some("Use one spelling on every page to choose how it's shown."),
            ),
            (
                Kind::DraftLeftOut {
                    path: "content/post-1.md".into(),
                },
                "Draft left out: `content/post-1.md`",
                Some("Run with `--drafts` to build it as if published."),
            ),
            (
                Kind::WaitingForItsDay {
                    path: "content/post-2.md".into(),
                    date: "2026-10-05".into(),
                },
                "Waiting for its day: `content/post-2.md` is dated `2026-10-05`",
                Some("Run with `--drafts` to build it as if published."),
            ),
            (
                Kind::NumberedFolder {
                    folder: "content/days/".into(),
                },
                "Numbered folder: `content/days/` is ordered by the number each name opens with",
                Some(
                    "If the numbers are part of the names, write `numbered: false` in \
                     `content/days/index.md`.",
                ),
            ),
            (
                Kind::UnorderedPages {
                    names: vec!["day-four".into(), "day-five".into()],
                },
                "Unordered pages: `order` leaves out `day-four` and `day-five`",
                Some("They follow in their usual order; name them in `order` to place them."),
            ),
            (
                Kind::UnknownWord {
                    word: Word::Page,
                    written: "day-for".into(),
                    nearest: Some("day-four".into()),
                },
                "Unknown page: `day-for`",
                Some("Did you mean `day-four`?"),
            ),
            (
                Kind::WrongPlace {
                    key: "numbered".into(),
                    why: WrongPlace::FolderOnly,
                },
                "Wrong place: `numbered` belongs in a folder's `index.md`",
                Some("Move it to the folder's `index.md`."),
            ),
            (
                Kind::OtherSpelling {
                    word: Word::Marker,
                    written: "<!-- More -->".into(),
                    ours: "<!--more-->".into(),
                },
                "Unknown marker: `<!-- More -->`",
                Some("Statac calls this `<!--more-->`."),
            ),
            (
                Kind::OtherSpelling {
                    word: Word::Marker,
                    written: "+++".into(),
                    ours: "---".into(),
                },
                "Unknown marker: `+++`",
                Some("Write the front matter as YAML between `---` lines."),
            ),
            (
                Kind::WrongPlace {
                    key: "<!--more-->".into(),
                    why: WrongPlace::More(MoreAt::Quotation),
                },
                "Wrong place: `<!--more-->` can't end an excerpt inside a quotation",
                Some("Move it to a line of its own between two paragraphs."),
            ),
            (
                Kind::WrongPlace {
                    key: "<!--more-->".into(),
                    why: WrongPlace::More(MoreAt::Call("box".into())),
                },
                "Wrong place: `<!--more-->` can't end an excerpt inside `@box`",
                Some("Move it to a line of its own between two paragraphs."),
            ),
            (
                Kind::WrongPlace {
                    key: "<!--more-->".into(),
                    why: WrongPlace::More(MoreAt::Element("div".into())),
                },
                "Wrong place: `<!--more-->` can't end an excerpt inside `<div>`",
                Some("Move it outside the `<div>`."),
            ),
            (
                Kind::WrongPlace {
                    key: "<!-- more -->".into(),
                    why: WrongPlace::More(MoreAt::Words),
                },
                "Wrong place: `<!-- more -->` ends an excerpt only on a line of its own",
                Some("Move it to a line of its own between two paragraphs."),
            ),
            (
                Kind::MoreWrittenTwice {
                    written: "<!--more-->".into(),
                    first,
                },
                "More written twice: the excerpt ends at the first `<!--more-->`",
                Some("Delete the later one."),
            ),
            (
                malformed(ListFault::NotFromThePosts("site.pages".into())),
                "Malformed list: a list starts from `site.posts`, not `site.pages`",
                Some("Start it with `site.posts`, as `site.posts | tagged(\"Rust\")`."),
            ),
            (
                malformed(ListFault::NothingAfterPipe),
                "Malformed list: nothing follows `|`",
                Some("Name a filter after it, or take it out."),
            ),
            (
                malformed(ListFault::NotChainable("sort".into())),
                "Malformed list: `sort` can't choose a list's posts",
                Some("A list chooses its posts with `tagged`, `in` and `where`."),
            ),
            (
                malformed(ListFault::NoBrackets("tagged".into())),
                "Malformed list: `tagged` needs brackets",
                Some("Write it as `tagged(...)`."),
            ),
            (
                malformed(ListFault::Unclosed('(')),
                "Malformed list: `(` is never closed",
                Some("Close it with `)`."),
            ),
            (
                malformed(ListFault::Unclosed('"')),
                "Malformed list: `\"` is never closed",
                Some("End the value with another `\"`."),
            ),
            (
                malformed(ListFault::Values {
                    filter: "where".into(),
                    wants: 2,
                    given: 1,
                }),
                "Malformed list: `where` takes 2 values, not 1",
                Some("Write it as `where(\"field\", \"value\")`."),
            ),
            (
                malformed(ListFault::Values {
                    filter: "tagged".into(),
                    wants: 1,
                    given: 0,
                }),
                "Malformed list: `tagged` takes 1 value, not 0",
                Some("Write it as `tagged(\"...\")`."),
            ),
            (
                Kind::NotAPost {
                    path: "content/tags/rust.md".into(),
                    word: Word::Tag,
                    name: "Rust".into(),
                },
                "Not a post: `content/tags/rust.md` speaks for the tag `Rust`",
                Some(
                    "It stands at the list's address, so no list holds it; give it a \
                     `permalink` of its own to list it as a post.",
                ),
            ),
            (
                Kind::NotAPost {
                    path: "content/notes.md".into(),
                    word: Word::Folder,
                    name: "notes".into(),
                },
                "Not a post: `content/notes.md` speaks for the folder `content/notes/`",
                Some(
                    "It stands at the list's address, so no list holds it; give it a \
                     `permalink` of its own to list it as a post.",
                ),
            ),
            (
                Kind::NotAPost {
                    path: "content/home.md".into(),
                    word: Word::Folder,
                    name: String::new(),
                },
                "Not a post: `content/home.md` speaks for the site's own list",
                Some(
                    "It stands at the list's address, so no list holds it; give it a \
                     `permalink` of its own to list it as a post.",
                ),
            ),
            (
                Kind::RedirectToItself {
                    written: "/post-1/".into(),
                },
                "Redirect to itself: `/post-1/` is this page's own address",
                Some("Point `redirect_to` at the address readers should reach."),
            ),
            (
                Kind::RedirectedBack {
                    written: "/post-2/".into(),
                },
                "Redirect to itself: `/post-2/` sends readers back to this page",
                None,
            ),
            (
                Kind::WritingNotPublished {
                    path: "content/post-1.md".into(),
                },
                "Writing not published: `content/post-1.md` sends readers on",
                Some("Move it to where readers are sent, or take out `redirect_to` to publish it."),
            ),
            (
                Kind::SortedLast {
                    field: "rank".into(),
                    count: 1,
                    first: "content/post-2.md".into(),
                    instead: Some(
                        "site.pages | selectattr(\"rank\", \"defined\") | sort(attribute=\"rank\")"
                            .into(),
                    ),
                },
                "Sorted last: `content/post-2.md` has no `rank`",
                Some(
                    "Write `site.pages | selectattr(\"rank\", \"defined\") | \
                     sort(attribute=\"rank\")` to leave out the items without it.",
                ),
            ),
            (
                Kind::SortedLast {
                    field: "rank".into(),
                    count: 3,
                    first: Named::Item("item 2".into()),
                    instead: None,
                },
                "Sorted last: 3 items have no `rank`, starting with item 2",
                None,
            ),
            (
                Kind::LoopOverEveryPage {
                    written: Some("site.pages".into()),
                    pages: 11243,
                    instead: Some("site.pages | selectattr(\"menu\")".into()),
                },
                "Loop over every page: `site.pages` is gone through on 11,243 pages",
                Some(
                    "Write `site.pages | selectattr(\"menu\")` to choose the pages once for \
                     the site.",
                ),
            ),
            (
                Kind::LoopOverEveryPage {
                    written: None,
                    pages: 4000,
                    instead: None,
                },
                "Loop over every page: 4,000 pages each go through a list of the whole site",
                None,
            ),
            (
                Kind::CallInHtml {
                    name: "book".into(),
                },
                "Call in HTML: `@book` is built as a snippet",
                Some(r"To show it as text, write `\@book`."),
            ),
            (
                Kind::UntitledPage {
                    page: Named::File("content/post-1.md".into()),
                    by: Called::OpeningWords,
                    name: "Post 1 opens\u{2026}".into(),
                },
                "Untitled page: `content/post-1.md` is called by its opening words, `Post 1 opens\u{2026}`",
                Some(
                    "Give it a `title`, or open its writing with a `# heading`. A folder whose pages go without titles says `pages.untitled: true`.",
                ),
            ),
            (
                Kind::UntitledPage {
                    page: Named::ListPage(Word::Folder, String::new()),
                    by: Called::Address,
                    name: "/".into(),
                },
                "Untitled page: the site's home page is called by its address, `/`",
                Some("Add `title:` to `statac.yaml`; the home page is called by it."),
            ),
            (
                Kind::SetAside {
                    field: "tags".into(),
                    why: SetAside::SendsOn,
                },
                "Field set aside: a page sending readers on publishes only its redirect",
                Some("Take it out, or take out `redirect_to` to publish the page."),
            ),
        ]
    }

    #[test]
    fn list_problems_read_as_written() {
        for (kind, line, suggested) in lists() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    #[test]
    fn what_drafts_included_is_counted() {
        assert_eq!(included(2, 0), "Included 2 drafts as if published.");
        assert_eq!(
            included(1, 1),
            "Included 1 draft and 1 post waiting for its day as if published."
        );
        assert_eq!(
            included(0, 3),
            "Included 3 posts waiting for their day as if published."
        );
    }

    fn data() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let mut sources = crate::source::Sources::default();
        let first = crate::source::Place {
            source: sources.add("data/people.yaml", "- id: k3m9x2\n".into()),
            span: crate::source::Span::new(6, 12),
        };
        vec![
            (
                Kind::NotData {
                    path: "data/people.txt".into(),
                },
                "Not data: `data/people.txt` isn't YAML or JSON",
                Some("Save it as `.yaml` or `.json`, or move it out of `data/`."),
            ),
            (
                Kind::DataNamedTwice {
                    first: "data/people.json".into(),
                    second: "data/people.yaml".into(),
                    name: "site.data.people".into(),
                },
                "Data named twice: `data/people.json` and `data/people.yaml` are both \
                 `site.data.people`",
                Some("Rename one of them."),
            ),
            (
                Kind::TemplateStopped {
                    reason: Stopped::NotText,
                },
                "Template stopped: a `!!binary` value holds bytes that aren't text",
                Some("Show something else here: a page can only show text."),
            ),
            (
                Kind::MergeKey,
                "Merge key: Statac doesn't copy fields in with `<<`",
                Some("Write the fields out in full."),
            ),
            (
                Kind::SameId {
                    id: "k3m9x2".into(),
                    first,
                },
                "Same id: two pages have the id `k3m9x2`",
                Some("Give each page its own id."),
            ),
            (
                Kind::TwoLists {
                    first: "posts".into(),
                    second: "each".into(),
                    first_at: first,
                },
                "Two lists: this page has both `posts:` and `each:`",
                Some("Move one of them to a page of its own."),
            ),
            (
                Kind::WrongKind {
                    field: "each".into(),
                    expected: Expected::TableName,
                    found: Found::Written("people".into()),
                    respelt: None,
                },
                "Wrong kind of value: `each` is `site.posts` or a table in `data/`, such as \
                 `data.people`, not `people`",
                None,
            ),
            (
                Kind::WrongKind {
                    field: "data.people".into(),
                    expected: Expected::Table,
                    found: Found::Written("Person 1".into()),
                    respelt: None,
                },
                "Wrong kind of value: `data.people` is a table of items, not `Person 1`",
                None,
            ),
            (
                Kind::UnknownWord {
                    word: Word::Table,
                    written: "data.peple".into(),
                    nearest: Some("data.people".into()),
                },
                "Unknown table: `data.peple`",
                Some("Did you mean `data.people`?"),
            ),
            (
                Kind::NamesNothing {
                    written: "persn-1".into(),
                    among: NamedIn::Data("data/people.yaml".into()),
                    nearest: Some("person-1".into()),
                },
                "Names nothing: `data/people.yaml` has no `persn-1`",
                Some("Did you mean `person-1`?"),
            ),
            (
                Kind::NamesNothing {
                    written: "book-12".into(),
                    among: NamedIn::Folder("content/books/".into()),
                    nearest: Some("book-1".into()),
                },
                "Names nothing: no page in `content/books/` is called `book-12`",
                Some("Did you mean `book-1`?"),
            ),
            (
                Kind::NamesNothing {
                    written: "post-1".into(),
                    among: NamedIn::Folder("content/books/".into()),
                    nearest: None,
                },
                "Names nothing: no page in `content/books/` is called `post-1`",
                None,
            ),
            (
                Kind::MissingData {
                    table: "data.poeple".into(),
                    declared: "fields.author".into(),
                    nearest: Some("data.people".into()),
                },
                "Missing data: there is no `data.poeple` for `fields.author` to name",
                Some("Did you mean `data.people`?"),
            ),
            (
                Kind::MissingContentFolder {
                    folder: "content/boks/".into(),
                    declared: "types.review.book".into(),
                    nearest: Some("folder.books".into()),
                },
                "Missing folder: there is no `content/boks/` for `types.review.book` to name",
                Some("Did you mean `folder.books`?"),
            ),
            (
                Kind::NoNamesToCheck {
                    path: "data/people.yaml".into(),
                    declared: "fields.author".into(),
                },
                "No names to check: `data/people.yaml` is a list, so `fields.author` can't name \
                 one of its items",
                Some(
                    "Give each item a name with its fields beneath, or declare the field as `text`.",
                ),
            ),
            (
                Kind::MissingField {
                    field: "permalink".into(),
                    required_by: Requirer::Each,
                },
                "Missing field: `permalink`",
                Some(
                    "Add a `permalink:` pattern, such as `/people/{key}/`, to give each item its address.",
                ),
            ),
            (
                Kind::WrongPlace {
                    key: "each".into(),
                    why: WrongPlace::Item,
                },
                "Wrong place: an item can't set `each`",
                Some("Write it on a page in `content/` instead."),
            ),
            (
                Kind::UnusableAddress {
                    fault: AddressFault::ItemLacks {
                        item: "data/people.yaml: item 3".into(),
                        field: "isbn".into(),
                        piece: "{isbn}".into(),
                    },
                },
                "Unusable address: `data/people.yaml: item 3` has no `isbn` for `{isbn}`",
                Some("Add `isbn:` to the item, or take `{isbn}` out of the pattern."),
            ),
            (
                Kind::UnusableAddress {
                    fault: AddressFault::ItemLacks {
                        item: "data/people.yaml: item 3".into(),
                        field: "key".into(),
                        piece: "{key}".into(),
                    },
                },
                "Unusable address: `data/people.yaml: item 3` has no `key` for `{key}`",
                Some("Give the table's items names, or take `{key}` out of the pattern."),
            ),
            (
                Kind::UnusableAddress {
                    fault: AddressFault::PostLacks {
                        post: "content/post-1.md".into(),
                        field: "isbn".into(),
                        piece: "{isbn}".into(),
                    },
                },
                "Unusable address: `content/post-1.md` has no `isbn` for `{isbn}`",
                Some("Add `isbn:` to the post, or take `{isbn}` out of the pattern."),
            ),
            (
                Kind::MissingField {
                    field: "permalink".into(),
                    required_by: Requirer::EachPost,
                },
                "Missing field: `permalink`",
                Some(
                    "Add a `permalink:` pattern, such as `/cards/{name}/`, to give the page made \
                     for each post its address.",
                ),
            ),
        ]
    }

    #[test]
    fn data_problems_read_as_written() {
        for (kind, line, suggested) in data() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    fn files() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let same = |alike| Kind::SameAddress {
            first: "static/Logo.png".into(),
            second: "static/logo.png".into(),
            alike,
            address: "/logo.png".into(),
        };
        let unwritable = |why| Kind::UnwritableName {
            path: "static/con.txt".into(),
            why,
        };
        let occupied = |found| Kind::NotStatacsFolder {
            path: "docs".into(),
            found,
        };
        vec![
            (
                Kind::MissingFile {
                    path: "assets/sit.css".into(),
                    hint: Some(Hint::Closest("site.css".into())),
                },
                "Missing file: there is no `assets/sit.css`",
                Some("Did you mean `site.css`?"),
            ),
            (
                Kind::MissingFile {
                    path: "assets/robots.txt".into(),
                    hint: Some(Hint::InStatic("/robots.txt".into())),
                },
                "Missing file: there is no `assets/robots.txt`",
                Some("Link it as `/robots.txt`, where `static/` publishes it under its own name."),
            ),
            (
                Kind::MissingFile {
                    path: "assets/print.css".into(),
                    hint: None,
                },
                "Missing file: there is no `assets/print.css`",
                None,
            ),
            (
                Kind::SassStopped {
                    message: "Undefined variable.".into(),
                },
                "Sass stopped: undefined variable",
                None,
            ),
            (
                Kind::SassWarning {
                    message: "$columns is deprecated".into(),
                },
                "Sass warning: `$columns is deprecated`",
                None,
            ),
            (
                Kind::WorkerOutsideImportMap {
                    written: "./worker.js".into(),
                },
                "Worker outside the import map: `./worker.js` isn't published under that name",
                Some(
                    "Keep the worker and the files it imports in `static/`, which publishes \
                     them under their own names.",
                ),
            ),
            (
                Kind::SameAddress {
                    first: "assets/site.css".into(),
                    second: "assets/site.scss".into(),
                    alike: Alike::Exactly,
                    address: "/site.css".into(),
                },
                "Same address: `assets/site.css` and `assets/site.scss` are both published at \
                 `/site.css`",
                Some("Rename one of them."),
            ),
            (
                Kind::LaterPageTaken {
                    list: "/blog/".into(),
                    number: 2,
                    page: "content/blog/page/2.md".into(),
                    address: "/blog/page/2/".into(),
                },
                "Same address: page 2 of `/blog/` and `content/blog/page/2.md` are both \
                 published at `/blog/page/2/`",
                Some(
                    "Give `content/blog/page/2.md` another address, or put later pages \
                     elsewhere with `later_pages` in `statac.yaml`.",
                ),
            ),
            (
                Kind::AddressTaken {
                    held: Held::Draft,
                    address: "/post-1/".into(),
                    taken: "content/post-2.md".into(),
                },
                "Address taken: this draft would be published at `/post-1/`, as \
                 `content/post-2.md` is",
                Some("Give one of them another `permalink` before it is published."),
            ),
            (
                Kind::AddressTaken {
                    held: Held::Waiting,
                    address: "/post-1/".into(),
                    taken: "content/post-2.md".into(),
                },
                "Address taken: this post would be published at `/post-1/`, as \
                 `content/post-2.md` is",
                Some("Give one of them another `permalink` before its day comes."),
            ),
            (
                same(Alike::ByCase),
                "Same address: `static/Logo.png` and `static/logo.png` differ only in capitals",
                Some("Most disks hold them as one file, so rename one of them."),
            ),
            (
                same(Alike::ByNormalisation),
                "Same address: `static/Logo.png` and `static/logo.png` differ only in how an \
                 accent is stored",
                Some("Most disks hold them as one file, so rename one of them."),
            ),
            (
                unwritable(Unwritable::Device("con".into())),
                "Unwritable name: `static/con.txt` uses `con`, a device name on Windows",
                None,
            ),
            (
                unwritable(Unwritable::Character(':')),
                "Unwritable name: Windows refuses `:` in `static/con.txt`",
                None,
            ),
            (
                Kind::UnwritableName {
                    path: "static/a\u{7}b".into(),
                    why: Unwritable::Control,
                },
                "Unwritable name: `static/a\\u{7}b` holds a control character",
                None,
            ),
            (
                unwritable(Unwritable::TrailingDot),
                "Unwritable name: Windows drops the dot ending a name in `static/con.txt`",
                None,
            ),
            (
                unwritable(Unwritable::TrailingSpace),
                "Unwritable name: Windows drops the space ending a name in `static/con.txt`",
                None,
            ),
            (
                unwritable(Unwritable::NotText),
                "Unwritable name: `static/con.txt` isn't text",
                None,
            ),
            (
                occupied(Occupant::Entries {
                    files: 2,
                    folders: 1,
                }),
                "Not Statac's folder: `docs` already holds 2 files and 1 folder",
                Some(
                    "Statac replaces only folders it built, so build into a new folder or \
                     empty this one first.",
                ),
            ),
            (
                occupied(Occupant::File),
                "Not Statac's folder: `docs` is a file",
                Some(
                    "Statac replaces only folders it built, so build into a new folder or \
                     empty this one first.",
                ),
            ),
            (
                occupied(Occupant::Link),
                "Not Statac's folder: `docs` is a link",
                Some(
                    "Statac replaces only folders it built, so build into a new folder or \
                     empty this one first.",
                ),
            ),
            (
                occupied(Occupant::Entries {
                    files: 1,
                    folders: 0,
                }),
                "Not Statac's folder: `docs` already holds 1 file",
                Some(
                    "Statac replaces only folders it built, so build into a new folder or \
                     empty this one first.",
                ),
            ),
            (
                Kind::OutputInTheWay {
                    out: "content/built".into(),
                    folder: "content".into(),
                    whole: false,
                },
                "Output folder in the way: `content/built` is inside `content/`",
                Some("Build into a folder Statac doesn't read, such as `--out=_site`."),
            ),
            (
                Kind::OutputInTheWay {
                    out: "content".into(),
                    folder: "content".into(),
                    whole: true,
                },
                "Output folder in the way: `content` is the site's `content/` folder",
                Some("Build into a folder Statac doesn't read, such as `--out=_site`."),
            ),
            (
                Kind::CantRead {
                    path: "assets/site.css".into(),
                    reason: "permission denied".into(),
                },
                "Can't read `assets/site.css`: permission denied",
                None,
            ),
            (
                Kind::UnreadablePicture {
                    path: "content/photo-1.jpg".into(),
                    fault: PictureFault::CutShort,
                },
                "Unreadable picture: `content/photo-1.jpg` is cut short or damaged",
                Some("Copy or export the file again."),
            ),
            (
                Kind::UnreadablePicture {
                    path: "content/photo-1.jpg".into(),
                    fault: PictureFault::Unsupported("JPEG without a frame header".into()),
                },
                "Unreadable picture: `content/photo-1.jpg` is saved in a form Statac doesn't read",
                Some(
                    "To publish it as the file it is, write `optimise: false` in \
                     `content/photo-1.jpg.yaml`.",
                ),
            ),
            (
                Kind::UnreadablePicture {
                    path: "static/photo-1.avif".into(),
                    fault: PictureFault::Unsupported("AVIF without an image size".into()),
                },
                "Unreadable picture: `static/photo-1.avif` is saved in a form Statac doesn't read",
                None,
            ),
            (
                Kind::UnreadablePicture {
                    path: "content/photo-1.jpg".into(),
                    fault: PictureFault::TooLarge {
                        width: 40000,
                        height: 30000,
                    },
                },
                "Unreadable picture: `content/photo-1.jpg` is 40,000 by 30,000 pixels, too large \
                 to open",
                Some(
                    "To publish it as the file it is, write `optimise: false` in \
                     `content/photo-1.jpg.yaml`.",
                ),
            ),
            (
                Kind::UnreadablePicture {
                    path: "static/photo-1.jpg".into(),
                    fault: PictureFault::CantMake("too large for JPEG".into()),
                },
                "Unreadable picture: no copy of `static/photo-1.jpg` could be made: too large \
                 for JPEG",
                None,
            ),
            (
                Kind::ColourConverted {
                    path: "content/photo-1.jpg".into(),
                    profile: Some("Display P3".into()),
                },
                "Colour converted: the copies of `content/photo-1.jpg` are in sRGB, converted \
                 from `Display P3`",
                None,
            ),
            (
                Kind::ColourConverted {
                    path: "content/photo-1.jpg".into(),
                    profile: None,
                },
                "Colour converted: the copies of `content/photo-1.jpg` are in sRGB, converted \
                 from its colour profile",
                None,
            ),
            (
                Kind::DescribesNoPicture {
                    picture: "content/photo-1.jpg".into(),
                    nearest: Some("photo-1.jpeg".into()),
                },
                "Describes no picture: there is no `content/photo-1.jpg`",
                Some("Did you mean `photo-1.jpeg`?"),
            ),
            (
                Kind::DescribesNoPicture {
                    picture: "content/photo-1.jpg".into(),
                    nearest: None,
                },
                "Describes no picture: there is no `content/photo-1.jpg`",
                None,
            ),
            (
                Kind::CacheNotWritten {
                    path: ".statac/pictures".into(),
                    reason: "permission denied".into(),
                },
                "Cache not written: can't write `.statac/pictures`: permission denied",
                None,
            ),
            (
                Kind::UndescribedPicture {
                    picture: "photo-1.jpg".into(),
                    shown: ShownBy::Markdown {
                        described_beside: true,
                    },
                },
                "Picture with no description: `photo-1.jpg`",
                Some("Describe it in the brackets, or mark it decorative."),
            ),
            (
                Kind::UndescribedPicture {
                    picture: "https://example.org/photo-1.jpg".into(),
                    shown: ShownBy::Markdown {
                        described_beside: false,
                    },
                },
                "Picture with no description: `https://example.org/photo-1.jpg`",
                Some("Describe it in the brackets."),
            ),
            (
                Kind::UndescribedPicture {
                    picture: "photo-1.jpg".into(),
                    shown: ShownBy::Html,
                },
                "Picture with no description: `photo-1.jpg`",
                Some("Give the `<img>` an `alt`, or `alt=\"\"` where it is decoration."),
            ),
            (
                Kind::UndescribedPicture {
                    picture: "content/photo-1.jpg".into(),
                    shown: ShownBy::Template {
                        beside: Some("content/photo-1.jpg.yaml".into()),
                    },
                },
                "Picture with no description: `content/photo-1.jpg`",
                Some(
                    "Describe it with `alt:` in `content/photo-1.jpg.yaml`, or mark it \
                     decorative.",
                ),
            ),
            (
                Kind::UndescribedPicture {
                    picture: "static/photo-1.jpg".into(),
                    shown: ShownBy::Template { beside: None },
                },
                "Picture with no description: `static/photo-1.jpg`",
                Some("Describe it where it is shown, or mark it decorative."),
            ),
            (
                Kind::HeicPicture {
                    path: "content/photo-1.heic".into(),
                },
                "HEIC picture: `content/photo-1.heic` is published as it is, with no copies made",
                Some("Export it as a JPEG: most browsers can't show a HEIC picture."),
            ),
            (
                Kind::LocationPublished {
                    path: "content/photo-1.jpg".into(),
                },
                "Location published: `content/photo-1.jpg` holds where it was taken in its \
                 metadata",
                Some(
                    "Publish a copy without it, or add `location-published` to `quiet:` in \
                     `statac.yaml`.",
                ),
            ),
            (
                Kind::PicturesOutOfProportion {
                    property: "max-width".into(),
                },
                "Pictures out of proportion: `img` has a `max-width` and no `height`",
                Some("Add `height: auto` to the rule."),
            ),
            (
                Kind::PicturesOutOfProportion {
                    property: "width".into(),
                },
                "Pictures out of proportion: `img` has a `width` and no `height`",
                Some("Add `height: auto` to the rule."),
            ),
            (
                Kind::PicturesOutOfProportion {
                    property: "max-inline-size".into(),
                },
                "Pictures out of proportion: `img` has a `max-inline-size` and no `block-size`",
                Some("Add `block-size: auto` to the rule."),
            ),
            (
                Kind::PicturesOutOfProportion {
                    property: "inline-size".into(),
                },
                "Pictures out of proportion: `img` has an `inline-size` and no `block-size`",
                Some("Add `block-size: auto` to the rule."),
            ),
            (
                Kind::UnusableCardFile {
                    path: "templates/og/background.png".into(),
                    fault: CardFileFault::BackgroundSize {
                        width: 1600,
                        height: 900,
                    },
                },
                "Unusable card file: `templates/og/background.png` is 1,600 by 900 pixels, not \
                 1,200 by 630",
                Some("Save the background 1,200 pixels wide and 630 tall."),
            ),
            (
                Kind::UnusableCardFile {
                    path: "templates/og/background.gif".into(),
                    fault: CardFileFault::NotAStill,
                },
                "Unusable card file: `templates/og/background.gif` isn't a JPEG, a PNG or a \
                 still WebP",
                Some("Save the background as a PNG or a JPEG."),
            ),
            (
                Kind::UnusableCardFile {
                    path: "templates/og/text.ttf".into(),
                    fault: CardFileFault::Font,
                },
                "Unusable card file: `templates/og/text.ttf` doesn't read as a font",
                Some("Name a TrueType or OpenType font file."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::TooTall {
                        text: "title".into(),
                        size: 45,
                        smallest: true,
                    },
                    pages: 1,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: the `title` of `content/post-1.md` doesn't fit, even at 45 \
                 pixels",
                Some("Give the page fewer words there, or the middle of the card more room."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::TooTall {
                        text: "summary".into(),
                        size: 30,
                        smallest: false,
                    },
                    pages: 1234,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: the `summary` of 1,234 pages doesn't fit at 30 pixels, \
                 starting with `content/post-1.md`",
                Some("Give the page fewer words there, or the middle of the card more room."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::NoRoom,
                    pages: 1,
                    first: Named::ListPage(Word::Tag, "Rust".into()),
                },
                "Card not drawn: the top and bottom texts of the page for the tag `Rust` leave \
                 no room between them",
                Some("Make the texts at the top and bottom smaller or shorter."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::NoRoom,
                    pages: 2,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: the top and bottom texts of 2 pages leave no room between \
                 them, starting with `content/post-1.md`",
                Some("Make the texts at the top and bottom smaller or shorter."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::NoFont {
                        characters: vec!['→'],
                        more: 0,
                        font: None,
                    },
                    pages: 1,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: `content/post-1.md` needs `→`, which no font has",
                Some("Add a font that has it to the card's `font`."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::NoFont {
                        characters: vec!['😀'],
                        more: 0,
                        font: Some(Box::new(FontWithThem::Brought("NotoEmoji.ttf".into()))),
                    },
                    pages: 1,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: `content/post-1.md` needs `😀`, which no font has",
                Some(
                    "Run `statac new --update --missing` to write \
                     `templates/og/NotoEmoji.ttf`, which has it, then add `NotoEmoji.ttf` to \
                     the card's `font`.",
                ),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::NoFont {
                        characters: vec!['😀'],
                        more: 0,
                        font: Some(Box::new(FontWithThem::Here("NotoEmoji.ttf".into()))),
                    },
                    pages: 1,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: `content/post-1.md` needs `😀`, which no font has",
                Some("Add `NotoEmoji.ttf`, which has it, to the card's `font`."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::NoFont {
                        characters: vec!['→', '✓'],
                        more: 0,
                        font: None,
                    },
                    pages: 3,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: 3 pages need `→` and `✓`, which no font has, starting with \
                 `content/post-1.md`",
                Some("Add a font that has them to the card's `font`."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::NoFont {
                        characters: vec!['α', 'β', 'γ'],
                        more: 21,
                        font: None,
                    },
                    pages: 443,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: 443 pages need `α`, `β`, `γ` and 21 others, which no font \
                 has, starting with `content/post-1.md`",
                Some("Add a font that has them to the card's `font`."),
            ),
            (
                Kind::UnusedCard { name: None },
                "Unused card: no layout reads `page.card`",
                Some(
                    "Write the card's `<meta>` lines in a layout, or remove the card's settings \
                     and `templates/og/`.",
                ),
            ),
            (
                Kind::UnusedCard {
                    name: Some("wide".into()),
                },
                "Unused card: no page picks `wide`",
                Some("Write `card: wide` on the pages it is for, or remove the card's settings."),
            ),
            (
                Kind::NoSiteAddress {
                    waiting: Waiting::Cards,
                },
                "No site address: sharing cards are left out until the site has a `url`",
                Some("Add the site's address to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "content/cover.webp".into(),
                    holds: Some(crate::pictures::Kind::WebP),
                    pages: 1,
                },
                "Sharing image may not show: `content/cover.webp` is a WebP",
                Some("For a picture more link previews show, save it as a PNG."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "static/shared/cover.svg".into(),
                    holds: Some(crate::pictures::Kind::Svg),
                    pages: 3,
                },
                "Sharing image may not show: the `static/shared/cover.svg` of 3 pages is an SVG",
                Some("For a picture more link previews show, save it as a PNG."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "content/cover.gif".into(),
                    holds: Some(crate::pictures::Kind::Gif),
                    pages: 1,
                },
                "Sharing image may not show: `content/cover.gif` is a GIF",
                Some("For a picture more link previews show, save it as a PNG."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "content/cover.bmp".into(),
                    holds: None,
                    pages: 1,
                },
                "Sharing image may not show: `content/cover.bmp` is a kind of picture some link \
                 previews don't show",
                Some("For a picture more link previews show, save it as a PNG."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "content/cover.bmp".into(),
                    holds: None,
                    pages: 1200,
                },
                "Sharing image may not show: the `content/cover.bmp` of 1,200 pages is a kind of \
                 picture some link previews don't show",
                Some("For a picture more link previews show, save it as a PNG."),
            ),
            (
                Kind::CantRemove {
                    path: "._site.statac-old".into(),
                    reason: "permission denied".into(),
                },
                "Can't remove `._site.statac-old`: permission denied",
                None,
            ),
            (
                Kind::CantListen {
                    port: 3000,
                    last: 3000,
                    why: Unheard::InUse,
                },
                "Can't listen on port 3000: another program is using it",
                Some("Choose another port with `--port=<number>`."),
            ),
            (
                Kind::CantListen {
                    port: 3000,
                    last: 3010,
                    why: Unheard::InUse,
                },
                "Can't listen on ports 3000 to 3010: other programs are using them",
                Some("Choose another port with `--port=<number>`."),
            ),
            (
                Kind::CantListen {
                    port: 80,
                    last: 80,
                    why: Unheard::System("permission denied".into()),
                },
                "Can't listen on port 80: permission denied",
                Some("Choose another port with `--port=<number>`."),
            ),
            (
                Kind::OutsideSite {
                    reach: Reach::Load("../../outside".into()),
                },
                "Outside the site: `../../outside` isn't in `assets/`",
                Some("Copy the file into `assets/` and load it from there."),
            ),
            (
                Kind::OutsideSite {
                    reach: Reach::Link {
                        path: "static/key.txt".into(),
                        within: Some("static/".into()),
                    },
                },
                "Outside the site: the link `static/key.txt` leads out of `static/`",
                Some("Put the file itself in `static/` instead of a link to it."),
            ),
        ]
    }

    #[test]
    fn file_problems_read_as_written() {
        for (kind, line, suggested) in files() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    fn others() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let call = |found| Kind::MalformedCall { found };
        let maths = |found| Kind::MalformedMaths { found };
        vec![
            (
                call(markdown::Malformed::Unclosed("@image(".into())),
                "Malformed call: `@image(` is never closed",
                Some("Close it with `)`."),
            ),
            (
                call(markdown::Malformed::UnclosedQuote('\'')),
                "Malformed call: `'` is never closed",
                Some("End the value with another `'` on the same line."),
            ),
            (
                call(markdown::Malformed::TextAfterQuote),
                "Malformed call: text after the closing quote",
                Some("Put the whole value inside the quotes."),
            ),
            (
                call(markdown::Malformed::Name("café".into())),
                "Malformed call: a snippet can't be called `café`",
                Some(
                    r"Name the snippet with the letters a to z, digits and `_`, or write `\@` to show this as text.",
                ),
            ),
            (
                call(markdown::Malformed::NoEnd("note".into())),
                "Malformed call: `@note` has no `@end`",
                Some("Put `@end` on a line of its own where the writing it wraps ends."),
            ),
            (
                call(markdown::Malformed::StrayEnd),
                "Malformed call: `@end` closes nothing",
                Some(
                    "Take the line out: only a call to a snippet that shows `body` wraps the writing up to an `@end`.",
                ),
            ),
            (
                call(markdown::Malformed::WrapsNothing("note".into())),
                "Malformed call: `@note` isn't on a line of its own",
                Some(
                    "Write the call on a line of its own, the writing it wraps below it, and `@end` on a line after that.",
                ),
            ),
            (
                call(markdown::Malformed::NotOneAddress),
                "Malformed call: `@url` takes one address from the top of the site",
                Some("Write it as `@url(/about/)`."),
            ),
            (
                maths(MathsFault::UnknownCommand(r"\fracc".into())),
                r"Malformed maths: Statac doesn't know `\fracc`",
                None,
            ),
            (
                maths(MathsFault::UnknownEnvironment("matrx".into())),
                "Malformed maths: Statac doesn't know the environment `matrx`",
                None,
            ),
            (
                maths(MathsFault::UnknownColour("redd".into())),
                "Malformed maths: Statac doesn't know the colour `redd`",
                None,
            ),
            (
                maths(MathsFault::Unclosed {
                    opening: Some("{".into()),
                    closing: "}".into(),
                }),
                "Malformed maths: `{` is never closed",
                Some("Close it with `}`."),
            ),
            (
                maths(MathsFault::Unclosed {
                    opening: Some(r"\begin{cases}".into()),
                    closing: r"\end{cases}".into(),
                }),
                r"Malformed maths: `\begin{cases}` is never closed",
                Some(r"Close it with `\end{cases}`."),
            ),
            (
                maths(MathsFault::Unclosed {
                    opening: None,
                    closing: "]".into(),
                }),
                "Malformed maths: `]` is missing",
                None,
            ),
            (
                maths(MathsFault::ClosesNothing("}".into())),
                "Malformed maths: `}` closes nothing",
                Some("Take it out, or open it with `{` where it starts."),
            ),
            (
                maths(MathsFault::ClosesNothing(r"\end{cases}".into())),
                r"Malformed maths: `\end{cases}` closes nothing",
                Some(r"Take it out, or open it with `\begin{cases}` where it starts."),
            ),
            (
                maths(MathsFault::ClosesNothing(r"\right)".into())),
                r"Malformed maths: `\right)` closes nothing",
                Some(r"Take it out, or open it with `\left` where it starts."),
            ),
            (
                maths(MathsFault::Mismatched {
                    written: r"\end{bmatrix}".into(),
                    expected: r"\end{pmatrix}".into(),
                }),
                r"Malformed maths: `\end{bmatrix}` closes `\begin{pmatrix}`",
                Some(r"Did you mean `\end{pmatrix}`?"),
            ),
            (
                maths(MathsFault::MissingBefore("$".into())),
                "Malformed maths: something is missing before `$`",
                Some(r"Give the command before it everything it takes, as in `\frac{1}{2}`."),
            ),
            (
                maths(MathsFault::Twice('^')),
                "Malformed maths: two `^` on one thing",
                Some("Put braces round what goes together, as in `x^{a^b}`."),
            ),
            (
                maths(MathsFault::Twice('_')),
                "Malformed maths: two `_` on one thing",
                Some("Put braces round what goes together, as in `x_{i_j}`."),
            ),
            (
                maths(MathsFault::NoSuchLabel("eq:1".into())),
                "Malformed maths: no equation is labelled `eq:1`",
                Some(r"Write `\label{eq:1}` in the equation it points to."),
            ),
            (
                maths(MathsFault::Unreadable(r"\hspace{1qq}".into())),
                r"Malformed maths: Statac can't read `\hspace{1qq}`",
                None,
            ),
            (
                Kind::ColonInCall {
                    name: "caption".into(),
                },
                "Colon in call: `caption:` was read as `caption=`",
                Some("Write `caption=` instead."),
            ),
            (
                Kind::PublishedAsText {
                    written: "{{< figure src=\"photo-1.jpg\" >}}".into(),
                    why: PublishedAsText::Call,
                },
                "Published as text: `{{< figure src=\"photo-1.jpg\" >}}` is another tool's call",
                Some("Call a snippet in its place, written `@name(...)`, or take it out."),
            ),
            (
                Kind::PublishedAsText {
                    written: "{: .box}".into(),
                    why: PublishedAsText::Mark,
                },
                "Published as text: `{: .box}` is another tool's mark",
                Some("Take it out, or write the HTML it stood for."),
            ),
            (
                Kind::PublishedAsText {
                    written: "title: Post 1".into(),
                    why: PublishedAsText::FrontMatter,
                },
                "Published as text: `title: Post 1` isn't between `---` lines",
                Some("Put a `---` line above the first field and another below the last."),
            ),
            (
                Kind::ControlCharacter { character: '\u{b}' },
                "Control character: U+000B is left out of the page",
                Some("Delete it, or type the character you meant."),
            ),
            (
                Kind::ControlCharacter {
                    character: '\u{92}',
                },
                "Control character: U+0092 is left out of the page",
                Some(
                    "This is usually a character such as \u{2019} saved in an old encoding: type it again.",
                ),
            ),
            (
                Kind::UnknownProblemName {
                    written: "brokn-link".into(),
                    suggestion: Some("broken-link".into()),
                },
                "Unknown problem name: `brokn-link`",
                Some("Did you mean `broken-link`?"),
            ),
            (
                Kind::CantQuieten {
                    name: "duplicate-key".into(),
                    because: Unquietenable::Error,
                },
                "Can't quieten: `duplicate-key` stops the build",
                Some("Take it out of `quiet:`."),
            ),
            (
                Kind::CantQuieten {
                    name: "duplicate-key".into(),
                    because: Unquietenable::LeavesAPageOut,
                },
                "Can't quieten: `duplicate-key` leaves a page out",
                Some("Take it out of `quiet:`."),
            ),
            (
                Kind::UnknownCommand {
                    written: "biuld".into(),
                    suggestion: Some("build".into()),
                },
                "Unknown command: `biuld`",
                Some("Did you mean `statac build`?"),
            ),
            (
                Kind::UnknownCommand {
                    written: "post-1".into(),
                    suggestion: None,
                },
                "Unknown command: `post-1`",
                Some("Run `statac --help` to see the commands."),
            ),
            (
                Kind::MissingCommand,
                "Missing command: no `build`, `check`, `serve`, `stamp`, `refresh`, `refuse`, `new`, `import` or `explain`",
                Some("Run `statac --help` to see the commands."),
            ),
            (
                Kind::UnknownOption {
                    written: "--sight=docs".into(),
                    hint: OptionHint::Similar("--site=docs".into()),
                },
                "Unknown option: `--sight=docs`",
                Some("Did you mean `--site=docs`?"),
            ),
            (
                Kind::UnknownOption {
                    written: "--out=public".into(),
                    hint: OptionHint::ForOther {
                        command: "check".into(),
                        option: "--out".into(),
                        takers: Box::new(["build"]),
                    },
                },
                "Unknown option: `statac check` has no `--out`",
                Some("`--out` is for `statac build`."),
            ),
            (
                Kind::UnknownOption {
                    written: "--site=docs".into(),
                    hint: OptionHint::ForOther {
                        command: "explain".into(),
                        option: "--site".into(),
                        takers: Box::new(["build", "check"]),
                    },
                },
                "Unknown option: `statac explain` has no `--site`",
                Some("`--site` is for `statac build` and `statac check`."),
            ),
            (
                Kind::UnknownOption {
                    written: "--drafts".into(),
                    hint: OptionHint::Takes {
                        command: "build".into(),
                        options: Box::new(["--site", "--out"]),
                    },
                },
                "Unknown option: `--drafts`",
                Some("`statac build` takes `--site` and `--out`."),
            ),
            (
                Kind::UnknownOption {
                    written: "--drafts".into(),
                    hint: OptionHint::Takes {
                        command: "explain".into(),
                        options: Box::new([]),
                    },
                },
                "Unknown option: `--drafts`",
                Some("`statac explain` takes no options."),
            ),
            (
                Kind::UnknownOption {
                    written: "--missing".into(),
                    hint: OptionHint::OnlyWithUpdate,
                },
                "Unknown option: `statac new` takes `--missing` only with `--update`",
                Some("Add `--update`, or leave `--missing` out."),
            ),
            (
                Kind::UnknownOption {
                    written: "--yes".into(),
                    hint: OptionHint::NotWithUpdate,
                },
                "Unknown option: `statac new --update` has no `--yes`",
                Some("`--update` asks nothing, so leave `--yes` out."),
            ),
            (
                Kind::UnknownOption {
                    written: "--cards".into(),
                    hint: OptionHint::CardsWithUpdate,
                },
                "Unknown option: `statac new --update` has no `--cards`",
                Some("Run `statac new --cards` by itself."),
            ),
            (
                Kind::UnknownOption {
                    written: "--drafts".into(),
                    hint: OptionHint::ShownAlready,
                },
                "Unknown option: `statac serve` has no `--drafts`",
                Some(
                    "`statac serve` shows drafts and posts waiting for their day already. \
                     `--no-drafts` leaves them out.",
                ),
            ),
            (
                Kind::UnknownOption {
                    written: "--drafts".into(),
                    hint: OptionHint::None,
                },
                "Unknown option: `--drafts`",
                Some("Run `statac --help` to see the options."),
            ),
            (
                Kind::MissingValue {
                    written: "--site".into(),
                    option: "--site".into(),
                    value: Some("docs".into()),
                },
                "Missing value: `--site` needs a folder",
                Some("Write it as `--site=docs`."),
            ),
            (
                Kind::MissingValue {
                    written: "--out=".into(),
                    option: "--out".into(),
                    value: None,
                },
                "Missing value: `--out=` needs a folder",
                Some("Write it as `--out=<folder>`."),
            ),
            (
                Kind::MissingValue {
                    written: "--url".into(),
                    option: "--url".into(),
                    value: None,
                },
                "Missing value: `--url` needs an address",
                Some("Write it as `--url=<address>`."),
            ),
            (
                Kind::OptionGivenTwice {
                    option: "--site".into(),
                },
                "Option given twice: `--site`",
                Some("Keep one of them."),
            ),
            (
                Kind::UnexpectedArgument {
                    written: "docs".into(),
                    suggestion: Some("--site=docs".into()),
                },
                "Unexpected argument: `docs`",
                Some("Did you mean `--site=docs`?"),
            ),
            (
                Kind::MissingName,
                "Missing name: `statac explain` needs the name of a problem",
                Some("The name is the last part of a problem's `More:` link."),
            ),
            (
                Kind::MissingAddress,
                "Missing address: `statac refuse` needs the address of a webmention or a sender",
                Some(
                    "Give the address after the command, as `statac refuse https://a.example/post-1/`.",
                ),
            ),
            (
                Kind::NotAWebAddress {
                    written: "a.example".into(),
                    respelt: Some("https://a.example".into()),
                },
                "Not a web address: `statac refuse` takes an address starting `https://` or \
                 `http://`, not `a.example`",
                Some("Did you mean `https://a.example`?"),
            ),
            (
                Kind::NotAWebAddress {
                    written: "ftp://a.example/post-1".into(),
                    respelt: None,
                },
                "Not a web address: `statac refuse` takes an address starting `https://` or \
                 `http://`, not `ftp://a.example/post-1`",
                Some("Give the whole address, as the browser shows it."),
            ),
            (
                Kind::UnreadableName {
                    written: "site-\u{FFFD}".into(),
                },
                "Unreadable name: `site-\u{FFFD}` holds bytes that aren't UTF-8",
                None,
            ),
            (
                Kind::UnusableName {
                    written: "site-1\u{1b}".into(),
                },
                "Unusable name: `site-1\u{1b}` holds a character that can't be shown",
                Some("Name the folder without it."),
            ),
            (
                Kind::FolderNotEmpty {
                    by: Maker::New,
                    folder: "site-1".into(),
                    names: vec!["a.txt".into()],
                    more: 0,
                },
                "Folder not empty: `site-1` holds `a.txt`",
                Some("Give `statac new` an empty folder, or the name of a new one."),
            ),
            (
                Kind::FolderNotEmpty {
                    by: Maker::New,
                    folder: "site-1".into(),
                    names: vec![
                        "a.txt".into(),
                        "b.txt".into(),
                        "c.txt".into(),
                        "d.txt".into(),
                    ],
                    more: 3,
                },
                "Folder not empty: `site-1` holds `a.txt`, `b.txt`, `c.txt`, `d.txt` and 3 more",
                Some("Give `statac new` an empty folder, or the name of a new one."),
            ),
            (
                Kind::AlreadyASite {
                    by: Maker::New,
                    folder: "site-1".into(),
                    has: SiteMark::Settings,
                },
                "Already a site: `site-1` has a `statac.yaml`",
                Some("To bring its starter files up to date, run `statac new --update`."),
            ),
            (
                Kind::AlreadyASite {
                    by: Maker::New,
                    folder: "site-1".into(),
                    has: SiteMark::Content,
                },
                "Already a site: `site-1` has a `content/` folder",
                None,
            ),
            (
                Kind::FolderNotEmpty {
                    by: Maker::Import,
                    folder: "site-1".into(),
                    names: vec!["a.txt".into()],
                    more: 0,
                },
                "Folder not empty: `site-1` holds `a.txt`",
                Some("Give `statac import` an empty folder, or the name of a new one."),
            ),
            (
                Kind::AlreadyASite {
                    by: Maker::Import,
                    folder: "site-1".into(),
                    has: SiteMark::Settings,
                },
                "Already a site: `site-1` has a `statac.yaml`",
                None,
            ),
            (
                Kind::NoSiteHere { folder: ".".into() },
                "No site here: `.` has no `content/` folder and no `statac.yaml`",
                Some("Run it in a site's folder, or make one with `statac new`."),
            ),
            (
                Kind::NoSiteHere {
                    folder: "notes".into(),
                },
                "No site here: `notes` has no `content/` folder and no `statac.yaml`",
                Some("Run it in a site's folder, or make one with `statac new`."),
            ),
            (
                Kind::NewerStarter { files: 1 },
                "Newer starter files: 1 of the files `statac new` wrote has a newer version",
                Some(
                    "Run `statac new --update`: it replaces those you never changed and shows \
                     what is new in the rest.",
                ),
            ),
            (
                Kind::NewerStarter { files: 3 },
                "Newer starter files: 3 of the files `statac new` wrote have a newer version",
                Some(
                    "Run `statac new --update`: it replaces those you never changed and shows \
                     what is new in the rest.",
                ),
            ),
            (
                Kind::CantTellWhatMade {
                    folder: "old".into(),
                    several: false,
                    tools: vec!["jekyll".into()],
                },
                "Can't tell what made the site: `old` has no settings file Statac knows",
                Some("Say what made it with `--from=jekyll`."),
            ),
            (
                Kind::CantTellWhatMade {
                    folder: "old".into(),
                    several: true,
                    tools: vec!["jekyll".into(), "tool-2".into()],
                },
                "Can't tell what made the site: `old` has the settings files of more than one tool",
                Some("Say what made it with `--from=jekyll` or `--from=tool-2`."),
            ),
            (
                Kind::MissingFolder,
                "Missing folder: `statac import` needs the folder of the site to import",
                Some("Name it after the command: `statac import <folder>`."),
            ),
        ]
    }

    fn content() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let mut sources = crate::source::Sources::default();
        let first = crate::source::Place {
            source: sources.add("content/post-1.md", "book:\n  author: Author 1\n".into()),
            span: crate::source::Span::new(0, 4),
        };
        let wrong = |field: &str, expected, found, respelt: Option<&str>| Kind::WrongKind {
            field: field.into(),
            expected,
            found,
            respelt: respelt.map(Into::into),
        };
        let unusable = |fault| Kind::UnusableAddress { fault };
        vec![
            (
                Kind::OtherSpelling {
                    word: Word::File,
                    written: "statac.yml".into(),
                    ours: "statac.yaml".into(),
                },
                "Unknown file: `statac.yml`",
                Some("Statac calls this `statac.yaml`."),
            ),
            (
                Kind::OtherSpelling {
                    word: Word::FieldKind,
                    written: "yes/no".into(),
                    ours: "true/false".into(),
                },
                "Unknown kind: `yes/no`",
                Some("Statac calls this `true/false`."),
            ),
            (
                Kind::OtherSpelling {
                    word: Word::AddressPiece,
                    written: ":year".into(),
                    ours: "{year}".into(),
                },
                "Unknown address piece: `:year`",
                Some("Statac calls this `{year}`."),
            ),
            (
                Kind::CantRead {
                    path: "content/post-1.md".into(),
                    reason: "it isn't UTF-8 text".into(),
                },
                "Can't read `content/post-1.md`: it isn't UTF-8 text",
                None,
            ),
            (
                Kind::UnknownWord {
                    word: Word::Setting,
                    written: "tags.permalnk".into(),
                    nearest: Some("tags.permalink".into()),
                },
                "Unknown setting: `tags.permalnk`",
                Some("Did you mean `tags.permalink`?"),
            ),
            (
                Kind::UnknownWord {
                    word: Word::FieldKind,
                    written: "colour".into(),
                    nearest: None,
                },
                "Unknown kind: `colour`",
                None,
            ),
            (
                Kind::ReadAsYourOwn {
                    own: Own::Field("titel".into()),
                    written: "titel".into(),
                    nearest: "title".into(),
                },
                "Read as your own field: `titel`",
                Some(
                    "Did you mean `title`? To keep `titel`, declare it in `statac.yaml` as \
                     `fields.titel: text`.",
                ),
            ),
            (
                Kind::ReadAsYourOwn {
                    own: Own::Field("titel".into()),
                    written: "pages.titel".into(),
                    nearest: "pages.title".into(),
                },
                "Read as your own field: `pages.titel`",
                Some(
                    "Did you mean `pages.title`? To keep `pages.titel`, declare it in \
                     `statac.yaml` as `fields.titel: text`.",
                ),
            ),
            (
                Kind::ReadAsYourOwn {
                    own: Own::Setting,
                    written: "posts_per_pge".into(),
                    nearest: "posts_per_page".into(),
                },
                "Read as your own setting: `posts_per_pge`",
                Some("Did you mean `posts_per_page`?"),
            ),
            (
                Kind::ReadAsYourOwn {
                    own: Own::Type,
                    written: "repyl".into(),
                    nearest: "reply".into(),
                },
                "Read as your own type: `repyl`",
                Some("Did you mean `reply`?"),
            ),
            (
                Kind::NestedSetting {
                    key: "layout".into(),
                    under: "pages".into(),
                    flat: "pages.layout: post".into(),
                },
                "Nested setting: `layout` under `pages`",
                Some("Write it flat: `pages.layout: post`."),
            ),
            (
                Kind::FieldWrittenTwoWays {
                    on: Root::Page,
                    flat: "book.author".into(),
                    nested_under: "book".into(),
                    nested: first,
                },
                "Field written two ways: `book.author` is also nested under `book`",
                Some("Nest it under `book`, so templates read it as `page.book.author`."),
            ),
            (
                Kind::ReservedName {
                    name: "site.posts".into(),
                },
                "Reserved name: `site.posts` is Statac's own",
                Some("Give it a name of your own, such as `my_posts`."),
            ),
            (
                Kind::SetAside {
                    field: "source".into(),
                    why: SetAside::StatacFills(Root::Page),
                },
                "Field set aside: Statac fills `page.source` itself",
                Some("Give it a name of your own, such as `my_source`."),
            ),
            (
                Kind::SetAside {
                    field: "slug".into(),
                    why: SetAside::StatacFills(Root::Page),
                },
                "Field set aside: Statac fills `page.slug` itself",
                Some("Write `permalink:` to choose the page's address."),
            ),
            (
                Kind::SetAside {
                    field: "title".into(),
                    why: SetAside::NothingWritten(Word::Field),
                },
                "Field set aside: `title` has nothing after its colon",
                Some("Give it a value, or take the line out."),
            ),
            (
                Kind::SetAside {
                    field: "url".into(),
                    why: SetAside::NothingWritten(Word::Setting),
                },
                "Setting set aside: `url` has nothing after its colon",
                Some("Give it a value, or take the line out."),
            ),
            (
                Kind::SetAside {
                    field: "permalink".into(),
                    why: SetAside::FolderPermalink,
                },
                "Field set aside: a folder's own page stays at the folder's address",
                Some("Rename the folder to move its page."),
            ),
            (
                wrong(
                    "draft",
                    Expected::TrueFalse,
                    Found::Written("perhaps".into()),
                    None,
                ),
                "Wrong kind of value: `draft` is `true` or `false`, not `perhaps`",
                None,
            ),
            (
                wrong(
                    "draft",
                    Expected::TrueFalse,
                    Found::Written("yes".into()),
                    Some("true"),
                ),
                "Wrong kind of value: `draft` is `true` or `false`, not `yes`",
                Some("Write it as `true`."),
            ),
            (
                wrong("title", Expected::Text, Found::List, None),
                "Wrong kind of value: `title` is text, not a list",
                None,
            ),
            (
                wrong(
                    "date",
                    Expected::Date,
                    Found::Written("13/04/2026".into()),
                    Some("2026-04-13"),
                ),
                "Wrong kind of value: `date` is a date, not `13/04/2026`",
                Some("Write it as `2026-04-13`."),
            ),
            (
                wrong(
                    "posts_per_page",
                    Expected::WholeNumber { from: 0 },
                    Found::Written("-1".into()),
                    None,
                ),
                "Wrong kind of value: `posts_per_page` is a whole number, not `-1`",
                None,
            ),
            (
                wrong(
                    "max_width",
                    Expected::WholeNumber { from: 1 },
                    Found::Written("0".into()),
                    None,
                ),
                "Wrong kind of value: `max_width` is a whole number from 1, not `0`",
                None,
            ),
            (
                wrong(
                    "neighbours",
                    Expected::OneOf(vec![
                        "site".into(),
                        "folder".into(),
                        "series".into(),
                        "false".into(),
                    ]),
                    Found::Written("no".into()),
                    Some("false"),
                ),
                "Wrong kind of value: `neighbours` is `site`, `folder`, `series` or `false`, not `no`",
                Some("Write it as `false`."),
            ),
            (
                wrong(FRONT_MATTER, Expected::Fields, Found::List, None),
                "Wrong kind of value: front matter is `name: value` lines, not a list",
                None,
            ),
            (
                wrong("webmention_token", Expected::Text, Found::Hidden, None),
                "Wrong kind of value: `webmention_token` is text, not what YAML reads there",
                None,
            ),
            (
                Kind::NeedsQuotes {
                    written: "3.10".into(),
                    reading: "3.1".into(),
                },
                "Needs quotes: YAML reads `3.10` as 3.1",
                Some("Write it as `\"3.10\"` to keep it as written."),
            ),
            (
                Kind::OneTagNotTwo {
                    key: "tags".into(),
                    line: "tags: hills, water".into(),
                    tag: "hills, water".into(),
                    parts: ["hills".into(), "water".into()].into(),
                },
                "One tag, not two: `tags: hills, water` makes a single tag called `hills, water`",
                Some("For two tags, write `tags: [hills, water]`."),
            ),
            (
                Kind::OneTagNotTwo {
                    key: "pages.tags".into(),
                    line: "pages.tags: a, b, c".into(),
                    tag: "a, b, c".into(),
                    parts: ["a".into(), "b".into(), "c".into()].into(),
                },
                "One tag, not three: `pages.tags: a, b, c` makes a single tag called `a, b, c`",
                Some("For three tags, write `pages.tags: [a, b, c]`."),
            ),
            (
                Kind::UnclearDate {
                    written: "3/4/2026".into(),
                    readings: Box::new([
                        ("2026-04-03".into(), "3 April".into()),
                        ("2026-03-04".into(), "4 March".into()),
                    ]),
                },
                "Unclear date: `3/4/2026` could be 3 April or 4 March",
                Some("Write it as `2026-04-03` or `2026-03-04`."),
            ),
            (
                Kind::NoSuchDate {
                    written: "2026-02-30".into(),
                    missing: Missing::Day {
                        day: 30,
                        month: 2,
                        year: 2026,
                    },
                },
                "No such date: there is no 30 February in 2026",
                None,
            ),
            (
                Kind::NoSuchDate {
                    written: "2026-13-01".into(),
                    missing: Missing::Month(13),
                },
                "No such date: there is no month 13",
                None,
            ),
            (
                Kind::DatesDisagree {
                    named: "2026-10-04".into(),
                    written: "2026-10-05".into(),
                },
                "Dates disagree: `date` is `2026-10-05`, the file name `2026-10-04`",
                Some("Change one so they match; Statac uses the `date` line."),
            ),
            (
                Kind::NotStamped {
                    field: "date".into(),
                    written: "=monday 09:00".into(),
                },
                "Not stamped: `date` is still `=monday 09:00`",
                Some(
                    "Run `statac stamp` to write the date in its place, or `statac serve`, \
                     which writes it on every save.",
                ),
            ),
            (
                Kind::OtherSpelling {
                    word: Word::Stamp,
                    written: "=Mon 9am".into(),
                    ours: "=monday 09:00".into(),
                },
                "Unknown stamp: `=Mon 9am`",
                Some("Statac calls this `=monday 09:00`."),
            ),
            (
                Kind::UnknownWord {
                    word: Word::Stamp,
                    written: "=tomorow".into(),
                    nearest: Some("=tomorrow".into()),
                },
                "Unknown stamp: `=tomorow`",
                Some("Did you mean `=tomorrow`?"),
            ),
            (
                Kind::NoSuchDate {
                    written: "=today 25:00".into(),
                    missing: Missing::Time("25:00".into()),
                },
                "No such date: there is no time `25:00`",
                None,
            ),
            (
                Kind::MissingField {
                    field: "isbn".into(),
                    required_by: Requirer::Type("book".into()),
                },
                "Missing field: `isbn`",
                Some("Add `isbn:`, which `statac.yaml` requires on pages of type `book`."),
            ),
            (
                Kind::WrongPlace {
                    key: "pages.title".into(),
                    why: WrongPlace::EachPagesOwn,
                },
                "Wrong place: `pages.title` can't be handed down",
                Some("Write `title:` on each page instead."),
            ),
            (
                Kind::WrongPlace {
                    key: "tags.permalink".into(),
                    why: WrongPlace::SiteSetting,
                },
                "Wrong place: `tags.permalink` is a site setting",
                Some("Write it in `statac.yaml` instead."),
            ),
            (
                Kind::WrongPlace {
                    key: "pages.layout".into(),
                    why: WrongPlace::TopFolder,
                },
                "Wrong place: `pages.layout` at the top of `content/` reaches every page",
                Some("Write it in `statac.yaml` instead."),
            ),
            (
                Kind::WrongPlace {
                    key: "pages.layout".into(),
                    why: WrongPlace::NothingInside,
                },
                "Wrong place: `pages.layout` hands down only from a folder's `index.md`",
                Some("Move it to the folder's `index.md`."),
            ),
            (
                Kind::WrongPlace {
                    key: "pages.index_page".into(),
                    why: WrongPlace::WholeSite,
                },
                "Wrong place: `pages.index_page` can't be handed down from `statac.yaml`",
                Some("Write `index_page:` in a folder's `index.md` instead."),
            ),
            (
                Kind::WrongPlace {
                    key: "site.author".into(),
                    why: WrongPlace::SiteValue,
                },
                "Wrong place: a page can't set `site.author`",
                Some("Write `author:` in `statac.yaml` instead."),
            ),
            (
                unusable(AddressFault::UnknownPiece {
                    written: "{yaer}".into(),
                    nearest: Some("{year}".into()),
                }),
                "Unusable address: there is no piece `{yaer}`",
                Some("Did you mean `{year}`?"),
            ),
            (
                unusable(AddressFault::Unclosed),
                "Unusable address: `{` is never closed",
                Some("Close it with `}`."),
            ),
            (
                unusable(AddressFault::Stray),
                "Unusable address: `}` closes nothing",
                Some("Leave it out of the address."),
            ),
            (
                unusable(AddressFault::Character(':')),
                "Unusable address: `:` can't be in a name",
                Some("Leave it out of the address."),
            ),
            (
                unusable(AddressFault::Character('\\')),
                "Unusable address: `\\` can't be in a name",
                Some("Separate folders with `/`."),
            ),
            (
                unusable(AddressFault::Dots("..".into())),
                "Unusable address: `..` can't be a name",
                None,
            ),
            (
                unusable(AddressFault::TrailingSpaceOrDot('.')),
                "Unusable address: a name can't end with `.`",
                Some("Take it off: Windows drops it, and links would miss the page."),
            ),
            (
                unusable(AddressFault::TrailingSpaceOrDot(' ')),
                "Unusable address: a name can't end with a space",
                Some("Take it off: Windows drops it, and links would miss the page."),
            ),
            (
                unusable(AddressFault::Device("con".into())),
                "Unusable address: `con` is kept for a device on Windows",
                None,
            ),
            (
                unusable(AddressFault::Percent),
                "Unusable address: `%` can't be in a name",
                Some("Write the character it stands for instead."),
            ),
            (
                unusable(AddressFault::Hash),
                "Unusable address: `#` can't be in a name",
                Some("Leave out the `#` and what follows it."),
            ),
            (
                unusable(AddressFault::Control),
                "Unusable address: a control character can't be in a name",
                Some("Leave it out of the address."),
            ),
            (
                unusable(AddressFault::NoDate {
                    piece: "{year}".into(),
                }),
                "Unusable address: this page has no date for `{year}`",
                Some(
                    "Add a `date:` line, or start the file name with a date such as `2026-10-04-`.",
                ),
            ),
            (
                unusable(AddressFault::FillsNothing {
                    pattern: "/blog/".into(),
                }),
                "Unusable address: `/blog/` puts every page at one address",
                Some("Add a piece such as `{slug}` so each page has its own."),
            ),
        ]
    }

    fn dates() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let unusable = |fault| Kind::UnusableDateFormat {
            fault: Box::new(fault),
        };
        let reference = || "Wednesday 31 December 2025, 3.04pm".to_owned();
        let examples = || vec!["31/12/2025".to_owned(), "31 December 2025".to_owned()];
        let months = || vec!["December".to_owned(), "Dec".to_owned(), "12".to_owned()];
        let wrong = |field: &str, expected, found| Kind::WrongKind {
            field: field.into(),
            expected,
            found,
            respelt: None,
        };
        vec![
            (
                unusable(FormatFault::Missing {
                    example: "31 December".into(),
                    pieces: vec![Piece::Year],
                    months: months(),
                }),
                "Unusable date format: `31 December` has no year",
                Some("Add the year, which every date on the site is written with."),
            ),
            (
                unusable(FormatFault::Empty {
                    reference: reference(),
                    examples: examples(),
                }),
                "Unusable date format: it is empty",
                Some(
                    "Write `Wednesday 31 December 2025, 3.04pm` the way dates should look, such as \
                     `31/12/2025` or `31 December 2025`.",
                ),
            ),
            (
                unusable(FormatFault::NoPattern {
                    example: "dd/mm/yyyy".into(),
                    reference: "2025-12-31 15:04".into(),
                    examples: vec!["2025-12-31".into()],
                }),
                "Unusable date format: nothing in `dd/mm/yyyy` is in `2025-12-31 15:04`",
                Some("Write `2025-12-31 15:04` the way dates should look, such as `2025-12-31`."),
            ),
            (
                unusable(FormatFault::Twice {
                    first: "2025".into(),
                    second: "2025".into(),
                    piece: Piece::Year,
                }),
                "Unusable date format: `2025` is a second year",
                Some("`2025` is already the year, so keep one of them."),
            ),
            (
                unusable(FormatFault::NotInReference {
                    written: "30".into(),
                    reference: reference(),
                    corrected: Some("31 December 2025".into()),
                }),
                "Unusable date format: `30` isn't in `Wednesday 31 December 2025, 3.04pm`",
                Some("Did you mean `31 December 2025`?"),
            ),
            (
                unusable(FormatFault::NotInReference {
                    written: "1".into(),
                    reference: reference(),
                    corrected: None,
                }),
                "Unusable date format: `1` isn't in `Wednesday 31 December 2025, 3.04pm`",
                None,
            ),
            (
                unusable(FormatFault::LoneHour {
                    written: "3".into(),
                    am: "am".into(),
                    pm: "pm".into(),
                }),
                "Unusable date format: `3` is an hour with no am or pm",
                Some("Write the time as `15:04`, or add `pm`, such as `3.04pm`."),
            ),
            (
                unusable(FormatFault::MinuteWithoutHour {
                    written: "04".into(),
                    examples: vec!["15:04".into(), "3.04pm".into()],
                }),
                "Unusable date format: `04` is a minute with no hour",
                Some("Add the hour before it, such as `15:04` or `3.04pm`."),
            ),
            (
                unusable(FormatFault::OnlyATime {
                    example: "3.04pm".into(),
                    with_date: "31 December 2025, 3.04pm".into(),
                }),
                "Unusable date format: `3.04pm` is a time with no date",
                Some("Write the date with it, such as `31 December 2025, 3.04pm`."),
            ),
            (
                unusable(FormatFault::OnlyAWeekday {
                    example: "Wednesday".into(),
                    with_date: Some("Wednesday 31 December 2025".into()),
                }),
                "Unusable date format: `Wednesday` is a weekday with no date",
                Some("Write the date with it, such as `Wednesday 31 December 2025`."),
            ),
            (
                unusable(FormatFault::Missing {
                    example: "31 grudnia 2025".into(),
                    pieces: vec![Piece::Month],
                    months: months(),
                }),
                "Unusable date format: `31 grudnia 2025` has no month",
                Some("Write the month as `December`, `Dec` or `12`."),
            ),
            (
                unusable(FormatFault::Missing {
                    example: "December".into(),
                    pieces: vec![Piece::Day, Piece::Year],
                    months: months(),
                }),
                "Unusable date format: `December` has no day or year",
                Some("Add the day and year, which every date on the site is written with."),
            ),
            (
                unusable(FormatFault::TwoDigitYear {
                    example: "31/12/25".into(),
                }),
                "Unusable date format: `31/12/25` writes the year in two digits",
                Some("Write the year in full, `2025`, so no date is read in the wrong century."),
            ),
            (
                unusable(FormatFault::OtherWeekday {
                    written: "Mon".into(),
                    day: "31 December 2025".into(),
                    weekday: "Wednesday".into(),
                    corrected: "Wed 31 December 2025".into(),
                }),
                "Unusable date format: 31 December 2025 is a Wednesday, not `Mon`",
                Some("Did you mean `Wed 31 December 2025`?"),
            ),
            (
                unusable(FormatFault::OtherMonth {
                    written: "March".into(),
                    day: "31 December 2025".into(),
                    month: "December".into(),
                    corrected: "31 December 2025".into(),
                }),
                "Unusable date format: 31 December 2025 is in December, not `March`",
                Some("Did you mean `31 December 2025`?"),
            ),
            (
                unusable(FormatFault::NoWords {
                    list: WordsWanted::Months,
                    from: WordsFrom::Site("languages/fr.yaml".into()),
                }),
                "Unusable date format: `languages/fr.yaml` has no `months`",
                Some(
                    "List the 12 months under `months:` in `languages/fr.yaml`, or write the \
                     month in figures.",
                ),
            ),
            (
                unusable(FormatFault::NoWords {
                    list: WordsWanted::Weekdays,
                    from: WordsFrom::Nowhere("sw".into()),
                }),
                "Unusable date format: Statac has no `weekdays` for `sw`",
                Some(
                    "List the 7 weekdays, Monday first, under `weekdays:` in \
                     `languages/sw.yaml`, or leave the weekday out.",
                ),
            ),
            (
                unusable(FormatFault::NoWords {
                    list: WordsWanted::Days,
                    from: WordsFrom::Statac("de".into()),
                }),
                "Unusable date format: Statac has no `days` for `de`",
                Some("Write the day as a number."),
            ),
            (
                unusable(FormatFault::NoWords {
                    list: WordsWanted::AmPm,
                    from: WordsFrom::Statac("de".into()),
                }),
                "Unusable date format: Statac has no `am_pm` for `de`",
                Some("Write the time as `15:04`."),
            ),
            (
                Kind::NoTimeZone,
                "No time zone: dates without an offset are read as UTC",
                Some(
                    "Add a line such as `timezone: Europe/London` to `statac.yaml`, naming the \
                     zone your dates are written in.",
                ),
            ),
            (
                Kind::HookStopped {
                    hook: "after_build".into(),
                    ended: Ended::Code(3),
                    said: "one\ntwo\n".into(),
                },
                "Hook stopped: `after_build` ended with exit code 3",
                None,
            ),
            (
                Kind::HookStopped {
                    hook: "before_build".into(),
                    ended: Ended::Cut,
                    said: String::new(),
                },
                "Hook stopped: `before_build` was ended before it finished",
                None,
            ),
            (
                Kind::HookStopped {
                    hook: "before_build".into(),
                    ended: Ended::NotStarted("permission denied".into()),
                    said: String::new(),
                },
                "Hook stopped: can't start a shell for `before_build`: permission denied",
                None,
            ),
            (
                Kind::HookNotRun {
                    hooks: vec!["before_build".into(), "after_build".into()],
                },
                "Hook not run: `check` doesn't run `before_build` or `after_build`",
                None,
            ),
            (
                Kind::HookNotRun {
                    hooks: vec!["after_build".into()],
                },
                "Hook not run: `check` doesn't run `after_build`",
                None,
            ),
            (
                Kind::OutputFolderNamed {
                    folder: "_site".into(),
                    rewritten: "./index.sh \"$STATAC_OUT\"".into(),
                },
                "Output folder named: `_site` holds the site built last time while \
                 `after_build` runs",
                Some("Write it as `./index.sh \"$STATAC_OUT\"`."),
            ),
            (
                Kind::NothingMade {
                    place: "search/".into(),
                },
                "Nothing made: `after_build` left nothing at `search/`",
                None,
            ),
            (
                Kind::WrongPlace {
                    key: "after_build_makes".into(),
                    why: WrongPlace::NoCommand("after_build"),
                },
                "Wrong place: `after_build_makes` is for a site that sets `after_build`",
                Some("Add the `after_build:` line it describes, or take it out."),
            ),
            (
                Kind::SameAddress {
                    first: "static/search/index.js".into(),
                    second: Box::new(Named::MadeAfter("search/".into())),
                    alike: Alike::Exactly,
                    address: "/search/index.js".into(),
                },
                "Same address: `static/search/index.js` and what `after_build` makes at \
                 `search/` are both published at `/search/index.js`",
                Some("Rename one of them."),
            ),
            (
                Kind::UnknownLanguage { tag: "sw".into() },
                "Unknown language: Statac has no words for `sw`",
                Some("Write its months, weekdays and quotation marks in `languages/sw.yaml`."),
            ),
            (
                Kind::LanguageWrittenTwice {
                    written: "en".into(),
                    first_written: "en".into(),
                    first: crate::source::Place {
                        source: crate::source::SourceId::from_index(0),
                        span: crate::source::Span::new(0, 1),
                    },
                },
                "Language written twice: `en`",
                None,
            ),
            (
                Kind::LanguageWrittenTwice {
                    written: "EN".into(),
                    first_written: "en".into(),
                    first: crate::source::Place {
                        source: crate::source::SourceId::from_index(0),
                        span: crate::source::Span::new(0, 1),
                    },
                },
                "Language written twice: `en` and `EN` are one language",
                None,
            ),
            (
                Kind::MissingWords {
                    tag: "fr".into(),
                    file: "languages/fr.yaml".into(),
                    words: ShownWords::BackToTheText,
                    pages: 1,
                },
                "Missing words: the footnotes of 1 page in `fr` have no words for the way back",
                Some(
                    "Add them to `languages/fr.yaml`, translating the English line \
                     `back_to_the_text: back to the text`.",
                ),
            ),
            (
                Kind::MissingWords {
                    tag: "fr-CA".into(),
                    file: "languages/fr-CA.yaml".into(),
                    words: ShownWords::Callouts,
                    pages: 1200,
                },
                "Missing words: the callouts of 1,200 pages in `fr-CA` have no names",
                Some(
                    "Add them to `languages/fr-CA.yaml`, translating the English line `callouts: \
                     [Note, Tip, Important, Warning, Caution]`.",
                ),
            ),
            (
                Kind::NoPagesInLanguage { tag: "en".into() },
                "No pages in `en`: there is no `content/en/` folder",
                Some("Put the pages written in `en` in `content/en/`."),
            ),
            (
                Kind::LanguageFolderInPattern {
                    pattern: "/en/{year}/{slug}/".into(),
                    folder: "/en/".into(),
                    without: "/{year}/{slug}/".into(),
                },
                "Language folder in pattern: `/en/{year}/{slug}/` is read from `/en/` already",
                Some("Write it as `/{year}/{slug}/`."),
            ),
            (
                Kind::SameTranslation {
                    tag: "en".into(),
                    by: Translated::Key("about".into()),
                    first: None,
                },
                "Same translation: two pages in `en` have `translation_key: about`",
                Some("Give each page in `en` its own key."),
            ),
            (
                Kind::SameTranslation {
                    tag: "en".into(),
                    by: Translated::Place("content/about.md".into()),
                    first: None,
                },
                "Same translation: two pages in `en` stand where `content/about.md` does",
                Some("Rename one of them, or give one a `translation_key`."),
            ),
            (
                Kind::UnmatchedTranslationKey {
                    key: "abuot".into(),
                    nearest: Some("about".into()),
                },
                "Unmatched translation key: no page in another language has `abuot`",
                Some("Did you mean `about`?"),
            ),
            (
                Kind::UnmatchedTranslationKey {
                    key: "about".into(),
                    nearest: None,
                },
                "Unmatched translation key: no page in another language has `about`",
                None,
            ),
            (
                Kind::NotLinkedByPlace {
                    keyed: "content/about.md".into(),
                    key: "about".into(),
                },
                "Not linked by place: `content/about.md` has `translation_key: about`",
                Some("To link the two, give this page `translation_key: about` too."),
            ),
            (
                Kind::UnknownCodeLanguage {
                    written: "cosole".into(),
                    nearest: Some("console".into()),
                },
                "Unknown code language: Statac has no colours for `cosole`",
                Some("Did you mean `console`?"),
            ),
            (
                Kind::UnknownCodeLanguage {
                    written: "qbasic".into(),
                    nearest: None,
                },
                "Unknown code language: Statac has no colours for `qbasic`",
                None,
            ),
            (
                Kind::CodeNotColoured {
                    characters: 52310,
                    block: true,
                },
                "Code not coloured: a line in this block is 52,310 characters long",
                Some("Break the line up, or take the language off the block's first line."),
            ),
            (
                Kind::CodeNotColoured {
                    characters: 5001,
                    block: false,
                },
                "Code not coloured: a line in this block is 5,001 characters long",
                None,
            ),
            (
                Kind::CodeColoursNotLoaded,
                "Code colours not loaded: no template asks for `code.css`",
                Some("Add `{{ css(\"code.css\") }}` inside a template's `<head>`."),
            ),
            (
                Kind::UnknownWord {
                    word: Word::TimeZone,
                    written: "Europe/Londn".into(),
                    nearest: Some("Europe/London".into()),
                },
                "Unknown time zone: `Europe/Londn`",
                Some("Did you mean `Europe/London`?"),
            ),
            (
                wrong(
                    "months",
                    Expected::Words(WordList::Months),
                    Found::Items(11),
                ),
                "Wrong kind of value: `months` is 12 months, January first, not 11",
                None,
            ),
            (
                Kind::WrongKind {
                    field: "am_pm".into(),
                    expected: Expected::Words(WordList::AmPm),
                    found: Found::Items(1),
                    respelt: Some("[am, pm]".into()),
                },
                "Wrong kind of value: `am_pm` is 2 words, the morning's first, not 1",
                Some("Write it as `[am, pm]`."),
            ),
            (
                wrong(
                    "months",
                    Expected::Words(WordList::Months),
                    Found::Twice("mars".into()),
                ),
                "Wrong kind of value: `months` is 12 months, January first, not `mars` twice",
                None,
            ),
            (
                wrong(
                    "weekdays",
                    Expected::Words(WordList::Weekdays),
                    Found::Items(5),
                ),
                "Wrong kind of value: `weekdays` is 7 weekdays, Monday first, not 5",
                None,
            ),
            (
                wrong("days", Expected::Words(WordList::Days), Found::Items(30)),
                "Wrong kind of value: `days` is 31 days, the first first, not 30",
                None,
            ),
            (
                wrong("digits", Expected::Words(WordList::Digits), Found::Items(9)),
                "Wrong kind of value: `digits` is 10 digits, zero first, not 9",
                None,
            ),
            (
                wrong("quotes", Expected::Words(WordList::Quotes), Found::Items(2)),
                "Wrong kind of value: `quotes` is 4 marks, the outer pair then the inner pair, \
                 not 2",
                None,
            ),
            (
                wrong(
                    "callouts",
                    Expected::Words(WordList::Callouts),
                    Found::Items(2),
                ),
                "Wrong kind of value: `callouts` is 5 names, for note, tip, important, warning \
                 and caution, not 2",
                None,
            ),
            (
                wrong(
                    "digits",
                    Expected::OneCharacter,
                    Found::Written("99".into()),
                ),
                "Wrong kind of value: `digits` is one character each, not `99`",
                None,
            ),
            (
                wrong("inside_quotes", Expected::Space, Found::Written("x".into())),
                "Wrong kind of value: `inside_quotes` is one space, such as `\"\\u00A0\"`, not `x`",
                None,
            ),
        ]
    }

    #[test]
    fn date_problems_read_as_written() {
        for (kind, line, suggested) in dates() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    #[test]
    fn content_problems_read_as_written() {
        for (kind, line, suggested) in content() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    fn templates() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let wrong = |fault| Kind::WrongArgument {
            snippet: "card".into(),
            fault,
        };
        let asked = |call: &str, needed| Kind::WrongArgument {
            snippet: call.into(),
            fault: ArgumentFault::Needs(needed),
        };
        let stopped = |reason| Kind::TemplateStopped { reason };
        let no_value = |lack, instead| Kind::NoValue {
            written: "page.summary".into(),
            lack,
            instead,
        };
        vec![
            (
                Kind::MalformedTemplate {
                    found: TemplateFault::Syntax(
                        "Unexpected end of input, expected end of block.".into(),
                    ),
                },
                "Malformed template: unexpected end of input, expected end of block",
                None,
            ),
            (
                Kind::MalformedTemplate {
                    found: TemplateFault::Takes(TakesFault::NeverRead("alts".into())),
                },
                "Malformed template: the snippet never reads `alts`",
                Some("Take it out of the line."),
            ),
            (
                Kind::MalformedTemplate {
                    found: TemplateFault::Takes(TakesFault::TooMany),
                },
                "Malformed template: the `takes` line names more than 2",
                Some("Have calls name the rest, as `caption=...`."),
            ),
            (
                Kind::MissingTemplate {
                    path: "templates/folder.html".into(),
                    chosen: Chooser::Default,
                    nearest: None,
                },
                "Missing template: there is no `templates/folder.html`",
                Some("Add the template, or choose another with `layout:`."),
            ),
            (
                Kind::MissingTemplate {
                    path: "templates/psot.html".into(),
                    chosen: Chooser::Written,
                    nearest: Some("post".into()),
                },
                "Missing template: there is no `templates/psot.html`",
                Some("Did you mean `post`?"),
            ),
            (
                no_value(
                    Lack::OnPage("content/post-1.md".into()),
                    Instead::Default("page.summary | default(\"\")".into()),
                ),
                "No value: `page.summary` isn't set on `content/post-1.md`",
                Some("Write `page.summary | default(\"\")` to show nothing where it isn't set."),
            ),
            (
                Kind::NoValue {
                    written: "page.title".into(),
                    lack: Lack::OnPage(Named::ListPage(Word::Tag, "Rust".into())),
                    instead: Instead::Headline("headline(page)".into()),
                },
                "No value: `page.title` isn't set on the page for the tag `Rust`",
                Some("A page Statac makes has no title: write `headline(page)` to name it."),
            ),
            (
                no_value(Lack::Unset, Instead::Handed("post.summary".into())),
                "No value: nothing is set for `page.summary`",
                Some("Did you mean `post.summary`?"),
            ),
            (
                no_value(Lack::Empty, Instead::Nothing),
                "No value: `page.summary` is empty",
                None,
            ),
            (
                Kind::NoValue {
                    written: "site.data.people.person_2".into(),
                    lack: Lack::InDataFile("data/people.yaml".into()),
                    instead: Instead::Nothing,
                },
                "No value: `site.data.people.person_2` isn't set in `data/people.yaml`",
                None,
            ),
            (
                Kind::NoValue {
                    written: "site.data.peple".into(),
                    lack: Lack::InDataFolder("data/".into()),
                    instead: Instead::Nothing,
                },
                "No value: `site.data.peple` names nothing in `data/`",
                None,
            ),
            (
                no_value(Lack::InLiteral { map: false }, Instead::Nothing),
                "No value: `page.summary` may have no value inside a list",
                None,
            ),
            (
                Kind::NoValue {
                    written: "date".into(),
                    lack: Lack::ByField(Box::new(crate::problem::ByField {
                        item: "content/post-1.md".into(),
                        field: "date".into(),
                        filter: "groupby".into(),
                    })),
                    instead: Instead::Selected(
                        "site.pages | selectattr(\"date\") | groupby(\"date\")".into(),
                    ),
                },
                "No value: `content/post-1.md` has no `date` for `groupby`",
                Some(
                    "Write `site.pages | selectattr(\"date\") | groupby(\"date\")` to leave out \
                     the items without it.",
                ),
            ),
            (
                Kind::NotSupported {
                    what: Unsupported::Statement {
                        statement: "include".into(),
                        calls: vec!["{{ card(post) }}".into()],
                    },
                },
                "Not supported: `include`",
                Some("Call the snippet instead: `{{ card(post) }}`."),
            ),
            (
                Kind::NotSupported {
                    what: Unsupported::Statement {
                        statement: "from".into(),
                        calls: vec!["{{ card(post) }}".into()],
                    },
                },
                "Not supported: `from`",
                Some("Take this line out, and call `{{ card(post) }}`."),
            ),
            (
                Kind::NotSupported {
                    what: Unsupported::NotYet("page.photo".into()),
                },
                "Not supported: `page.photo` isn't available yet",
                None,
            ),
            (
                wrong(ArgumentFault::NoSuchName {
                    given: "pots".into(),
                    nearest: Some("post".into()),
                }),
                "Wrong argument: `card` takes no `pots`",
                Some("Did you mean `post`?"),
            ),
            (
                wrong(ArgumentFault::NeedsNames {
                    wanted: vec!["post".into(), "size".into()],
                }),
                "Wrong argument: `card` needs each value named",
                Some("Write it as `card(post=..., size=...)`."),
            ),
            (
                wrong(ArgumentFault::NeedsNames { wanted: vec![] }),
                "Wrong argument: `card` takes no values",
                Some("Take the values out of the call."),
            ),
            (
                wrong(ArgumentFault::TooMany {
                    takes: vec!["photo".into(), "alt".into()],
                    others: vec!["caption".into()],
                }),
                "Wrong argument: `card` takes only `photo` and `alt` without a name",
                Some("Name the rest, as `caption=...`."),
            ),
            (
                wrong(ArgumentFault::Twice("photo".into())),
                "Wrong argument: `card` is given `photo` twice",
                Some("Keep one of them."),
            ),
            (
                wrong(ArgumentFault::Supplied("standalone".into())),
                "Wrong argument: Statac hands `card` its `standalone` itself",
                Some("Take it out of the call."),
            ),
            (
                wrong(ArgumentFault::NotGiven("post".into())),
                "Wrong argument: `card` wasn't given `post`",
                Some("Write it as `card(post=...)`."),
            ),
            (
                asked("page.photo", Needed::Name(Sort::Picture)),
                "Wrong argument: `page.photo` needs the name of one picture",
                Some("Write it as `page.photo(\"photo-1.jpg\")`."),
            ),
            (
                asked("page.audio", Needed::Name(Sort::Recording)),
                "Wrong argument: `page.audio` needs the name of one recording",
                Some("Write it as `page.audio(\"recording-1.mp3\")`."),
            ),
            (
                asked("page.search_box", Needed::Brackets),
                "Wrong argument: `page.search_box` needs its brackets",
                Some("Write it as `page.search_box()`."),
            ),
            (
                asked("page.photos", Needed::Names),
                "Wrong argument: `page.photos` needs a list of names",
                Some(
                    "Write it as `page.photos([\"photo-1.jpg\", \"photo-2.jpg\"])`, or \
                     `page.photos()` for every photograph beside the page.",
                ),
            ),
            (
                asked("page.photo", Needed::Text("sizes".into())),
                "Wrong argument: `page.photo` needs `sizes` as text",
                Some(
                    "Write it as a browser reads it: `sizes=\"(max-width: 40rem) 100vw, \
                     40rem\"`.",
                ),
            ),
            (
                asked("page.photo", Needed::TrueFalse("full".into())),
                "Wrong argument: `page.photo` needs `full` to be `true` or `false`",
                None,
            ),
            (
                asked(
                    "page.photo",
                    Needed::OnSite("https://example.org/photo-1.jpg".into()),
                ),
                "Wrong argument: `page.photo` needs a file of the site, not \
                 `https://example.org/photo-1.jpg`",
                None,
            ),
            (
                asked(
                    "page.youtube",
                    Needed::VideoOn {
                        site: "YouTube",
                        written: Some("video-1".into()),
                    },
                ),
                "Wrong argument: `page.youtube` needs a YouTube address or id, not `video-1`",
                Some(
                    "Write the address YouTube gives for the video, such as \
                     `https://www.youtube.com/watch?v=abcdefghijk`.",
                ),
            ),
            (
                asked(
                    "page.vimeo",
                    Needed::VideoOn {
                        site: "Vimeo",
                        written: None,
                    },
                ),
                "Wrong argument: `page.vimeo` needs a Vimeo address or id",
                Some(
                    "Write the address Vimeo gives for the video, such as \
                     `https://vimeo.com/123456789`.",
                ),
            ),
            (
                asked(
                    "page.video",
                    Needed::Playable {
                        sort: Sort::Video,
                        written: "video-1.avi".into(),
                    },
                ),
                "Wrong argument: `page.video` needs a video a browser plays, not `video-1.avi`",
                Some("Save it as `.mp4`, `.webm` or `.ogv`."),
            ),
            (
                asked(
                    "page.audio",
                    Needed::Playable {
                        sort: Sort::Recording,
                        written: "recording-1.aiff".into(),
                    },
                ),
                "Wrong argument: `page.audio` needs a recording a browser plays, not \
                 `recording-1.aiff`",
                Some("Save it as `.mp3`, `.m4a`, `.ogg`, `.wav` or `.flac`."),
            ),
            (
                stopped(Stopped::NoSiteAddress),
                "Template stopped: `absolute` needs the site's `url`",
                Some("Add `url:` to `statac.yaml`."),
            ),
            (
                stopped(Stopped::Engine("Tried to add incompatible types.".into())),
                "Template stopped: tried to add incompatible types",
                None,
            ),
            (
                stopped(Stopped::NotAPage("headline".into())),
                "Template stopped: `headline` needs a page",
                Some("Write it as `headline(post)`."),
            ),
            (
                stopped(Stopped::NotAnAddress),
                "Template stopped: `url` needs an address from the top of the site",
                Some(r#"Write it as `url("/about/")`."#),
            ),
            (
                stopped(Stopped::CallsItself("card".into())),
                "Template stopped: `card` calls itself",
                Some("Take the call out of one of them."),
            ),
            (
                stopped(Stopped::RanOn {
                    template: "templates/post.html".into(),
                    steps: 100_000_000,
                }),
                "Template stopped: `templates/post.html` ran 100,000,000 steps without finishing",
                Some("Look for a loop that never ends, or a loop inside a loop over long lists."),
            ),
            (
                Kind::PageInSnippet {
                    snippet: "card".into(),
                    written: "page.title".into(),
                    handed: "post.title".into(),
                },
                "Page in snippet: `card` shows `page.title`, the page being built",
                Some("Did you mean `post.title`?"),
            ),
            (
                Kind::TextNeverShown {
                    extended: "templates/base.html".into(),
                    block: "content".into(),
                    nearest: Some("main".into()),
                },
                "Text never shown: `templates/base.html` has no block called `content`",
                Some("Did you mean `main`?"),
            ),
            (
                Kind::WritingNeverShown {
                    layout: "templates/hollow.html".into(),
                    pages: 3,
                },
                "Writing never shown: 3 pages built by `templates/hollow.html` have writing it \
                 never shows",
                Some("Add `{{ page.content }}` where the writing belongs."),
            ),
            (
                Kind::WritingNeverShown {
                    layout: "templates/hollow.html".into(),
                    pages: 1,
                },
                "Writing never shown: 1 page built by `templates/hollow.html` has writing it \
                 never shows",
                Some("Add `{{ page.content }}` where the writing belongs."),
            ),
            (
                Kind::UnknownWord {
                    word: Word::Filter,
                    written: "uper".into(),
                    nearest: Some("upper".into()),
                },
                "Unknown filter: `uper`",
                Some("Did you mean `upper`?"),
            ),
            (
                Kind::AlwaysFalse {
                    name: "subtitel".into(),
                    meant: "page.subtitle".into(),
                },
                "Always false: no page has `subtitel`",
                Some("Did you mean `page.subtitle`?"),
            ),
            (
                Kind::AlwaysFalse {
                    name: "subtitel".into(),
                    meant: "post.subtitle".into(),
                },
                "Always false: no page has `subtitel`",
                Some("Did you mean `post.subtitle`?"),
            ),
        ]
    }

    #[test]
    fn template_problems_read_as_written() {
        for (kind, line, suggested) in templates() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    #[test]
    fn every_kind_is_worded_here() {
        let mut codes: Vec<_> = kinds().iter().map(Kind::code).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes, crate::problem::Code::ALL);
    }

    #[test]
    fn other_problems_read_as_written() {
        for (kind, line, suggested) in others() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    #[test]
    fn what_a_command_printed_is_shown_whole_or_by_its_last_lines() {
        let stopped = |said: &str, ended| Kind::HookStopped {
            hook: "after_build".into(),
            ended,
            said: said.into(),
        };
        assert_eq!(
            printed(&stopped("one\r\ntwo\n\n", Ended::Code(1))),
            Some(("What it printed:".into(), vec!["one", "two"]))
        );
        assert_eq!(
            printed(&stopped(" \n", Ended::Cut)),
            Some(("It printed nothing.".into(), Vec::new()))
        );
        let long: String = (1..=1200).map(|n| format!("line {n}\n")).collect();
        let kind = stopped(&long, Ended::Code(1));
        let (heading, lines) = printed(&kind).unwrap();
        assert_eq!(heading, "The last 40 of the 1,200 lines it printed:");
        assert_eq!(lines.len(), 40);
        assert_eq!(lines[0], "line 1161");
        assert_eq!(lines[39], "line 1200");
        // A shell that never started ran nothing that could print.
        let unstarted = Ended::NotStarted("permission denied".into());
        assert_eq!(printed(&stopped("", unstarted)), None);
        assert_eq!(printed(&Kind::NoTimeZone), None);
        assert_eq!(ran_hook("before_build"), "Ran `before_build`.");
        assert_eq!(ran_after(1, 0, 0), "Ran `after_build`, which added 1 file.");
        assert_eq!(
            ran_after(1284, 0, 2),
            "Ran `after_build`, which added 1,284 files and removed 2."
        );
        assert_eq!(
            ran_after(2, 0, 0),
            "Ran `after_build`, which added 2 files."
        );
        assert_eq!(
            ran_after(2, 1, 0),
            "Ran `after_build`, which added 2 files and changed 1."
        );
        assert_eq!(
            ran_after(2, 0, 3),
            "Ran `after_build`, which added 2 files and removed 3."
        );
        assert_eq!(
            ran_after(2, 1, 3),
            "Ran `after_build`, which added 2 files, changed 1 and removed 3."
        );
        assert_eq!(
            ran_after(0, 1, 0),
            "Ran `after_build`, which added 0 files and changed 1."
        );
    }

    #[test]
    fn counts_have_digits_and_right_plurals() {
        let cases = [
            (
                Severity::Warning,
                &[("broken-link", 2), ("duplicate-key", 1)][..],
                0,
                "3 warnings: 2 broken-link, 1 duplicate-key",
            ),
            (
                Severity::Note,
                &[("broken-link", 1)],
                4,
                "1 note: 1 broken-link, and 4 quietened",
            ),
            (Severity::Warning, &[], 4, "4 warnings, all quietened"),
            (Severity::Warning, &[], 1, "1 warning, quietened"),
            (
                Severity::Error,
                &[("duplicate-key", 2)],
                0,
                "2 errors: 2 duplicate-key",
            ),
        ];
        for (severity, kinds, quietened, line) in cases {
            assert_eq!(count(severity, kinds, quietened), line);
        }
        assert_eq!(
            count(
                Severity::Warning,
                &[("broken-link", 1200), ("set-aside", 3)],
                2000
            ),
            "1,203 warnings: 1,200 broken-link, 3 set-aside, and 2,000 quietened"
        );
        assert_eq!(
            gone(&[("page", 1200), ("file", 2)]),
            "Gone since the last build: 1,200 page, 2 file."
        );
        assert_eq!(see_them(1), "Run `statac check` to see it.");
        assert_eq!(see_them(3), "Run `statac check` to see them.");
    }

    fn yaml() -> Vec<(Kind, &'static str, Option<&'static str>)> {
        let malformed = |found| Kind::MalformedYaml { found };
        let mut sources = crate::source::Sources::default();
        let first = crate::source::Place {
            source: sources.add("content/post-1.md", "title: Post 1\n".into()),
            span: crate::source::Span::new(0, 5),
        };
        vec![
            (
                malformed(Malformed::UnclosedQuote('"')),
                "Malformed YAML: `\"` is never closed",
                Some("End the value with another `\"`."),
            ),
            (
                malformed(Malformed::ShallowLine {
                    quote: '"',
                    back: 12,
                    spaces: 2,
                }),
                "Malformed YAML: a line of a text in quotes isn't indented far enough",
                Some("Start each later line of the text with at least 2 spaces."),
            ),
            (
                malformed(Malformed::ShallowLine {
                    quote: '\'',
                    back: 12,
                    spaces: 1,
                }),
                "Malformed YAML: a line of a text in quotes isn't indented far enough",
                Some("Start each later line of the text with at least 1 space."),
            ),
            (
                malformed(Malformed::UnclosedBracket('[')),
                "Malformed YAML: `[` is never closed",
                Some("Close it with `]`."),
            ),
            (
                malformed(Malformed::MismatchedBracket {
                    open: '{',
                    close: ']',
                }),
                "Malformed YAML: `{` is closed by `]`",
                Some("Close it with `}`."),
            ),
            (
                malformed(Malformed::UnknownEscape(r"\q".into())),
                r"Malformed YAML: unknown escape `\q`",
                Some(r"Write `\\` for a backslash, or use single quotes."),
            ),
            (
                malformed(Malformed::TextAfterQuote),
                "Malformed YAML: text after the closing quote",
                Some("Put the whole value inside the quotes."),
            ),
            (
                malformed(Malformed::Indentation),
                "Malformed YAML: unexpected indentation",
                None,
            ),
            (
                malformed(Malformed::Tab),
                "Malformed YAML: a tab where YAML needs spaces",
                Some("Use spaces instead."),
            ),
            (
                malformed(Malformed::UnknownAnchor("*book".into())),
                "Malformed YAML: `*book` refers to no anchor",
                Some("Mark the value it repeats with `&book`, earlier in the file."),
            ),
            (
                malformed(Malformed::SecondDocument),
                "Malformed YAML: more than one document",
                None,
            ),
            (
                malformed(Malformed::TooDeep),
                "Malformed YAML: nested too deeply",
                None,
            ),
            (
                malformed(Malformed::EndsEarly),
                "Malformed YAML: ends too soon",
                None,
            ),
            (
                malformed(Malformed::Unexpected(":".into())),
                "Malformed YAML: unexpected `:`",
                None,
            ),
            (
                malformed(Malformed::ControlCharacter('\u{92}')),
                "Malformed YAML: U+0092, a control character YAML allows nowhere",
                Some(
                    "This is usually a character such as \u{2019} saved in an old encoding: type it again.",
                ),
            ),
            (
                malformed(Malformed::NotBase64),
                "Malformed YAML: the `!!binary` value isn't base64",
                Some("Write it in base64, or take off `!!binary` to keep it as text."),
            ),
            (
                Kind::MalformedJson {
                    found: JsonFault::TrailingComma,
                },
                "Malformed JSON: a comma with nothing after it",
                Some("Take the comma out."),
            ),
            (
                Kind::MalformedJson {
                    found: JsonFault::ControlCharacter('\t'),
                },
                "Malformed JSON: U+0009, a control character JSON allows in text only as an escape",
                Some("Write it as `\\u0009`."),
            ),
            (
                Kind::MalformedJson {
                    found: JsonFault::UnclosedBracket('['),
                },
                "Malformed JSON: `[` is never closed",
                Some("Close it with `]`."),
            ),
            (
                Kind::MalformedJson {
                    found: JsonFault::NotANumber("01".into()),
                },
                "Malformed JSON: `01` isn't a number as JSON writes one",
                None,
            ),
            (
                Kind::MalformedJson {
                    found: JsonFault::HalfCharacter("\\ud83d".into()),
                },
                "Malformed JSON: `\\ud83d` is half of a character written in two escapes",
                None,
            ),
            (
                malformed(Malformed::ControlCharacter('\u{1}')),
                "Malformed YAML: U+0001, a control character YAML allows nowhere",
                Some("Delete it, or type the character you meant."),
            ),
            (
                Kind::DuplicateKey {
                    key: "title".into(),
                    first,
                },
                "Duplicate key: `title`",
                None,
            ),
            (
                Kind::AliasLimit {
                    alias: "*book".into(),
                    limit: Repeated::Values(10_000),
                },
                "Alias limit: by `*book`, aliases have repeated more than 10,000 values",
                None,
            ),
            (
                Kind::AliasLimit {
                    alias: "*book".into(),
                    limit: Repeated::TextBytes(1_000_000),
                },
                "Alias limit: by `*book`, aliases have repeated more than 1 MB of text",
                None,
            ),
            (
                Kind::NestedTooDeep,
                "Nested too deep: the writing here is nested more than 200 deep",
                None,
            ),
            (
                Kind::UnknownYamlTag {
                    tag: "!local".into(),
                },
                "Unknown YAML tag: `!local`",
                None,
            ),
            (
                Kind::UnclosedFrontMatter,
                "Unclosed front matter: no `---` line ends it",
                Some("Add a `---` line after the last field."),
            ),
        ]
    }

    #[test]
    fn yaml_problems_read_as_written() {
        for (kind, line, suggested) in yaml() {
            assert_eq!(headline(&kind), line);
            assert_eq!(suggestion(&kind).as_deref(), suggested, "{line}");
        }
    }

    #[test]
    fn large_counts_are_grouped_in_threes() {
        let cases = [
            (0, "0"),
            (999, "999"),
            (1000, "1,000"),
            (1_234_567, "1,234,567"),
        ];
        for (n, written) in cases {
            assert_eq!(grouped(n), written);
        }
    }

    #[test]
    fn headlines_follow_the_wording_guide() {
        for kind in kinds() {
            let line = headline(&kind);
            assert!(!line.ends_with('.'), "{line}");
            assert!(line.chars().next().unwrap().is_uppercase(), "{line}");
            assert!(line.contains(": "), "{line}");
            // The author's text, quoted in backticks, may hold anything.
            let ours: String = line.split('`').step_by(2).collect();
            assert!(!ours.contains(['\u{2013}', '\u{2014}', '!']), "{line}");
            for banned in ["invalid", "fatal", "failed to", "please", "simply"] {
                assert!(!ours.to_lowercase().contains(banned), "{line}");
            }
        }
    }

    #[test]
    fn nothing_is_set_aside_for_the_kind_of_file_it_is() {
        let formats = ["JPEG", "PNG", "GIF", "WebP", "SVG", "AVIF", "HEIC"];
        let names_one = |text: &str| formats.iter().any(|format| text.contains(format));
        for kind in kinds() {
            if kind.code() == crate::problem::Code::SetAside {
                let line = headline(&kind);
                assert!(!names_one(&line), "{line}");
                let suggested = suggestion(&kind).unwrap_or_default();
                assert!(!names_one(&suggested), "{suggested}");
            }
        }
        assert!(!names_one(crate::problem::Code::SetAside.explanation()));
    }

    #[test]
    fn suggestions_are_sentences() {
        for kind in kinds() {
            if let Some(s) = suggestion(&kind) {
                assert!(s.ends_with(['.', '?']), "{s}");
            }
        }
    }

    /// A kind's short form: what the terminal prints of it without
    /// colour, less its place.
    fn short_form(kind: &Kind) -> String {
        let mut out = format!(
            "{}  {}\n",
            severity(kind.severity()),
            crate::report::shown(&headline(kind))
        );
        if let Some(suggestion) = suggestion(kind) {
            out.push_str(&format!("  {}\n", crate::report::shown(&suggestion)));
        }
        out.push_str(&format!("  {}\n", more(&explanation_link(kind.name()))));
        out
    }

    /// Each explanation shows its kind's first fixture in short form,
    /// straight after its opening.
    #[test]
    fn every_explanation_shows_its_first_fixture() {
        let mut seen = Vec::new();
        for kind in kinds() {
            let code = kind.code();
            if seen.contains(&code) {
                continue;
            }
            seen.push(code);
            let section = format!(
                "\n## What you'll see {{#what-youll-see}}\n\n```text\n{}```\n\n## ",
                short_form(&kind)
            );
            let text = code.explanation().replace("\r\n", "\n");
            let sections = text.find("\n## ").map_or("", |at| &text[at..]);
            assert!(
                sections.starts_with(&section),
                "explanations/{}.md should open its sections with{section}",
                code.name()
            );
        }
    }

    #[test]
    fn the_examples_read_as_agreed() {
        let k = kinds();
        assert_eq!(
            headline(&k[0]),
            "Broken link: nothing is published at `/about-us/`"
        );
        assert_eq!(suggestion(&k[0]).unwrap(), "Did you mean `/about/`?");
        assert_eq!(headline(&k[1]), "Unknown field: `template`");
        assert_eq!(suggestion(&k[1]).unwrap(), "Statac calls this `layout`.");
        assert_eq!(
            headline(&k[2]),
            "Can't write `out/index.html`: a folder already has that name"
        );
        assert_eq!(
            explanation_link("broken-link"),
            "https://statac.dev/e/broken-link/"
        );
    }
}
