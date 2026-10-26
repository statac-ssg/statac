//! Every sentence Statac says about a problem, and the words problems are
//! drawn and counted with.
//!
//! It words only what `problem` and the modules below it hold, so any
//! module can use it.
//!
//! A headline leads with what happened, as a short label and the specifics,
//! with no full stop. A suggestion is a full sentence, offered only when there
//! is a good one.

use crate::fetch::Refusal;
use crate::platform::{Alike, Unwritable};
use crate::problem::FeedFormat;
use crate::problem::{
    AddressFault, AllOf, ArgumentFault, Awaited, Because, Braced, CallFault, Called, CardFileFault,
    Carried, Chooser, CommandHint, Ended, Expected, FormatFault, Former, Found, Held, Hint, Holds,
    InPlace, Instead, JsonFault, Kind, LabelledBy, Lack, ListFault, Look, Maker, Malformed,
    MathsFault, Missing, MoreAt, Named, NamedIn, Needed, NotDrawn, Occupant, Opener, OptionHint,
    Own, PictureFault, Piece, Problem, Published, PublishedAsText, Reach, Reason, RecordFault,
    Related, Relation, Repeated, Requirer, Rewrite, Root, SassFault, SassType, SetAside, Severity,
    Shape, ShownBy, ShownWords, SiteMark, Sort, Stopped, SyntaxFault, Taken, TakesFault,
    TemplateFault, Told, Translated, Unheard, Unmatched, Unquietenable, Unread, Unsupported,
    Unwritten, Waiting, Wanted, Word, WordList, WordsFrom, WordsWanted, WrongPlace,
};

pub(crate) use crate::described::{
    Described, field as described_field, setting as described_setting,
};

/// Where the docs say more of a cascade line.
pub(crate) const HANDING_DOWN: &str = "https://statac.dev/front-matter/#handing-down";

/// Where the docs say more of a setting for one language.
pub(crate) const FOR_ONE_LANGUAGE: &str =
    "https://statac.dev/dates-and-languages/#settings-for-one-language";

/// Where the docs say more of a custom setting.
pub(crate) const OWN_VALUE: &str = "https://statac.dev/settings/";

/// Where the docs say more of a custom field.
pub(crate) const OWN_FIELD: &str = "https://statac.dev/front-matter/#your-own";

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
            unknown(*word, written)
        }
        Kind::RetiredSetting { setting, .. } => {
            format!("Retired setting: Statac no longer reads `{setting}`")
        }
        Kind::LoopInForIf { word } => unknown(*word, "loop"),
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
            "Nested too deep: the Markdown here is nested more than {} deep",
            crate::problem::DEEPEST_NESTING
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
            "Missing command: no `build`, `check`, `serve`, `stamp`, `refresh`, `new`, `import` or \
             `explain`"
                .into()
        }
        Kind::UnknownOption { written, hint } => match hint {
            OptionHint::ForOther {
                command, option, ..
            } => format!("Unknown option: `statac {command}` has no `{option}`"),
            OptionHint::ShownAlready => "Unknown option: `statac serve` has no `--drafts`".into(),
            _ => format!("Unknown option: `{written}`"),
        },
        Kind::MissingValue {
            written, option, ..
        } => format!("Missing value: `{written}` needs {}", needs(option)),
        Kind::OptionGivenTwice { option } => format!("Option given twice: `{option}`"),
        Kind::UnexpectedArgument { written, .. } => format!("Unexpected argument: `{written}`"),
        Kind::MissingName => "Missing name: `statac explain` needs the name of a problem".into(),
        Kind::NothingToSearch { site: None } => {
            "Nothing to search for: `statac search` needs the words to look for".into()
        }
        Kind::NothingToSearch { site: Some(site) } => {
            format!("Nothing to search for: `{site}` is a site's folder, not words")
        }
        Kind::UnreadableName { written } => {
            format!("Unreadable name: `{written}` holds bytes that aren't UTF-8")
        }
        Kind::UnusableName { written } => {
            format!("Unusable name: `{written}` holds a character that can't be shown")
        }
        Kind::MissingFile { path, .. } => format!("Missing file: there is no `{path}`"),
        Kind::SassStopped { fault } => format!("Sass stopped: {}", sass_fault(fault)),
        Kind::WorkerOutsideImportMap { written } => {
            format!("Worker outside the import map: `{written}` isn't published under that name")
        }
        Kind::SameUrl {
            first,
            second,
            alike,
            address,
        } => {
            let (first, second) = (named(first), named(second));
            match alike {
                Alike::Exactly => {
                    format!("Same URL: {first} and {second} are both published at `{address}`")
                }
                Alike::ByCase => {
                    format!("Same URL: {first} and {second} differ only in capitals")
                }
                Alike::ByNormalisation => {
                    format!("Same URL: {first} and {second} differ only in how an accent is stored")
                }
            }
        }
        Kind::LaterPageTaken {
            list,
            number,
            page,
            address,
            ..
        } => format!(
            "Same URL: {} and {} are both published at `{address}`",
            later(*number, list),
            named(page)
        ),
        Kind::UrlTaken {
            held,
            address,
            taken,
        } => {
            let what = match held {
                Held::Draft => "draft",
                Held::Waiting => "post",
            };
            format!(
                "URL taken: this {what} would be published at `{address}`, as {} is",
                named(taken)
            )
        }
        Kind::UrlGone { address, .. } => {
            format!("URL gone: `{address}` was published before, and nothing is there now")
        }
        Kind::AddressesGone { under, gone, .. } => format!(
            "URLs gone: the {} under `{under}` were published before, and nothing is \
             at any of them now",
            how_many(gone.len(), "URL", "URLs")
        ),
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
        Kind::ReadAsCustom { own, written, .. } => {
            let own = match own {
                Own::Field(_) => "field",
                Own::Setting => "setting",
                Own::Type => "type",
            };
            format!("Read as a custom {own}: `{written}`")
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
            SetAside::SendsOn => {
                "Field set aside: a page that redirects publishes only its redirect".into()
            }
            SetAside::NotOnCard { .. } => format!("Field set aside: a card has no `{field}`"),
            SetAside::OwnPicture => {
                "Field set aside: `og_image` names this page's social image".into()
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
            WrongPlace::EachPagesOwn => format!("Wrong place: `{key}` can't cascade"),
            WrongPlace::SiteSetting => format!("Wrong place: `{key}` is a site setting"),
            WrongPlace::TopFolder => {
                format!("Wrong place: `{key}` at the top of `content/` reaches every page")
            }
            WrongPlace::NothingInside => {
                format!("Wrong place: `{key}` cascades only from a folder's `index.md`")
            }
            WrongPlace::WholeSite => {
                format!("Wrong place: `{key}` can't cascade from `statac.yaml`")
            }
            WrongPlace::SiteValue => format!("Wrong place: a page can't set `{key}`"),
            WrongPlace::FolderOnly => {
                format!("Wrong place: `{key}` belongs in a folder's `index.md`")
            }
            WrongPlace::Item => format!("Wrong place: an item can't set `{key}`"),
            WrongPlace::LanguageFile => format!("Wrong place: a language file can't set `{key}`"),
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
        Kind::UnusableUrl { fault } => format!("Unusable URL: {}", address_fault(fault)),
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
            format!("Unknown language: Statac has no translations for `{tag}`")
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
            tag, words, pages, ..
        } => {
            let pages = how_many(*pages, "page", "pages");
            match words {
                ShownWords::BackToTheText => format!(
                    "Missing words: the footnotes of {pages} in `{tag}` have no translation for \
                     the back link"
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
        Kind::MalformedTemplate { found } => format!(
            "Malformed template: {}",
            match found {
                TemplateFault::Syntax(fault) => syntax_fault(fault),
                TemplateFault::Takes(fault) => takes_fault(fault),
            }
        ),
        Kind::MissingTemplate { path, .. } => format!("Missing template: there is no `{path}`"),
        Kind::BuiltWithPageTemplate { path, pages } => format!(
            "Built with page template: `templates/page.html` builds {} in place of `{path}`",
            how_many(*pages, "page", "pages")
        ),
        Kind::BuiltWithoutTemplate { redirects, count } => {
            let (one, many, published) = match (redirects, count) {
                (false, 1) => ("page is", "pages are", "its content alone"),
                (false, _) => ("page is", "pages are", "their content alone"),
                (true, _) => ("redirect is", "redirects are", "a refresh and a link"),
            };
            format!(
                "Built without template: {} published as {published}",
                how_many(*count, one, many)
            )
        }
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
                    } => format!("`{snippet}` needs a {site} URL or id, not `{written}`"),
                    Needed::VideoOn {
                        site,
                        written: None,
                    } => format!("`{snippet}` needs a {site} URL or id"),
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
                Stopped::Operator { op, left, right } =>
                    format!("can't use `{op}` on {} and {}", shape(*left), shape(*right)),
                Stopped::DividedByZero => "can't divide by zero".into(),
                Stopped::TooLarge(written) => format!("`{written}` is too large a number"),
                Stopped::RepeatNotWhole => {
                    "`*` repeats text only a whole number of times".into()
                }
                Stopped::RepeatedTooLong => "the repeated text would be too long".into(),
                Stopped::CantWorkOut(written) => format!("can't work out `{written}`"),
                Stopped::Needs {
                    callee,
                    wanted,
                    given,
                } => match given {
                    Some(given) => format!(
                        "`{callee}` needs {}, not {}",
                        wanted_words(*wanted),
                        shape(*given)
                    ),
                    None => format!("`{callee}` needs {}", wanted_words(*wanted)),
                },
                Stopped::CantSlice(of) => format!("can't take a slice of {}", shape(*of)),
                Stopped::SliceStep => "a slice can't step by 0".into(),
                Stopped::NotCallable(written) => format!("`{written}` can't be called"),
                Stopped::NoMethod { of, name, .. } => {
                    format!("{} has no method called `{name}`", shape(*of))
                }
                Stopped::MissingArgument { callee, .. } => {
                    format!("`{callee}` is missing an argument")
                }
                Stopped::TooManyArguments(callee) => {
                    format!("too many arguments for `{callee}`")
                }
                Stopped::NoSuchArgument { callee, name, .. } => {
                    format!("`{callee}` has no argument called `{name}`")
                }
                Stopped::GivenTwice { name, callee } => {
                    format!("`{name}` is given to `{callee}` twice")
                }
                Stopped::NoLength(of) => format!("{} has no length", shape(*of)),
                Stopped::CantLoop(of) => format!("can't loop over {}", shape(*of)),
                Stopped::CantList { callee, of } => {
                    format!("`{callee}` can't make a list of {}", shape(*of))
                }
                Stopped::NotANumber(callee) => {
                    format!("`{callee}` was given text that isn't a number")
                }
                Stopped::EmptyText(callee) => format!("`{callee}` was given empty text"),
                Stopped::CantNumber { callee, of } => {
                    format!("`{callee}` can't make a number of {}", shape(*of))
                }
                Stopped::FormatNeeds { mark, given } =>
                    format!("`{mark}` in `format` needs a number, not {}", shape(*given)),
                Stopped::FormatCharacter(mark) => {
                    format!("`{mark}` in `format` needs one character")
                }
                Stopped::FormatWhole(mark) => format!("`{mark}` in `format` needs a whole number"),
                Stopped::FormatHasNo(mark) => format!("`format` has no `{mark}`"),
                Stopped::FormatTooFew => "`format` has more `%` marks than values".into(),
                Stopped::FormatNamed(mark) => format!("`{mark}` in `format` needs named values"),
                Stopped::RangeTooLong(most) => {
                    format!("`range` would make more than {} numbers", grouped(most))
                }
                Stopped::RangeStep => "`range` can't step by 0".into(),
                Stopped::LoopNotRecursive => "`loop()` works only in a `recursive` loop".into(),
                Stopped::LoopTakesOne => "`loop()` takes one value".into(),
                Stopped::Unpack(names) => format!("`{names}` needs each item to be a list"),
                Stopped::UnpackCount {
                    names,
                    wants,
                    given,
                } => format!("`{names}` needs {wants} values in each item, not {given}"),
                Stopped::NotANamespace(name) => format!("`{name}` isn't a namespace"),
                Stopped::ExtendsTwice => "a template can extend only one other".into(),
                Stopped::ExtendsName => {
                    "`{% extends %}` needs a template's name in quotes".into()
                }
                Stopped::ExtendsItself(path) => format!("`{path}` extends itself"),
                Stopped::ExtendsAgain(path) => {
                    format!("`{path}` is reached again through `extends`")
                }
                Stopped::SuperOutside => "`super()` works only inside a block".into(),
                Stopped::CallsTooDeep(Some(name)) => {
                    format!("`{name}` calls itself too many times")
                }
                Stopped::CallsTooDeep(None) => "calls nest too deeply".into(),
                Stopped::NoSiteUrl => "`absolute` needs the site's `url`".into(),
                Stopped::NoFeedKey(_) => "`feed_id` needs the site's `feed_key`".into(),
                Stopped::NotAPage(function) => format!("`{function}` needs a page"),
                Stopped::NotAnAddress => "`url` needs a URL starting with `/`".into(),
                Stopped::NotADate(filter) => format!("`{filter}` needs a date"),
                Stopped::NotAList(filter) => format!("`{filter}` needs a list of pages"),
                Stopped::CallsItself(snippet) => format!("`{snippet}` calls itself"),
                Stopped::ContentInItself => "a page's content shows itself".into(),
                Stopped::RanOn { template, steps } => format!(
                    "`{template}` ran {} steps without finishing",
                    grouped(steps)
                ),
                Stopped::NotText => "a `!!binary` value holds bytes that aren't text".into(),
                Stopped::CardInWriting => {
                    "`page.card` isn't answered in a page's content".into()
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
        Kind::ContentNeverShown { layout, pages } => format!(
            "Content never shown: {} built by `{layout}` {} content it never shows",
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
        Kind::ScheduledPost { path, date } => {
            format!("Scheduled post: `{path}` is dated `{date}`")
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
            format!(
                "Not a post: `{path}` is the list page for {}",
                the_list(*word, name)
            )
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
                Unwritten::NoTemplate(templates) => {
                    format!(
                        "Feed not written: {lists} {need} {}",
                        quoted_either(templates)
                    )
                }
                Unwritten::PageThere { page, address } => {
                    format!(
                        "Feed not written: {} is published at `{address}`",
                        named(page)
                    )
                }
                Unwritten::Undated => {
                    format!("Feed not written: {lists} {} nothing dated", has(*more))
                }
                Unwritten::NoPage => {
                    format!("Feed not written: {lists} {} no list page", has(*more))
                }
                Unwritten::PageUnpublished => {
                    format!(
                        "Feed not written: {lists} {} `index_page: false`",
                        has(*more)
                    )
                }
            }
        }
        Kind::FeedWrittenTwice {
            written,
            first_written,
            address,
            ..
        } => match written == first_written {
            true => format!("Feed written twice: `{written}`"),
            false => format!(
                "Feed written twice: `{first_written}` and `{written}` are both at `{address}`"
            ),
        },
        Kind::FeedFormatMismatch { file, built, .. } => {
            format!(
                "Feed format mismatch: `{file}` is built as {}",
                a_feed(*built)
            )
        }
        Kind::NoFeedKey { .. } => "No feed key: `statac.yaml` has no `feed_key`".into(),
        Kind::UnusedFeedTemplate { .. } => "Unused feed template: no list asks for a feed".into(),
        Kind::UnusedLanguageFile { path, .. } => {
            format!("Unused language file: no language of the site reads `{path}`")
        }
        Kind::NoSiteUrl { waiting } => {
            let what = match waiting {
                Waiting::Page(path) => format!("`{path}` is"),
                Waiting::Feeds {
                    list,
                    more: 0,
                    feeds: 1,
                } => {
                    format!("the feed of {} is", lists(list, 0))
                }
                Waiting::Feeds { list, more, .. } => {
                    format!("the feeds of {} are", lists(list, *more))
                }
                Waiting::Cards => "social cards are".into(),
                Waiting::Webmentions => "webmentions are".into(),
            };
            format!("No site URL: {what} left out until the site has a `url`")
        }
        Kind::FeedIdChanged { was, .. } => {
            format!("Feed id changed: this post was published at `{was}`")
        }
        Kind::NoLanguage => "No language set: the site is read as English".into(),
        Kind::NoListPage {
            list: (word, name),
            more,
        } => {
            let (first, others, lacking) = match word {
                Word::Folder => (
                    format!("`content/{name}/`"),
                    ("other folder", "other folders"),
                    "no `index.md`",
                ),
                Word::Series => (
                    the_list(*word, name),
                    ("other series", "other series"),
                    "none",
                ),
                _ => (the_list(*word, name), ("other tag", "other tags"), "none"),
            };
            match more {
                0 => format!("No list page: {first} has {lacking}"),
                n => format!(
                    "No list page: {first} and {} have {lacking}",
                    how_many(*n, others.0, others.1)
                ),
            }
        }
        Kind::NoHomePage { addresses, .. } => {
            let quoted: Vec<String> = addresses.iter().map(|at| format!("`{at}`")).collect();
            format!(
                "No home page: nothing is published at {}",
                either_of(&quoted)
            )
        }
        Kind::RobotsBeneathPath { top } => {
            format!("Robots beneath a path: crawlers read only `{top}`")
        }
        Kind::RedirectToItself { written } => {
            format!("Redirect to itself: `{written}` is this page's own URL")
        }
        Kind::RedirectFromItself { written } => {
            format!("Redirect to itself: `{written}` is this page's own URL")
        }
        Kind::RedirectedBack { written } => {
            format!("Redirect to itself: `{written}` redirects back to this page")
        }
        Kind::ContentNotPublished { path } => {
            format!("Content not published: `{path}` redirects")
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
                Called::Address => "its URL",
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
        Kind::IdUsedTwice { id, taken, .. } => match taken {
            Taken::Heading(_) => format!("Id used twice: a heading above has the id `{id}`"),
            Taken::Table(_) => format!("Id used twice: a table above has the id `{id}`"),
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
        Kind::UnreadableImage { path, fault } => match fault {
            PictureFault::CutShort => {
                format!("Unreadable image: `{path}` is cut short or damaged")
            }
            PictureFault::Unsupported(_) => {
                format!("Unreadable image: `{path}` is saved in a form Statac doesn't read")
            }
            PictureFault::TooLarge { width, height } => format!(
                "Unreadable image: `{path}` is {} by {} pixels, too large to open",
                grouped(width),
                grouped(height)
            ),
            PictureFault::CantMake(message) => {
                format!("Unreadable image: no resized copy of `{path}` could be made: {message}")
            }
            PictureFault::NotAStill => {
                format!("Unreadable image: `{path}` isn't a JPEG, a PNG or a still WebP")
            }
        },
        Kind::ColourConverted { path, profile } => match profile {
            Some(profile) => format!(
                "Colour converted: the resized copies of `{path}` are in sRGB, converted from \
                 `{profile}`"
            ),
            None => format!(
                "Colour converted: the resized copies of `{path}` are in sRGB, converted from \
                 its colour profile"
            ),
        },
        Kind::DescribesNoImage { picture, .. } => {
            format!("Describes no image: there is no `{picture}`")
        }
        Kind::CacheNotWritten { path, reason } => format!(
            "Cache not written: can't write `{}`: {reason}",
            path.display()
        ),
        Kind::MissingAltText { picture, .. } => {
            format!("Missing alt text: `{picture}`")
        }
        Kind::MissingLinkText { href, .. } => match href {
            Some(to) => {
                format!(
                    "Missing link text: the link to `{to}` has nothing a screen reader can read out"
                )
            }
            None => "Missing link text: a link has nothing a screen reader can read out".into(),
        },
        Kind::MissingButtonText { .. } => {
            "Missing button text: a `<button>` has nothing a screen reader can read out".into()
        }
        Kind::MissingIframeTitle { src } => match src {
            Some(src) => {
                format!("Missing iframe title: the `<iframe>` showing `{src}` has no `title`")
            }
            None => "Missing iframe title: an `<iframe>` has no `title`".into(),
        },
        Kind::MissingLabel { control, name, .. } => match name {
            Some(name) => format!("Missing label: the `<{control}>` named `{name}` has no label"),
            None => {
                let a = if control == "input" { "an" } else { "a" };
                format!("Missing label: {a} `<{control}>` has no label")
            }
        },
        Kind::MissingTitle { empty } => match empty {
            false => "Missing title: the page has no `<title>`".into(),
            true => "Missing title: the page's `<title>` is empty".into(),
        },
        Kind::HeicImage { path } => {
            format!("HEIC image: `{path}` is published as it is, with no resized copies made")
        }
        Kind::LocationPublished { path } => {
            format!("Location published: `{path}` holds where it was taken in its metadata")
        }
        Kind::ImagesOutOfProportion { property } => format!(
            "Images out of proportion: `img` has {} `{property}` and no `{}`",
            if property.to_ascii_lowercase().starts_with('i') {
                "an"
            } else {
                "a"
            },
            height_beside(property)
        ),
        Kind::NoThumbnail { host, absent, .. } => match absent {
            false => "No thumbnail: no image has been fetched for this video yet".into(),
            true => format!("No thumbnail: {} has no image for this video", host.site()),
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
                RecordFault::NotAnAddress(written) => {
                    format!("`{written}` isn't a URL starting with `/`")
                }
            }
        ),
        Kind::TokenInSettings => {
            "Token in settings: `webmention_token` doesn't belong in a file kept with the site"
                .into()
        }
        Kind::WebmentionsOff => "Webmentions off: `webmention()` writes nothing here".into(),
        Kind::RetiredFile { .. } => "Retired file: Statac no longer reads `fetched/refused`".into(),
        Kind::MentionOfNoPage { target } => {
            format!("Mention of no page: nothing is published at `{target}`")
        }
        Kind::RefusedWebmentions { count } => match count {
            1 => "Refused webmentions: 1 in `fetched/mentions/` is left out".into(),
            _ => format!(
                "Refused webmentions: {} in `fetched/mentions/` are left out",
                grouped(count)
            ),
        },
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
                "Can't tell what made the site: `{folder}` has the settings files of more than \
                 one tool"
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
                    characters,
                    more,
                    field,
                    ..
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
                    let font = match field {
                        Some(field) => format!("font in `{field}`"),
                        None => "font".into(),
                    };
                    format!(
                        "Card not drawn: {of} {} {needed}, which no {font} has{starting}",
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
                Some(kind) => format!("Social image may not show: {of} is {}", a_kind(*kind)),
                None => format!(
                    "Social image may not show: {of} is in a format some link previews don't \
                     show"
                ),
            }
        }
    }
}

/// The property that gives an image its height, in the words `property`
/// gives its width in: `block-size` beside `max-inline-size`.
fn height_beside(property: &str) -> &'static str {
    match property.to_ascii_lowercase().ends_with("inline-size") {
        true => "block-size",
        false => "height",
    }
}

/// An image format, as a file is said to be one.
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
        Picture::Avif => "an AVIF image",
        Picture::Heic | Picture::Heif => "a HEIC image",
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
        Sort::Picture => "image",
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

/// A feed in `format`, in words: `an RSS feed`.
pub(crate) fn a_feed(format: FeedFormat) -> &'static str {
    match format {
        FeedFormat::Atom => "an Atom feed",
        FeedFormat::Rss => "an RSS feed",
        FeedFormat::Json => "a JSON feed",
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

/// The verb for lists named by the first and counting `more`.
fn has(more: usize) -> &'static str {
    match more {
        0 => "has",
        _ => "have",
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
        Named::Search => "the site's search index".into(),
        Named::Redirect { page, .. } => format!("a redirect in {}", self::named(page)),
        Named::Permalink { page, .. } => self::named(page),
    }
}

/// What to do about two things at one URL: take the URL out of a
/// `redirect_from` that names it, or else change a `permalink` that gives
/// it, or else rename one of them; to follow a comma, opening with a
/// letter of ASCII.
fn same_address_help(first: &Named, second: &Named) -> String {
    let page = |side: &Named| match side {
        Named::Redirect { page, .. } | Named::Permalink { page, .. } => named(page),
        _ => String::new(),
    };
    match (first, second) {
        (Named::Redirect { .. }, Named::Redirect { .. }) => {
            "take it out of the `redirect_from` of one of them.".into()
        }
        (Named::Redirect { .. }, _) => {
            format!("take it out of `redirect_from` in {}.", page(first))
        }
        (_, Named::Redirect { .. }) => {
            format!("take it out of `redirect_from` in {}.", page(second))
        }
        (Named::Permalink { .. }, Named::Permalink { .. }) => {
            "change the `permalink` of one of them.".into()
        }
        (Named::Permalink { .. }, _) => format!("change the `permalink` of {}.", page(first)),
        (_, Named::Permalink { .. }) => format!("change the `permalink` of {}.", page(second)),
        _ => "rename one of them.".into(),
    }
}

/// A paginated page of a list, by the list's URL.
fn later(number: usize, list: &str) -> String {
    format!("page {number} of `{list}`")
}

/// A list, as the end of a sentence: `the tag `Rust``.
fn the_list(word: Word, name: &str) -> String {
    match (word, name) {
        (Word::Folder, "") => "the site's own list".into(),
        (Word::Folder, path) => format!("the folder `content/{path}/`"),
        (Word::Page, path) => format!("the list on `{path}`"),
        (word, name) => format!("the {} `{name}`", word_name(word)),
    }
}

/// What `--drafts` brought into the run: drafts and scheduled posts, built
/// as if published.
pub(crate) fn included(drafts: usize, waiting: usize) -> String {
    let mut what = Vec::new();
    if drafts > 0 {
        what.push(how_many(drafts, "draft", "drafts"));
    }
    if waiting > 0 {
        what.push(how_many(waiting, "scheduled post", "scheduled posts"));
    }
    format!("Included {} as if published.", listed(&what))
}

/// What `serve` says as it opens of the drafts and scheduled posts that it
/// shows and a build leaves out.
pub(crate) fn showing(drafts: usize, waiting: usize) -> String {
    let mut what = Vec::new();
    if drafts > 0 {
        what.push(how_many(drafts, "draft", "drafts"));
    }
    if waiting > 0 {
        what.push(how_many(waiting, "scheduled post", "scheduled posts"));
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
        FormatFault::English { written, .. } => format!("`{written}` is in English"),
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
    let write = |reference: &str, examples: &[String], looks: bool| {
        let looks = match looks {
            true => {
                let names: Vec<String> = (Look::ALL.iter())
                    .map(|look| look.name().to_owned())
                    .collect();
                format!(", or name a look: {}", quoted_either(&names))
            }
            false => String::new(),
        };
        format!(
            "Write `{reference}` the way dates should look, such as {}{looks}.",
            quoted_either(examples)
        )
    };
    match fault {
        FormatFault::Empty {
            reference,
            examples,
            looks,
        }
        | FormatFault::NoPattern {
            reference,
            examples,
            looks,
            ..
        } => Some(write(reference, examples, *looks)),
        FormatFault::Twice { first, piece, .. } => Some(format!(
            "`{first}` is already the {}, so keep one of them.",
            piece_name(*piece)
        )),
        FormatFault::NotInReference { corrected, .. } | FormatFault::English { corrected, .. } => {
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

fn syntax_fault(fault: &SyntaxFault) -> String {
    match fault {
        SyntaxFault::Unclosed(opener) => format!("`{}` is never closed", opened(opener)),
        SyntaxFault::ClosesNothing { end, .. } => format!("`{{% {end} %}}` closes nothing"),
        SyntaxFault::Astray(tag) => format!("`{{% {tag} %}}` belongs to no `if` or `for`"),
        SyntaxFault::SecondElse(block) => {
            format!("this `{block}` already has an `{{% else %}}`")
        }
        SyntaxFault::ElifAfterElse => "`{% elif %}` comes after `{% else %}`".into(),
        SyntaxFault::UnknownTag { written, .. } => format!("there is no `{{% {written} %}}` tag"),
        SyntaxFault::Foreign(written) => {
            format!("`{written}` isn't part of the template language")
        }
        SyntaxFault::NoFilter => "`|` has no filter after it".into(),
        SyntaxFault::NoField => "`.` has no field after it".into(),
        SyntaxFault::BlockUnnamed => "`block` needs a name".into(),
        SyntaxFault::FilterUnnamed => "`filter` needs a filter's name".into(),
        SyntaxFault::EmptyTag => "`{% %}` is empty".into(),
        SyntaxFault::Escape(written) => format!("unfinished escape `{written}`"),
        SyntaxFault::NotANumber(written) => format!("`{written}` isn't a number"),
        SyntaxFault::TooLarge(written) => format!("`{written}` is too large a number"),
        SyntaxFault::TooDeep => "nested too deeply".into(),
        SyntaxFault::NamedFirst => "a value without a name follows one with a name".into(),
        SyntaxFault::OutsideLoop(tag) => format!("`{{% {tag} %}}` is outside a loop"),
        SyntaxFault::Kept(name) => format!("`{name}` can't be set"),
        SyntaxFault::BlockInMacro(tag) => {
            format!("a `{{% block %}}` can't be inside a `{{% {tag} %}}`")
        }
        SyntaxFault::BlockTwice(name) => format!("there are two blocks called `{name}`"),
        SyntaxFault::RequiredHolds => "a required block can hold only comments".into(),
        SyntaxFault::EndNames { written, block } => {
            format!("`{{% endblock {written} %}}` closes `{{% block {block} %}}`")
        }
        SyntaxFault::NoCall(tag) => format!("`{{% {tag} %}}` needs a call"),
        SyntaxFault::Unexpected {
            found: Some(found),
            awaited,
            ..
        } => format!("unexpected `{found}`, expected {}", awaited_words(awaited)),
        SyntaxFault::Unexpected {
            found: None,
            awaited,
            ..
        } => format!(
            "the template ends too soon, expected {}",
            awaited_words(awaited)
        ),
        SyntaxFault::Engine(detail) => sentence_part(detail),
    }
}

fn syntax_suggestion(fault: &SyntaxFault) -> Option<String> {
    match fault {
        SyntaxFault::Unclosed(opener) => {
            closed_by(opener).map(|close| format!("Close it with `{close}`."))
        }
        SyntaxFault::ClosesNothing {
            open: Some((block, line)),
            ..
        } => Some(format!(
            "The `{{% {block} %}}` on line {line} is still open: close it with \
             `{{% end{block} %}}` first."
        )),
        SyntaxFault::ElifAfterElse => Some("Move it before the `{% else %}`.".into()),
        SyntaxFault::UnknownTag {
            nearest: Some(nearest),
            ..
        } => Some(format!("Did you mean `{{% {nearest} %}}`?")),
        SyntaxFault::Foreign(written) => match written.as_str() {
            "&&" | "&" => Some("Write `and`.".into()),
            "||" => Some("Write `or`.".into()),
            "!" => Some("Write `not`.".into()),
            "?" => Some("Write it as `x if condition else y`.".into()),
            "$" => Some("Take off the `$`.".into()),
            _ => None,
        },
        SyntaxFault::Escape(written) if written.starts_with("\\u") => {
            Some("`\\u` takes four hex digits, as in `\\u00e9`.".into())
        }
        SyntaxFault::Escape(written) if written.starts_with("\\x") => {
            Some("`\\x` takes two hex digits, as in `\\x41`.".into())
        }
        SyntaxFault::NamedFirst => Some("Put the values with names last.".into()),
        SyntaxFault::Kept(_) => {
            Some("The template language keeps this name: choose another.".into())
        }
        SyntaxFault::Unexpected {
            rewrite: Some(rewrite),
            ..
        } => Some(match rewrite {
            Rewrite::Call(call) => format!("Write it as `{call}`."),
            Rewrite::Close(close) => format!("Close it with `{close}`."),
            Rewrite::Compare => "Write `==` to compare.".into(),
            Rewrite::Elif => "Write it as `{% elif %}`.".into(),
            Rewrite::In(names) => format!("Write `in` after `{names}`."),
            Rewrite::Assign(names) => format!("Put `=` after `{names}`."),
            Rewrite::DoubleQuotes(text) => {
                format!("Write the text in double quotes: `\"{text}\"`.")
            }
        }),
        _ => None,
    }
}

/// What opens a thing that is never closed, as a template writes it.
fn opened(opener: &Opener) -> String {
    match opener {
        Opener::Block(keyword) => format!("{{% {keyword} %}}"),
        Opener::Tag => "{%".into(),
        Opener::Output => "{{".into(),
        Opener::Comment => "{#".into(),
        Opener::Quote(quote) => quote.to_string(),
    }
}

fn closed_by(opener: &Opener) -> Option<String> {
    match opener {
        Opener::Block(keyword) => Some(format!("{{% end{keyword} %}}")),
        Opener::Tag => Some("%}".into()),
        Opener::Output => Some("}}".into()),
        Opener::Comment => Some("#}".into()),
        Opener::Quote(_) => None,
    }
}

fn awaited_words(awaited: &Awaited) -> String {
    match awaited {
        Awaited::Written(written) => format!("`{written}`"),
        Awaited::Name => "a name".into(),
        Awaited::NameOrNumber => "a name or a number".into(),
        Awaited::Value => "a value".into(),
        Awaited::TagName => "a tag's name".into(),
    }
}

/// A kind of value, as a sentence names one.
fn shape(shape: Shape) -> &'static str {
    match shape {
        Shape::Text => "text",
        Shape::Number => "a number",
        Shape::TrueFalse => "`true` or `false`",
        Shape::None => "`none`",
        Shape::List => "a list",
        Shape::Map => "a map",
        Shape::Date => "a date",
        Shape::Bytes => "bytes",
        Shape::Other => "this value",
    }
}

fn wanted_words(wanted: Wanted) -> &'static str {
    match wanted {
        Wanted::Whole => "a whole number",
        Wanted::WholeFromZero => "a whole number of 0 or more",
        Wanted::Number => "a number",
        Wanted::TrueFalse => "`true` or `false`",
        Wanted::Character => "one character",
        Wanted::Text => "text",
        Wanted::List => "a list",
        Wanted::Map => "a map",
        Wanted::ListOrText => "a list or text",
        Wanted::Numbers => "a list of numbers",
        Wanted::Lists => "lists",
        Wanted::LookIn => "text, a list or a map to look in",
        Wanted::CountAboveZero => "a count above 0",
        Wanted::FilterName => "a filter's name in quotes",
        Wanted::TestName => "a test's name in quotes",
        Wanted::FieldName => "a field's name in quotes",
        Wanted::NamedValues => "named values",
    }
}

/// The values one of Statac's functions or filters is called with in the
/// documentation, for one called without a value it needs.
fn written_with(callee: &str) -> Option<&'static str> {
    Some(match callee {
        "css" => "\"site.css\"",
        "js" => "\"app.js\"",
        "asset" => "\"fonts/font-1.woff2\"",
        "url" => "\"/about/\"",
        "headline" | "feed_id" => "post",
        "related" => "page",
        "highlight" => "\"rust\"",
        "truncate" => "160, \"…\"",
        "tagged" => "\"Rust\"",
        "in" => "\"blog\"",
        "where" => "\"field\", \"value\"",
        _ => return None,
    })
}

/// Each item in backticks.
fn quoted(items: &[String]) -> Vec<String> {
    items.iter().map(|item| format!("`{item}`")).collect()
}

fn malformed_call(found: &CallFault) -> String {
    match found {
        CallFault::Unclosed(opening) => format!("`{opening}` is never closed"),
        CallFault::UnclosedQuote(quote) => format!("`{quote}` is never closed"),
        CallFault::TextAfterQuote => "text after the closing quote".into(),
        CallFault::Name(name) => format!("a snippet can't be called `{name}`"),
        CallFault::NoEnd(name) => format!("`@{name}` has no `@end`"),
        CallFault::StrayEnd => "`@end` closes nothing".into(),
        CallFault::WrapsNothing(name) => {
            format!("`@{name}` isn't on a line of its own")
        }
        CallFault::NotOneAddress => "`@url` takes one URL starting with `/`".into(),
    }
}

fn malformed_maths(found: &MathsFault) -> String {
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

impl Problem {
    pub fn headline(&self) -> String {
        headline(&self.kind)
    }

    pub fn suggestion(&self) -> Option<String> {
        suggestion(&self.kind)
    }

    pub(crate) fn explanation_link(&self) -> String {
        explanation_link(self.kind.name())
    }

    pub fn related(&self) -> Vec<Related> {
        let met = self.context.iter().cloned();
        self.kind
            .related()
            .into_iter()
            .chain(met.filter(|(relation, _)| !matches!(relation, Relation::Building(_))))
            .map(|(how, place)| Related {
                label: relation(&how),
                place,
            })
            .collect()
    }
}

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&reason(self))
    }
}

/// Why a file or folder couldn't be read, written or removed, to follow a
/// colon.
pub(crate) fn reason(reason: &Reason) -> String {
    match reason {
        Reason::Link => "it is a link".into(),
        Reason::LinkOutside => "it is a link that leads outside the old site".into(),
        Reason::NotAFile => "it is not a file".into(),
        Reason::NotAFolder => "it isn't a folder".into(),
        Reason::NotASite => "it has no `statac.yaml`, so it isn't a site".into(),
        Reason::OutsideBuilt => "it is outside the built site".into(),
        Reason::OutsideNew => "it is outside the new site".into(),
        Reason::InsideOld(old) => {
            format!("it is inside the site being imported, `{}`", old.display())
        }
        Reason::HoldsOld(old) => {
            format!("the site being imported, `{}`, is inside it", old.display())
        }
        Reason::TopOfDisk => "it is the top of the disk".into(),
        Reason::Taken => "something else has its name".into(),
        Reason::Changed => "it changed while Statac was reading the site".into(),
        Reason::NamedTwice => "the same file has another name somewhere".into(),
        Reason::NamesNoFolder => "it names no folder".into(),
        Reason::NoPlaceForId => "its front matter has no place for an `id:` line".into(),
        Reason::Address(fault) => address_fault(fault),
        Reason::NotToml => "this line doesn't read as TOML".into(),
        Reason::NotText => "it isn't text".into(),
        Reason::NotForSass => "Sass couldn't read it".into(),
        Reason::WatchLimit => "the system is watching as many files as it allows".into(),
        Reason::Denied => "permission denied".into(),
        Reason::FolderNamed => "a folder already has that name".into(),
        Reason::FileInTheWay => "a file is in the way of a folder".into(),
        Reason::DiskFull => "the disk is full".into(),
        Reason::Missing => "it doesn't exist".into(),
        Reason::ReadOnly => "the disk is read-only".into(),
        Reason::NotUtf8 => "it isn't UTF-8 text".into(),
        Reason::Said(said) => said.clone(),
    }
}

/// The line `statac search` opens what it found for `words` with: pages
/// holding every word, or where none does, `some` pages holding some.
pub(crate) fn found(words: &str, pages: usize, some: bool) -> String {
    match (pages, some) {
        (0, _) => format!("No page holds `{words}`."),
        (1, false) => format!("1 page holds `{words}`:"),
        (pages, false) => format!("{} pages hold `{words}`, best first:", grouped(pages)),
        (1, true) => format!("No page holds every word of `{words}`; 1 holds some:"),
        (pages, true) => format!(
            "No page holds every word of `{words}`; {} hold some, best first:",
            grouped(pages)
        ),
    }
}

/// The line after `found`'s where no page holds the words, offering
/// `meant`, the words respelt as the site writes them.
pub(crate) fn meant(meant: &str) -> String {
    format!("Did you mean `{meant}`?")
}

/// The line after the pages `statac search` shows, where it found more.
pub(crate) fn more_found(more: usize) -> String {
    let pages = if more == 1 { "page" } else { "pages" };
    format!("And {} more {pages}.", grouped(more))
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
        "--out" => "a folder",
        "--today" => "a date",
        "--url" => "a full URL",
        "--port" => "a number",
        "--watch" => "a way",
        _ => "a value",
    }
}

fn placeholder(option: &str) -> String {
    let needs = needs(option);
    let named = needs.rsplit(' ').next().unwrap_or(needs);
    format!("<{}>", named.to_lowercase())
}

/// `a`, `a and b`, `a, b and c`.
pub(crate) fn listed(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

fn unknown(word: Word, written: &str) -> String {
    format!("Unknown {}: `{written}`", word_name(word))
}

fn word_name(word: Word) -> &'static str {
    match word {
        Word::Field => "field",
        Word::Setting => "setting",
        Word::FieldKind => "kind",
        Word::AddressPiece => "placeholder",
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
        Word::Line => "line",
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
        Expected::Address => "a URL, such as `https://example.org` or `/about/`".into(),
        Expected::SiteAddress => "a full URL, such as `https://example.org`".into(),
        Expected::OneOf(words) => {
            let quoted: Vec<String> = words.iter().map(|w| format!("`{w}`")).collect();
            match quoted.as_slice() {
                [one] => one.clone(),
                [rest @ .., last] => format!("{} or {last}", rest.join(", ")),
                [] => "nothing".into(),
            }
        }
        Expected::LanguageCode => "a language code, such as `en-GB`".into(),
        Expected::TrueFalseOrAddress => "`true`, `false`, a URL or a list of URLs".into(),
        Expected::Fields => "`name: value` lines".into(),
        Expected::ListAddress => "a URL ending in `{slug}/`, such as `/topics/{slug}/`".into(),
        Expected::LaterAddress => {
            "a URL starting with `{list}` and holding `{n}`, such as `{list}page/{n}/`".into()
        }
        Expected::Id => {
            "six letters and digits, such as `k3m9x2`, or a UUID in small letters".into()
        }
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
        Expected::OwnPicture => "an image of the site's".into(),
        Expected::FaceName => "an avatar's file name, such as `0123456789abcdef.jpg`".into(),
        Expected::TableName => "`site.posts` or a table in `data/`, such as `data.people`".into(),
    }
}

/// Why a URL can't be written, as the end of a sentence.
pub(crate) fn address_fault(fault: &AddressFault) -> String {
    match fault {
        AddressFault::UnknownPiece { written, .. } => {
            format!("there is no placeholder `{written}`")
        }
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
            format!("`{pattern}` puts every page at one URL")
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
        }
        | AddressFault::PageLacks {
            page: named,
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

fn sass_fault(fault: &SassFault) -> String {
    match fault {
        SassFault::Raised(said) => format!("`{said}`"),
        SassFault::NoVariable(name) => format!("no variable called `{name}`"),
        SassFault::NoMixin(name) => format!("no mixin called `{name}`"),
        SassFault::NoFunction(name) => format!("no function called `{name}`"),
        SassFault::Missing(c) => format!("a `{c}` is missing"),
        SassFault::ReadsAsDeclaration(_) => "a `{` is missing".into(),
        SassFault::Unclosed(opening) => format!("`{opening}` is never closed"),
        SassFault::ClosesNothing => "`}` closes nothing".into(),
        SassFault::NoValue => "a value is missing".into(),
        SassFault::NoName => "a name is missing".into(),
        SassFault::NoQuotedText => "a text in quotes is missing".into(),
        SassFault::EndsEarly => "the stylesheet ends too soon".into(),
        SassFault::UnknownFlag(flag) => format!("unknown flag `{flag}`"),
        SassFault::Unspaced(operator) => {
            format!("`{operator}` needs a space on each side in a calculation")
        }
        SassFault::TopLevelParent => "`&` has nothing to stand for outside a rule".into(),
        SassFault::NotHere(rule) => format!("`{rule}` can't be used here"),
        SassFault::ReturnOutsideFunction => "`@return` works only inside `@function`".into(),
        SassFault::LoadAfterRules(rule) => format!("`{rule}` must come before any other rule"),
        SassFault::ContentOutsideMixin => "`@content` works only inside `@mixin`".into(),
        SassFault::ExtendOutsideRule => "`@extend` works only inside a rule".into(),
        SassFault::InFunction(what) => format!("`@function` can't hold {what}"),
        SassFault::NoContentBlock(name) => format!("the mixin `{name}` takes no content block"),
        SassFault::NoReturn(name) => format!("`@function {name}` ends without `@return`"),
        SassFault::MissingArgument { callable, argument } => {
            format!("`{callable}` is missing `{argument}`")
        }
        SassFault::TooManyArguments {
            callable,
            takes,
            given,
        } => format!(
            "`{callable}` takes {takes} {}, not {given}",
            if *takes == 1 { "argument" } else { "arguments" }
        ),
        SassFault::NoArgument { callable, argument } => {
            format!("`{callable}` has no argument called `{argument}`")
        }
        SassFault::NoStylesheet { written, .. } => format!("no stylesheet called `{written}`"),
        SassFault::NoBuiltInModule(name) => format!("Sass has no module called `{name}`"),
        SassFault::NoNamespace { name, .. } => format!("no module called `{name}`"),
        SassFault::TwoModules(name) => format!("two modules are called `{name}`"),
        SassFault::Private(name) => format!("`{name}` is private to its module"),
        SassFault::NotDefault(name) => {
            format!("`{name}` isn't set with `!default` in that module")
        }
        SassFault::LoadsItself => "this stylesheet loads itself".into(),
        SassFault::Units(one, other) => format!("can't combine `{one}` and `{other}`"),
        SassFault::Operation(operation) => format!("can't work out `{operation}`"),
        SassFault::NotCss { value, .. } => format!("`{value}` can't be written as CSS"),
        SassFault::WrongType {
            argument,
            wanted,
            value,
        } => {
            let wanted = match wanted {
                SassType::Number => "a number",
                SassType::Colour => "a colour",
                SassType::Map => "a map",
                SassType::String => "a string",
                SassType::Function => "a function",
                SassType::Arguments => "a list of arguments",
                SassType::Calculation => "a calculation",
                SassType::WholeNumber => "a whole number",
            };
            format!("`{argument}` needs {wanted}, not `{value}`")
        }
        SassFault::HasUnits { argument, value } => {
            format!("`{argument}` needs a number without units, not `{value}`")
        }
        SassFault::OutOfRange {
            argument,
            index,
            items,
        } => format!(
            "`{argument}` is {index}, but the list has {items} {}",
            if *items == 1 { "item" } else { "items" }
        ),
        SassFault::RepeatedKey(key) => format!("the map has `{key}` twice"),
        SassFault::ForUnits { from, to } => format!("`@for` can't count from `{from}` to `{to}`"),
        SassFault::CompoundExtend(_) => "`@extend` can't take a compound selector".into(),
        SassFault::IndentedSemicolon => "a `;` can't be used in a `.sass` file".into(),
        SassFault::MixedIndent { tab: true } => "a tab where this file indents with spaces".into(),
        SassFault::MixedIndent { tab: false } => "a space where this file indents with tabs".into(),
        SassFault::FirstLineIndented => "the first line is indented".into(),
        SassFault::Other(said) => sentence_part(said),
    }
}

fn sass_suggestion(fault: &SassFault) -> Option<String> {
    match fault {
        SassFault::ReadsAsDeclaration(declaration) => Some(format!("Write it as `{declaration}`.")),
        SassFault::UnknownFlag(_) => Some("Did you mean `!important`?".into()),
        SassFault::NoStylesheet {
            nearest: Some(nearest),
            ..
        } => Some(format!("Did you mean `{nearest}`?")),
        SassFault::NoNamespace {
            name,
            built_in: true,
        } => Some(format!(
            "Add `@use \"sass:{name}\";` at the top of the file."
        )),
        SassFault::Units(..) => Some("Mix units with `calc()`.".into()),
        SassFault::CompoundExtend(Some(rule)) => Some(format!("Write it as `{rule}`.")),
        _ => None,
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

/// `written` in double quotes, so YAML keeps it as written.
pub(crate) fn kept_quoted(written: &str) -> String {
    format!("\"{}\"", written.replace('\\', "\\\\").replace('"', "\\\""))
}

/// What a fix that writes `text` in place of what the problem is placed at
/// is offered as.
pub(crate) fn fix_written(text: &str) -> String {
    format!("Write it as `{text}`")
}

/// The fix taking a URL gone out of `published.txt`.
pub(crate) fn fix_forget(written: &str) -> String {
    format!("Take `{written}` out of `published.txt`")
}

/// The fix taking every URL gone under a folder out of
/// `published.txt`.
pub(crate) fn fix_forget_all(count: usize, under: &str) -> String {
    let urls = how_many(count, "URL", "URLs");
    format!("Take the {urls} under `{under}` out of `published.txt`")
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
        Kind::RetiredSetting { in_place, .. } => Some(match in_place {
            InPlace::Nothing => {
                "Delete the line: Statac colours no code unless `code_colours: true` asks.".into()
            }
            InPlace::CodeColours { theme } => format!(
                "Write `code_colours: true` in its place, and copy {} rules from {CODE_COLOURS} \
                 into a stylesheet of the site's own.",
                theme.as_ref().map_or_else(
                    || "a theme's".to_owned(),
                    |theme| format!("the `{theme}` theme's")
                )
            ),
        }),
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
            JsonFault::UnclosedBracket(open) => {
                Some(format!("Close it with `{}`.", closing(*open)))
            }
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
        Kind::UnknownProblemName {
            suggestion: Some(ours),
            former: Former::Renamed,
            ..
        } => Some(format!("Statac calls this `{ours}`.")),
        Kind::UnknownProblemName {
            former: Former::Retired,
            ..
        } => Some("This kind of problem no longer exists.".into()),
        Kind::UnknownProblemName { suggestion, .. } => {
            suggestion.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        // An argument the command has no use for has no right spelling to offer.
        Kind::UnexpectedArgument { .. } => None,
        Kind::CantQuieten { .. } => Some("Take it out of `quiet:`.".into()),
        Kind::UnknownCommand { hint, .. } => Some(match hint {
            CommandHint::Similar(s) => format!("Did you mean `statac {s}`?"),
            CommandHint::Setting(setting) => {
                format!("Statac does this with `{setting}:` in `statac.yaml`.")
            }
            CommandHint::None => "Run `statac --help` to see the commands.".into(),
        }),
        Kind::MissingCommand => Some("Run `statac --help` to see the commands.".into()),
        Kind::UnknownOption { written, hint } => Some(match hint {
            OptionHint::Similar(s) => format!("Did you mean `{s}`?"),
            OptionHint::ForOther { option, takers, .. } => {
                let takers: Vec<_> = takers.iter().map(|c| format!("`statac {c}`")).collect();
                format!("`{option}` is for {}.", listed(&takers))
            }
            OptionHint::ShownAlready => "`statac serve` shows drafts and scheduled posts already. \
                                         `--no-drafts` leaves them out."
                .into(),
            OptionHint::Takes { command, options } => {
                let options: Vec<_> = options.iter().map(|o| format!("`{o}`")).collect();
                match options.len() {
                    0 => format!("`statac {command}` takes no options."),
                    _ => format!("`statac {command}` takes {}.", listed(&options)),
                }
            }
            OptionHint::None => "Run `statac --help` to see the options.".into(),
            OptionHint::Retired => match option_name(written) {
                "--no-starter" => "Statac no longer writes templates into a site, so leave \
                                   `--no-starter` out."
                    .into(),
                _ => "Statac no longer writes templates into a site: see \
                      https://statac.dev/templates/."
                    .into(),
            },
            OptionHint::FolderAfterCommand { command, folder } => {
                let folder = folder.as_deref().unwrap_or("<folder>");
                match command.as_deref() {
                    Some("search") => format!(
                        "Name the folder after the words, as `statac search <words> {folder}`."
                    ),
                    command => format!(
                        "Name the folder after the command, as `statac {} {folder}`.",
                        command.unwrap_or("<command>")
                    ),
                }
            }
        }),
        Kind::MissingValue { option, value, .. } => Some(match value {
            Some(value) => format!("Write it as `{option}={value}`."),
            None => format!("Write it as `{option}={}`.", placeholder(option)),
        }),
        Kind::OptionGivenTwice { .. } => Some("Keep one of them.".into()),
        Kind::MissingName => Some("The name is the last part of a problem's `More:` link.".into()),
        Kind::NothingToSearch { site: None } => {
            Some("Give them after the command, as `statac search \"raspberry pi\"`.".into())
        }
        Kind::NothingToSearch { site: Some(site) } => Some(format!(
            "Give the words first, then the folder, as `statac search \"raspberry pi\" {site}`."
        )),
        Kind::UnreadableName { .. } => None,
        Kind::UnusableName { .. } => Some("Name the folder without it.".into()),
        Kind::MissingFile { hint, .. } => hint.as_ref().map(|hint| match hint {
            Hint::Closest(name) => format!("Did you mean `{name}`?"),
            Hint::InStatic(address) => {
                format!("Link it as `{address}`, where `static/` publishes it under its own name.")
            }
            Hint::NoLongerWritten => format!(
                "Statac no longer writes `code.css`. Coloured code needs `code_colours: true` and \
                 a theme's rules from {CODE_COLOURS} in a stylesheet of the site's own."
            ),
        }),
        Kind::WorkerOutsideImportMap { .. } => Some(
            "Keep the worker and the files it imports in `static/`, which publishes them under \
             their own names."
                .into(),
        ),
        Kind::SameUrl {
            first,
            second,
            alike,
            ..
        } => {
            let help = same_address_help(first, second);
            Some(match alike {
                Alike::Exactly => {
                    let mut help = help;
                    help[..1].make_ascii_uppercase();
                    help
                }
                Alike::ByCase | Alike::ByNormalisation => {
                    format!("Most disks hold them as one file, so {help}")
                }
            })
        }
        Kind::LaterPageTaken { page, .. } => Some(format!(
            "Give {} another URL, or put the paginated pages elsewhere with \
             `pagination.permalink` in `statac.yaml`.",
            named(page)
        )),
        Kind::UrlTaken { held, taken, .. } => {
            let (what, when) = match held {
                Held::Draft => ("draft", "before it is published"),
                Held::Waiting => ("post", "before its scheduled date"),
            };
            Some(match &**taken {
                Named::Redirect { page, .. } => format!(
                    "Take it out of `redirect_from` in {}, or give this {what} a `permalink`, \
                     {when}.",
                    named(page)
                ),
                _ => format!("Give one of them another `permalink` {when}."),
            })
        }
        Kind::AddressesGone { .. } => Some(
            "Redirect them with `redirect_from` in the pages that took their places, or take \
             them out of `published.txt` if they are gone for good."
                .into(),
        ),
        Kind::UrlGone { written, .. } => Some(format!(
            "Redirect it with `redirect_from: [{written}]` in the page that took its place, or \
             take it out of `published.txt` if it is gone for good."
        )),
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
        Kind::SassStopped { fault } => sass_suggestion(fault),
        Kind::SassWarning { .. }
        | Kind::UnwritableName { .. }
        | Kind::CantRead { .. }
        | Kind::CantRemove { .. } => None,
        Kind::CantListen { .. } => Some("Choose another port with `--port=<number>`.".into()),
        Kind::MalformedCall { found } => Some(match found {
            CallFault::Unclosed(_) => "Close it with `)`.".into(),
            CallFault::UnclosedQuote(quote) => {
                format!("End the value with another `{quote}` on the same line.")
            }
            CallFault::TextAfterQuote => "Put the whole value inside the quotes.".into(),
            CallFault::Name(_) => {
                "Name the snippet with the letters a to z, digits and `_`, or write `\\@` to show \
                 this as text."
                    .into()
            }
            CallFault::NoEnd(_) => {
                "Put `@end` on a line of its own where the content it wraps ends.".into()
            }
            CallFault::StrayEnd => {
                "Take the line out: only a call to a snippet that shows `body` wraps the content \
                 up to an `@end`."
                    .into()
            }
            CallFault::WrapsNothing(_) => {
                "Write the call on a line of its own, the content it wraps below it, and `@end` \
                 on a line after that."
                    .into()
            }
            CallFault::NotOneAddress => "Write it as `@url(/about/)`.".into(),
        }),
        Kind::MalformedMaths { found } => match found {
            MathsFault::Unclosed {
                opening: Some(_),
                closing,
            } => Some(format!("Close it with `{closing}`.")),
            MathsFault::ClosesNothing(written) => Some(format!(
                "Take it out, or open it with `{}` where it starts.",
                opener_of(written)
            )),
            MathsFault::Mismatched { expected, .. } => Some(format!("Did you mean `{expected}`?")),
            MathsFault::MissingBefore(_) => {
                Some(r"Give the command before it everything it takes, as in `\frac{1}{2}`.".into())
            }
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
        },
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
        Kind::LoopInForIf { .. } => {
            Some("`loop` is set only inside the loop: test it with an `{% if %}` there.".into())
        }
        Kind::UnknownWord { nearest, .. } => {
            nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::AlwaysFalse { meant, .. } => Some(format!("Did you mean `{meant}`?")),
        Kind::ReadAsCustom {
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
                Some("Write `permalink:` to choose the page's URL.".into())
            }
            SetAside::StatacFills(_) => {
                Some(format!("Give it a name of your own, such as `my_{field}`."))
            }
            SetAside::SendsOn => {
                Some("Take it out, or take out `redirect_to` to publish the page.".into())
            }
            SetAside::NotOnCard {
                nearest: Some(name),
            } => Some(format!("Did you mean `{name}`?")),
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
            "Write it as `{}` to keep it as written.",
            kept_quoted(written)
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
                format!(
                    "Add a `{field}:` pattern, such as `/people/{{key}}/`, to give each item \
                     its URL."
                )
            }
            Requirer::EachPost => format!(
                "Add a `{field}:` pattern, such as `/cards/{{name}}/`, to give the page made for \
                 each post its URL."
            ),
            Requirer::Card => format!("Add `{field}:`, which every card needs."),
            Requirer::CardText => format!("Add `{field}:`, which each text on a card needs."),
            Requirer::Mention => {
                format!("Add `{field}:`, which every webmention Statac keeps has.")
            }
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
            WrongPlace::LanguageFile => {
                Some("Set how dates look with `date_format:` in `statac.yaml` instead.".into())
            }
            WrongPlace::NoCommand(command) => Some(format!(
                "Add the `{command}:` line it describes, or take it out."
            )),
            WrongPlace::More(MoreAt::Element(name)) => {
                Some(format!("Move it outside the `<{name}>`."))
            }
            WrongPlace::More(_) => {
                Some("Move it to a line of its own between two paragraphs.".into())
            }
        },
        Kind::UnusableUrl { fault } => match fault {
            AddressFault::UnknownPiece { nearest, .. } => {
                nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
            }
            AddressFault::Unclosed => Some("Close it with `}`.".into()),
            AddressFault::Character('\\') => Some("Separate folders with `/`.".into()),
            AddressFault::Character(_) | AddressFault::Control | AddressFault::Stray => {
                Some("Leave it out of the URL.".into())
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
                Some("Add a placeholder such as `{slug}` so each page has its own.".into())
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
            AddressFault::PageLacks { field, piece, .. } => Some(format!(
                "Add `{field}:` to the page, or take `{piece}` out of the pattern."
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
        Kind::LanguageFolderInPattern { without, .. } => Some(format!("Write it as `{without}`.")),
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
        Kind::MalformedTemplate { found } => match found {
            TemplateFault::Syntax(fault) => syntax_suggestion(fault),
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
                TakesFault::TooMany => "Have calls name the rest, as `caption=...`.".into(),
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
            (None, Chooser::Made) => {
                Some("Add the template, or a page of your own at the same URL.".into())
            }
            (None, Chooser::Written) => None,
        },
        Kind::BuiltWithPageTemplate { path, pages } => Some(format!(
            "Add `{path}` to give {} a template of {} own.",
            if *pages == 1 { "it" } else { "them" },
            if *pages == 1 { "its" } else { "their" },
        )),
        Kind::BuiltWithoutTemplate { redirects, count } => Some(format!(
            "Add `templates/{}.html` to build {} with a template.",
            if *redirects { "redirect" } else { "page" },
            if *count == 1 { "it" } else { "them" },
        )),
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
            ArgumentFault::NotGiven(name) => Some(format!("Write it as `{snippet}({name}=...)`.")),
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
                     `{snippet}()` for every JPEG, PNG or WebP image beside the page."
                )),
                Needed::Brackets => Some(format!("Write it as `{snippet}()`.")),
                Needed::Text(option) if option == "sizes" => Some(
                    "Write it as a browser reads it: `sizes=\"(max-width: 40rem) 100vw, 40rem\"`."
                        .into(),
                ),
                Needed::Text(_) | Needed::TrueFalse(_) | Needed::OnSite(_) => None,
                Needed::VideoOn { site, .. } => Some(format!(
                    "Write the URL {site} gives for the video, such as `{}`.",
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
            Stopped::NoSiteUrl => Some("Add `url:` to `statac.yaml`.".into()),
            Stopped::NoFeedKey(key) => Some(feed_key_line(key)),
            Stopped::NotAPage(function) => Some(format!("Write it as `{function}(post)`.")),
            Stopped::NotAnAddress => Some("Write it as `url(\"/about/\")`.".into()),
            Stopped::NotAList(filter) => Some(format!("Write it as `site.posts | {filter}`.")),
            Stopped::CallsItself(_) => Some("Take the call out of one of them.".into()),
            Stopped::RanOn { .. } => Some(
                "Look for a loop that never ends, or a loop inside a loop over long lists.".into(),
            ),
            Stopped::Operator { op, left, right }
                if op == "+" && (*left == Shape::Text || *right == Shape::Text) =>
            {
                Some("Join text with `~`.".into())
            }
            Stopped::NotCallable(_) => Some("Take off the `()`.".into()),
            Stopped::NoMethod {
                filter: Some(filter),
                ..
            } => Some(format!("Write it as a filter: `{filter}`.")),
            Stopped::MissingArgument { callee, input } => {
                let values = written_with(callee)?;
                Some(match input {
                    Some(input) => format!("Write it as `{input} | {callee}({values})`."),
                    None => format!("Write it as `{callee}({values})`."),
                })
            }
            Stopped::NoSuchArgument {
                nearest: Some(nearest),
                ..
            } => Some(format!("Did you mean `{nearest}`?")),
            Stopped::NotANamespace(name) => {
                Some(format!("Make it with `{{% set {name} = namespace() %}}`."))
            }
            Stopped::Engine(_)
            | Stopped::NotADate(_)
            | Stopped::ContentInItself
            | Stopped::CardInCard
            | Stopped::Operator { .. }
            | Stopped::DividedByZero
            | Stopped::TooLarge(_)
            | Stopped::RepeatNotWhole
            | Stopped::RepeatedTooLong
            | Stopped::CantWorkOut(_)
            | Stopped::Needs { .. }
            | Stopped::CantSlice(_)
            | Stopped::SliceStep
            | Stopped::NoMethod { .. }
            | Stopped::TooManyArguments(_)
            | Stopped::NoSuchArgument { .. }
            | Stopped::GivenTwice { .. }
            | Stopped::NoLength(_)
            | Stopped::CantLoop(_)
            | Stopped::CantList { .. }
            | Stopped::NotANumber(_)
            | Stopped::EmptyText(_)
            | Stopped::CantNumber { .. }
            | Stopped::FormatNeeds { .. }
            | Stopped::FormatCharacter(_)
            | Stopped::FormatWhole(_)
            | Stopped::FormatHasNo(_)
            | Stopped::FormatTooFew
            | Stopped::FormatNamed(_)
            | Stopped::RangeTooLong(_)
            | Stopped::RangeStep
            | Stopped::LoopNotRecursive
            | Stopped::LoopTakesOne
            | Stopped::Unpack(_)
            | Stopped::UnpackCount { .. }
            | Stopped::ExtendsTwice
            | Stopped::ExtendsName
            | Stopped::ExtendsItself(_)
            | Stopped::ExtendsAgain(_)
            | Stopped::SuperOutside
            | Stopped::CallsTooDeep(_) => None,
            Stopped::CardInWriting => Some("Read `page.card` in a layout.".into()),
            Stopped::NotText => Some("Show something else here: a page can only show text.".into()),
        },
        Kind::PageInSnippet { handed, .. } => Some(format!("Did you mean `{handed}`?")),
        Kind::TextNeverShown { nearest, .. } => {
            nearest.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::ContentNeverShown { .. } => {
            Some("Add `{{ page.content }}` where the content belongs.".into())
        }
        Kind::SpelledTwoWays { .. } => {
            Some("Use one spelling on every page to choose how it's shown.".into())
        }
        Kind::DraftLeftOut { .. } | Kind::ScheduledPost { .. } => {
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
            "A list page is in no list; give it a `permalink` of its own to list it as a post."
                .into(),
        ),
        Kind::FeedNotWritten { more, why, .. } => Some(match why {
            Unwritten::NoTitle => "Add `title:` to `statac.yaml`; a feed is named by it.".into(),
            Unwritten::NoTemplate(_) => {
                let (them, lines) = match more {
                    0 => ("it", "line"),
                    _ => ("them", "lines"),
                };
                format!(
                    "Add the template to publish {them}, or take out the `feed:` {lines} \
                     asking for {them}."
                )
            }
            Unwritten::PageThere { .. } => {
                "The page is published there instead; give the feed another URL with \
                 `feed:` on the list page."
                    .into()
            }
            Unwritten::Undated => {
                "Give a page in it a `date:`, or take the `feed:` line out.".into()
            }
            Unwritten::NoPage => format!(
                "A feed links to its list's page; add an `index.md` to give {} one, or take the \
                 `feed:` line out.",
                if *more == 0 { "the list" } else { "each list" }
            ),
            Unwritten::PageUnpublished => format!(
                "A feed links to its list's page; take out `index_page: false` to publish {}, or \
                 take the `feed:` line out.",
                if *more == 0 { "it" } else { "them" }
            ),
        }),
        Kind::FeedWrittenTwice { .. } => {
            Some("The feed is published once; take one of them out of the `feed:` line.".into())
        }
        Kind::FeedFormatMismatch {
            template,
            own,
            named,
            ..
        } => Some(match template == own {
            true => format!(
                "Write `{own}` as {}, or give the feed a name that doesn't say it is one.",
                a_feed(*named)
            ),
            false => format!(
                "It is built with `{template}`; add `{own}` to build it as {}.",
                a_feed(*named)
            ),
        }),
        Kind::NoFeedKey { key } => Some(feed_key_line(key)),
        Kind::UnusedFeedTemplate { template, line } => Some(format!(
            "Write `{line}` in `statac.yaml` for the site's feed, built with `{template}`."
        )),
        Kind::UnusedLanguageFile { languages, .. } => Some(match languages.as_slice() {
            [] => "The site names no `language`; write one in `statac.yaml` for the file to be \
                   read."
                .into(),
            [one] => format!(
                "Name it for the site's language, as `languages/{one}.yaml`, or take it out."
            ),
            _ => format!(
                "Name it for one of the site's languages, {}, or take it out.",
                quoted_either(languages)
            ),
        }),
        Kind::NoSiteUrl { .. } => {
            Some("Add the site's full URL to `statac.yaml`, as `url: https://example.org/`.".into())
        }
        Kind::FeedIdChanged {
            id, item: false, ..
        } => Some(format!(
            "Add `id: {id}` to its front matter to keep it the same post in feed readers."
        )),
        Kind::FeedIdChanged { id, item: true, .. } => Some(format!(
            "Add `id: {id}` to the item to keep it the same post in feed readers."
        )),
        Kind::NoLanguage => {
            Some("Add the site's language to `statac.yaml`, as `language: en`.".into())
        }
        Kind::NoListPage {
            list: (word, name),
            more,
        } => Some(match (word, more) {
            (Word::Folder, 0) => format!("Add `content/{name}/index.md` to publish one."),
            (Word::Folder, _) => "Add an `index.md` to a folder to publish its list page.".into(),
            (Word::Series, _) => {
                "Add `series.pages: true` to `statac.yaml` to publish a page for each series."
                    .into()
            }
            _ => "Add `tags.pages: true` to `statac.yaml` to publish a page for each tag.".into(),
        }),
        Kind::NoHomePage { addresses, index } => Some(match addresses.len() {
            1 => format!("Add `{index}`, or `templates/home.html` to list the posts there."),
            _ => "Add an `index.md` at the top of each language, or `templates/home.html` to \
                  list the posts there."
                .into(),
        }),
        Kind::RobotsBeneathPath { .. } => Some(
            "Copy its lines into the `robots.txt` at the root of the host to have them read."
                .into(),
        ),
        Kind::RedirectToItself { .. } => {
            Some("Point `redirect_to` at the URL readers should reach.".into())
        }
        Kind::RedirectFromItself { .. } => Some("Take it out of `redirect_from`.".into()),
        Kind::RedirectedBack { .. } => None,
        Kind::ContentNotPublished { .. } => Some(
            "Move it to the page it redirects to, or take out `redirect_to` to publish it.".into(),
        ),
        Kind::SortedLast { instead, .. } => instead
            .as_ref()
            .map(|written| format!("Write `{written}` to leave out the items without it.")),
        Kind::LoopOverEveryPage { instead, .. } => instead
            .as_ref()
            .map(|written| format!("Write `{written}` to choose the pages once for the site.")),
        Kind::CallInHtml { name } => Some(format!("To show it as text, write `\\@{name}`.")),
        Kind::UntitledPage { page, .. } => {
            let words = match page {
                Named::ListPage(..) => {
                    "Add `title:` to `statac.yaml`; the home page is called by it."
                }
                _ => {
                    "Give it a `title`, or open its content with a `# heading`. A folder whose \
                     pages go without titles says `pages.untitled: true`."
                }
            };
            Some(words.into())
        }
        Kind::MissingDefinition { nearest, .. } => nearest
            .as_ref()
            .map(|label| format!("Did you mean `{label}`?")),
        Kind::MissingAnchor { suggestion, .. } => {
            suggestion.as_ref().map(|s| format!("Did you mean `{s}`?"))
        }
        Kind::FootnoteNeverShown { nearest, .. } => nearest
            .as_ref()
            .map(|marker| format!("Is it the definition for `[^{marker}]`?")),
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
        Kind::IdUsedTwice { on, .. } => Some(
            match on {
                Braced::Heading => "Give this heading an id of its own.",
                Braced::Table => "Give this table an id of its own.",
            }
            .into(),
        ),
        Kind::UnmatchedTag { unmatched } => Some(match unmatched {
            Unmatched::NeverClosed(name) => {
                format!("Close it with `</{name}>` where it should end.")
            }
            Unmatched::CommentNeverClosed => "Close it with `-->`.".into(),
            Unmatched::ClosesNothing(_) => "Take it out, or open the element above it.".into(),
        }),
        Kind::SizeKept { field, size, .. } => Some(format!(
            "Write `{field}: {size}`; only a text at `left`, `centre` or `right` is fitted."
        )),
        Kind::UnreadableImage { path, fault } => match fault {
            PictureFault::CutShort => Some("Copy or export the file again.".into()),
            PictureFault::NotAStill => {
                Some("Delete it, then run `statac refresh` to fetch one.".into())
            }
            _ if path.starts_with("content/") => Some(format!(
                "To publish it as the file it is, write `optimise: false` in `{path}.yaml`."
            )),
            _ => None,
        },
        Kind::DescribesNoImage { nearest, .. } => {
            nearest.as_ref().map(|n| format!("Did you mean `{n}`?"))
        }
        Kind::ColourConverted { .. } | Kind::CacheNotWritten { .. } => None,
        Kind::MissingAltText {
            shown,
            all_of: Some(all_of),
            ..
        } => {
            let (saying, what) = match all_of {
                AllOf::Link(Some(to)) => (format!("where the link to `{to}` leads"), "link"),
                AllOf::Link(None) => ("where the link leads".to_owned(), "link"),
                AllOf::Button => ("what the button does".to_owned(), "button"),
            };
            let all = format!("as the image is all the {what} shows");
            Some(match shown {
                ShownBy::Markdown { .. } => format!("Say in the brackets {saying}, {all}."),
                ShownBy::Html => format!("Give the `<img>` an `alt` saying {saying}, {all}."),
                ShownBy::Template { .. } => format!("Give it alt text saying {saying}, {all}."),
            })
        }
        Kind::MissingAltText { shown, .. } => Some(match shown {
            ShownBy::Markdown {
                described_beside: true,
            } => "Describe it in the brackets, or mark it decorative.".into(),
            ShownBy::Markdown {
                described_beside: false,
            } => "Describe it in the brackets.".into(),
            ShownBy::Html => {
                "Give the `<img>` an `alt`, or `alt=\"\"` where it is decorative.".into()
            }
            ShownBy::Template {
                beside: Some(beside),
            } => format!("Describe it with `alt:` in `{beside}`, or mark it decorative."),
            ShownBy::Template { beside: None } => {
                "Describe it where it is shown, or mark it decorative.".into()
            }
        }),
        Kind::MissingLinkText { image, .. } => Some(match image {
            true => "Give the `<img>` inside it an `alt` saying where the link leads.".into(),
            false => "Put words inside it, or give it an `aria-label`.".into(),
        }),
        Kind::MissingButtonText { image } => Some(match image {
            true => "Give the `<img>` inside it an `alt` saying what the button does.".into(),
            false => "Put words inside it, or give it an `aria-label`.".into(),
        }),
        Kind::MissingIframeTitle { .. } => Some("Give it a `title` saying what it shows.".into()),
        Kind::MissingLabel { id, by, .. } => Some(match (by, id) {
            (LabelledBy::Label, Some(id)) => {
                format!("Add `<label for=\"{id}\">` saying what it is for.")
            }
            (LabelledBy::Label, None) => {
                "Put it inside a `<label>` saying what it is for, or give it an `aria-label`."
                    .into()
            }
            (LabelledBy::Value, _) => "Give it a `value`, the words shown on the button.".into(),
            (LabelledBy::Alt, _) => "Give it an `alt` saying what the button does.".into(),
        }),
        Kind::MissingTitle { empty } => Some(match empty {
            false => {
                "Add a `<title>` naming the page, such as `<title>{{ page.title }}</title>`.".into()
            }
            true => "Write the page's name inside it.".into(),
        }),
        Kind::HeicImage { .. } => {
            Some("Export it as a JPEG: most browsers can't show a HEIC image.".into())
        }
        Kind::LocationPublished { .. } => Some(
            "Publish a copy without it, or add `location-published` to `quiet:` in \
             `statac.yaml`."
                .into(),
        ),
        Kind::ImagesOutOfProportion { property } => Some(format!(
            "Add `{}: auto` to the rule.",
            height_beside(property)
        )),
        Kind::NoThumbnail { absent, .. } => Some(
            match absent {
                false => "Run `statac refresh` to fetch one.",
                true => "Show an image of your own in its place.",
            }
            .into(),
        ),
        Kind::MalformedRecord { fault } => match fault {
            RecordFault::Opening => Some("Write `statac fetched 1` as its first line.".into()),
            RecordFault::Fields(_) => Some(
                "Write the file's path, its hash or `absent`, and the day, parted by tabs.".into(),
            ),
            RecordFault::Path(_) => None,
            RecordFault::Held(_) => {
                Some("Write the first 16 hex digits of the file's BLAKE3 hash, or `absent`.".into())
            }
            RecordFault::Day(_) => Some("Write the day as `2026-10-06`.".into()),
            RecordFault::Twice(_) => Some("Keep one of them.".into()),
            RecordFault::NotAnAddress(_) => Some(
                "Write it as a URL starting with `/`, such as `/about/`, or take the line out."
                    .into(),
            ),
        },
        Kind::TokenInSettings => Some(
            "Remove the line, add `webmentions: true`, and put the token in \
             `STATAC_WEBMENTION_TOKEN` or in the site's `.statac/token`."
                .into(),
        ),
        Kind::WebmentionsOff => {
            Some("Add `webmentions: true` to `statac.yaml`, or remove the call.".into())
        }
        Kind::RetiredFile { urls, unread } => Some(match (urls.is_empty(), unread) {
            (false, Unread::Nothing) => "Delete `fetched/refused` after adding them.".into(),
            (false, Unread::Lines(1)) => "Delete `fetched/refused` after adding them, and the URL \
                                          on the line that couldn't be read."
                .into(),
            (false, Unread::Lines(lines)) => format!(
                "Delete `fetched/refused` after adding them, and the URLs on the {} lines that \
                 couldn't be read.",
                grouped(lines)
            ),
            (true, Unread::Nothing) => "Delete it: it lists no URLs.".into(),
            (_, Unread::Lines(_) | Unread::File(_)) => {
                "Copy its URLs to `refuse_webmentions_from:` in `statac.yaml`, then delete it."
                    .into()
            }
        }),
        Kind::MentionOfNoPage { .. } => {
            Some("If the page moved, add the old URL to its `redirect_from`.".into())
        }
        Kind::RefusedWebmentions { count } => Some(match count {
            1 => "Run `statac refresh` to delete its file.".into(),
            _ => "Run `statac refresh` to delete their files.".into(),
        }),
        Kind::FolderNotEmpty { by, .. } => Some(match by {
            Maker::New => "Give `statac new` an empty folder, or the name of a new one.".into(),
            Maker::Import => {
                "Give `statac import` an empty folder, or the name of a new one.".into()
            }
        }),
        Kind::AlreadyASite { .. } => None,
        Kind::NoSiteHere { .. } => {
            Some("Run it in a site's folder, or make one with `statac new`.".into())
        }
        Kind::CantTellWhatMade { tools, .. } => {
            let options: Vec<String> = tools
                .iter()
                .map(|tool| format!("`--from={tool}`"))
                .collect();
            let named = match options.split_last() {
                Some((last, [])) => last.clone(),
                Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
                None => "`--from`".into(),
            };
            Some(format!("Say what made it with {named}."))
        }
        Kind::MissingFolder => Some("Name it after the command: `statac import <folder>`.".into()),
        Kind::UnusableCardFile { fault, .. } => Some(
            match fault {
                CardFileFault::BackgroundSize { .. } => {
                    "Save the background 1,200 pixels wide and 630 tall."
                }
                CardFileFault::NotAStill => "Save the background as a PNG or a JPEG.",
                CardFileFault::Font => "Name a TrueType or OpenType font file.",
            }
            .into(),
        ),
        Kind::CardNotDrawn { why, .. } => Some(match why {
            NotDrawn::NoFont {
                characters,
                more,
                font,
                field,
            } => {
                let them = if characters.len() + more == 1 {
                    "it"
                } else {
                    "them"
                };
                let list = match field {
                    Some(field) => format!("`{field}`"),
                    None => "the card's `font`".into(),
                };
                match font.as_deref() {
                    Some(font) => format!("Add `{font}`, which has {them}, to {list}."),
                    None => format!("Add a font that has {them} to {list}."),
                }
            }
            NotDrawn::TooTall { .. } => {
                "Give the page fewer words there, or the middle of the card more room.".into()
            }
            NotDrawn::NoRoom => "Make the texts at the top and bottom smaller or shorter.".into(),
        }),
        Kind::UnusedCard { name: None } => Some(
            "Write the card's `<meta>` lines in a layout, or remove the card's settings and \
                 `templates/og/`."
                .into(),
        ),
        Kind::UnusedCard { name: Some(name) } => Some(format!(
            "Write `card: {name}` on the pages it is for, or remove the card's settings."
        )),
        Kind::ImageMayNotShow { .. } => {
            Some("For an image more link previews show, save it as a PNG.".into())
        }
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
        Relation::AlsoPublished => "Also published there".into(),
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
            format!("An embed with no URL, which may run its own, {on}")
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

/// What `build --timings` heads the stages of a build with.
pub(crate) const TIMINGS_STAGES: &str = "Where the time went:";

/// The stages of a build, as `build --timings` names them.
pub(crate) const STAGE_BEFORE: &str = "Running before_build";
pub(crate) const STAGE_READING: &str = "Reading the site";
pub(crate) const STAGE_BUILDING: &str = "Building the pages";
pub(crate) const STAGE_CONCLUDING: &str = "Concluding";
pub(crate) const STAGE_COPIES: &str = "Resizing images";
pub(crate) const STAGE_CARDS: &str = "Drawing social cards";
pub(crate) const STAGE_AFTER: &str = "Running after_build";
pub(crate) const STAGE_WRITING: &str = "Writing the rest";

/// What `build --timings` heads the work of building the pages with.
pub(crate) fn timings_work(took: &str, threads: usize) -> String {
    let threads = how_many(threads, "thread", "threads");
    format!("Building the pages was {took} of work across {threads}:")
}

/// The parts of building the pages, as `build --timings` names them.
pub(crate) const WORK_SAVING: &str = "Saving files";
pub(crate) const WORK_WRITING: &str = "Markdown";
pub(crate) const WORK_TEMPLATES: &str = "Templates";
pub(crate) const WORK_LINKS: &str = "Checking links";
pub(crate) const WORK_REST: &str = "The rest";

/// What `build --timings` heads the slowest of each kind with.
pub(crate) const SLOWEST_TEMPLATES: &str = "Slowest templates, by time in the template itself:";
pub(crate) const SLOWEST_WRITING: &str = "Slowest Markdown:";
pub(crate) const SLOWEST_PHOTOGRAPHS: &str =
    "Slowest images (resized copies made this build only):";

/// What `build --timings` heads the slowest pages with, given the time
/// half of them took less than, and the time 95 in 100 took no more than.
pub(crate) fn slowest_pages(median: &str, high: &str) -> String {
    format!("Slowest pages (median {median}, 95th percentile {high}):")
}

/// How many times a template ran, beside its time.
pub(crate) fn runs(n: usize) -> String {
    how_many(n, "run", "runs")
}

/// How many resized copies of an image were made, beside their time.
pub(crate) fn copies_made(n: usize) -> String {
    how_many(n, "copy", "copies")
}

/// What `build` says of the URLs it added to `published.txt`, or of
/// starting the file.
pub(crate) fn kept_published(added: usize, started: bool) -> String {
    let addresses = how_many(added, "URL", "URLs");
    match started {
        true => format!(
            "Started `published.txt` with the {addresses} this site publishes, so a later \
             build can say when one is no longer published. Commit it with the site."
        ),
        false => format!("Added {addresses} to `published.txt`."),
    }
}

/// What `build` says of the resized copies of images it made.
pub(crate) fn made_copies(copies: usize, photographs: usize) -> String {
    format!(
        "Made {} of {}.",
        how_many(copies, "resized copy", "resized copies"),
        how_many(photographs, "image", "images")
    )
}

/// What `build` says of the resized copies it kept that no page shows any
/// more.
pub(crate) fn removed_copies(copies: usize) -> String {
    let copies = how_many(copies, "resized copy", "resized copies");
    format!("Removed {copies} no page shows any more from `.statac`.")
}

/// How far a build has got with the resized copies of images it is making.
pub(crate) fn making_copies(done: usize, of: usize) -> String {
    format!(
        "Making resized copies of images: {} of {}.",
        grouped(done),
        grouped(of)
    )
}

/// What `build` says of the social cards it drew.
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

/// What a problem shows beneath its headline, as a heading and the lines
/// under it: what a hook printed before it stopped, the last of it where
/// there is much, or the lines of `statac.yaml` that carry a retired
/// file's URLs over. None for a kind that carries no such thing.
pub(crate) fn printed(kind: &Kind) -> Option<(String, Vec<String>)> {
    match kind {
        Kind::HookStopped { said, ended, .. } => hook_printed(said, ended),
        Kind::RetiredFile { urls, unread } => carried_over(urls, unread),
        _ => None,
    }
}

fn hook_printed(said: &str, ended: &Ended) -> Option<(String, Vec<String>)> {
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
    let lines = lines[from..].iter().map(|line| (*line).to_owned());
    Some((heading, lines.collect()))
}

/// The lines that carry the URLs of `fetched/refused` over into
/// `statac.yaml`, or why there are none.
fn carried_over(urls: &[String], unread: &Unread) -> Option<(String, Vec<String>)> {
    match unread {
        _ if !urls.is_empty() => {
            let mut lines = vec!["refuse_webmentions_from:".to_owned()];
            lines.extend(
                urls.iter()
                    .map(|url| format!("  - {}", crate::yaml::write::text(url))),
            );
            Some((
                "To keep refusing what it lists, add these lines to `statac.yaml`:".into(),
                lines,
            ))
        }
        Unread::Nothing => None,
        Unread::Lines(lines) => Some((
            format!("Its {} can't be read.", how_many(*lines, "line", "lines")),
            Vec::new(),
        )),
        Unread::File(reason) => Some((format!("It can't be read: {reason}."), Vec::new())),
    }
}

/// What `build` says on a terminal while a hook runs.
pub(crate) fn running_hook(hook: &str) -> String {
    format!("Running `{hook}`.")
}

/// What `build` and `serve` say where another build of the site is
/// running, before waiting for it to end.
pub(crate) const WAITING: &str = "Waiting for another build of the site to finish.";

/// What `build` says of a hook it ran.
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

/// What an editor's completion says beside one of Statac's settings.
pub(crate) const OFFERED_SETTING: &str = "setting";

/// What an editor's completion says beside one of Statac's own fields.
pub(crate) const OFFERED_FIELD: &str = "field";

/// What an editor's completion says beside a `pages.` field.
pub(crate) const OFFERED_HANDED: &str = "cascade";

/// What an editor's completion says beside a type the site declares.
pub(crate) const OFFERED_TYPE: &str = "type";

/// What an editor's completion says beside `@url(...)` in the content.
pub(crate) const OFFERED_IN_SITE: &str = "URL in the site";

/// What an editor's completion says beside a name a snippet call may
/// give a value by.
pub(crate) const OFFERED_VALUE: &str = "value";

/// What an editor's completion says beside one of the site's tags.
pub(crate) fn offered_tag(pages: usize) -> String {
    format!("tag, {}", how_many(pages, "page", "pages"))
}

/// What an editor's completion says beside one of the site's series.
pub(crate) fn offered_series(pages: usize) -> String {
    format!("series, {}", how_many(pages, "page", "pages"))
}

/// What an editor's hover links to the docs with.
pub(crate) const HOVER_MORE: &str = "More on statac.dev";

/// What an editor's hover says one of Statac's settings is.
pub(crate) fn hover_setting(name: &str) -> String {
    format!("`{name}`, a setting")
}

/// What an editor's hover says a setting for one language is.
pub(crate) fn hover_for_a_language(name: &str) -> String {
    format!("`{name}`, a setting for one language")
}

/// What an editor's hover says a setting for one language does.
pub(crate) fn for_a_language(setting: &str, tag: &str) -> String {
    format!("`{setting}` for the pages in `{tag}`.")
}

/// What an editor's hover says one of Statac's front matter words is.
pub(crate) fn hover_field(name: &str) -> String {
    format!("`{name}`, a field")
}

/// What an editor's hover says a `pages.` line is.
pub(crate) fn hover_handed(name: &str) -> String {
    format!("`{name}`, a cascade")
}

/// What an editor's hover says a `pages.` line does, in `statac.yaml` or
/// in a folder's page.
pub(crate) fn hands_down(field: &str, in_folder: bool) -> String {
    match in_folder {
        false => format!("Cascades `{field}` to every page."),
        true => {
            format!("Cascades `{field}` to every page in this folder and the folders inside it.")
        }
    }
}

/// What an editor's hover says a custom setting is.
pub(crate) fn hover_own_value(name: &str) -> String {
    format!("`{name}`, a custom setting")
}

/// What an editor's hover says a custom field is.
pub(crate) fn hover_own_field(name: &str) -> String {
    format!("`{name}`, a custom field")
}

/// How an editor's hover says a template reaches a value, as `site.author`.
pub(crate) fn shown_as(reached: &str) -> String {
    format!("Templates show it as `{reached}`.")
}

/// How an editor's hover says a custom field is declared.
pub(crate) fn declared_as(kind: &str) -> String {
    format!("Declared in `statac.yaml` as `{kind}`.")
}

/// What an editor's hover says a kind of problem is.
pub(crate) fn hover_kind(name: &str, of: Severity) -> String {
    let article = match of {
        Severity::Error => "an",
        Severity::Warning | Severity::Note => "a",
    };
    format!("`{name}`, {article} {}", severity(of))
}

/// What an editor's hover says one of the site's tags is.
pub(crate) fn hover_tag(name: &str, pages: usize) -> String {
    format!("`{name}`, a tag on {}", how_many(pages, "page", "pages"))
}

/// What an editor's hover says one of the site's series is.
pub(crate) fn hover_series(name: &str, pages: usize) -> String {
    format!("`{name}`, a series of {}", how_many(pages, "page", "pages"))
}

/// Where an editor's hover says a tag's or a series' list page is.
pub(crate) fn list_page_at(address: &str) -> String {
    format!("Its list page is `{address}`.")
}

/// What an editor's hover says one of the site's templates is.
pub(crate) fn hover_template(name: &str) -> String {
    format!("`{name}`, a template")
}

/// Where an editor's hover says the file of a template, a page or a
/// snippet is.
pub(crate) fn its_file(path: &str) -> String {
    format!("Its file is `{path}`.")
}

/// What an editor's hover says a name or field a template reads holds,
/// `written` as the template writes it: `post.title`; none where that
/// isn't known.
pub(crate) fn hover_holds(written: &str, holds: Holds) -> Option<String> {
    let held = match holds {
        Holds::Text => "text",
        Holds::Html => "HTML",
        Holds::Number => "a number",
        Holds::Truth => "true or false",
        Holds::Date => "a date",
        Holds::Page => "a page",
        Holds::Pages => "pages",
        Holds::List | Holds::Items => "a list",
        Holds::Lists => "lists",
        Holds::Fields => "fields",
        Holds::Mentions => "webmentions",
        Holds::Call | Holds::Unknown => return None,
    };
    Some(format!("`{written}` holds {held}"))
}

/// What an editor's hover says a name is where what it holds isn't known.
pub(crate) fn hover_name(written: &str) -> String {
    format!("`{written}`, a name")
}

/// What an editor's hover says a function is, as `css` or `page.image`.
pub(crate) fn hover_function(written: &str) -> String {
    format!("`{written}`, a function")
}

/// What an editor's hover says a filter is.
pub(crate) fn hover_filter(name: &str) -> String {
    format!("`{name}`, a filter")
}

/// What an editor's hover says a page of the site is.
pub(crate) fn hover_page(address: &str) -> String {
    format!("`{address}`, a page")
}

/// What an editor's hover says a page's title is.
pub(crate) fn titled(title: &str) -> String {
    format!("Its title is `{title}`.")
}

/// What an editor's hover says a heading of a page is.
pub(crate) fn hover_heading(address: &str) -> String {
    format!("`{address}`, a heading")
}

/// What an editor's hover says a heading's words are.
pub(crate) fn heading_words(words: &str) -> String {
    format!("Its words are `{words}`.")
}

/// Which page an editor's hover says a heading is on, by its title and
/// the path of its file.
pub(crate) fn heading_on(title: &str, path: &str) -> String {
    format!("It is on the page `{title}`, in `{path}`.")
}

/// What an editor's hover says one of the site's snippets is, as a call
/// to it is written: `figure(photo, alt)`.
pub(crate) fn hover_snippet(call: &str) -> String {
    format!("`{call}`, a snippet")
}

/// What an editor's hover says of a snippet that wraps the content below
/// a call to it.
pub(crate) const WRAPS_WRITING: &str = "It wraps the content below the call, up to `@end`.";

/// What an editor's hover says a file beside the page is: an image, or
/// any other file.
pub(crate) fn hover_beside(name: &str, picture: bool) -> String {
    match picture {
        true => format!("`{name}`, an image beside the page"),
        false => format!("`{name}`, a file beside the page"),
    }
}

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

/// What `stamp` says where no stamp waits.
pub(crate) const NOTHING_TO_STAMP: &str = "Nothing to stamp.";

/// What `stamp` and `serve` say of stamps they left as they were.
pub(crate) const NOT_STAMPED: &str = "Didn't stamp: `statac.yaml` has an error.";

/// Why a fetch of `address` brought nothing back, to follow a colon: the
/// same words whatever was being fetched.
pub(crate) fn refusal(address: &str, refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotAnAddress | Refusal::Scheme { .. } | Refusal::NoHost => {
            format!("`{address}` isn't an `https://` or `http://` URL")
        }
        Refusal::Credentials => format!("`{address}` holds a user name or password"),
        Refusal::UnknownHost { host } => format!("nothing answers to the name `{host}`"),
        Refusal::PrivateAddress { host, address } => {
            format!("`{host}` is at `{address}`, which isn't on the open web")
        }
        Refusal::Missing { .. } => "the server has nothing at that URL".into(),
        Refusal::Status { status } => format!("the server answered {status}"),
        Refusal::Redirects { limit } => format!("the URL redirected more than {limit} times"),
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

/// Why an answer fetched as an image wasn't kept.
pub(crate) const NOT_A_PICTURE: &str = "the answer isn't an image";

/// Why a video site's answer naming its thumbnail led to none.
pub(crate) const NO_PICTURE_NAMED: &str = "the video site named no image";

/// What `refresh` says of a file it fetched, by its path from the site's
/// folder.
pub(crate) fn fetched_file(file: &str) -> String {
    format!("Fetched `{file}`.")
}

/// What `refresh` says where a video site answered it has no thumbnail.
pub(crate) fn no_still_there(site: &str, id: &str) -> String {
    format!("No thumbnail: {site} has no image for `{id}`.")
}

/// What `refresh` says of a thumbnail it couldn't fetch, and why.
pub(crate) fn cant_fetch_still(id: &str, reason: &str) -> String {
    format!("Can't fetch the thumbnail for `{id}`: {reason}.")
}

/// `refresh`'s last line, where some thumbnail was missing: what it fetched,
/// what was there already, and what it couldn't fetch, each where there
/// is any.
pub(crate) fn refreshed(fetched: usize, here: usize, failed: usize) -> String {
    let mut parts = Vec::new();
    if fetched > 0 {
        parts.push(format!(
            "Fetched {}.",
            how_many(fetched, "thumbnail", "thumbnails")
        ));
    }
    if here > 0 {
        parts.push(format!("{} already here.", grouped(here)));
    }
    if failed > 0 {
        parts.push(format!("{} couldn't be fetched.", grouped(failed)));
    }
    match parts.is_empty() {
        true => "Fetched no thumbnails.".into(),
        false => parts.join(" "),
    }
}

/// What `refresh` says after its last line where the site stops with its
/// drafts and scheduled posts, and builds without them.
pub(crate) fn drafts_not_refreshed(errors: usize) -> String {
    let them = if errors == 1 { "it" } else { "them" };
    let errors = how_many(errors, "error", "errors");
    format!(
        "Drafts weren't looked at: the site has {errors} with {them}. Run `statac check --drafts` \
         to see {them}."
    )
}

/// `refresh`'s last line, where no thumbnail was missing.
pub(crate) const NOTHING_TO_FETCH: &str = "Nothing to fetch.";

/// What `refresh` says where the site uses webmentions and the webmention
/// service's token wasn't given.
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

/// What `refresh` says where the webmention service couldn't be asked, and
/// why.
pub(crate) fn cant_ask(reason: &str) -> String {
    format!("Can't ask for webmentions: {reason}.")
}

/// Why the webmention service couldn't be asked, where it refused the
/// token.
pub(crate) const TOKEN_REFUSED: &str = "the webmention service didn't accept the token";

/// Why the webmention service couldn't be asked, where its answer isn't
/// the feed it sends.
pub(crate) const UNREADABLE_ANSWER: &str = "the webmention service's answer couldn't be read";

/// What `refresh` says where it stopped asking the webmention service with
/// more to come.
pub(crate) fn asked_enough(asks: usize) -> String {
    format!(
        "Stopped asking for webmentions after {asks} asks: run `statac refresh` again for the rest."
    )
}

/// What `refresh` says of a mention of a kind Statac doesn't read.
pub(crate) fn not_read(id: u64, property: &str) -> String {
    format!("Webmention `{id}` not kept: Statac doesn't read `{property}`.")
}

/// What `refresh` says of a mention the webmention service gave no kind
/// for.
pub(crate) fn no_kind_said(id: u64) -> String {
    format!("Webmention `{id}` not kept: it doesn't say what kind it is.")
}

/// What `refresh` says of a mention naming no page of its sender's.
pub(crate) fn no_address_named(id: u64) -> String {
    format!("Webmention `{id}` not kept: it names no URL.")
}

/// What `refresh` says of a mention whose sender's page isn't on the web.
pub(crate) fn not_a_web_address(id: u64, url: &str) -> String {
    format!("Webmention `{id}` not kept: `{url}` isn't an `https://` or `http://` URL.")
}

/// What `refresh` says of a mention of a URL elsewhere.
pub(crate) fn not_on_site(id: u64, target: &str) -> String {
    format!("Webmention `{id}` not kept: `{target}` isn't on this site.")
}

/// What `refresh` says of each file of a refused mention it removed, by
/// its path from the site's folder.
pub(crate) fn removed_file(file: &str) -> String {
    format!("Removed `{file}`.")
}

/// `refresh`'s line for the refused mentions it removed, with the avatars
/// no mention kept names any more.
pub(crate) fn removed_refused(mentions: usize, faces: usize) -> String {
    let mentions = how_many(mentions, "refused webmention", "refused webmentions");
    match faces {
        0 => format!("Removed {mentions}."),
        faces => format!(
            "Removed {mentions} and {}.",
            how_many(faces, "avatar", "avatars")
        ),
    }
}

/// What `refresh` says of a new mention a line of
/// `refuse_webmentions_from` refuses, by the line as written.
pub(crate) fn refused_here(id: u64, address: &str) -> String {
    format!("Webmention `{id}` not kept: `{address}` is refused.")
}

/// What `refresh` says of a mention kept without its sender's URL.
pub(crate) fn sender_not_kept(id: u64, address: &str) -> String {
    format!(
        "Webmention `{id}`: `{address}` isn't an `https://` or `http://` URL, so the sender's \
         own URL isn't kept."
    )
}

/// What `refresh` says of a mention kept without its sender's avatar.
pub(crate) fn cant_fetch_face(id: u64, reason: &str) -> String {
    format!("Can't fetch the avatar for `{id}`: {reason}.")
}

/// Why an avatar fetched wasn't kept: its pixels would take too much to
/// read.
pub(crate) fn too_large_to_open(width: u32, height: u32) -> String {
    format!(
        "the image is {} by {} pixels, too large to open",
        grouped(width),
        grouped(height)
    )
}

/// Why an avatar fetched wasn't kept: no copy of it could be made.
pub(crate) const NOT_MADE_AGAIN: &str = "the image couldn't be made again";

/// `refresh`'s line for webmentions, where the webmention service
/// answered: what it kept, the avatars it fetched, and what it didn't keep,
/// each where there is any.
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
            how_many(faces, "avatar", "avatars")
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

/// The part of [`fetched_ages`] for the thumbnails: how many, the age of the
/// oldest the record dates, in days before the build's, and how many it
/// doesn't date.
pub(crate) fn stills_age(stills: usize, oldest: Option<i64>, undated: usize) -> String {
    let counted = how_many(stills, "thumbnail", "thumbnails");
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

/// Where `serve` answers, and the folder it answers from. The URL comes
/// last and bare, so a terminal makes a link of exactly it.
pub(crate) fn serving(folder: &str, address: &str) -> String {
    format!("Serving `{folder}` at {address}")
}

/// What `serve` says where the port asked for was another program's.
pub(crate) fn port_moved(asked: u16, port: u16) -> String {
    format!("Port {asked} is in use: serving on port {port} instead.")
}

/// What `serve` says where the system kept it from answering the browser,
/// once for each run of such faults.
pub(crate) fn cant_answer(reason: &Reason) -> String {
    format!("Can't answer the browser: {reason}.")
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

/// The files of the site a rebuild of `serve` answers to, as its line
/// opens. With none named, the system lost count of them.
pub(crate) fn files_changed(files: &[String]) -> String {
    match files {
        [] => "Files changed.".into(),
        [one] => format!("`{one}` changed."),
        [one, other] => format!("`{one}` and `{other}` changed."),
        [one, rest @ ..] => format!("`{one}` and {} other files changed.", grouped(rest.len())),
    }
}

/// What a rebuild of `serve` built.
pub(crate) fn built_round(built: usize, of: usize, seconds: f64) -> String {
    format!(
        "Built {} of {} in {seconds:.2} seconds.",
        grouped(built),
        how_many(of, "file", "files")
    )
}

/// A rebuild of `serve` that errors stopped.
pub(crate) fn stopped_round(errors: usize) -> String {
    format!("{} stopped the build.", how_many(errors, "error", "errors"))
}

/// What a rebuild of `serve` asked of the pages open in a browser.
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

/// After a rebuild of `serve` that stopped, in the terminal.
pub(crate) const BROWSER_KEEPS: &str = "The browser keeps the pages as last built.";

/// After a rebuild of `serve` that stopped, on the page.
pub(crate) const PAGE_KEPT: &str = "This page is as last built.";

/// What the debug panel on a served page calls the page it is on and the
/// site.
pub(crate) const PANEL_PAGE: &str = "This page";
pub(crate) const PANEL_SITE: &str = "The site";
pub(crate) const PANEL_WRITTEN: &str = "Written into your files";

/// How the debug panel marks a draft and a scheduled post, and how
/// `serve`'s list of pages at the top of a site with no home page does.
pub(crate) const PANEL_DRAFT: &str = "Draft";
pub(crate) fn panel_waits(day: &str) -> String {
    format!("Scheduled for {day}")
}

/// What the debug panel says of the page it is on.
pub(crate) const PANEL_IS_DRAFT: &str = "A draft: `statac build` leaves it out.";
pub(crate) fn panel_is_waiting(day: &str) -> String {
    format!("Scheduled for {day}: `statac build` leaves it out until then.")
}
pub(crate) const PANEL_PAGE_CLEAR: &str = "No problems on this page.";

/// The debug panel's account of the page it is on: what builds it and
/// which lists hold it.
pub(crate) const PANEL_FILE: &str = "File";
pub(crate) const PANEL_ADDRESS: &str = "URL";
pub(crate) const PANEL_LAYOUT: &str = "Layout";
pub(crate) const PANEL_FEEDS: &str = "Feeds";
pub(crate) const PANEL_NO_FILE: &str = "None: Statac made this page.";
pub(crate) const PANEL_NO_LAYOUT: &str = "None.";
pub(crate) const PANEL_IN_NO_FEED: &str = "None.";

/// A list a page is in, as the debug panel names it.
pub(crate) fn panel_list(word: Word, name: &str) -> String {
    let list = the_list(word, name);
    let list = list.strip_prefix("the ").unwrap_or(&list);
    format!("The {list}")
}

/// What a page's build read that is no one file of the site's.
pub(crate) const PANEL_READ_PAGES: &str = "Which pages there are";
pub(crate) const PANEL_READ_LISTS: &str = "Which lists there are";
pub(crate) const PANEL_READ_ORDER: &str = "The order of the pages in their lists";
pub(crate) const PANEL_READ_TRANSLATIONS: &str = "Which pages are translations of which";
pub(crate) const PANEL_READ_FILES: &str = "Which images and other files the site has";
pub(crate) const PANEL_READ_MENTIONS: &str = "The webmentions in `fetched/mentions/`";
pub(crate) const PANEL_READ_TODAY: &str = "Today's date";
pub(crate) fn panel_read_list(word: Word, name: &str) -> String {
    format!("The pages of {}", the_list(word, name))
}
pub(crate) fn panel_read_field(name: &str) -> String {
    format!("What every page writes as `{name}`")
}

/// The debug panel's table of every name a template can write on the
/// page.
pub(crate) const PANEL_COLUMNS: [&str; 4] = ["Write", "Holds", "On this page", "Written at"];
pub(crate) const PANEL_NO_VALUE: &str = "no value";
pub(crate) const PANEL_FIELDS_BELOW: &str = "the fields beneath";
pub(crate) const PANEL_A_CALL: &str = "called, not shown";

/// What a name holds, in a word or two; nothing where that isn't known.
pub(crate) fn panel_holds(holds: Holds) -> &'static str {
    match holds {
        Holds::Text => "text",
        Holds::Html => "HTML",
        Holds::Number => "number",
        Holds::Truth => "true or false",
        Holds::Date => "date",
        Holds::Page => "page",
        Holds::Pages => "pages",
        Holds::List | Holds::Items => "list",
        Holds::Lists => "lists",
        Holds::Fields => "fields",
        Holds::Mentions => "webmentions",
        Holds::Call => "call",
        Holds::Unknown => "",
    }
}

/// A list a name holds, counted in what it holds.
pub(crate) fn panel_count(holds: Holds, count: usize) -> String {
    match holds {
        Holds::Pages => how_many(count, "page", "pages"),
        Holds::Lists => how_many(count, "list", "lists"),
        Holds::Mentions => how_many(count, "webmention", "webmentions"),
        _ => how_many(count, "item", "items"),
    }
}

/// After the fields of a value with more than the debug panel shows.
pub(crate) fn panel_more_fields(more: usize) -> String {
    format!("and {}", how_many(more, "more field", "more fields"))
}

/// Above the debug panel's links to the pages a build leaves out.
pub(crate) const PANEL_DRAFTS: &str = "Drafts:";
pub(crate) const PANEL_WAITING: &str = "Scheduled posts:";

/// A word of the debug panel's, which the page fills in where it has holes,
/// each written as `{name}` with a count's as `{n}`.
pub(crate) enum PanelWord {
    Said(String),
    /// Said of one, and of any other count.
    Counted(&'static str, &'static str),
}

/// Every word the debug panel on a served page shows that the page itself
/// puts together, by the name the page knows it by.
pub(crate) fn panel_words() -> Vec<(&'static str, PanelWord)> {
    use PanelWord::{Counted, Said};
    let said = |text: &str| Said(text.to_owned());
    vec![
        // The tab, closed, and what a screen reader is told of it.
        ("name", said(NAME)),
        ("building", said("Building")),
        ("buildStopped", said("Build stopped")),
        ("draft", said(PANEL_DRAFT)),
        ("waits", Said(panel_waits("{day}"))),
        ("tabLabel", said("Statac: {said}. Open the debug panel")),
        ("labelBuiltAt", said("built at {time}")),
        ("labelBuilding", said("building")),
        ("labelStopped", said("build stopped")),
        ("labelDraft", said("a draft")),
        ("labelWaits", said("scheduled for {day}")),
        ("errors", Counted("{n} error", "{n} errors")),
        ("warnings", Counted("{n} warning", "{n} warnings")),
        ("notes", Counted("{n} note", "{n} notes")),
        // The tab, where serve answers in a way the page can't read.
        ("serveChanged", said("Serve has changed")),
        ("reload", said("Reload the page")),
        (
            "reloadLabel",
            said("Statac: serve has changed. Reload the page"),
        ),
        // The summary the tab shows under a pointer or focus.
        ("builtAt", said("Built at {time}")),
        ("stoppedAt", said("Build stopped at {time}")),
        (
            "errorsElsewhere",
            Counted(
                "{n} error elsewhere on the site",
                "{n} errors elsewhere on the site",
            ),
        ),
        (
            "warningsElsewhere",
            Counted(
                "{n} warning elsewhere on the site",
                "{n} warnings elsewhere on the site",
            ),
        ),
        (
            "notesElsewhere",
            Counted(
                "{n} note elsewhere on the site",
                "{n} notes elsewhere on the site",
            ),
        ),
        ("clickToOpen", said("Click to open")),
        // The card a stopped build opens.
        ("pageKept", said(PAGE_KEPT)),
        ("atOf", said("{at} of {of}")),
        ("previousError", said("Previous error")),
        ("nextError", said("Next error")),
        ("putAway", said("Put the card away")),
        ("place", said("{file}, line {line}")),
        ("openInEditor", said("Open in editor")),
        ("openPlace", said("Open {place} in {editor}")),
        ("copyPlace", said("Copy file and line")),
        ("copyPlaceLabel", said("Copy {place}")),
        ("holdsHere", said("On this page it holds {value}.")),
        ("holdsNothing", said("On this page it holds no value.")),
        ("showInNames", said("Show in Names")),
        ("explain", said("What it means, and how to fix it")),
        ("allProblems", said("All problems")),
        ("builtThis", said("While building this page")),
        (
            "builtThisAnd",
            said("While building this page, and {n} more:"),
        ),
        ("builtOn", said("While building")),
        ("builtMore", said("and {n} more")),
        ("more", Said(more("{link}"))),
        ("quietened", said("Quietened in `statac.yaml`")),
        // The drawer, and its tabs.
        ("problems", said("Problems")),
        ("names", said("Names")),
        ("page", said(PANEL_PAGE)),
        ("site", said(PANEL_SITE)),
        ("parts", said("What the debug panel shows")),
        (
            "statusStopped",
            said("Stopped at {time}; the page is as built at {built}"),
        ),
        ("statusStoppedFirst", said("Stopped at {time}")),
        ("find", said("Find a name")),
        ("findHow", said("Find a name or a value")),
        ("settings", said("Settings")),
        ("dockSide", said("Dock to the side")),
        ("dockBottom", said("Dock to the bottom")),
        ("close", said("Close the debug panel")),
        ("resize", said("Resize the debug panel")),
        // Problems.
        ("onThisPage", said("On this page")),
        ("wholeSite", said("The whole site")),
        ("errorsShown", said("Errors")),
        ("warningsShown", said("Warnings")),
        ("notesShown", said("Notes")),
        ("goneNone", said("Gone since the last build: none.")),
        ("times", Counted("{title}, once", "{title}, {n} times")),
        ("pageClear", said(PANEL_PAGE_CLEAR)),
        ("siteClear", said(NO_PROBLEMS)),
        ("filteredOut", said("The filters hide every problem here.")),
        ("onlyKind", said("Only {title}")),
        ("everyKind", said("Show every kind")),
        (
            "moreProblems",
            said("And {n} more, which `statac check` shows."),
        ),
        ("asking", said("Asking serve for the site's problems.")),
        ("aboutKind", said("About this kind of problem")),
        ("fromExplain", said("From `statac explain {kind}`")),
        ("howToFix", said("How to fix it")),
        ("readOnline", said("Read it on statac.dev")),
        // Names.
        ("held", said("{held} of {count} hold a value")),
        ("found", said("{n} found")),
        ("hideEmpty", said("Hide names with no value here")),
        ("onlyUsed", said("Only names this page uses")),
        ("layoutFrom", said("Layout `{name}`, from `{file}`")),
        ("columnWrite", said(PANEL_COLUMNS[0])),
        ("columnHolds", said(PANEL_COLUMNS[1])),
        ("columnHere", said(PANEL_COLUMNS[2])),
        ("columnAt", said(PANEL_COLUMNS[3])),
        ("holdsUnknown", said("nothing yet")),
        (
            "noMatch",
            said(
                "No name here matches \u{201c}{query}\u{201d}. Names are written as a \
                 template writes them, like `page.title`.",
            ),
        ),
        ("noneLeft", said("The filters leave no name here.")),
        ("noPage", said("No page is published at this URL.")),
        ("chosen", said("The chosen name")),
        ("copy", said("Copy")),
        ("copied", said("Copied")),
        ("here", said("Here")),
        (
            "uses",
            said("Uses it: the page read it while it was built."),
        ),
        ("usesNot", said("Doesn't use it.")),
        ("answers", said("It answers these in turn")),
        ("ways", said("Ways to write it")),
        ("wayAsWritten", said("As the site writes dates")),
        ("wayYear", said("Only the year, written by example")),
        ("wayTime", said("In a time element, for machines to read")),
        ("wayEachPost", said("Each post in it")),
        ("wayLink", said("A link to it")),
        ("wayEachList", said("A link to each")),
        ("wayEachPage", said("Each page")),
        ("wayLinkIf", said("A link to it, where there is one")),
        ("wayTrue", said("Only where it is true")),
        ("wayHtml", said("As it is, as HTML")),
        ("wayReactions", said("Each like, repost or bookmark")),
        ("wayEachItem", said("Each item in it")),
        ("wayHasOne", said("Only on the pages that have one")),
        ("wayAsIs", said("As it is")),
        // This page.
        ("published", said("Published by `statac build`")),
        ("isDraft", said(PANEL_IS_DRAFT)),
        ("isWaiting", Said(panel_is_waiting("{day}"))),
        ("file", said(PANEL_FILE)),
        ("address", said(PANEL_ADDRESS)),
        ("layout", said(PANEL_LAYOUT)),
        ("language", said("Language")),
        ("feeds", said(PANEL_FEEDS)),
        ("noFile", said(PANEL_NO_FILE)),
        ("noLayout", said(PANEL_NO_LAYOUT)),
        ("noFeed", said(PANEL_IN_NO_FEED)),
        ("extends", said("extends")),
        ("layoutNamed", said("Named `layout: {name}` at {place}.")),
        (
            "layoutHanded",
            said("Named `layout: {name}` at {place}, and cascaded to every page in the folder."),
        ),
        (
            "layoutUnnamed",
            said("Nothing names one, so it is `{name}`."),
        ),
        ("leftToRight", said("{tag}, left to right")),
        ("rightToLeft", said("{tag}, right to left")),
        ("inLists", Counted("In {n} list", "In {n} lists")),
        ("inNoList", said("In no list")),
        ("previous", said("Previous: {title}")),
        ("next", said("Next: {title}")),
        ("rebuilt", said("Rebuilt when these change")),
        (
            "rebuiltHow",
            said("What building this page read, from serve's own record."),
        ),
        (
            "errorsHere",
            Counted("{n} error on this page", "{n} errors on this page"),
        ),
        (
            "warningsHere",
            Counted("{n} warning on this page", "{n} warnings on this page"),
        ),
        (
            "notesHere",
            Counted("{n} note on this page", "{n} notes on this page"),
        ),
        ("show", said("Show")),
        // The site.
        ("lastRound", said("The last rebuild")),
        ("changedOne", said("`{file}` changed.")),
        ("changedTwo", said("`{file}` and `{other}` changed.")),
        ("changedMany", said("`{file}` and {n} other files changed.")),
        ("tookBefore", said("Getting ready")),
        ("tookReading", said("Reading")),
        ("tookComparing", said("Comparing")),
        ("tookBuilding", said("Building")),
        ("tookConcluding", said("Concluding")),
        ("tookWriting", said("Writing")),
        ("ms", said("{n} ms")),
        ("rounds", said("Rebuilds since serve started")),
        ("firstRound", said("first")),
        ("acrossSite", said("Problems across the site")),
        ("leftOut", said("Left out of `statac build`")),
        ("drafts", said(PANEL_DRAFTS)),
        ("waiting", said(PANEL_WAITING)),
        (
            "leftOutNone",
            said(
                "No drafts and no scheduled posts. A page with `draft: true`, or a date after \
                 today, is listed here with a link to it.",
            ),
        ),
        (
            "leftOutUnbuilt",
            said("Serve was started with `--no-drafts`, so these have no page to open."),
        ),
        ("written", said(PANEL_WRITTEN)),
        (
            "writtenNone",
            said(
                "Nothing in the last rebuild. When a stamp such as `=today` is turned into a \
                 date, the file and line show here.",
            ),
        ),
        // Settings.
        ("settingsLabel", said("Panel settings")),
        ("sits", said("Where the debug panel sits")),
        ("corner", said("Corner")),
        ("bar", said("Bar")),
        ("side", said("Side")),
        ("whichCorner", said("Which corner")),
        ("bottomLeft", said("Bottom left")),
        ("bottomRight", said("Bottom right")),
        ("topLeft", said("Top left")),
        ("topRight", said("Top right")),
        ("colours", said("Colours")),
        ("system", said("As the system")),
        ("light", said("Light")),
        ("dark", said("Dark")),
        ("openIn", said("Open files in")),
        ("editorNone", said("None")),
        ("editorCode", said("VS Code")),
        ("editorInsiders", said("VS Code Insiders")),
        ("editorCursor", said("Cursor")),
        ("editorZed", said("Zed")),
        ("editorSublime", said("Sublime Text")),
        (
            "sublimeNeeds",
            said("Sublime Text opens these links once the free SublimeUrl app is installed."),
        ),
        ("whenStopped", said("When a build stops")),
        ("openCard", said("Open the error card")),
        ("tabOnly", said("Only turn the tab red")),
        ("hide", said("Hide until I reload")),
        ("shortcut", said("Shortcut")),
        ("change", said("Change")),
        (
            "pressKeys",
            said("Press the new keys, or Esc to keep these"),
        ),
        ("alt", said("Alt")),
        ("shift", said("Shift")),
        ("ctrl", said("Ctrl")),
        ("meta", said("Meta")),
        ("keptHere", said("Kept in this browser.")),
        // Search.
        ("search", said("Search")),
        ("searchLabel", said("Search the site")),
        ("searchHow", said("Words a reader might search for")),
        (
            "searchStart",
            said(
                "Shows the pages a reader would find, best first, and why each ranks where it does.",
            ),
        ),
        (
            "searchDrafts",
            said("Drafts and scheduled posts are searched too, as serve shows them."),
        ),
        ("pagesFound", Counted("{n} page found.", "{n} pages found.")),
        ("foundSome", said("No page holds every word.")),
        ("foundNone", said("No pages found.")),
        ("meant", said("Did you mean {words}?")),
        ("moreFound", said("And {n} more.")),
        ("opening", said("The page's opening")),
        ("whyRanked", said("Why it ranks here")),
        ("wordsHeld", said("Words held: {n} of {count}")),
        ("score", said("Score: {score}")),
        (
            "rankRule",
            said("Parts holding more of the words come first, then those scoring higher."),
        ),
        ("foundAs", said("Found as {forms}")),
        ("notHere", said("Not in this part.")),
        (
            "starting",
            said("Matched as the start of a longer word, counting 0.8 of a whole one."),
        ),
        ("columnWhere", said("Where")),
        ("columnTimes", said("Times")),
        ("columnAdds", said("Adds")),
        ("placeTitle", said("Title")),
        ("placeHeading", said("Heading")),
        ("placeTags", said("Tags")),
        ("placeSummary", said("Summary")),
        ("placePictures", said("Image alt text")),
        ("placeText", said("Text")),
        ("holding", said("In {n} of the site's {count} parts")),
        ("adds", said("Adds {score}")),
        (
            "partLength",
            said(
                "Length of its text: {n}, against the site's average of {average}. Longer text \
                 counts each word in it for less.",
            ),
        ),
        (
            "titleLength",
            said(
                "Length of the page's title: {n}, against the site's average of {average}. A \
                 longer title counts each word in it for less.",
            ),
        ),
    ]
}

/// The problems a rebuild of `serve` no longer has, counted by kind.
pub(crate) fn gone(kinds: &[(&str, usize)]) -> String {
    let kinds: Vec<_> = kinds
        .iter()
        .map(|(name, n)| format!("{} {name}", grouped(n)))
        .collect();
    format!("Gone since the last build: {}.", kinds.join(", "))
}

/// The page `serve` answers with where the site has nothing at a URL and
/// no `404.html` of its own.
pub(crate) const NOT_FOUND_TITLE: &str = "Not found";
pub(crate) const NOT_FOUND: &str = "Nothing is published at this URL.";

/// The page `serve` answers the top of a site with where the site has no
/// home page, listing the pages the last build made. `build` never
/// publishes it.
pub(crate) const NO_HOME_TITLE: &str = "No home page";
pub(crate) const NO_HOME: &str = "`statac serve` shows this list of the site's pages \
                                  in its place. `statac build` doesn't publish it.";
/// A page on that list that is one of its list's pages after the first.
pub(crate) fn no_home_later(title: &str, number: usize) -> String {
    format!("{title}, page {number}")
}
/// On that page in place of the list, where the last build made no pages.
pub(crate) const NO_HOME_NO_PAGES: &str = "The last build made no pages.";

/// The page `serve` answers the top of a site with where nothing is
/// published there and no build has finished without an error.
pub(crate) const NOTHING_BUILT_TITLE: &str = "Nothing built yet";
pub(crate) const NOTHING_BUILT: &str =
    "Every build so far has stopped. The terminal and the debug panel say why.";

/// Before the link to the top of a site published under a path, on the
/// same page.
pub(crate) const PUBLISHED_UNDER: &str = "The site is published under";

/// `serve`'s answer to a URL that could name a file outside the site.
pub(crate) const ADDRESS_REFUSED: &str = "This URL can't name a file of the site.";

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

pub(crate) const ASK_TITLE: &str = "What is the site title?";
pub(crate) const ASK_AUTHOR: &str = "What is the author's name?";
pub(crate) const ASK_NONE_HINT: &str = "Enter for none.";
pub(crate) const ASK_ADDRESS: &str = "What is the site URL?";
pub(crate) const ASK_ADDRESS_HINT: &str = "Enter for none yet.";
pub(crate) const ASK_OLD_ADDRESS: &str = "What is the site's URL?";
pub(crate) const ASK_OLD_ADDRESS_HINT: &str = "The old site's settings don't say.";
pub(crate) const ASK_LANGUAGE: &str = "What is the site language?";
pub(crate) const ASK_LANGUAGE_HINT: &str = "For more than one, the main one first, as `en, fr`.";
pub(crate) const ASK_ZONE: &str = "What is the site time zone?";
pub(crate) const ASK_ZONE_HINT: &str = "As `Europe/London`. Enter for none.";
/// Said under a question whose answer in brackets is how this machine is set.
pub(crate) const ASK_MACHINES: &str = "In brackets, as this machine is set.";

/// The answer Enter gives, as the line the answer is typed on shows it.
pub(crate) fn ask_default(default: &str) -> String {
    format!("[{default}]")
}

/// Marks where the answer is typed.
pub(crate) const ASK_MARK: &str = ">";

pub(crate) fn not_a_site_address(written: &str) -> String {
    format!("`{written}` isn't a full URL. Write it as `https://example.org`.")
}

pub(crate) fn not_a_language_code(written: &str) -> String {
    format!("`{written}` isn't a language's code. Write it as `en`, or `en-GB`.")
}

pub(crate) fn language_given_twice(written: &str, first: &str) -> String {
    format!("`{written}` is `{first}` again. Name each language once.")
}

pub(crate) fn no_words_for(tag: &str) -> String {
    format!(
        "Statac has no translations for `{tag}`: write its months and days in \
         `languages/{tag}.yaml`."
    )
}

/// Where the docs give each theme's rules for coloured code.
pub(crate) const CODE_COLOURS: &str = "https://statac.dev/code-colours/";

/// Where the docs describe a language's translations.
pub(crate) const LANGUAGE_FILES: &str = "https://statac.dev/dates-and-languages/#languages";

pub(crate) fn made_site(folder: &str, files: usize) -> String {
    format!(
        "Made a new site in `{folder}`: {}.",
        how_many(files, "file", "files")
    )
}

pub(crate) fn not_a_time_zone(written: &str, nearest: Option<&str>) -> String {
    match nearest {
        Some(nearest) => {
            format!("`{written}` isn't a time zone Statac knows. Did you mean `{nearest}`?")
        }
        None => format!("`{written}` isn't a time zone Statac knows. Write it as `Europe/London`."),
    }
}

/// Where the translations Statac has only in English for `tag` wait in a
/// new site, in `file`.
pub(crate) fn translate_words(tag: &str, file: &str) -> String {
    format!("Statac has some translations for `{tag}` only in English: translate them in `{file}`.")
}

/// The last thing `statac new` says, with the way to the site's folder
/// where it isn't the folder Statac was run in.
pub(crate) fn next_steps(folder: Option<&str>) -> String {
    let then = "write a page in `content/`, such as `content/hello.md`, and run `statac serve`";
    match folder {
        Some(folder) => format!("Next: `cd {folder}`, then {then}."),
        None => format!("Next: {then}."),
    }
}

/// An option as written, without the value given it.
fn option_name(written: &str) -> &str {
    written.split('=').next().unwrap_or(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::problem::Host;

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
                reason: Reason::FolderNamed,
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
        let no_still = |host, absent| Kind::NoThumbnail {
            host,
            id: "abcdefghijk".into(),
            absent,
        };
        let record = |fault| Kind::MalformedRecord { fault };
        let retired = |urls: &[&str], unread| Kind::RetiredFile {
            urls: urls.iter().map(|url| (*url).to_owned()).collect(),
            unread,
        };
        vec![
            (
                no_still(Host::YouTube, false),
                "No thumbnail: no image has been fetched for this video yet",
                Some("Run `statac refresh` to fetch one."),
            ),
            (
                no_still(Host::YouTube, true),
                "No thumbnail: YouTube has no image for this video",
                Some("Show an image of your own in its place."),
            ),
            (
                no_still(Host::Vimeo, true),
                "No thumbnail: Vimeo has no image for this video",
                Some("Show an image of your own in its place."),
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
                record(RecordFault::NotAnAddress("about/".into())),
                "Malformed record: `about/` isn't a URL starting with `/`",
                Some(
                    "Write it as a URL starting with `/`, such as `/about/`, or take the line out.",
                ),
            ),
            (
                Kind::UnreadableImage {
                    path: "fetched/stills/youtube-abcdefghijk.jpg".into(),
                    fault: PictureFault::NotAStill,
                },
                "Unreadable image: `fetched/stills/youtube-abcdefghijk.jpg` isn't a JPEG, a \
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
                Some("If the page moved, add the old URL to its `redirect_from`."),
            ),
            (
                Kind::RefusedWebmentions { count: 2 },
                "Refused webmentions: 2 in `fetched/mentions/` are left out",
                Some("Run `statac refresh` to delete their files."),
            ),
            (
                Kind::RefusedWebmentions { count: 1 },
                "Refused webmentions: 1 in `fetched/mentions/` is left out",
                Some("Run `statac refresh` to delete its file."),
            ),
            (
                retired(&["https://a.example/"], Unread::Nothing),
                "Retired file: Statac no longer reads `fetched/refused`",
                Some("Delete `fetched/refused` after adding them."),
            ),
            (
                retired(&["https://a.example/"], Unread::Lines(1)),
                "Retired file: Statac no longer reads `fetched/refused`",
                Some(
                    "Delete `fetched/refused` after adding them, and the URL on the line that \
                     couldn't be read.",
                ),
            ),
            (
                retired(&["https://a.example/"], Unread::Lines(2)),
                "Retired file: Statac no longer reads `fetched/refused`",
                Some(
                    "Delete `fetched/refused` after adding them, and the URLs on the 2 lines \
                     that couldn't be read.",
                ),
            ),
            (
                retired(&[], Unread::Lines(2)),
                "Retired file: Statac no longer reads `fetched/refused`",
                Some(
                    "Copy its URLs to `refuse_webmentions_from:` in `statac.yaml`, then delete it.",
                ),
            ),
            (
                retired(&[], Unread::File(Reason::Denied)),
                "Retired file: Statac no longer reads `fetched/refused`",
                Some(
                    "Copy its URLs to `refuse_webmentions_from:` in `statac.yaml`, then delete it.",
                ),
            ),
            (
                retired(&[], Unread::Nothing),
                "Retired file: Statac no longer reads `fetched/refused`",
                Some("Delete it: it lists no URLs."),
            ),
            (
                Kind::NoSiteUrl {
                    waiting: Waiting::Webmentions,
                },
                "No site URL: webmentions are left out until the site has a `url`",
                Some("Add the site's full URL to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::WrongKind {
                    field: "face".into(),
                    expected: Expected::FaceName,
                    found: Found::Written("\"../record\"".into()),
                    respelt: None,
                },
                "Wrong kind of value: `face` is an avatar's file name, such as \
                 `0123456789abcdef.jpg`, not `\"../record\"`",
                None,
            ),
            (
                Kind::MissingField {
                    field: "target".into(),
                    required_by: Requirer::Mention,
                },
                "Missing field: `target`",
                Some("Add `target:`, which every webmention Statac keeps has."),
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
            "Can't ask for webmentions: the webmention service didn't accept the token."
        );
        assert_eq!(
            cant_ask(UNREADABLE_ANSWER),
            "Can't ask for webmentions: the webmention service's answer couldn't be read."
        );
        assert_eq!(
            not_read(1, "follow-of"),
            "Webmention `1` not kept: Statac doesn't read `follow-of`."
        );
        assert_eq!(
            not_a_web_address(1, "javascript:alert(1)"),
            "Webmention `1` not kept: `javascript:alert(1)` isn't an `https://` or `http://` URL."
        );
        assert_eq!(
            not_on_site(1, "https://example.com/"),
            "Webmention `1` not kept: `https://example.com/` isn't on this site."
        );
        assert_eq!(
            sender_not_kept(1, "ftp://a.example/"),
            "Webmention `1`: `ftp://a.example/` isn't an `https://` or `http://` URL, so the \
             sender's own URL isn't kept."
        );
        assert_eq!(
            cant_fetch_face(1, NOT_A_PICTURE),
            "Can't fetch the avatar for `1`: the answer isn't an image."
        );
        assert_eq!(
            mentions_refreshed(3, 2, 1),
            "Fetched 3 webmentions and 2 avatars. 1 not kept."
        );
        assert_eq!(mentions_refreshed(1, 0, 0), "Fetched 1 webmention.");
        assert_eq!(
            mentions_refreshed(1, 1, 0),
            "Fetched 1 webmention and 1 avatar."
        );
        assert_eq!(mentions_refreshed(0, 0, 2), "2 not kept.");
        assert_eq!(mentions_refreshed(0, 0, 0), "No new webmentions.");
    }

    #[test]
    fn the_age_of_the_webmentions_is_said_of_the_newest() {
        let line = |parts: &[String]| fetched_ages(parts);
        assert_eq!(
            line(&[stills_age(87, Some(6), 0), mentions_age(12, Some(0))]),
            "Fetched: 87 thumbnails, the oldest 6 days ago; 12 webmentions, the newest today."
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
                format!("`{address}` isn't an `https://` or `http://` URL"),
            ),
            (
                Refusal::Scheme {
                    scheme: "ftp".into(),
                },
                format!("`{address}` isn't an `https://` or `http://` URL"),
            ),
            (
                Refusal::NoHost,
                format!("`{address}` isn't an `https://` or `http://` URL"),
            ),
            (
                Refusal::Credentials,
                format!("`{address}` holds a user name or password"),
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
                "the server has nothing at that URL".into(),
            ),
            (
                Refusal::Status { status: 503 },
                "the server answered 503".into(),
            ),
            (
                Refusal::Redirects { limit: 5 },
                "the URL redirected more than 5 times".into(),
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
        assert_eq!(NOT_A_PICTURE, "the answer isn't an image");
        assert_eq!(NO_PICTURE_NAMED, "the video site named no image");
    }

    #[test]
    fn every_reason_reads_as_written() {
        let cases = [
            (Reason::Link, "it is a link"),
            (
                Reason::LinkOutside,
                "it is a link that leads outside the old site",
            ),
            (Reason::NotAFile, "it is not a file"),
            (Reason::NotAFolder, "it isn't a folder"),
            (
                Reason::NotASite,
                "it has no `statac.yaml`, so it isn't a site",
            ),
            (Reason::OutsideBuilt, "it is outside the built site"),
            (Reason::OutsideNew, "it is outside the new site"),
            (
                Reason::InsideOld("old".into()),
                "it is inside the site being imported, `old`",
            ),
            (
                Reason::HoldsOld("old".into()),
                "the site being imported, `old`, is inside it",
            ),
            (Reason::TopOfDisk, "it is the top of the disk"),
            (Reason::Taken, "something else has its name"),
            (
                Reason::Changed,
                "it changed while Statac was reading the site",
            ),
            (
                Reason::NamedTwice,
                "the same file has another name somewhere",
            ),
            (Reason::NamesNoFolder, "it names no folder"),
            (
                Reason::NoPlaceForId,
                "its front matter has no place for an `id:` line",
            ),
            (
                Reason::Address(Box::new(AddressFault::Unclosed)),
                "`{` is never closed",
            ),
            (Reason::NotToml, "this line doesn't read as TOML"),
            (Reason::NotText, "it isn't text"),
            (Reason::NotForSass, "Sass couldn't read it"),
            (
                Reason::WatchLimit,
                "the system is watching as many files as it allows",
            ),
            (Reason::Denied, "permission denied"),
            (Reason::FolderNamed, "a folder already has that name"),
            (Reason::FileInTheWay, "a file is in the way of a folder"),
            (Reason::DiskFull, "the disk is full"),
            (Reason::Missing, "it doesn't exist"),
            (Reason::ReadOnly, "the disk is read-only"),
            (Reason::NotUtf8, "it isn't UTF-8 text"),
            (Reason::Said("file exists".into()), "file exists"),
        ];
        for (given, words) in cases {
            assert_eq!(reason(&given), words, "{given:?}");
        }
    }

    #[test]
    fn what_refresh_fetched_is_counted_rightly() {
        assert_eq!(
            refreshed(2, 85, 1),
            "Fetched 2 thumbnails. 85 already here. 1 couldn't be fetched."
        );
        assert_eq!(refreshed(1, 0, 0), "Fetched 1 thumbnail.");
        assert_eq!(refreshed(0, 1, 0), "1 already here.");
        assert_eq!(refreshed(0, 0, 0), "Fetched no thumbnails.");
        assert_eq!(
            drafts_not_refreshed(1),
            "Drafts weren't looked at: the site has 1 error with it. Run `statac check --drafts` \
             to see it."
        );
        assert_eq!(
            drafts_not_refreshed(2),
            "Drafts weren't looked at: the site has 2 errors with them. Run `statac check \
             --drafts` to see them."
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
            "Fetched: 87 thumbnails, the oldest 6 days ago."
        );
        assert_eq!(line(1, Some(0), 0), "Fetched: 1 thumbnail, today.");
        assert_eq!(line(1, Some(1), 0), "Fetched: 1 thumbnail, yesterday.");
        assert_eq!(
            line(87, Some(6), 2),
            "Fetched: 87 thumbnails, the oldest 6 days ago; 2 with no date."
        );
        assert_eq!(line(2, None, 2), "Fetched: 2 thumbnails, with no date.");
        // Another part joins after a semicolon.
        assert_eq!(
            fetched_ages(&["1 thumbnail, today".into(), "a part to come".into()]),
            "Fetched: 1 thumbnail, today; a part to come."
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
                    "Write `footer.size: 36`; only a text at `left`, `centre` or `right` is \
                     fitted.",
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
                "Field set aside: `og_image` names this page's social image",
                Some("Take out `card`, or take out `og_image` to have the card drawn."),
            ),
            (
                Kind::WrongKind {
                    field: "og_image".into(),
                    expected: Expected::OwnPicture,
                    found: Found::Written("https://example.org/photo-1.jpg".into()),
                    respelt: None,
                },
                "Wrong kind of value: `og_image` is an image of the site's, not \
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
                "Template stopped: `page.card` isn't answered in a page's content",
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
                Some("Is it the definition for `[^source]`?"),
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
                    on: Braced::Heading,
                },
                "Id used twice: a heading above has the id `rust`",
                Some("Give this heading an id of its own."),
            ),
            (
                Kind::IdUsedTwice {
                    id: "rust".into(),
                    taken: Taken::Table(crate::source::Place {
                        source: crate::source::SourceId::from_index(0),
                        span: crate::source::Span::new(0, 1),
                    }),
                    on: Braced::Table,
                },
                "Id used twice: a table above has the id `rust`",
                Some("Give this table an id of its own."),
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
            (
                Kind::MissingLinkText {
                    href: Some("https://example.org/".into()),
                    image: true,
                },
                "Missing link text: the link to `https://example.org/` has nothing a screen \
                 reader can read out",
                Some("Give the `<img>` inside it an `alt` saying where the link leads."),
            ),
            (
                Kind::MissingLinkText {
                    href: None,
                    image: false,
                },
                "Missing link text: a link has nothing a screen reader can read out",
                Some("Put words inside it, or give it an `aria-label`."),
            ),
            (
                Kind::MissingButtonText { image: true },
                "Missing button text: a `<button>` has nothing a screen reader can read out",
                Some("Give the `<img>` inside it an `alt` saying what the button does."),
            ),
            (
                Kind::MissingButtonText { image: false },
                "Missing button text: a `<button>` has nothing a screen reader can read out",
                Some("Put words inside it, or give it an `aria-label`."),
            ),
            (
                Kind::MissingIframeTitle {
                    src: Some("https://example.org/video/1".into()),
                },
                "Missing iframe title: the `<iframe>` showing `https://example.org/video/1` has \
                 no `title`",
                Some("Give it a `title` saying what it shows."),
            ),
            (
                Kind::MissingIframeTitle { src: None },
                "Missing iframe title: an `<iframe>` has no `title`",
                Some("Give it a `title` saying what it shows."),
            ),
            (
                Kind::MissingLabel {
                    control: "input".into(),
                    name: Some("q".into()),
                    id: None,
                    by: LabelledBy::Label,
                },
                "Missing label: the `<input>` named `q` has no label",
                Some(
                    "Put it inside a `<label>` saying what it is for, or give it an `aria-label`.",
                ),
            ),
            (
                Kind::MissingLabel {
                    control: "select".into(),
                    name: None,
                    id: Some("sort".into()),
                    by: LabelledBy::Label,
                },
                "Missing label: a `<select>` has no label",
                Some("Add `<label for=\"sort\">` saying what it is for."),
            ),
            (
                Kind::MissingLabel {
                    control: "input".into(),
                    name: None,
                    id: None,
                    by: LabelledBy::Value,
                },
                "Missing label: an `<input>` has no label",
                Some("Give it a `value`, the words shown on the button."),
            ),
            (
                Kind::MissingLabel {
                    control: "input".into(),
                    name: Some("go".into()),
                    id: None,
                    by: LabelledBy::Alt,
                },
                "Missing label: the `<input>` named `go` has no label",
                Some("Give it an `alt` saying what the button does."),
            ),
            (
                Kind::MissingTitle { empty: false },
                "Missing title: the page has no `<title>`",
                Some("Add a `<title>` naming the page, such as `<title>{{ page.title }}</title>`."),
            ),
            (
                Kind::MissingTitle { empty: true },
                "Missing title: the page's `<title>` is empty",
                Some("Write the page's name inside it."),
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
        let first = crate::source::Place {
            source: crate::source::SourceId::from_index(0),
            span: crate::source::Span::new(0, 1),
        };
        let not_written = |list, more, why| Kind::FeedNotWritten { list, more, why };
        vec![
            (
                not_written(
                    site.clone(),
                    0,
                    Unwritten::NoTemplate(vec!["templates/feed.xml".into()]),
                ),
                "Feed not written: the site's own list needs `templates/feed.xml`",
                Some("Add the template to publish it, or take out the `feed:` line asking for it."),
            ),
            (
                not_written(
                    site.clone(),
                    0,
                    Unwritten::NoTemplate(vec![
                        "templates/rss.json".into(),
                        "templates/feed.json".into(),
                        "templates/feed.xml".into(),
                    ]),
                ),
                "Feed not written: the site's own list needs `templates/rss.json`, \
                 `templates/feed.json` or `templates/feed.xml`",
                Some("Add the template to publish it, or take out the `feed:` line asking for it."),
            ),
            (
                not_written(
                    (Word::Folder, "posts".into()),
                    2,
                    Unwritten::NoTemplate(vec![
                        "templates/rss.xml".into(),
                        "templates/feed.xml".into(),
                    ]),
                ),
                "Feed not written: the folder `content/posts/` and 2 other lists need \
                 `templates/rss.xml` or `templates/feed.xml`",
                Some(
                    "Add the template to publish them, or take out the `feed:` lines asking \
                     for them.",
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
                    "The page is published there instead; give the feed another URL with \
                     `feed:` on the list page.",
                ),
            ),
            (
                not_written((Word::Folder, "notes".into()), 0, Unwritten::Undated),
                "Feed not written: the folder `content/notes/` has nothing dated",
                Some("Give a page in it a `date:`, or take the `feed:` line out."),
            ),
            (
                not_written((Word::Folder, "posts".into()), 0, Unwritten::NoPage),
                "Feed not written: the folder `content/posts/` has no list page",
                Some(
                    "A feed links to its list's page; add an `index.md` to give the list one, \
                     or take the `feed:` line out.",
                ),
            ),
            (
                not_written((Word::Folder, "posts".into()), 2, Unwritten::NoPage),
                "Feed not written: the folder `content/posts/` and 2 other lists have no list \
                 page",
                Some(
                    "A feed links to its list's page; add an `index.md` to give each list one, \
                     or take the `feed:` line out.",
                ),
            ),
            (
                not_written(
                    (Word::Folder, "notes".into()),
                    0,
                    Unwritten::PageUnpublished,
                ),
                "Feed not written: the folder `content/notes/` has `index_page: false`",
                Some(
                    "A feed links to its list's page; take out `index_page: false` to publish \
                     it, or take the `feed:` line out.",
                ),
            ),
            (
                Kind::FeedWrittenTwice {
                    written: "/posts/feed.xml".into(),
                    first_written: "feed.xml".into(),
                    first,
                    address: "/posts/feed.xml".into(),
                },
                "Feed written twice: `feed.xml` and `/posts/feed.xml` are both at \
                 `/posts/feed.xml`",
                Some("The feed is published once; take one of them out of the `feed:` line."),
            ),
            (
                Kind::FeedWrittenTwice {
                    written: "rss.xml".into(),
                    first_written: "rss.xml".into(),
                    first,
                    address: "/rss.xml".into(),
                },
                "Feed written twice: `rss.xml`",
                Some("The feed is published once; take one of them out of the `feed:` line."),
            ),
            (
                Kind::FeedFormatMismatch {
                    file: "rss.xml".into(),
                    template: "templates/feed.xml".into(),
                    own: "templates/rss.xml".into(),
                    named: FeedFormat::Rss,
                    built: FeedFormat::Atom,
                },
                "Feed format mismatch: `rss.xml` is built as an Atom feed",
                Some(
                    "It is built with `templates/feed.xml`; add `templates/rss.xml` to build it \
                     as an RSS feed.",
                ),
            ),
            (
                Kind::FeedFormatMismatch {
                    file: "atom.xml".into(),
                    template: "templates/atom.xml".into(),
                    own: "templates/atom.xml".into(),
                    named: FeedFormat::Atom,
                    built: FeedFormat::Rss,
                },
                "Feed format mismatch: `atom.xml` is built as an RSS feed",
                Some(
                    "Write `templates/atom.xml` as an Atom feed, or give the feed a name that \
                     doesn't say it is one.",
                ),
            ),
            (
                Kind::UnusedFeedTemplate {
                    template: "templates/feed.xml".into(),
                    line: "feed: true".into(),
                },
                "Unused feed template: no list asks for a feed",
                Some(
                    "Write `feed: true` in `statac.yaml` for the site's feed, built with \
                     `templates/feed.xml`.",
                ),
            ),
            (
                Kind::UnusedLanguageFile {
                    path: "languages/en-US.yaml".into(),
                    languages: vec!["en-GB".into()],
                },
                "Unused language file: no language of the site reads `languages/en-US.yaml`",
                Some("Name it for the site's language, as `languages/en-GB.yaml`, or take it out."),
            ),
            (
                Kind::UnusedLanguageFile {
                    path: "languages/de.yaml".into(),
                    languages: vec!["en".into(), "fr".into()],
                },
                "Unused language file: no language of the site reads `languages/de.yaml`",
                Some("Name it for one of the site's languages, `en` or `fr`, or take it out."),
            ),
            (
                Kind::UnusedLanguageFile {
                    path: "languages/en.yaml".into(),
                    languages: Vec::new(),
                },
                "Unused language file: no language of the site reads `languages/en.yaml`",
                Some(
                    "The site names no `language`; write one in `statac.yaml` for the file to be \
                     read.",
                ),
            ),
            (
                Kind::UnusedFeedTemplate {
                    template: "templates/feed.json".into(),
                    line: "feed: feed.json".into(),
                },
                "Unused feed template: no list asks for a feed",
                Some(
                    "Write `feed: feed.json` in `statac.yaml` for the site's feed, built with \
                     `templates/feed.json`.",
                ),
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
                Kind::NoSiteUrl {
                    waiting: Waiting::Page("content/sitemap.md".into()),
                },
                "No site URL: `content/sitemap.md` is left out until the site has a `url`",
                Some("Add the site's full URL to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::NoSiteUrl {
                    waiting: Waiting::Feeds {
                        list: site.clone(),
                        more: 0,
                        feeds: 1,
                    },
                },
                "No site URL: the feed of the site's own list is left out until the site \
                 has a `url`",
                Some("Add the site's full URL to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::NoSiteUrl {
                    waiting: Waiting::Feeds {
                        list: site.clone(),
                        more: 0,
                        feeds: 2,
                    },
                },
                "No site URL: the feeds of the site's own list are left out until the site \
                 has a `url`",
                Some("Add the site's full URL to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::NoSiteUrl {
                    waiting: Waiting::Feeds {
                        list: (Word::Series, "Series 1".into()),
                        more: 1,
                        feeds: 2,
                    },
                },
                "No site URL: the feeds of the series `Series 1` and 1 other list are left \
                 out until the site has a `url`",
                Some("Add the site's full URL to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::FeedIdChanged {
                    was: "/posts/post-1/".into(),
                    id: "fc370633-88f0-8092-9f96-7749dbfff0eb".into(),
                    item: false,
                },
                "Feed id changed: this post was published at `/posts/post-1/`",
                Some(
                    "Add `id: fc370633-88f0-8092-9f96-7749dbfff0eb` to its front matter to \
                     keep it the same post in feed readers.",
                ),
            ),
            (
                Kind::FeedIdChanged {
                    was: "/books/one/".into(),
                    id: "fc370633-88f0-8092-9f96-7749dbfff0eb".into(),
                    item: true,
                },
                "Feed id changed: this post was published at `/books/one/`",
                Some(
                    "Add `id: fc370633-88f0-8092-9f96-7749dbfff0eb` to the item to keep it the \
                     same post in feed readers.",
                ),
            ),
            (
                Kind::RobotsBeneathPath {
                    top: "https://example.org/robots.txt".into(),
                },
                "Robots beneath a path: crawlers read only `https://example.org/robots.txt`",
                Some(
                    "Copy its lines into the `robots.txt` at the root of the host to have them \
                     read.",
                ),
            ),
            (
                Kind::UnusableUrl {
                    fault: AddressFault::FeedAtFolder {
                        nearest: "/feeds/feed.xml".into(),
                    },
                },
                "Unusable URL: a feed can't be published at a folder",
                Some("Did you mean `/feeds/feed.xml`?"),
            ),
            (
                Kind::UnusableUrl {
                    fault: AddressFault::SlashBesidePage {
                        nearest: "/posts/feeds/atom.xml".into(),
                    },
                },
                "Unusable URL: a name beside the page can't hold `/`",
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
                "Tag spelled two ways: `Rust` (12 pages) and `rust` (2 pages) are one tag, shown \
                 as `Rust`",
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
                Kind::ScheduledPost {
                    path: "content/post-2.md".into(),
                    date: "2026-10-05".into(),
                },
                "Scheduled post: `content/post-2.md` is dated `2026-10-05`",
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
                "Not a post: `content/tags/rust.md` is the list page for the tag `Rust`",
                Some(
                    "A list page is in no list; give it a `permalink` of its own to list it as a post.",
                ),
            ),
            (
                Kind::NotAPost {
                    path: "content/notes.md".into(),
                    word: Word::Folder,
                    name: "notes".into(),
                },
                "Not a post: `content/notes.md` is the list page for the folder `content/notes/`",
                Some(
                    "A list page is in no list; give it a `permalink` of its own to list it as a post.",
                ),
            ),
            (
                Kind::NotAPost {
                    path: "content/home.md".into(),
                    word: Word::Folder,
                    name: String::new(),
                },
                "Not a post: `content/home.md` is the list page for the site's own list",
                Some(
                    "A list page is in no list; give it a `permalink` of its own to list it as a post.",
                ),
            ),
            (
                Kind::RedirectToItself {
                    written: "/post-1/".into(),
                },
                "Redirect to itself: `/post-1/` is this page's own URL",
                Some("Point `redirect_to` at the URL readers should reach."),
            ),
            (
                Kind::RedirectFromItself {
                    written: "/post-1/".into(),
                },
                "Redirect to itself: `/post-1/` is this page's own URL",
                Some("Take it out of `redirect_from`."),
            ),
            (
                Kind::RedirectedBack {
                    written: "/post-2/".into(),
                },
                "Redirect to itself: `/post-2/` redirects back to this page",
                None,
            ),
            (
                Kind::ContentNotPublished {
                    path: "content/post-1.md".into(),
                },
                "Content not published: `content/post-1.md` redirects",
                Some(
                    "Move it to the page it redirects to, or take out `redirect_to` to publish it.",
                ),
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
                "Untitled page: `content/post-1.md` is called by its opening words, `Post 1 \
                 opens\u{2026}`",
                Some(
                    "Give it a `title`, or open its content with a `# heading`. A folder whose \
                     pages go without titles says `pages.untitled: true`.",
                ),
            ),
            (
                Kind::UntitledPage {
                    page: Named::ListPage(Word::Folder, String::new()),
                    by: Called::Address,
                    name: "/".into(),
                },
                "Untitled page: the site's home page is called by its URL, `/`",
                Some("Add `title:` to `statac.yaml`; the home page is called by it."),
            ),
            (
                Kind::SetAside {
                    field: "tags".into(),
                    why: SetAside::SendsOn,
                },
                "Field set aside: a page that redirects publishes only its redirect",
                Some("Take it out, or take out `redirect_to` to publish the page."),
            ),
            (
                Kind::NoListPage {
                    list: (Word::Folder, "notes".into()),
                    more: 0,
                },
                "No list page: `content/notes/` has no `index.md`",
                Some("Add `content/notes/index.md` to publish one."),
            ),
            (
                Kind::NoListPage {
                    list: (Word::Folder, "notes".into()),
                    more: 2,
                },
                "No list page: `content/notes/` and 2 other folders have no `index.md`",
                Some("Add an `index.md` to a folder to publish its list page."),
            ),
            (
                Kind::NoListPage {
                    list: (Word::Tag, "Rust".into()),
                    more: 0,
                },
                "No list page: the tag `Rust` has none",
                Some("Add `tags.pages: true` to `statac.yaml` to publish a page for each tag."),
            ),
            (
                Kind::NoListPage {
                    list: (Word::Tag, "Rust".into()),
                    more: 11,
                },
                "No list page: the tag `Rust` and 11 other tags have none",
                Some("Add `tags.pages: true` to `statac.yaml` to publish a page for each tag."),
            ),
            (
                Kind::NoListPage {
                    list: (Word::Series, "Rust".into()),
                    more: 1,
                },
                "No list page: the series `Rust` and 1 other series have none",
                Some(
                    "Add `series.pages: true` to `statac.yaml` to publish a page for each \
                     series.",
                ),
            ),
            (
                Kind::NoHomePage {
                    addresses: vec!["/".into()],
                    index: "content/index.md".into(),
                },
                "No home page: nothing is published at `/`",
                Some("Add `content/index.md`, or `templates/home.html` to list the posts there."),
            ),
            (
                Kind::NoHomePage {
                    addresses: vec!["/en/".into(), "/fr/".into()],
                    index: "content/en/index.md".into(),
                },
                "No home page: nothing is published at `/en/` or `/fr/`",
                Some(
                    "Add an `index.md` at the top of each language, or `templates/home.html` to \
                     list the posts there.",
                ),
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
            "Included 1 draft and 1 scheduled post as if published."
        );
        assert_eq!(
            included(0, 3),
            "Included 3 scheduled posts as if published."
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
                    "Give each item a name with its fields beneath, or declare the field as \
                     `text`.",
                ),
            ),
            (
                Kind::MissingField {
                    field: "permalink".into(),
                    required_by: Requirer::Each,
                },
                "Missing field: `permalink`",
                Some(
                    "Add a `permalink:` pattern, such as `/people/{key}/`, to give each item its \
                     URL.",
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
                Kind::UnusableUrl {
                    fault: AddressFault::ItemLacks {
                        item: "data/people.yaml: item 3".into(),
                        field: "isbn".into(),
                        piece: "{isbn}".into(),
                    },
                },
                "Unusable URL: `data/people.yaml: item 3` has no `isbn` for `{isbn}`",
                Some("Add `isbn:` to the item, or take `{isbn}` out of the pattern."),
            ),
            (
                Kind::UnusableUrl {
                    fault: AddressFault::ItemLacks {
                        item: "data/people.yaml: item 3".into(),
                        field: "key".into(),
                        piece: "{key}".into(),
                    },
                },
                "Unusable URL: `data/people.yaml: item 3` has no `key` for `{key}`",
                Some("Give the table's items names, or take `{key}` out of the pattern."),
            ),
            (
                Kind::UnusableUrl {
                    fault: AddressFault::PostLacks {
                        post: "content/post-1.md".into(),
                        field: "isbn".into(),
                        piece: "{isbn}".into(),
                    },
                },
                "Unusable URL: `content/post-1.md` has no `isbn` for `{isbn}`",
                Some("Add `isbn:` to the post, or take `{isbn}` out of the pattern."),
            ),
            (
                Kind::UnusableUrl {
                    fault: AddressFault::PageLacks {
                        page: "content/a.md".into(),
                        field: "isbn".into(),
                        piece: "{isbn}".into(),
                    },
                },
                "Unusable URL: `content/a.md` has no `isbn` for `{isbn}`",
                Some("Add `isbn:` to the page, or take `{isbn}` out of the pattern."),
            ),
            (
                Kind::MissingField {
                    field: "permalink".into(),
                    required_by: Requirer::EachPost,
                },
                "Missing field: `permalink`",
                Some(
                    "Add a `permalink:` pattern, such as `/cards/{name}/`, to give the page made \
                     for each post its URL.",
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
        let same = |alike| Kind::SameUrl {
            first: "static/Logo.png".into(),
            second: "static/logo.png".into(),
            alike,
            address: "/logo.png".into(),
        };
        let at = crate::source::Place {
            source: crate::source::SourceId::from_index(0),
            span: crate::source::Span::new(0, 1),
        };
        let page = |path: &str| Named::File(path.into());
        let redirect = |path: &str| Named::Redirect {
            page: path.into(),
            at,
        };
        let permalink = |path: &str| Named::Permalink {
            page: path.into(),
            at,
        };
        let clash = |first, second, alike| Kind::SameUrl {
            first: Box::new(first),
            second: Box::new(second),
            alike,
            address: "/a/index.html".into(),
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
                    fault: SassFault::NoVariable("$link".into()),
                },
                "Sass stopped: no variable called `$link`",
                None,
            ),
            (
                Kind::SassStopped {
                    fault: SassFault::Raised("Something is wrong.".into()),
                },
                "Sass stopped: `Something is wrong.`",
                None,
            ),
            (
                Kind::SassStopped {
                    fault: SassFault::NoNamespace {
                        name: "math".into(),
                        built_in: true,
                    },
                },
                "Sass stopped: no module called `math`",
                Some("Add `@use \"sass:math\";` at the top of the file."),
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
                Kind::SameUrl {
                    first: "assets/site.css".into(),
                    second: "assets/site.scss".into(),
                    alike: Alike::Exactly,
                    address: "/site.css".into(),
                },
                "Same URL: `assets/site.css` and `assets/site.scss` are both published at \
                 `/site.css`",
                Some("Rename one of them."),
            ),
            (
                Kind::LaterPageTaken {
                    list: "/blog/".into(),
                    number: 2,
                    page: Box::new("content/blog/page/2.md".into()),
                    page_at: None,
                    address: "/blog/page/2/".into(),
                },
                "Same URL: page 2 of `/blog/` and `content/blog/page/2.md` are both \
                 published at `/blog/page/2/`",
                Some(
                    "Give `content/blog/page/2.md` another URL, or put the paginated \
                     pages elsewhere with `pagination.permalink` in `statac.yaml`.",
                ),
            ),
            (
                Kind::UrlTaken {
                    held: Held::Draft,
                    address: "/post-1/".into(),
                    taken: "content/post-2.md".into(),
                },
                "URL taken: this draft would be published at `/post-1/`, as \
                 `content/post-2.md` is",
                Some("Give one of them another `permalink` before it is published."),
            ),
            (
                Kind::UrlTaken {
                    held: Held::Waiting,
                    address: "/post-1/".into(),
                    taken: "content/post-2.md".into(),
                },
                "URL taken: this post would be published at `/post-1/`, as \
                 `content/post-2.md` is",
                Some("Give one of them another `permalink` before its scheduled date."),
            ),
            (
                Kind::UrlTaken {
                    held: Held::Draft,
                    address: "/post-1/".into(),
                    taken: Box::new(redirect("content/post-2.md")),
                },
                "URL taken: this draft would be published at `/post-1/`, as a redirect in \
                 `content/post-2.md` is",
                Some(
                    "Take it out of `redirect_from` in `content/post-2.md`, or give this draft \
                     a `permalink`, before it is published.",
                ),
            ),
            (
                Kind::UrlGone {
                    address: "/about/".into(),
                    written: "/about/".into(),
                },
                "URL gone: `/about/` was published before, and nothing is there now",
                Some(
                    "Redirect it with `redirect_from: [/about/]` in the page that took its \
                     place, or take it out of `published.txt` if it is gone for good.",
                ),
            ),
            (
                Kind::AddressesGone {
                    under: "/blog/notes/".into(),
                    written: "/notes/".into(),
                    gone: vec!["/notes/".into(), "/notes/note-1/".into()],
                },
                "URLs gone: the 2 URLs under `/blog/notes/` were published before, and \
                 nothing is at any of them now",
                Some(
                    "Redirect them with `redirect_from` in the pages that took their places, or \
                     take them out of `published.txt` if they are gone for good.",
                ),
            ),
            (
                Kind::UrlGone {
                    address: "/blog/about/".into(),
                    written: "/about/".into(),
                },
                "URL gone: `/blog/about/` was published before, and nothing is there now",
                Some(
                    "Redirect it with `redirect_from: [/about/]` in the page that took its \
                     place, or take it out of `published.txt` if it is gone for good.",
                ),
            ),
            (
                same(Alike::ByCase),
                "Same URL: `static/Logo.png` and `static/logo.png` differ only in capitals",
                Some("Most disks hold them as one file, so rename one of them."),
            ),
            (
                same(Alike::ByNormalisation),
                "Same URL: `static/Logo.png` and `static/logo.png` differ only in how an \
                 accent is stored",
                Some("Most disks hold them as one file, so rename one of them."),
            ),
            (
                clash(
                    page("content/a.md"),
                    redirect("content/b.md"),
                    Alike::Exactly,
                ),
                "Same URL: `content/a.md` and a redirect in `content/b.md` are both \
                 published at `/a/index.html`",
                Some("Take it out of `redirect_from` in `content/b.md`."),
            ),
            (
                clash(
                    redirect("content/a.md"),
                    redirect("content/b.md"),
                    Alike::Exactly,
                ),
                "Same URL: a redirect in `content/a.md` and a redirect in `content/b.md` \
                 are both published at `/a/index.html`",
                Some("Take it out of the `redirect_from` of one of them."),
            ),
            (
                clash(
                    permalink("content/a.md"),
                    redirect("content/b.md"),
                    Alike::ByCase,
                ),
                "Same URL: `content/a.md` and a redirect in `content/b.md` differ only in \
                 capitals",
                Some(
                    "Most disks hold them as one file, so take it out of `redirect_from` in \
                     `content/b.md`.",
                ),
            ),
            (
                clash(
                    permalink("content/a.md"),
                    page("content/b.md"),
                    Alike::Exactly,
                ),
                "Same URL: `content/a.md` and `content/b.md` are both published at \
                 `/a/index.html`",
                Some("Change the `permalink` of `content/a.md`."),
            ),
            (
                clash(
                    page("static/a.html"),
                    permalink("content/b.md"),
                    Alike::Exactly,
                ),
                "Same URL: `static/a.html` and `content/b.md` are both published at \
                 `/a/index.html`",
                Some("Change the `permalink` of `content/b.md`."),
            ),
            (
                clash(
                    permalink("content/a.md"),
                    permalink("content/b.md"),
                    Alike::Exactly,
                ),
                "Same URL: `content/a.md` and `content/b.md` are both published at \
                 `/a/index.html`",
                Some("Change the `permalink` of one of them."),
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
                    reason: Reason::Denied,
                },
                "Can't read `assets/site.css`: permission denied",
                None,
            ),
            (
                Kind::UnreadableImage {
                    path: "content/photo-1.jpg".into(),
                    fault: PictureFault::CutShort,
                },
                "Unreadable image: `content/photo-1.jpg` is cut short or damaged",
                Some("Copy or export the file again."),
            ),
            (
                Kind::UnreadableImage {
                    path: "content/photo-1.jpg".into(),
                    fault: PictureFault::Unsupported("JPEG without a frame header".into()),
                },
                "Unreadable image: `content/photo-1.jpg` is saved in a form Statac doesn't read",
                Some(
                    "To publish it as the file it is, write `optimise: false` in \
                     `content/photo-1.jpg.yaml`.",
                ),
            ),
            (
                Kind::UnreadableImage {
                    path: "static/photo-1.avif".into(),
                    fault: PictureFault::Unsupported("AVIF without an image size".into()),
                },
                "Unreadable image: `static/photo-1.avif` is saved in a form Statac doesn't read",
                None,
            ),
            (
                Kind::UnreadableImage {
                    path: "content/photo-1.jpg".into(),
                    fault: PictureFault::TooLarge {
                        width: 40000,
                        height: 30000,
                    },
                },
                "Unreadable image: `content/photo-1.jpg` is 40,000 by 30,000 pixels, too large \
                 to open",
                Some(
                    "To publish it as the file it is, write `optimise: false` in \
                     `content/photo-1.jpg.yaml`.",
                ),
            ),
            (
                Kind::UnreadableImage {
                    path: "static/photo-1.jpg".into(),
                    fault: PictureFault::CantMake("too large for JPEG".into()),
                },
                "Unreadable image: no resized copy of `static/photo-1.jpg` could be made: too \
                 large for JPEG",
                None,
            ),
            (
                Kind::ColourConverted {
                    path: "content/photo-1.jpg".into(),
                    profile: Some("Display P3".into()),
                },
                "Colour converted: the resized copies of `content/photo-1.jpg` are in sRGB, \
                 converted from `Display P3`",
                None,
            ),
            (
                Kind::ColourConverted {
                    path: "content/photo-1.jpg".into(),
                    profile: None,
                },
                "Colour converted: the resized copies of `content/photo-1.jpg` are in sRGB, \
                 converted from its colour profile",
                None,
            ),
            (
                Kind::DescribesNoImage {
                    picture: "content/photo-1.jpg".into(),
                    nearest: Some("photo-1.jpeg".into()),
                },
                "Describes no image: there is no `content/photo-1.jpg`",
                Some("Did you mean `photo-1.jpeg`?"),
            ),
            (
                Kind::DescribesNoImage {
                    picture: "content/photo-1.jpg".into(),
                    nearest: None,
                },
                "Describes no image: there is no `content/photo-1.jpg`",
                None,
            ),
            (
                Kind::CacheNotWritten {
                    path: ".statac/pictures".into(),
                    reason: Reason::Denied,
                },
                "Cache not written: can't write `.statac/pictures`: permission denied",
                None,
            ),
            (
                Kind::MissingAltText {
                    picture: "photo-1.jpg".into(),
                    shown: ShownBy::Markdown {
                        described_beside: true,
                    },
                    all_of: None,
                },
                "Missing alt text: `photo-1.jpg`",
                Some("Describe it in the brackets, or mark it decorative."),
            ),
            (
                Kind::MissingAltText {
                    picture: "https://example.org/photo-1.jpg".into(),
                    shown: ShownBy::Markdown {
                        described_beside: false,
                    },
                    all_of: None,
                },
                "Missing alt text: `https://example.org/photo-1.jpg`",
                Some("Describe it in the brackets."),
            ),
            (
                Kind::MissingAltText {
                    picture: "photo-1.jpg".into(),
                    shown: ShownBy::Html,
                    all_of: None,
                },
                "Missing alt text: `photo-1.jpg`",
                Some("Give the `<img>` an `alt`, or `alt=\"\"` where it is decorative."),
            ),
            (
                Kind::MissingAltText {
                    picture: "content/photo-1.jpg".into(),
                    shown: ShownBy::Template {
                        beside: Some("content/photo-1.jpg.yaml".into()),
                    },
                    all_of: None,
                },
                "Missing alt text: `content/photo-1.jpg`",
                Some(
                    "Describe it with `alt:` in `content/photo-1.jpg.yaml`, or mark it \
                     decorative.",
                ),
            ),
            (
                Kind::MissingAltText {
                    picture: "static/photo-1.jpg".into(),
                    shown: ShownBy::Template { beside: None },
                    all_of: None,
                },
                "Missing alt text: `static/photo-1.jpg`",
                Some("Describe it where it is shown, or mark it decorative."),
            ),
            (
                Kind::HeicImage {
                    path: "content/photo-1.heic".into(),
                },
                "HEIC image: `content/photo-1.heic` is published as it is, with no resized copies \
                 made",
                Some("Export it as a JPEG: most browsers can't show a HEIC image."),
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
                Kind::ImagesOutOfProportion {
                    property: "max-width".into(),
                },
                "Images out of proportion: `img` has a `max-width` and no `height`",
                Some("Add `height: auto` to the rule."),
            ),
            (
                Kind::ImagesOutOfProportion {
                    property: "width".into(),
                },
                "Images out of proportion: `img` has a `width` and no `height`",
                Some("Add `height: auto` to the rule."),
            ),
            (
                Kind::ImagesOutOfProportion {
                    property: "max-inline-size".into(),
                },
                "Images out of proportion: `img` has a `max-inline-size` and no `block-size`",
                Some("Add `block-size: auto` to the rule."),
            ),
            (
                Kind::ImagesOutOfProportion {
                    property: "inline-size".into(),
                },
                "Images out of proportion: `img` has an `inline-size` and no `block-size`",
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
                        field: None,
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
                        font: Some(Box::new("NotoEmoji.ttf".into())),
                        field: None,
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
                        characters: vec!['😀'],
                        more: 0,
                        font: Some(Box::new("NotoEmoji.ttf".into())),
                        field: Some(Box::new("title.font".into())),
                    },
                    pages: 1,
                    first: Named::File("content/post-1.md".into()),
                },
                "Card not drawn: `content/post-1.md` needs `😀`, which no font in \
                 `title.font` has",
                Some("Add `NotoEmoji.ttf`, which has it, to `title.font`."),
            ),
            (
                Kind::CardNotDrawn {
                    why: NotDrawn::NoFont {
                        characters: vec!['→', '✓'],
                        more: 0,
                        font: None,
                        field: None,
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
                        field: None,
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
                Kind::NoSiteUrl {
                    waiting: Waiting::Cards,
                },
                "No site URL: social cards are left out until the site has a `url`",
                Some("Add the site's full URL to `statac.yaml`, as `url: https://example.org/`."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "content/cover.webp".into(),
                    holds: Some(crate::pictures::Kind::WebP),
                    pages: 1,
                },
                "Social image may not show: `content/cover.webp` is a WebP",
                Some("For an image more link previews show, save it as a PNG."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "static/shared/cover.svg".into(),
                    holds: Some(crate::pictures::Kind::Svg),
                    pages: 3,
                },
                "Social image may not show: the `static/shared/cover.svg` of 3 pages is an SVG",
                Some("For an image more link previews show, save it as a PNG."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "content/cover.gif".into(),
                    holds: Some(crate::pictures::Kind::Gif),
                    pages: 1,
                },
                "Social image may not show: `content/cover.gif` is a GIF",
                Some("For an image more link previews show, save it as a PNG."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "content/cover.bmp".into(),
                    holds: None,
                    pages: 1,
                },
                "Social image may not show: `content/cover.bmp` is in a format some link previews \
                 don't show",
                Some("For an image more link previews show, save it as a PNG."),
            ),
            (
                Kind::ImageMayNotShow {
                    path: "content/cover.bmp".into(),
                    holds: None,
                    pages: 1200,
                },
                "Social image may not show: the `content/cover.bmp` of 1,200 pages is in a format \
                 some link previews don't show",
                Some("For an image more link previews show, save it as a PNG."),
            ),
            (
                Kind::CantRemove {
                    path: "._site.statac-old".into(),
                    reason: Reason::Denied,
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
                    why: Unheard::System(Reason::Denied),
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
                call(CallFault::Unclosed("@image(".into())),
                "Malformed call: `@image(` is never closed",
                Some("Close it with `)`."),
            ),
            (
                call(CallFault::UnclosedQuote('\'')),
                "Malformed call: `'` is never closed",
                Some("End the value with another `'` on the same line."),
            ),
            (
                call(CallFault::TextAfterQuote),
                "Malformed call: text after the closing quote",
                Some("Put the whole value inside the quotes."),
            ),
            (
                call(CallFault::Name("café".into())),
                "Malformed call: a snippet can't be called `café`",
                Some(
                    "Name the snippet with the letters a to z, digits and `_`, or write `\\@` to \
                     show this as text.",
                ),
            ),
            (
                call(CallFault::NoEnd("note".into())),
                "Malformed call: `@note` has no `@end`",
                Some("Put `@end` on a line of its own where the content it wraps ends."),
            ),
            (
                call(CallFault::StrayEnd),
                "Malformed call: `@end` closes nothing",
                Some(
                    "Take the line out: only a call to a snippet that shows `body` wraps the \
                     content up to an `@end`.",
                ),
            ),
            (
                call(CallFault::WrapsNothing("note".into())),
                "Malformed call: `@note` isn't on a line of its own",
                Some(
                    "Write the call on a line of its own, the content it wraps below it, and \
                     `@end` on a line after that.",
                ),
            ),
            (
                call(CallFault::NotOneAddress),
                "Malformed call: `@url` takes one URL starting with `/`",
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
                    "This is usually a character such as \u{2019} saved in an old encoding: type \
                     it again.",
                ),
            ),
            (
                Kind::UnknownProblemName {
                    written: "brokn-link".into(),
                    suggestion: Some("broken-link".into()),
                    former: Former::Nothing,
                },
                "Unknown problem name: `brokn-link`",
                Some("Did you mean `broken-link`?"),
            ),
            (
                Kind::UnknownProblemName {
                    written: "undescribed-picture".into(),
                    suggestion: Some("missing-alt-text".into()),
                    former: Former::Renamed,
                },
                "Unknown problem name: `undescribed-picture`",
                Some("Statac calls this `missing-alt-text`."),
            ),
            (
                Kind::UnknownProblemName {
                    written: "missing-url".into(),
                    suggestion: None,
                    former: Former::Retired,
                },
                "Unknown problem name: `missing-url`",
                Some("This kind of problem no longer exists."),
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
                    hint: CommandHint::Similar("build".into()),
                },
                "Unknown command: `biuld`",
                Some("Did you mean `statac build`?"),
            ),
            (
                Kind::UnknownCommand {
                    written: "post-1".into(),
                    hint: CommandHint::None,
                },
                "Unknown command: `post-1`",
                Some("Run `statac --help` to see the commands."),
            ),
            (
                Kind::UnknownCommand {
                    written: "refuse".into(),
                    hint: CommandHint::Setting("refuse_webmentions_from".into()),
                },
                "Unknown command: `refuse`",
                Some("Statac does this with `refuse_webmentions_from:` in `statac.yaml`."),
            ),
            (
                Kind::MissingCommand,
                "Missing command: no `build`, `check`, `serve`, `stamp`, `refresh`, `new`, \
                 `import` or `explain`",
                Some("Run `statac --help` to see the commands."),
            ),
            (
                Kind::UnknownOption {
                    written: "--otu=public".into(),
                    hint: OptionHint::Similar("--out=public".into()),
                },
                "Unknown option: `--otu=public`",
                Some("Did you mean `--out=public`?"),
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
                    written: "--url=https://example.org".into(),
                    hint: OptionHint::ForOther {
                        command: "explain".into(),
                        option: "--url".into(),
                        takers: Box::new(["build", "check"]),
                    },
                },
                "Unknown option: `statac explain` has no `--url`",
                Some("`--url` is for `statac build` and `statac check`."),
            ),
            (
                Kind::UnknownOption {
                    written: "--drafts".into(),
                    hint: OptionHint::Takes {
                        command: "build".into(),
                        options: Box::new(["--out", "--url"]),
                    },
                },
                "Unknown option: `--drafts`",
                Some("`statac build` takes `--out` and `--url`."),
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
                    written: "--update".into(),
                    hint: OptionHint::Retired,
                },
                "Unknown option: `--update`",
                Some(
                    "Statac no longer writes templates into a site: see \
                     https://statac.dev/templates/.",
                ),
            ),
            (
                Kind::UnknownOption {
                    written: "--no-starter".into(),
                    hint: OptionHint::Retired,
                },
                "Unknown option: `--no-starter`",
                Some("Statac no longer writes templates into a site, so leave `--no-starter` out."),
            ),
            (
                Kind::UnknownOption {
                    written: "--site=docs".into(),
                    hint: OptionHint::FolderAfterCommand {
                        command: Some("build".into()),
                        folder: Some("docs".into()),
                    },
                },
                "Unknown option: `--site=docs`",
                Some("Name the folder after the command, as `statac build docs`."),
            ),
            (
                Kind::UnknownOption {
                    written: "--site".into(),
                    hint: OptionHint::FolderAfterCommand {
                        command: Some("check".into()),
                        folder: None,
                    },
                },
                "Unknown option: `--site`",
                Some("Name the folder after the command, as `statac check <folder>`."),
            ),
            (
                Kind::UnknownOption {
                    written: "--site=docs".into(),
                    hint: OptionHint::FolderAfterCommand {
                        command: None,
                        folder: Some("docs".into()),
                    },
                },
                "Unknown option: `--site=docs`",
                Some("Name the folder after the command, as `statac <command> docs`."),
            ),
            (
                Kind::UnknownOption {
                    written: "--site=docs".into(),
                    hint: OptionHint::FolderAfterCommand {
                        command: Some("search".into()),
                        folder: Some("docs".into()),
                    },
                },
                "Unknown option: `--site=docs`",
                Some("Name the folder after the words, as `statac search <words> docs`."),
            ),
            (
                Kind::UnknownOption {
                    written: "--drafts".into(),
                    hint: OptionHint::ShownAlready,
                },
                "Unknown option: `statac serve` has no `--drafts`",
                Some(
                    "`statac serve` shows drafts and scheduled posts already. \
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
                    written: "--out".into(),
                    option: "--out".into(),
                    value: Some("public".into()),
                },
                "Missing value: `--out` needs a folder",
                Some("Write it as `--out=public`."),
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
                "Missing value: `--url` needs a full URL",
                Some("Write it as `--url=<url>`."),
            ),
            (
                Kind::OptionGivenTwice {
                    option: "--out".into(),
                },
                "Option given twice: `--out`",
                Some("Keep one of them."),
            ),
            (
                Kind::UnexpectedArgument {
                    written: "public".into(),
                },
                "Unexpected argument: `public`",
                None,
            ),
            (
                Kind::MissingName,
                "Missing name: `statac explain` needs the name of a problem",
                Some("The name is the last part of a problem's `More:` link."),
            ),
            (
                Kind::NothingToSearch { site: None },
                "Nothing to search for: `statac search` needs the words to look for",
                Some("Give them after the command, as `statac search \"raspberry pi\"`."),
            ),
            (
                Kind::NothingToSearch {
                    site: Some("sites/blog".into()),
                },
                "Nothing to search for: `sites/blog` is a site's folder, not words",
                Some(
                    "Give the words first, then the folder, as `statac search \"raspberry pi\" sites/blog`.",
                ),
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
                None,
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
        let unusable = |fault| Kind::UnusableUrl { fault };
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
                "Unknown placeholder: `:year`",
                Some("Statac calls this `{year}`."),
            ),
            (
                Kind::CantRead {
                    path: "content/post-1.md".into(),
                    reason: Reason::NotUtf8,
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
                Kind::LoopInForIf { word: Word::Name },
                "Unknown name: `loop`",
                Some("`loop` is set only inside the loop: test it with an `{% if %}` there."),
            ),
            (
                Kind::LoopInForIf {
                    word: Word::Snippet,
                },
                "Unknown snippet: `loop`",
                Some("`loop` is set only inside the loop: test it with an `{% if %}` there."),
            ),
            (
                Kind::UnknownWord {
                    word: Word::Name,
                    written: "loop".into(),
                    nearest: None,
                },
                "Unknown name: `loop`",
                None,
            ),
            (
                Kind::ReadAsCustom {
                    own: Own::Field("titel".into()),
                    written: "titel".into(),
                    nearest: "title".into(),
                },
                "Read as a custom field: `titel`",
                Some(
                    "Did you mean `title`? To keep `titel`, declare it in `statac.yaml` as \
                     `fields.titel: text`.",
                ),
            ),
            (
                Kind::ReadAsCustom {
                    own: Own::Field("titel".into()),
                    written: "pages.titel".into(),
                    nearest: "pages.title".into(),
                },
                "Read as a custom field: `pages.titel`",
                Some(
                    "Did you mean `pages.title`? To keep `pages.titel`, declare it in \
                     `statac.yaml` as `fields.titel: text`.",
                ),
            ),
            (
                Kind::ReadAsCustom {
                    own: Own::Setting,
                    written: "posts_per_pge".into(),
                    nearest: "posts_per_page".into(),
                },
                "Read as a custom setting: `posts_per_pge`",
                Some("Did you mean `posts_per_page`?"),
            ),
            (
                Kind::ReadAsCustom {
                    own: Own::Type,
                    written: "repyl".into(),
                    nearest: "reply".into(),
                },
                "Read as a custom type: `repyl`",
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
                Some("Write `permalink:` to choose the page's URL."),
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
                wrong("id", Expected::Id, Found::Written("nope".into()), None),
                "Wrong kind of value: `id` is six letters and digits, such as `k3m9x2`, or a \
                 UUID in small letters, not `nope`",
                None,
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
                "Wrong kind of value: `neighbours` is `site`, `folder`, `series` or `false`, not \
                 `no`",
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
                "Wrong place: `pages.title` can't cascade",
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
                "Wrong place: `pages.layout` cascades only from a folder's `index.md`",
                Some("Move it to the folder's `index.md`."),
            ),
            (
                Kind::WrongPlace {
                    key: "pages.index_page".into(),
                    why: WrongPlace::WholeSite,
                },
                "Wrong place: `pages.index_page` can't cascade from `statac.yaml`",
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
                Kind::WrongPlace {
                    key: "dates".into(),
                    why: WrongPlace::LanguageFile,
                },
                "Wrong place: a language file can't set `dates`",
                Some("Set how dates look with `date_format:` in `statac.yaml` instead."),
            ),
            (
                Kind::UnknownWord {
                    word: Word::Line,
                    written: "monts".into(),
                    nearest: Some("months".into()),
                },
                "Unknown line: `monts`",
                Some("Did you mean `months`?"),
            ),
            (
                unusable(AddressFault::UnknownPiece {
                    written: "{yaer}".into(),
                    nearest: Some("{year}".into()),
                }),
                "Unusable URL: there is no placeholder `{yaer}`",
                Some("Did you mean `{year}`?"),
            ),
            (
                unusable(AddressFault::Unclosed),
                "Unusable URL: `{` is never closed",
                Some("Close it with `}`."),
            ),
            (
                unusable(AddressFault::Stray),
                "Unusable URL: `}` closes nothing",
                Some("Leave it out of the URL."),
            ),
            (
                unusable(AddressFault::Character(':')),
                "Unusable URL: `:` can't be in a name",
                Some("Leave it out of the URL."),
            ),
            (
                unusable(AddressFault::Character('\\')),
                "Unusable URL: `\\` can't be in a name",
                Some("Separate folders with `/`."),
            ),
            (
                unusable(AddressFault::Dots("..".into())),
                "Unusable URL: `..` can't be a name",
                None,
            ),
            (
                unusable(AddressFault::TrailingSpaceOrDot('.')),
                "Unusable URL: a name can't end with `.`",
                Some("Take it off: Windows drops it, and links would miss the page."),
            ),
            (
                unusable(AddressFault::TrailingSpaceOrDot(' ')),
                "Unusable URL: a name can't end with a space",
                Some("Take it off: Windows drops it, and links would miss the page."),
            ),
            (
                unusable(AddressFault::Device("con".into())),
                "Unusable URL: `con` is kept for a device on Windows",
                None,
            ),
            (
                unusable(AddressFault::Percent),
                "Unusable URL: `%` can't be in a name",
                Some("Write the character it stands for instead."),
            ),
            (
                unusable(AddressFault::Hash),
                "Unusable URL: `#` can't be in a name",
                Some("Leave out the `#` and what follows it."),
            ),
            (
                unusable(AddressFault::Control),
                "Unusable URL: a control character can't be in a name",
                Some("Leave it out of the URL."),
            ),
            (
                unusable(AddressFault::NoDate {
                    piece: "{year}".into(),
                }),
                "Unusable URL: this page has no date for `{year}`",
                Some(
                    "Add a `date:` line, or start the file name with a date such as `2026-10-04-`.",
                ),
            ),
            (
                unusable(AddressFault::FillsNothing {
                    pattern: "/blog/".into(),
                }),
                "Unusable URL: `/blog/` puts every page at one URL",
                Some("Add a placeholder such as `{slug}` so each page has its own."),
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
                    looks: false,
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
                    looks: false,
                }),
                "Unusable date format: nothing in `dd/mm/yyyy` is in `2025-12-31 15:04`",
                Some("Write `2025-12-31 15:04` the way dates should look, such as `2025-12-31`."),
            ),
            (
                unusable(FormatFault::NoPattern {
                    example: "lnog".into(),
                    reference: reference(),
                    examples: examples(),
                    looks: true,
                }),
                "Unusable date format: nothing in `lnog` is in `Wednesday 31 December 2025, 3.04pm`",
                Some(
                    "Write `Wednesday 31 December 2025, 3.04pm` the way dates should look, such as \
                     `31/12/2025` or `31 December 2025`, or name a look: `long`, `short`, `day` \
                     or `stamp`.",
                ),
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
                unusable(FormatFault::English {
                    written: "December".into(),
                    corrected: Some("diciembre 2025".into()),
                }),
                "Unusable date format: `December` is in English",
                Some("Did you mean `diciembre 2025`?"),
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
                    ended: Ended::NotStarted(Reason::Denied),
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
                    key: "after_build_outputs".into(),
                    why: WrongPlace::NoCommand("after_build"),
                },
                "Wrong place: `after_build_outputs` is for a site that sets `after_build`",
                Some("Add the `after_build:` line it describes, or take it out."),
            ),
            (
                Kind::SameUrl {
                    first: "static/search/index.js".into(),
                    second: Box::new(Named::MadeAfter("search/".into())),
                    alike: Alike::Exactly,
                    address: "/search/index.js".into(),
                },
                "Same URL: `static/search/index.js` and what `after_build` makes at \
                 `search/` are both published at `/search/index.js`",
                Some("Rename one of them."),
            ),
            (
                Kind::UnknownLanguage { tag: "sw".into() },
                "Unknown language: Statac has no translations for `sw`",
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
                "Missing words: the footnotes of 1 page in `fr` have no translation for the back \
                 link",
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
                Kind::RetiredSetting {
                    setting: "code_style".into(),
                    in_place: InPlace::CodeColours {
                        theme: Some("ocean".into()),
                    },
                },
                "Retired setting: Statac no longer reads `code_style`",
                Some(
                    "Write `code_colours: true` in its place, and copy the `ocean` theme's rules \
                     from https://statac.dev/code-colours/ into a stylesheet of the site's own.",
                ),
            ),
            (
                Kind::RetiredSetting {
                    setting: "code_style".into(),
                    in_place: InPlace::CodeColours { theme: None },
                },
                "Retired setting: Statac no longer reads `code_style`",
                Some(
                    "Write `code_colours: true` in its place, and copy a theme's rules from \
                     https://statac.dev/code-colours/ into a stylesheet of the site's own.",
                ),
            ),
            (
                Kind::RetiredSetting {
                    setting: "code_style".into(),
                    in_place: InPlace::Nothing,
                },
                "Retired setting: Statac no longer reads `code_style`",
                Some("Delete the line: Statac colours no code unless `code_colours: true` asks."),
            ),
            (
                Kind::MissingFile {
                    path: "assets/code.css".into(),
                    hint: Some(Hint::NoLongerWritten),
                },
                "Missing file: there is no `assets/code.css`",
                Some(
                    "Statac no longer writes `code.css`. Coloured code needs \
                     `code_colours: true` and a theme's rules from \
                     https://statac.dev/code-colours/ in a stylesheet of the site's own.",
                ),
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
                    found: TemplateFault::Syntax(SyntaxFault::Unclosed(Opener::Block("if".into()))),
                },
                "Malformed template: `{% if %}` is never closed",
                Some("Close it with `{% endif %}`."),
            ),
            (
                Kind::MalformedTemplate {
                    found: TemplateFault::Syntax(SyntaxFault::Engine(
                        "Too many arguments in function call".into(),
                    )),
                },
                "Malformed template: too many arguments in function call",
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
                    chosen: Chooser::Made,
                    nearest: None,
                },
                "Missing template: there is no `templates/folder.html`",
                Some("Add the template, or a page of your own at the same URL."),
            ),
            (
                Kind::MissingTemplate {
                    path: "templates/page.html".into(),
                    chosen: Chooser::Default,
                    nearest: None,
                },
                "Missing template: there is no `templates/page.html`",
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
                Kind::BuiltWithPageTemplate {
                    path: "templates/home.html".into(),
                    pages: 1,
                },
                "Built with page template: `templates/page.html` builds 1 page in place of \
                 `templates/home.html`",
                Some("Add `templates/home.html` to give it a template of its own."),
            ),
            (
                Kind::BuiltWithPageTemplate {
                    path: "templates/folder.html".into(),
                    pages: 2,
                },
                "Built with page template: `templates/page.html` builds 2 pages in place of \
                 `templates/folder.html`",
                Some("Add `templates/folder.html` to give them a template of their own."),
            ),
            (
                Kind::BuiltWithoutTemplate {
                    redirects: false,
                    count: 2,
                },
                "Built without template: 2 pages are published as their content alone",
                Some("Add `templates/page.html` to build them with a template."),
            ),
            (
                Kind::BuiltWithoutTemplate {
                    redirects: false,
                    count: 1,
                },
                "Built without template: 1 page is published as its content alone",
                Some("Add `templates/page.html` to build it with a template."),
            ),
            (
                Kind::BuiltWithoutTemplate {
                    redirects: true,
                    count: 1,
                },
                "Built without template: 1 redirect is published as a refresh and a link",
                Some("Add `templates/redirect.html` to build it with a template."),
            ),
            (
                Kind::BuiltWithoutTemplate {
                    redirects: true,
                    count: 1_200,
                },
                "Built without template: 1,200 redirects are published as a refresh and a link",
                Some("Add `templates/redirect.html` to build them with a template."),
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
                    what: Unsupported::NotYet("page.image".into()),
                },
                "Not supported: `page.image` isn't available yet",
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
                asked("page.image", Needed::Name(Sort::Picture)),
                "Wrong argument: `page.image` needs the name of one image",
                Some("Write it as `page.image(\"photo-1.jpg\")`."),
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
                asked("page.images", Needed::Names),
                "Wrong argument: `page.images` needs a list of names",
                Some(
                    "Write it as `page.images([\"photo-1.jpg\", \"photo-2.jpg\"])`, or \
                     `page.images()` for every JPEG, PNG or WebP image beside the page.",
                ),
            ),
            (
                asked("page.image", Needed::Text("sizes".into())),
                "Wrong argument: `page.image` needs `sizes` as text",
                Some(
                    "Write it as a browser reads it: `sizes=\"(max-width: 40rem) 100vw, \
                     40rem\"`.",
                ),
            ),
            (
                asked("page.image", Needed::TrueFalse("full".into())),
                "Wrong argument: `page.image` needs `full` to be `true` or `false`",
                None,
            ),
            (
                asked(
                    "page.image",
                    Needed::OnSite("https://example.org/photo-1.jpg".into()),
                ),
                "Wrong argument: `page.image` needs a file of the site, not \
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
                "Wrong argument: `page.youtube` needs a YouTube URL or id, not `video-1`",
                Some(
                    "Write the URL YouTube gives for the video, such as \
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
                "Wrong argument: `page.vimeo` needs a Vimeo URL or id",
                Some(
                    "Write the URL Vimeo gives for the video, such as \
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
                stopped(Stopped::NoSiteUrl),
                "Template stopped: `absolute` needs the site's `url`",
                Some("Add `url:` to `statac.yaml`."),
            ),
            (
                stopped(Stopped::Operator {
                    op: "+".into(),
                    left: Shape::Text,
                    right: Shape::Number,
                }),
                "Template stopped: can't use `+` on text and a number",
                Some("Join text with `~`."),
            ),
            (
                stopped(Stopped::Engine("template \"home\" does not exist".into())),
                "Template stopped: template \"home\" does not exist",
                None,
            ),
            (
                stopped(Stopped::NotAPage("headline".into())),
                "Template stopped: `headline` needs a page",
                Some("Write it as `headline(post)`."),
            ),
            (
                stopped(Stopped::NotAnAddress),
                "Template stopped: `url` needs a URL starting with `/`",
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
                Kind::ContentNeverShown {
                    layout: "templates/hollow.html".into(),
                    pages: 3,
                },
                "Content never shown: 3 pages built by `templates/hollow.html` have content it \
                 never shows",
                Some("Add `{{ page.content }}` where the content belongs."),
            ),
            (
                Kind::ContentNeverShown {
                    layout: "templates/hollow.html".into(),
                    pages: 1,
                },
                "Content never shown: 1 page built by `templates/hollow.html` has content it \
                 never shows",
                Some("Add `{{ page.content }}` where the content belongs."),
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
    fn a_retired_list_is_shown_with_the_lines_that_carry_it_over() {
        let retired = |urls: &[&str], unread| Kind::RetiredFile {
            urls: urls.iter().map(|url| (*url).to_owned()).collect(),
            unread,
        };
        let urls = ["https://a.example/post-1/", "https://b.example/a b"];
        assert_eq!(
            printed(&retired(&urls, Unread::Lines(1))),
            Some((
                "To keep refusing what it lists, add these lines to `statac.yaml`:".into(),
                vec![
                    "refuse_webmentions_from:".into(),
                    "  - https://a.example/post-1/".into(),
                    "  - https://b.example/a b".into(),
                ]
            ))
        );
        assert_eq!(
            printed(&retired(&[], Unread::Lines(2))),
            Some(("Its 2 lines can't be read.".into(), Vec::new()))
        );
        assert_eq!(
            printed(&retired(&[], Unread::File(Reason::Denied))),
            Some(("It can't be read: permission denied.".into(), Vec::new()))
        );
        assert_eq!(printed(&retired(&[], Unread::Nothing)), None);
        assert_eq!(removed_refused(1, 0), "Removed 1 refused webmention.");
        assert_eq!(
            removed_refused(3, 2),
            "Removed 3 refused webmentions and 2 avatars."
        );
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
            Some(("What it printed:".into(), vec!["one".into(), "two".into()]))
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
        let unstarted = Ended::NotStarted(Reason::Denied);
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
                    "This is usually a character such as \u{2019} saved in an old encoding: type \
                     it again.",
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
                "Nested too deep: the Markdown here is nested more than 200 deep",
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
