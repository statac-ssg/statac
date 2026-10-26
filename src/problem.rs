//! What Statac finds, as data: problems with the details they carry, and
//! the other things it tells, such as what a build wrote. All of it is
//! worded in `wording`, so detection never builds a sentence and every
//! sentence lives in one catalogue. It uses no module that reports
//! problems, so any module can use it.

use std::io;
use std::path::PathBuf;

use crate::origin::Sourced;
use crate::platform::{Alike, Refused, Unwritable};
use crate::source::Place;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Severity {
    /// Stops the build.
    Error,
    /// Statac did something and says what.
    Warning,
    /// About the published page, not a mistake.
    Note,
}

macro_rules! codes {
    ($($code:ident $name:literal,)*) => {
        /// A kind of problem without its details.
        #[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
        pub enum Code {
            $($code,)*
        }

        impl Code {
            pub const ALL: &[Code] = &[$(Code::$code,)*];

            /// Stable, readable name: used by `quiet:`, `statac explain` and the explanation link.
            pub fn name(self) -> &'static str {
                match self {
                    $(Code::$code => $name,)*
                }
            }

            /// What the problem means and how to fix it, in Markdown.
            pub fn explanation(self) -> &'static str {
                match self {
                    $(Code::$code => include_str!(concat!("../explanations/", $name, ".md")),)*
                }
            }
        }
    };
}

codes! {
    BrokenLink "broken-link",
    OtherSpelling "other-spelling",
    RetiredSetting "retired-setting",
    CantWrite "cant-write",
    MalformedYaml "malformed-yaml",
    MalformedJson "malformed-json",
    DuplicateKey "duplicate-key",
    AliasLimit "alias-limit",
    NestedTooDeep "nested-too-deep",
    UnknownYamlTag "unknown-yaml-tag",
    NotData "not-data",
    DataNamedTwice "data-named-twice",
    MergeKey "merge-key",
    SameId "same-id",
    UnclosedFrontMatter "unclosed-front-matter",
    UnknownProblemName "unknown-problem-name",
    CantQuieten "cant-quieten",
    UnknownCommand "unknown-command",
    MissingCommand "missing-command",
    UnknownOption "unknown-option",
    MissingValue "missing-value",
    OptionGivenTwice "option-given-twice",
    UnexpectedArgument "unexpected-argument",
    MissingName "missing-name",
    NothingToSearch "nothing-to-search",
    UnreadableName "unreadable-name",
    UnusableName "unusable-name",
    MissingFile "missing-file",
    SassStopped "sass-stopped",
    WorkerOutsideImportMap "worker-outside-import-map",
    SameUrl "same-url",
    UrlTaken "url-taken",
    UrlGone "url-gone",
    UnwritableName "unwritable-name",
    NotStatacsFolder "not-statacs-folder",
    OutputInTheWay "output-in-the-way",
    CantRead "cant-read",
    CantRemove "cant-remove",
    CantListen "cant-listen",
    OutsideSite "outside-site",
    SassWarning "sass-warning",
    MalformedCall "malformed-call",
    MalformedMaths "malformed-maths",
    ControlCharacter "control-character",
    ColonInCall "colon-in-call",
    PublishedAsText "published-as-text",
    UnknownWord "unknown-word",
    AlwaysFalse "always-false",
    ReadAsCustom "read-as-custom",
    NestedSetting "nested-setting",
    FieldWrittenTwoWays "field-written-two-ways",
    ReservedName "reserved-name",
    SetAside "set-aside",
    WrongKind "wrong-kind-of-value",
    NamesNothing "names-nothing",
    NeedsQuotes "needs-quotes",
    OneTagNotTwo "one-tag-not-two",
    UnclearDate "unclear-date",
    NoSuchDate "no-such-date",
    DatesDisagree "dates-disagree",
    NotStamped "not-stamped",
    MissingField "missing-field",
    MissingData "missing-data",
    MissingContentFolder "missing-content-folder",
    NoNamesToCheck "no-names-to-check",
    WrongPlace "wrong-place",
    UnusableUrl "unusable-url",
    UnusableDateFormat "unusable-date-format",
    NoTimeZone "no-time-zone",
    HookStopped "hook-stopped",
    HookNotRun "hook-not-run",
    NothingMade "nothing-made",
    OutputFolderNamed "output-folder-named",
    UnknownLanguage "unknown-language",
    LanguageWrittenTwice "language-written-twice",
    NoPagesInLanguage "no-pages-in-language",
    MissingWords "missing-words",
    LanguageFolderInPattern "language-folder-in-pattern",
    SameTranslation "same-translation",
    UnmatchedTranslationKey "unmatched-translation-key",
    NotLinkedByPlace "not-linked-by-place",
    UnknownCodeLanguage "unknown-code-language",
    CodeNotColoured "code-not-coloured",
    MalformedTemplate "malformed-template",
    MissingTemplate "missing-template",
    BuiltWithPageTemplate "built-with-page-template",
    BuiltWithoutTemplate "built-without-template",
    NoValue "no-value",
    NotSupported "not-supported",
    WrongArgument "wrong-argument",
    TemplateStopped "template-stopped",
    PageInSnippet "page-in-snippet",
    TextNeverShown "text-never-shown",
    ContentNeverShown "content-never-shown",
    SpelledTwoWays "spelled-two-ways",
    DraftLeftOut "draft-left-out",
    ScheduledPost "scheduled-post",
    NumberedFolder "numbered-folder",
    UnorderedPages "unordered-pages",
    MalformedList "malformed-list",
    NotAPost "not-a-post",
    TwoLists "two-lists",
    FeedNotWritten "feed-not-written",
    FeedWrittenTwice "feed-written-twice",
    FeedFormatMismatch "feed-format-mismatch",
    NoFeedKey "no-feed-key",
    FeedIdChanged "feed-id-changed",
    NoSiteUrl "no-site-url",
    UnusedFeedTemplate "unused-feed-template",
    UnusedLanguageFile "unused-language-file",
    NoLanguage "no-language",
    NoListPage "no-list-page",
    NoHomePage "no-home-page",
    RobotsBeneathPath "robots-beneath-path",
    RedirectToItself "redirect-to-itself",
    ContentNotPublished "content-not-published",
    SortedLast "sorted-last",
    LoopOverEveryPage "loop-over-every-page",
    CallInHtml "call-in-html",
    MoreWrittenTwice "more-written-twice",
    UntitledPage "untitled-page",
    MissingDefinition "missing-definition",
    MissingAnchor "missing-anchor",
    FootnoteNeverShown "footnote-never-shown",
    DefinitionWrittenTwice "definition-written-twice",
    CellNeverShown "cell-never-shown",
    IdUsedTwice "id-used-twice",
    UnmatchedTag "unmatched-tag",
    SizeKept "size-kept",
    UnreadableImage "unreadable-image",
    ColourConverted "colour-converted",
    DescribesNoImage "describes-no-image",
    CacheNotWritten "cache-not-written",
    MissingAltText "missing-alt-text",
    MissingLinkText "missing-link-text",
    MissingButtonText "missing-button-text",
    MissingIframeTitle "missing-iframe-title",
    MissingLabel "missing-label",
    MissingTitle "missing-title",
    HeicImage "heic-image",
    LocationPublished "location-published",
    ImagesOutOfProportion "images-out-of-proportion",
    UnusableCardFile "unusable-card-file",
    CardNotDrawn "card-not-drawn",
    UnusedCard "unused-card",
    ImageMayNotShow "image-may-not-show",
    NoThumbnail "no-thumbnail",
    MalformedRecord "malformed-record",
    TokenInSettings "token-in-settings",
    WebmentionsOff "webmentions-off",
    MentionOfNoPage "mention-of-no-page",
    RefusedWebmentions "refused-webmentions",
    RetiredFile "retired-file",
    FolderNotEmpty "folder-not-empty",
    AlreadyASite "already-a-site",
    NoSiteHere "no-site-here",
    CantTellWhatMade "cant-tell-what-made",
    MissingFolder "missing-folder",
}

/// Names kinds of problem were known by, with each kind's name now.
const FORMER_NAMES: &[(&str, Code)] = &[
    ("address-gone", Code::UrlGone),
    ("address-taken", Code::UrlTaken),
    ("no-site-address", Code::NoSiteUrl),
    ("same-address", Code::SameUrl),
    ("unusable-address", Code::UnusableUrl),
    ("describes-no-picture", Code::DescribesNoImage),
    ("heic-picture", Code::HeicImage),
    ("pictures-out-of-proportion", Code::ImagesOutOfProportion),
    ("undescribed-picture", Code::MissingAltText),
    ("unreadable-picture", Code::UnreadableImage),
    ("waiting-for-its-day", Code::ScheduledPost),
    ("writing-never-shown", Code::ContentNeverShown),
    ("writing-not-published", Code::ContentNotPublished),
    ("read-as-your-own", Code::ReadAsCustom),
    ("no-still", Code::NoThumbnail),
];

/// Names of kinds of problem Statac no longer has, each with the kind
/// whose page on the docs site its page sends readers on to.
const RETIRED_NAMES: &[(&str, Code)] = &[
    ("missing-address", Code::RefusedWebmentions),
    ("missing-url", Code::RefusedWebmentions),
    ("not-a-url", Code::RefusedWebmentions),
    ("not-a-web-address", Code::RefusedWebmentions),
    ("newer-starter", Code::UnknownOption),
    ("update-available", Code::UnknownOption),
    ("code-colours-not-loaded", Code::RetiredSetting),
];

impl Code {
    pub(crate) fn from_name(name: &str) -> Option<Code> {
        Code::ALL.iter().copied().find(|code| code.name() == name)
    }

    /// The kind once called `name`, which is refused with the name it
    /// has now.
    pub(crate) fn renamed(name: &str) -> Option<Code> {
        FORMER_NAMES
            .iter()
            .find(|(former, _)| *former == name)
            .map(|&(_, code)| code)
    }

    /// Whether `name` named a kind Statac no longer has.
    pub(crate) fn retired(name: &str) -> bool {
        RETIRED_NAMES.iter().any(|(retired, _)| *retired == name)
    }

    /// The problem with `written`, a name given to `quiet:` or `statac
    /// explain` that names no kind.
    pub(crate) fn unknown(written: &str) -> Kind {
        let (former, suggestion) = match Code::renamed(written) {
            _ if Code::retired(written) => (Former::Retired, None),
            Some(code) => (Former::Renamed, Some(code.name())),
            None => (
                Former::Nothing,
                crate::suggest::nearest(written, Code::ALL.iter().map(|c| c.name())),
            ),
        };
        Kind::UnknownProblemName {
            written: written.to_owned(),
            suggestion: suggestion.map(str::to_owned),
            former,
        }
    }

    /// The names this kind was known by before, and those of kinds
    /// retired in its favour, each of whose pages on the docs site sends
    /// readers on to this one.
    pub fn former_names(self) -> impl Iterator<Item = &'static str> {
        (FORMER_NAMES.iter().chain(RETIRED_NAMES))
            .filter(move |(_, code)| *code == self)
            .map(|&(former, _)| former)
    }

    /// The explanation as a terminal shows it: a heading's `{#id}`, which
    /// only a page can use, is left out.
    pub(crate) fn explained(self) -> String {
        let mut out = String::new();
        // The backticks that opened the code block a line is in.
        let mut fence = 0;
        for line in self.explanation().lines() {
            let ticks = line.len() - line.trim_start_matches('`').len();
            if fence == 0 && ticks >= 3 {
                fence = ticks;
            } else if fence > 0 && ticks >= fence && line[ticks..].trim().is_empty() {
                fence = 0;
            } else if fence == 0 && line.starts_with('#') {
                out.push_str(unanchored(line));
                out.push('\n');
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    pub fn severity(self) -> Severity {
        match self {
            Code::BrokenLink
            | Code::WorkerOutsideImportMap
            | Code::UrlTaken
            | Code::UrlGone
            | Code::SassWarning
            | Code::ColonInCall
            | Code::PublishedAsText
            | Code::ControlCharacter
            | Code::FieldWrittenTwoWays
            | Code::ReadAsCustom
            | Code::SetAside
            | Code::OneTagNotTwo
            | Code::DatesDisagree
            | Code::NoTimeZone
            | Code::NothingMade
            | Code::OutputFolderNamed
            | Code::UnknownLanguage
            | Code::NoPagesInLanguage
            | Code::LanguageFolderInPattern
            | Code::UnmatchedTranslationKey
            | Code::NotLinkedByPlace
            | Code::UnknownCodeLanguage
            | Code::CodeNotColoured
            | Code::PageInSnippet
            | Code::AlwaysFalse
            | Code::ContentNeverShown
            | Code::SpelledTwoWays
            | Code::UnorderedPages
            | Code::NotAPost
            | Code::ContentNotPublished
            | Code::SortedLast
            | Code::NotData
            | Code::FeedNotWritten
            | Code::FeedWrittenTwice
            | Code::FeedFormatMismatch
            | Code::FeedIdChanged
            | Code::MoreWrittenTwice
            | Code::MissingDefinition
            | Code::MissingAnchor
            | Code::FootnoteNeverShown
            | Code::DefinitionWrittenTwice
            | Code::CellNeverShown
            | Code::IdUsedTwice
            | Code::UnmatchedTag
            | Code::SizeKept
            | Code::DescribesNoImage
            | Code::CacheNotWritten
            | Code::HeicImage
            | Code::ImagesOutOfProportion
            | Code::CardNotDrawn
            | Code::UnusedCard
            | Code::ImageMayNotShow
            | Code::NoThumbnail
            | Code::WebmentionsOff
            | Code::MentionOfNoPage => Severity::Warning,
            Code::DraftLeftOut
            | Code::HookNotRun
            | Code::MissingAltText
            | Code::MissingLinkText
            | Code::MissingButtonText
            | Code::MissingIframeTitle
            | Code::MissingLabel
            | Code::MissingTitle
            | Code::ColourConverted
            | Code::ScheduledPost
            | Code::NumberedFolder
            | Code::CallInHtml
            | Code::UntitledPage
            | Code::NoSiteUrl
            | Code::NoLanguage
            | Code::NoListPage
            | Code::NoHomePage
            | Code::MissingWords
            | Code::BuiltWithPageTemplate
            | Code::BuiltWithoutTemplate
            | Code::LocationPublished
            | Code::RobotsBeneathPath
            | Code::UnusedFeedTemplate
            | Code::UnusedLanguageFile
            | Code::LoopOverEveryPage
            | Code::RefusedWebmentions => Severity::Note,
            Code::OtherSpelling
            | Code::RetiredSetting
            | Code::CantWrite
            | Code::MalformedYaml
            | Code::MalformedJson
            | Code::DuplicateKey
            | Code::AliasLimit
            | Code::NestedTooDeep
            | Code::UnknownYamlTag
            | Code::DataNamedTwice
            | Code::MergeKey
            | Code::SameId
            | Code::TwoLists
            | Code::UnclosedFrontMatter
            | Code::UnknownProblemName
            | Code::CantQuieten
            | Code::UnknownCommand
            | Code::MissingCommand
            | Code::UnknownOption
            | Code::MissingValue
            | Code::OptionGivenTwice
            | Code::UnexpectedArgument
            | Code::MissingName
            | Code::NothingToSearch
            | Code::UnreadableName
            | Code::UnusableName
            | Code::MissingFile
            | Code::SassStopped
            | Code::SameUrl
            | Code::UnwritableName
            | Code::NotStatacsFolder
            | Code::OutputInTheWay
            | Code::CantRead
            | Code::CantRemove
            | Code::CantListen
            | Code::OutsideSite
            | Code::MalformedCall
            | Code::MalformedMaths
            | Code::UnknownWord
            | Code::NestedSetting
            | Code::ReservedName
            | Code::WrongKind
            | Code::NamesNothing
            | Code::NeedsQuotes
            | Code::UnclearDate
            | Code::NoSuchDate
            | Code::NotStamped
            | Code::MissingField
            | Code::MissingData
            | Code::MissingContentFolder
            | Code::NoNamesToCheck
            | Code::WrongPlace
            | Code::UnusableUrl
            | Code::RedirectToItself
            | Code::UnusableDateFormat
            | Code::LanguageWrittenTwice
            | Code::SameTranslation
            | Code::MalformedTemplate
            | Code::MissingTemplate
            | Code::NoValue
            | Code::NotSupported
            | Code::WrongArgument
            | Code::TemplateStopped
            | Code::TextNeverShown
            | Code::MalformedList
            | Code::NoFeedKey
            | Code::UnreadableImage
            | Code::UnusableCardFile
            | Code::HookStopped
            | Code::MalformedRecord
            | Code::RetiredFile
            | Code::TokenInSettings
            | Code::FolderNotEmpty
            | Code::AlreadyASite
            | Code::NoSiteHere
            | Code::CantTellWhatMade
            | Code::MissingFolder => Severity::Error,
        }
    }

    /// Whether this kind leaves a page out of the published site.
    pub(crate) fn leaves_a_page_out(self) -> bool {
        match self {
            Code::BrokenLink
            | Code::WorkerOutsideImportMap
            | Code::UrlTaken
            | Code::UrlGone
            | Code::SassWarning
            | Code::ColonInCall
            | Code::PublishedAsText
            | Code::ControlCharacter
            | Code::FieldWrittenTwoWays
            | Code::ReadAsCustom
            | Code::SetAside
            | Code::OneTagNotTwo
            | Code::DatesDisagree
            | Code::NoTimeZone
            | Code::HookNotRun
            | Code::NothingMade
            | Code::OutputFolderNamed
            | Code::UnknownLanguage
            | Code::NoPagesInLanguage
            | Code::MissingWords
            | Code::BuiltWithPageTemplate
            | Code::BuiltWithoutTemplate
            | Code::LanguageFolderInPattern
            | Code::UnmatchedTranslationKey
            | Code::NotLinkedByPlace
            | Code::UnknownCodeLanguage
            | Code::CodeNotColoured
            | Code::PageInSnippet
            | Code::AlwaysFalse
            | Code::ContentNeverShown
            | Code::SpelledTwoWays
            | Code::NumberedFolder
            | Code::NoLanguage
            | Code::NoListPage
            | Code::NoHomePage
            | Code::UnorderedPages
            | Code::NotAPost
            | Code::ContentNotPublished
            | Code::SortedLast
            | Code::NotData
            | Code::FeedNotWritten
            | Code::FeedWrittenTwice
            | Code::FeedFormatMismatch
            | Code::FeedIdChanged
            | Code::RobotsBeneathPath
            | Code::UnusedFeedTemplate
            | Code::UnusedLanguageFile
            | Code::CallInHtml
            | Code::UntitledPage
            | Code::MoreWrittenTwice
            | Code::MissingDefinition
            | Code::MissingAnchor
            | Code::FootnoteNeverShown
            | Code::DefinitionWrittenTwice
            | Code::CellNeverShown
            | Code::IdUsedTwice
            | Code::UnmatchedTag
            | Code::SizeKept
            | Code::ColourConverted
            | Code::DescribesNoImage
            | Code::CacheNotWritten
            | Code::MissingAltText
            | Code::MissingLinkText
            | Code::MissingButtonText
            | Code::MissingIframeTitle
            | Code::MissingLabel
            | Code::MissingTitle
            | Code::HeicImage
            | Code::LocationPublished
            | Code::ImagesOutOfProportion
            | Code::CardNotDrawn
            | Code::UnusedCard
            | Code::ImageMayNotShow
            | Code::NoThumbnail
            | Code::WebmentionsOff
            | Code::MentionOfNoPage
            | Code::RefusedWebmentions
            | Code::LoopOverEveryPage => false,
            Code::OtherSpelling
            | Code::RetiredSetting
            | Code::CantWrite
            | Code::MalformedYaml
            | Code::MalformedJson
            | Code::DuplicateKey
            | Code::AliasLimit
            | Code::NestedTooDeep
            | Code::UnknownYamlTag
            | Code::DataNamedTwice
            | Code::MergeKey
            | Code::SameId
            | Code::TwoLists
            | Code::UnclosedFrontMatter
            | Code::UnknownProblemName
            | Code::CantQuieten
            | Code::UnknownCommand
            | Code::MissingCommand
            | Code::UnknownOption
            | Code::MissingValue
            | Code::OptionGivenTwice
            | Code::UnexpectedArgument
            | Code::MissingName
            | Code::NothingToSearch
            | Code::UnreadableName
            | Code::UnusableName
            | Code::MissingFile
            | Code::SassStopped
            | Code::SameUrl
            | Code::UnwritableName
            | Code::NotStatacsFolder
            | Code::OutputInTheWay
            | Code::CantRead
            | Code::CantRemove
            | Code::CantListen
            | Code::OutsideSite
            | Code::MalformedCall
            | Code::MalformedMaths
            | Code::UnknownWord
            | Code::NestedSetting
            | Code::ReservedName
            | Code::WrongKind
            | Code::NamesNothing
            | Code::NeedsQuotes
            | Code::UnclearDate
            | Code::NoSuchDate
            | Code::NotStamped
            | Code::MissingField
            | Code::MissingData
            | Code::MissingContentFolder
            | Code::NoNamesToCheck
            | Code::WrongPlace
            | Code::UnusableUrl
            | Code::RedirectToItself
            | Code::UnusableDateFormat
            | Code::LanguageWrittenTwice
            | Code::SameTranslation
            | Code::MalformedTemplate
            | Code::MissingTemplate
            | Code::NoValue
            | Code::NotSupported
            | Code::WrongArgument
            | Code::TemplateStopped
            | Code::TextNeverShown
            | Code::MalformedList
            | Code::DraftLeftOut
            | Code::ScheduledPost
            | Code::NoFeedKey
            | Code::NoSiteUrl
            | Code::UnreadableImage
            | Code::UnusableCardFile
            | Code::HookStopped
            | Code::MalformedRecord
            | Code::RetiredFile
            | Code::TokenInSettings
            | Code::FolderNotEmpty
            | Code::AlreadyASite
            | Code::NoSiteHere
            | Code::CantTellWhatMade
            | Code::MissingFolder => true,
        }
    }

    /// Whether an author may quieten this kind by name. Errors, and anything
    /// that leaves a page out, never can be.
    pub fn quietenable(self) -> bool {
        self.severity() != Severity::Error && !self.leaves_a_page_out()
    }

    /// Whether only a build can find this kind, so that `check` has
    /// nothing to show of it: what comes of running the site's commands.
    pub(crate) fn found_by_build_alone(self) -> bool {
        matches!(self, Code::NothingMade)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A link, or a page's `redirect_to`, to an address on the site where
    /// the built site has nothing: the address as written, and the nearest
    /// address it publishes. `inside` where the suggestion is instead how
    /// to write the same address inside a site published under a path,
    /// which the site publishes.
    BrokenLink {
        target: String,
        suggestion: Option<String>,
        inside: bool,
    },
    /// A second spelling of one of Statac's own words, refused however clear.
    OtherSpelling {
        word: Word,
        written: String,
        ours: String,
    },
    /// A setting Statac once had and no longer reads, refused, with what
    /// the site writes in its place.
    RetiredSetting {
        setting: String,
        in_place: InPlace,
    },
    /// A problem in no file names what Statac was doing.
    CantWrite {
        path: PathBuf,
        reason: Reason,
    },
    /// YAML that can't be read, placed where the author would fix it.
    MalformedYaml {
        found: Malformed,
    },
    /// JSON that can't be read, placed where the author would fix it.
    MalformedJson {
        found: JsonFault,
    },
    /// A second more marker, placed at it: the excerpt ends at the first,
    /// as written at `first`.
    MoreWrittenTwice {
        written: String,
        first: Place,
    },
    /// A key given twice in one mapping, placed at the second.
    DuplicateKey {
        key: String,
        first: Place,
    },
    /// Aliases repeating more nodes than one document may hold.
    AliasLimit {
        alias: String,
        limit: Repeated,
    },
    /// Writing nested past the most Statac reads, placed at the opening of
    /// the first level past it.
    NestedTooDeep,
    /// A tag outside YAML's core schema, such as `!local`.
    UnknownYamlTag {
        tag: String,
    },
    /// A file in `data/` that is neither YAML nor JSON, which is left out.
    NotData {
        path: String,
    },
    /// Two things in `data/` that templates would reach by one name, each
    /// by its path in the site.
    DataNamedTwice {
        first: String,
        second: String,
        name: String,
    },
    /// YAML's merge key, `<<`, in a data file, which is read as it stands.
    MergeKey,
    /// An `id` given to two pages, placed at the second.
    SameId {
        id: String,
        first: Place,
    },
    /// A page choosing its list with both `each:` and `posts:`, placed at
    /// the second written; each line is named by its word.
    TwoLists {
        first: String,
        second: String,
        first_at: Place,
    },
    /// A `---` line opening front matter that no later `---` line closes.
    UnclosedFrontMatter,
    /// A name given to `quiet:` or `statac explain` that names no kind.
    UnknownProblemName {
        written: String,
        suggestion: Option<String>,
        former: Former,
    },
    /// A kind given to `quiet:` that can never be quietened.
    CantQuieten {
        name: String,
        because: Unquietenable,
    },
    UnknownCommand {
        written: String,
        hint: CommandHint,
    },
    /// Options with no command to apply them to.
    MissingCommand,
    UnknownOption {
        written: String,
        hint: OptionHint,
    },
    /// An option written without a value after its `=`.
    MissingValue {
        written: String,
        option: String,
        /// The next argument, where it looks like the value given apart.
        value: Option<String>,
    },
    OptionGivenTwice {
        option: String,
    },
    /// An argument the command has no use for.
    UnexpectedArgument {
        written: String,
    },
    /// `statac explain` with no name after it.
    MissingName,
    /// `statac search` given no words to search for.
    /// `statac search` given no words, or only a site's folder, by its
    /// path as written, which reads as the words.
    NothingToSearch {
        site: Option<String>,
    },
    /// A folder or path given on the command line that holds bytes that
    /// aren't UTF-8, as the system's reading of it shows them replaced.
    UnreadableName {
        written: String,
    },
    /// A folder or path given on the command line that holds a control
    /// character.
    UnusableName {
        written: String,
    },
    /// A file asked for by name that `assets/` doesn't have, as the author
    /// would find it (`assets/sit.css`).
    MissingFile {
        path: String,
        hint: Option<Hint>,
    },
    /// Why Sass stopped.
    SassStopped {
        fault: SassFault,
    },
    /// A worker named by a string in a script. Workers don't read the
    /// import map, so neither the name nor the worker's own imports reach
    /// the fingerprinted files.
    WorkerOutsideImportMap {
        written: String,
    },
    /// Two sources published at one address, or at two a disk holds as one.
    SameUrl {
        first: Box<Named>,
        second: Box<Named>,
        alike: Alike,
        address: String,
    },
    /// An author's page standing where a list's page after its first is
    /// published.
    LaterPageTaken {
        /// Where the list's first page stands.
        list: String,
        number: usize,
        page: Box<Named>,
        /// Where the page is shown, where the problem is told at
        /// `pagination.permalink`.
        page_at: Option<Place>,
        address: String,
    },
    /// A page left out as a draft or until its day, at whose address, as
    /// the site's own path makes it, `taken` is already published.
    UrlTaken {
        held: Held,
        address: String,
        taken: Box<Named>,
    },
    /// An address the site published before that nothing answers at any
    /// more: with the site's own path, and as `published.txt` and
    /// `redirect_from` write it, without.
    UrlGone {
        address: String,
        written: String,
    },
    /// Every address the site published before under a folder, two or
    /// more, that nothing answers at any more: the folder with the site's
    /// own path, and as `published.txt` writes it, and each address as it
    /// writes them, in the order of their lines.
    AddressesGone {
        under: String,
        written: String,
        gone: Vec<String>,
    },
    /// A name some common disk can't hold.
    UnwritableName {
        path: String,
        why: Unwritable,
    },
    /// An output folder Statac didn't build, which it won't replace.
    NotStatacsFolder {
        path: PathBuf,
        found: Occupant,
    },
    /// An output folder that is one of the folders the site is read from,
    /// or inside one: the output as given, the folder by its name, and
    /// whether the output is that folder itself.
    OutputInTheWay {
        out: PathBuf,
        folder: String,
        whole: bool,
    },
    CantRead {
        path: PathBuf,
        reason: Reason,
    },
    CantRemove {
        path: PathBuf,
        reason: Reason,
    },
    /// No port for serve to answer on, from `port` to `last`.
    CantListen {
        port: u16,
        last: u16,
        why: Unheard,
    },
    /// Something in the site reaching for a file outside it, which is
    /// never read.
    OutsideSite {
        reach: Reach,
    },
    /// What a Sass `@warn` or `@debug` said, or a warning of Sass's own.
    SassWarning {
        message: String,
    },
    /// A snippet call that can't be read, placed at the exact text.
    MalformedCall {
        found: CallFault,
    },
    /// Maths that can't be made MathML, placed at the exact text.
    MalformedMaths {
        found: MathsFault,
    },
    /// A control character in the writing, left out of the page.
    ControlCharacter {
        character: char,
    },
    /// An argument named `name:` where a call names with `=`; the value is
    /// used as though it were.
    ColonInCall {
        name: String,
    },
    /// Writing Statac reads as words that another tool reads otherwise, so
    /// it reaches the page as text: as written, and what it is.
    PublishedAsText {
        written: String,
        why: PublishedAsText,
    },
    /// A word where Statac knows every word, and this isn't one.
    UnknownWord {
        word: Word,
        written: String,
        nearest: Option<String>,
    },
    /// `loop` read in a for loop's own `if`, where it isn't set yet: as a
    /// name, or as a snippet for `loop(...)` in a recursive loop's `if`.
    LoopInForIf {
        word: Word,
    },
    /// A template's test of whether a page has a field no page has, one
    /// slip from a field a page can have, so it is false on every page:
    /// the field's name, and the field meant as the template reads it.
    AlwaysFalse {
        name: String,
        meant: String,
    },
    /// A name kept as the author's own that is one slip from a word
    /// Statac or the site knows, and that word.
    ReadAsCustom {
        own: Own,
        written: String,
        nearest: String,
    },
    /// One of Statac's settings nested under its prefix, where it has one
    /// flat spelling.
    NestedSetting {
        key: String,
        under: String,
        flat: String,
    },
    /// An author's field written flat in one place (`book.author:`) and
    /// nested in another (`book:` with `author:`).
    FieldWrittenTwoWays {
        on: Root,
        flat: String,
        nested_under: String,
        nested: Place,
    },
    /// A name of Statac's own taken for one of the author's values.
    ReservedName {
        name: String,
    },
    /// A field Statac reads nothing from.
    SetAside {
        field: String,
        why: SetAside,
    },
    WrongKind {
        field: String,
        expected: Expected,
        found: Found,
        /// The value written as Statac reads it, where that is clear.
        respelt: Option<Box<str>>,
    },
    /// A value for a field declared `one of` a table's names or a folder's
    /// pages that is none of them, placed at the value: what is written,
    /// what holds the names, and the nearest of them.
    NamesNothing {
        written: String,
        among: NamedIn,
        nearest: Option<String>,
    },
    /// A text value YAML reads as something else, changing its characters.
    NeedsQuotes {
        written: String,
        reading: String,
    },
    /// `tags` written as one plain text holding commas, which makes one
    /// tag: the key and the line as written, the tag made, and the parts
    /// the commas make.
    OneTagNotTwo {
        key: Box<str>,
        line: Box<str>,
        tag: Box<str>,
        parts: Box<[String]>,
    },
    /// A date with two readings, each as ISO and in words.
    UnclearDate {
        written: String,
        readings: Box<[(String, String); 2]>,
    },
    NoSuchDate {
        written: String,
        missing: Missing,
    },
    /// A page dated by its file name and by a `date` line, differently.
    DatesDisagree {
        named: String,
        written: String,
    },
    /// A value in front matter standing for a date that nothing has
    /// written yet, placed at it: the field it is the value of, and the
    /// value.
    NotStamped {
        field: String,
        written: String,
    },
    /// A field the author's own declaration requires.
    MissingField {
        field: String,
        required_by: Requirer,
    },
    /// A field declared `one of` a table `data/` doesn't have, placed at
    /// the declaration: the table as written, `data.people`, the
    /// declaration's name as written, and the nearest name `data/` has.
    MissingData {
        table: String,
        declared: String,
        nearest: Option<String>,
    },
    /// A field declared `one of` a folder `content/` doesn't have, placed
    /// at the declaration: the folder by its path in the site, the
    /// declaration's name as written, and the nearest folder there is, as
    /// a declaration names it.
    MissingContentFolder {
        folder: String,
        declared: String,
        nearest: Option<String>,
    },
    /// A field declared `one of` a data file that is a list, whose items
    /// have no names, placed at the declaration: the file, and the
    /// declaration's name as written.
    NoNamesToCheck {
        path: String,
        declared: String,
    },
    /// A setting written where it can't take effect.
    WrongPlace {
        key: String,
        why: WrongPlace,
    },
    /// An address no disk can write, or a pattern that can't make one.
    UnusableUrl {
        fault: AddressFault,
    },
    /// A `date_format`, or a template's example of one, that can't be read
    /// as the reference moment written the author's way.
    UnusableDateFormat {
        fault: Box<FormatFault>,
    },
    /// A site naming no `timezone`, with dates written without an
    /// offset, which are read as UTC.
    NoTimeZone,
    /// One of the site's own commands that didn't end well, placed at the
    /// command: the setting it is written in, how it ended, and everything
    /// it printed.
    HookStopped {
        hook: String,
        ended: Ended,
        said: String,
    },
    /// The site's own commands `check` doesn't run, by their settings.
    HookNotRun {
        hooks: Vec<String>,
    },
    /// Something `after_build_outputs` names that `after_build` left
    /// nothing at.
    NothingMade {
        place: String,
    },
    /// An `after_build` naming the folder the built site is published in,
    /// which holds the site built last time while the command runs,
    /// placed at the name: the folder as written, and the command with
    /// the folder Statac hands it in its place.
    OutputFolderNamed {
        folder: String,
        rewritten: String,
    },
    /// A language tag neither the site nor Statac has words for.
    UnknownLanguage {
        tag: String,
    },
    /// A tag `language:` declares twice, in any capitals, placed at the
    /// second.
    LanguageWrittenTwice {
        written: String,
        first_written: String,
        first: Place,
    },
    /// A language other than the main one with no folder under
    /// `content/`, which publishes nothing in it.
    NoPagesInLanguage {
        tag: String,
    },
    /// Pages whose writing shows what Statac writes words beside, in a
    /// language whose words have none for it: the language as the site
    /// names it, the file its words go in, which words, and how many
    /// pages.
    MissingWords {
        tag: String,
        file: String,
        words: ShownWords,
        pages: usize,
    },
    /// An address pattern for a language's pages that opens with that
    /// language's folder, which its patterns are read from already: read
    /// once, as `without`. `folder` is the folder's address, `/en/`.
    LanguageFolderInPattern {
        pattern: String,
        folder: String,
        without: String,
    },
    /// Two pages in one language that would both be the translation of
    /// the same pages in others, placed at the second: by one
    /// `translation_key`, or by standing where a page in another language
    /// does, named by its file.
    SameTranslation {
        tag: String,
        by: Translated,
        first: Option<Place>,
    },
    /// A `translation_key` no page in another language has, with the
    /// nearest one in another language that is no page's translation.
    UnmatchedTranslationKey {
        key: String,
        nearest: Option<String>,
    },
    /// A page with no `translation_key` standing where a page in another
    /// language with one stands, by that page's file and key, so not
    /// linked to it.
    NotLinkedByPlace {
        keyed: String,
        key: String,
    },
    /// A fence's first word naming no language Statac colours, as written,
    /// with the nearest that it does.
    UnknownCodeLanguage {
        written: String,
        nearest: Option<String>,
    },
    /// Code in a language Statac colours, published without colour
    /// because a line of it is longer than Statac colours: the longest
    /// line's length, in characters, and whether the code is a block of
    /// the writing, not a file or text a template shows.
    CodeNotColoured {
        characters: usize,
        block: bool,
    },
    /// A template or snippet that can't be read.
    MalformedTemplate {
        found: TemplateFault,
    },
    /// A layout or `extends` naming a template `templates/` doesn't have,
    /// by the file looked for (`templates/folder.html`).
    MissingTemplate {
        path: String,
        chosen: Chooser,
        nearest: Option<String>,
    },
    /// Pages with content of their own whose default template the site
    /// lacks, by its file (`templates/home.html`), built with
    /// `templates/page.html` instead: how many.
    BuiltWithPageTemplate {
        path: String,
        pages: usize,
    },
    /// Pages with content of their own published as that content alone,
    /// as the site has no template to build them with, or pages left at
    /// old URLs published as a refresh and a link, as it has no
    /// `templates/redirect.html`: how many.
    BuiltWithoutTemplate {
        redirects: bool,
        count: usize,
    },
    /// A value a template asked for that isn't there, as written in the
    /// template.
    NoValue {
        written: String,
        lack: Lack,
        instead: Instead,
    },
    /// Something written in a template that Statac doesn't do, or doesn't
    /// do yet.
    NotSupported {
        what: Unsupported,
    },
    /// A snippet call that hands the snippet something it can't take.
    WrongArgument {
        snippet: String,
        fault: ArgumentFault,
    },
    /// A template that stopped while a page was built, for a reason other
    /// than a missing value.
    TemplateStopped {
        reason: Stopped,
    },
    /// A snippet handed a page that shows a field of `page`, which is the
    /// page being built rather than the one it was handed.
    PageInSnippet {
        snippet: String,
        written: String,
        handed: String,
    },
    /// A block in a template that extends another, where none of the
    /// templates it extends has a block of that name to show it in.
    TextNeverShown {
        /// The template it extends, as the author would find it.
        extended: String,
        block: String,
        nearest: Option<String>,
    },
    /// Pages with writing, built by a layout that asks for neither it nor
    /// its excerpt: the layout's file, and how many pages.
    ContentNeverShown {
        layout: String,
        pages: usize,
    },
    /// Spellings of one tag or series whose address parts come out alike,
    /// each with how many pages write it, made one list under `chosen`.
    SpelledTwoWays {
        word: Word,
        spellings: Vec<(String, usize)>,
        chosen: String,
    },
    /// A page marked `draft: true`, by its file.
    DraftLeftOut {
        path: String,
    },
    /// A post dated after the build's day, by its file and its day.
    ScheduledPost {
        path: String,
        date: String,
    },
    /// A folder whose pages' names all open with a number, which orders
    /// them.
    NumberedFolder {
        /// The folder as the author finds it, `content/days/`.
        folder: String,
    },
    /// Pages a hand-written `order:` list leaves out, by their names.
    UnorderedPages {
        names: Vec<String>,
    },
    /// A `posts:` line that can't be read as a list.
    MalformedList {
        fault: ListFault,
    },
    /// A dated page standing at a list's address, which speaks for the
    /// list and so is in no list as a post. The list is a tag, a series or
    /// a folder by its path, the site's own where the path is empty.
    NotAPost {
        path: String,
        word: Word,
        name: String,
    },
    /// Feeds lists ask for that aren't published, by the first list, as
    /// `NotAPost` names one, and how many more there are.
    FeedNotWritten {
        list: (Word, String),
        more: usize,
        why: Unwritten,
    },
    /// A feed a `feed:` list names twice, by one name or two giving one
    /// address, placed at the second: as written there, as first written
    /// and where, and its address, with the site's own path.
    FeedWrittenTwice {
        written: String,
        first_written: String,
        first: Place,
        address: String,
    },
    /// A feed whose file name says one format, built as another: its file
    /// name, the template it was built with, the template named as its
    /// file, the format its name says and the one it was built as.
    FeedFormatMismatch {
        file: String,
        template: String,
        own: String,
        named: FeedFormat,
        built: FeedFormat,
    },
    /// Lists that have feeds on a site with no `feed_key`, told with a key
    /// freshly made for it.
    NoFeedKey {
        key: String,
    },
    /// What is left out of a site with no `url`: a page with `needs: url`,
    /// or the feeds lists have.
    NoSiteUrl {
        waiting: Waiting,
    },
    /// A feed template on a site where no list asks for a feed, by its
    /// path, with the setting that would ask for the site's feed it builds.
    UnusedFeedTemplate {
        template: String,
        line: String,
    },
    /// A file in `languages/`, by its path, that no language of the site
    /// reads, with the site's languages as `language:` names them.
    UnusedLanguageFile {
        path: String,
        languages: Vec<String>,
    },
    /// A post in a feed published at another address than feed readers
    /// know it at, with no `id:` of its own: where it was, with the site's
    /// own path, and the `id:` that gives it the id it had there.
    FeedIdChanged {
        was: String,
        id: String,
        /// Whether the post is made from an item of a table, which takes
        /// the line in its record.
        item: bool,
    },
    /// A site naming no language, read as English.
    NoLanguage,
    /// Lists with pages that nothing asks a list page for, of one kind: the
    /// first, as `NotAPost` names one, and how many more there are. A
    /// folder asks for one with its `index.md`; the tags and the series
    /// with `tags.pages` and `series.pages`.
    NoListPage {
        list: (Word, String),
        more: usize,
    },
    /// The tops of languages with pages where nothing is published, with
    /// no `index.md` there and no `templates/home.html`: each top's
    /// address, and the path of the first one's `index.md`.
    NoHomePage {
        addresses: Vec<String>,
        index: String,
    },
    /// A `robots.txt` published beneath the site's path, where crawlers
    /// never look; `top` is the one address they read.
    RobotsBeneathPath {
        top: String,
    },
    /// A `redirect_to` naming the page's own address, as written.
    RedirectToItself {
        written: String,
    },
    /// An old address in `redirect_from` that is the page's own, as
    /// written.
    RedirectFromItself {
        written: String,
    },
    /// A `redirect_to`, as written, that following the redirects on the
    /// site brings back to its own page.
    RedirectedBack {
        written: String,
    },
    /// Writing on a page sending readers on, by the page's file.
    ContentNotPublished {
        path: String,
    },
    /// Items `sort` was handed without the field it sorts by, put after
    /// the rest in the order they came: how many, and the first of them.
    SortedLast {
        field: String,
        count: usize,
        first: Named,
        /// The sort written to leave them out, where the template's text
        /// gives it.
        instead: Option<String>,
    },
    /// A loop over a list of the whole site's pages that so many pages
    /// run that the build went through pages past counting.
    LoopOverEveryPage {
        /// The list as the loop names it, where a template's text gives
        /// the loop.
        written: Option<String>,
        /// How many pages were built running it.
        pages: usize,
        /// The list written to choose the pages the loop keeps, where the
        /// loop says which in a way a filter can.
        instead: Option<String>,
    },
    /// The first snippet call built inside a page's HTML, by the snippet's
    /// name, said once for the build.
    CallInHtml {
        name: String,
    },
    /// A page a template asked the name of that has none the author wrote,
    /// so is called by what `by` says: `name`.
    UntitledPage {
        page: Named,
        by: Called,
        name: String,
    },
    /// A reference in the writing, `[text][label]` or `[^label]`, that no
    /// line on the page defines, left as words: the reference's label in
    /// its brackets, and the nearest label defined.
    MissingDefinition {
        written: String,
        nearest: Option<String>,
    },
    /// A link to a place on a page the site publishes, `#id`, where no
    /// element on that page has the id: the id, with its `%XX` read; the
    /// page's address, where it isn't the page the link is on; and the
    /// nearest id on it, written as a link names it.
    MissingAnchor {
        id: String,
        page: Option<String>,
        suggestion: Option<String>,
    },
    /// A footnote's note that no marker on the page points at, so it is
    /// shown nowhere: its label as written, and the nearest label of a
    /// marker with no note.
    FootnoteNeverShown {
        label: String,
        nearest: Option<String>,
    },
    /// A footnote's note whose label a later note on the page has, so only
    /// the later is shown: the label as written in this one, and where the
    /// later one is, where that can be told.
    DefinitionWrittenTwice {
        label: String,
        shown: Option<Place>,
    },
    /// Cells a table's row has past its columns, which are dropped: each
    /// that holds something, as written, and how many columns it has.
    CellNeverShown {
        cells: Vec<String>,
        columns: usize,
    },
    /// An id the author gave a heading or a table that something before
    /// it on the page has, so links to it land there: the id, what has
    /// it, and what this id is on.
    IdUsedTwice {
        id: String,
        taken: Taken,
        on: Braced,
    },
    /// Raw HTML in the writing, or in a value read as Markdown, that
    /// reaches past it into the page around it.
    UnmatchedTag {
        unmatched: Unmatched,
    },
    /// A size written `up to` where nothing is fitted, so drawn at the
    /// size given: the size field, its value and the place, each as
    /// written, and the size drawn.
    SizeKept {
        field: String,
        written: String,
        at: String,
        size: u32,
    },
    /// A picture the writing shows that can't be read, or a photograph
    /// whose copies can't be made: its file, and what is wrong with it.
    UnreadableImage {
        path: String,
        fault: PictureFault,
    },
    /// A photograph whose copies are made in sRGB from another colour
    /// space: its file, and the name its profile gives the space, where it
    /// gives one.
    ColourConverted {
        path: String,
        profile: Option<String>,
    },
    /// A file describing a picture that isn't beside it: the picture it
    /// names, and the nearest picture in its folder.
    DescribesNoImage {
        picture: String,
        nearest: Option<String>,
    },
    /// Copies that couldn't be kept for the next build: where they are
    /// kept, and the system's reason.
    CacheNotWritten {
        path: PathBuf,
        reason: Reason,
    },
    /// A picture a page shows with nothing said of it for a reader who
    /// can't see it: the picture as it is named where it is shown, how it
    /// is shown, and the link or button it is all of, where it is.
    MissingAltText {
        picture: String,
        shown: ShownBy,
        all_of: Option<AllOf>,
    },
    /// A link on a built page with nothing a screen reader can read out as
    /// its name: its `href` with its character references read, where that
    /// isn't empty, and whether an image with no alt text is all it shows.
    MissingLinkText {
        href: Option<String>,
        image: bool,
    },
    /// A `<button>` on a built page with nothing a screen reader can read
    /// out as its name, and whether an image with no alt text is all it
    /// shows.
    MissingButtonText {
        image: bool,
    },
    /// An `<iframe>` on a built page with no `title`: its `src`, with its
    /// character references read, where it has one.
    MissingIframeTitle {
        src: Option<String>,
    },
    /// An `<input>`, `<select>` or `<textarea>` on a built page with no
    /// label: the element's name, its `name` and its `id`, each with its
    /// character references read, and what labels one of its kind.
    MissingLabel {
        control: String,
        name: Option<String>,
        id: Option<String>,
        by: LabelledBy,
    },
    /// A page built with a template that has no `<title>`, or whose
    /// `<title>` is empty.
    MissingTitle {
        empty: bool,
    },
    /// A HEIC or HEIF picture a page shows, published as it is: its file.
    HeicImage {
        path: String,
    },
    /// A JPEG, PNG or WebP beside the writing published as it is, that
    /// holds where it was taken: its file. Told by `check` alone.
    LocationPublished {
        path: String,
    },
    /// A stylesheet rule giving `img` a width, on a site whose pages show
    /// pictures with their width and height and whose stylesheets nowhere
    /// say what a picture's height is: the property, as written.
    ImagesOutOfProportion {
        property: String,
    },
    /// A file a card's description names that no card can be drawn with:
    /// the file, and what is wrong with it.
    UnusableCardFile {
        path: String,
        fault: CardFileFault,
    },
    /// Pages left without a card for one cause: the cause, how many pages,
    /// and the first of them.
    CardNotDrawn {
        why: NotDrawn,
        pages: usize,
        first: Named,
    },
    /// A card described that no page is ever shared with: the name pages
    /// would pick it by, or none where no layout asks for any card.
    UnusedCard {
        name: Option<String>,
    },
    /// A picture pages are shared with as `og_image` names it, of a kind
    /// not every link preview is taken to show: its file, what its bytes
    /// say it is, where they say, and how many pages name it.
    ImageMayNotShow {
        path: String,
        holds: Option<crate::pictures::Kind>,
        pages: usize,
    },
    /// A video a page reads the still of, with no still in `fetched/`:
    /// the site keeping it, its id, and whether the record says the site
    /// answered it has none.
    NoThumbnail {
        host: Host,
        id: String,
        absent: bool,
    },
    /// A line of `fetched/record` that doesn't read as Statac writes it.
    MalformedRecord {
        fault: RecordFault,
    },
    /// `fetched/refused`, which Statac once kept the refused webmentions
    /// in: the URLs it lists, as written, and what of it couldn't be read.
    RetiredFile {
        urls: Vec<String>,
        unread: Unread,
    },
    /// `webmention_token:` in `statac.yaml`, whose value is never drawn.
    TokenInSettings,
    /// `webmention()` called on a site without `webmentions: true`.
    WebmentionsOff,
    /// A mention in `fetched/mentions/` of an address on the site where
    /// no page is, nor any page's `redirect_from`, as kept.
    MentionOfNoPage {
        target: String,
    },
    /// Mentions in `fetched/mentions/` that `refuse_webmentions_from`
    /// leaves out, by how many.
    RefusedWebmentions {
        count: usize,
    },
    /// A folder a command that makes a site was given that holds something:
    /// the folder as given, the first few names in it in order, and how
    /// many more.
    FolderNotEmpty {
        by: Maker,
        folder: String,
        names: Vec<String>,
        more: usize,
    },
    /// A folder a command that makes a site was given that is a site
    /// already, by what it has.
    AlreadyASite {
        by: Maker,
        folder: String,
        has: SiteMark,
    },
    /// A folder a command that works on a site was given, or ran in with
    /// no site at or above it, that has neither mark of a site: the folder
    /// as given, `.` for the current one.
    NoSiteHere {
        folder: String,
    },
    /// A site `statac import` was given whose files no reader, or more
    /// than one, says are its tool's: the folder as given, whether it was
    /// more than one, and the tools there are readers for.
    CantTellWhatMade {
        folder: String,
        several: bool,
        tools: Vec<String>,
    },
    /// `statac import` with no folder to import.
    MissingFolder,
}

/// A command that makes a site in a folder.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Maker {
    New,
    Import,
}

/// What tells a folder is a site.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SiteMark {
    Settings,
    Content,
}

/// What of a retired file couldn't be read.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Unread {
    Nothing,
    /// Lines that don't read as Statac wrote them, by how many.
    Lines(usize),
    /// The whole file, and why.
    File(Reason),
}

/// What is wrong with a line of `fetched/record`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RecordFault {
    /// The first line isn't the one Statac writes.
    Opening,
    /// A line of other than three fields, by how many it has.
    Fields(usize),
    /// A path, as written, that is no path inside `fetched/`.
    Path(String),
    /// What is written where the hash or `absent` goes.
    Held(String),
    /// What is written where the day goes.
    Day(String),
    /// A path written on a line before.
    Twice(String),
    /// A line of `published.txt` that is no address from the top of the
    /// site, as written.
    NotAnAddress(String),
}

/// What is wrong with a file a card's description names.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CardFileFault {
    /// A background of another size than a card's.
    BackgroundSize { width: u32, height: u32 },
    /// A background that is no still JPEG, PNG or WebP.
    NotAStill,
    /// A font file that doesn't read as one.
    Font,
}

/// Why a page's card can't be drawn.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NotDrawn {
    /// Characters no font of a list has: those the most pages need
    /// first, how many more there are than are named, a font in
    /// `templates/og/` with all of them that the list doesn't name, by its
    /// name there, where there is one, and the text's own field naming the
    /// list, such as `title.font`, or none for the card's `font`.
    NoFont {
        characters: Vec<char>,
        more: usize,
        font: Option<Box<String>>,
        field: Option<Box<String>>,
    },
    /// A text in the middle too tall for its room at the size tried, by
    /// its name, and whether that size is the smallest a fitted text is
    /// tried at.
    TooTall {
        text: String,
        size: u32,
        smallest: bool,
    },
    /// Texts at the top and bottom that leave no room between them.
    NoRoom,
}

/// How a page shows a picture.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ShownBy {
    /// As a picture in Markdown, and whether a file beside it may describe
    /// it, as one may a picture under `content/`.
    Markdown { described_beside: bool },
    /// As an `<img>` the writing holds.
    Html,
    /// By a template or a snippet, with the file that may describe it,
    /// for a picture under `content/`.
    Template { beside: Option<String> },
}

/// A link or a button an image with no alt text is all of, so that its
/// alt text is what a screen reader names the link or the button by.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AllOf {
    /// A link, with its `href` where it isn't empty.
    Link(Option<String>),
    Button,
}

/// What gives a form control of its kind its label, where nothing else
/// does.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LabelledBy {
    /// A `<label>`.
    Label,
    /// Its `value`, as for `<input type="button">`.
    Value,
    /// Its `alt`, as for `<input type="image">`.
    Alt,
}

/// What is wrong with a picture that can't be read.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PictureFault {
    /// It ends before the picture does, or is damaged within it.
    CutShort,
    /// It is a known kind written in a form not read, named.
    Unsupported(String),
    /// Decoded, it would take more memory than a photograph may.
    TooLarge { width: u32, height: u32 },
    /// A copy's encoder refused, with its message.
    CantMake(String),
    /// A file in `fetched/` shown as a photograph that is none.
    NotAStill,
}

/// What has an id before a heading or a table the author gave it to.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Taken {
    /// Another heading, where its id is written.
    Heading(Place),
    /// A table, where its id is written.
    Table(Place),
}

/// What the author gives an id in braces.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Braced {
    Heading,
    Table,
}

/// What a page with no name of the author's is called by.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Called {
    Summary,
    OpeningWords,
    Date,
    Address,
}

/// Why a page is left out of the build.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Held {
    Draft,
    /// A post dated after the build's day.
    Waiting,
}

/// What waits for the site's `url`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Waiting {
    /// A page, by its file.
    Page(String),
    /// Feeds, by the first list, as `NotAPost` names one, how many more
    /// lists there are, and how many feeds all of them have.
    Feeds {
        list: (Word, String),
        more: usize,
        feeds: usize,
    },
    /// The pictures pages are shared with.
    Cards,
    /// The mentions in `fetched/mentions/`, and the relay's address.
    Webmentions,
}

/// The kinds the author has quietened, by name.
#[derive(Default, Debug)]
pub struct Quiet {
    codes: Vec<Code>,
}

impl Quiet {
    /// Takes the names as written, and gives a problem for each that names
    /// no kind or a kind that can't be quietened.
    pub(crate) fn new(names: &[Sourced<String>]) -> (Quiet, Vec<Problem>) {
        let mut quiet = Quiet::default();
        let mut problems = Vec::new();
        for name in names {
            let written = &name.value;
            let kind = match Code::from_name(written) {
                Some(code) if code.quietenable() => {
                    quiet.codes.push(code);
                    continue;
                }
                Some(code) => Kind::CantQuieten {
                    name: written.clone(),
                    because: match code.severity() {
                        Severity::Error => Unquietenable::Error,
                        _ => Unquietenable::LeavesAPageOut,
                    },
                },
                None => Code::unknown(written),
            };
            problems.push(Problem {
                kind,
                place: name.origin.place,
                context: Default::default(),
            });
        }
        (quiet, problems)
    }

    pub(crate) fn quietens(&self, kind: &Kind) -> bool {
        self.codes.contains(&kind.code())
    }
}

/// What a feed is written as.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FeedFormat {
    Atom,
    Rss,
    Json,
}

/// Why a list's feed isn't published.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Unwritten {
    NoTitle,
    /// No template to build it with, by the files looked for, in the order
    /// they are looked for.
    NoTemplate(Vec<String>),
    /// A page stands at the feed's address.
    PageThere {
        page: Box<Named>,
        address: String,
    },
    /// The list has no dated page, so a feed would carry nothing.
    Undated,
    /// The list has no page for the feed to link to and be known by.
    NoPage,
    /// The list's page has `index_page: false`, so it isn't published for
    /// the feed to link to.
    PageUnpublished,
}

/// What is wrong with a `posts:` line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ListFault {
    /// It starts from something other than `site.posts`.
    NotFromThePosts(String),
    /// A `|` with no filter after it.
    NothingAfterPipe,
    /// A filter that can't choose a list's posts.
    NotChainable(String),
    NoBrackets(String),
    /// A bracket or a quote nothing closes.
    Unclosed(char),
    /// A filter given the wrong number of values.
    Values {
        filter: String,
        wants: usize,
        given: usize,
    },
}

/// What is wrong in a template file, as its reader found it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TemplateFault {
    /// Where the template engine stopped reading it.
    Syntax(SyntaxFault),
    /// A `{# takes #}` line that doesn't say what a call's unnamed values fill.
    Takes(TakesFault),
}

/// What the template engine couldn't read. Text quoted from the template
/// is as written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SyntaxFault {
    Unclosed(Opener),
    /// An end tag, by its keyword (`endif`), with no block of its kind
    /// open; the innermost block open instead, by its keyword and line.
    ClosesNothing {
        end: String,
        open: Option<(String, usize)>,
    },
    /// `else` or `elif` with no `if` or `for` open to belong to.
    Astray(String),
    /// A second `else` in the `if` or `for` named.
    SecondElse(String),
    ElifAfterElse,
    /// A tag the template language doesn't have, and the one that may have
    /// been meant.
    UnknownTag {
        written: String,
        nearest: Option<String>,
    },
    /// Characters the template language doesn't have.
    Foreign(String),
    NoFilter,
    NoField,
    BlockUnnamed,
    FilterUnnamed,
    EmptyTag,
    /// An escape in a string that isn't finished, such as `\u12`.
    Escape(String),
    NotANumber(String),
    TooLarge(String),
    TooDeep,
    NamedFirst,
    /// `break` or `continue`, by its keyword.
    OutsideLoop(String),
    /// A name the template language keeps, such as `loop`.
    Kept(String),
    /// A block inside a `macro` or a `call`, by that tag's keyword.
    BlockInMacro(String),
    BlockTwice(String),
    RequiredHolds,
    /// `{% endblock other %}` closing `{% block main %}`.
    EndNames {
        written: String,
        block: String,
    },
    /// `call` or `do` without a call, by its keyword.
    NoCall(String),
    /// Text where the template language needs something else; none for
    /// the template's end.
    Unexpected {
        found: Option<String>,
        awaited: Awaited,
        rewrite: Option<Rewrite>,
    },
    /// The engine's own words, for what isn't told otherwise.
    Engine(String),
}

/// What is opened and never closed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Opener {
    /// A block, by its keyword (`if`).
    Block(String),
    Tag,
    Output,
    Comment,
    Quote(char),
}

/// What the template language needs where it found something else.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Awaited {
    /// Text as the template would write it, such as `%}` or `in`.
    Written(String),
    Name,
    NameOrNumber,
    Value,
    TagName,
}

/// How to write what the template language couldn't read.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Rewrite {
    /// A filter given its value without brackets, written with them.
    Call(String),
    /// The closing marks it needs.
    Close(String),
    Compare,
    Elif,
    /// `in` after the loop's names.
    In(String),
    /// `=` after the names set.
    Assign(String),
    /// Text cut short by an apostrophe, in double quotes.
    DoubleQuotes(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TakesFault {
    /// No `#}` on the line.
    Unclosed,
    NamesNothing,
    /// Two commas, or a comma at either end.
    Missing,
    /// A name for any number of values beside other names.
    NotAlone(String),
    NotAName(String),
    Twice(String),
    /// More than two names to fill in order.
    TooMany,
    /// A name the file never reads.
    NeverRead(String),
    /// A name Statac hands the snippet itself.
    Supplied(String),
}

/// What picked a template that isn't there.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Chooser {
    /// A `layout:`, `type:` or `extends` naming it.
    Written,
    /// Statac's own choice for a page that names none.
    Default,
    /// Statac's own choice for a list's page it made, which no file holds
    /// for a `layout:` to be written in.
    Made,
}

/// Why a value has none.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Lack {
    /// A field the page doesn't set.
    OnPage(Named),
    /// Something with nothing in it, where no page is to blame.
    Unset,
    /// A key a data file doesn't have, by the file's path.
    InDataFile(String),
    /// A name nothing in a folder of `data/` has, by the folder's path.
    InDataFolder(String),
    /// A value that is there, written empty.
    Empty,
    /// A value that may be missing inside a list or map the template
    /// builds itself, where nothing would say so when it is.
    InLiteral { map: bool },
    /// One of a filter's items lacking the field it groups or tells items
    /// apart by.
    ByField(Box<ByField>),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ByField {
    pub(crate) item: Named,
    pub(crate) field: String,
    pub(crate) filter: String,
}

/// What a missing value's suggestion offers.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Instead {
    /// The name the snippet was handed the page by, with the field.
    Handed(String),
    /// The value written with a `default`, for where it may be missing.
    Default(String),
    /// The filter's input kept to the items that have the field.
    Selected(String),
    /// `headline` of a page Statac made, which has no title, where its
    /// `title` is missing.
    Headline(String),
    Nothing,
}

/// How one of the site's own commands ended, where it didn't end well.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ended {
    /// With an exit code of its own.
    Code(i32),
    /// By something else, before it finished.
    Cut,
    /// Never begun, for the system's reason.
    NotStarted(Reason),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Unsupported {
    /// `include`, `import` or `from`, with the snippet calls that do the
    /// same, filled in where they can be.
    Statement {
        statement: String,
        calls: Vec<String>,
    },
    /// One of Statac's names whose work isn't built yet.
    NotYet(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ArgumentFault {
    /// A name the snippet doesn't read.
    NoSuchName {
        given: String,
        nearest: Option<String>,
    },
    /// Values without names, where the snippet can't tell which is which.
    NeedsNames { wanted: Vec<String> },
    /// More values without names than the snippet's `takes` line names.
    TooMany {
        takes: Vec<String>,
        others: Vec<String>,
    },
    /// A value given both without a name and by its name.
    Twice(String),
    /// A name Statac hands the snippet itself.
    Supplied(String),
    /// A name the snippet reads and the call didn't hand it.
    NotGiven(String),
    /// One of Statac's own calls handed something other than it takes.
    Needs(Needed),
}

/// What one of Statac's own calls on a page takes.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Needed {
    /// The name of one file.
    Name(Sort),
    /// A list of pictures' names.
    Names,
    /// Its brackets, where a call that takes nothing is named without
    /// them.
    Brackets,
    /// Text for the option named.
    Text(String),
    /// True or false for the option named.
    TrueFalse(String),
    /// A file of the site, where the address written leads elsewhere.
    OnSite(String),
    /// A video on the site named, with what was written where that names
    /// none.
    VideoOn {
        site: &'static str,
        written: Option<String>,
    },
    /// A file a browser plays, where the one named is of another kind.
    Playable { sort: Sort, written: String },
}

/// A sort of file a page is asked for.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Sort {
    Picture,
    File,
    Video,
    Recording,
}

/// Why a template stopped, where it wasn't for a missing value.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Stopped {
    /// The template engine's own words for it.
    Engine(String),
    /// `absolute` on a site with no `url` setting.
    NoSiteUrl,
    /// `feed_id` on a site with no `feed_key`, with a key freshly made
    /// for it.
    NoFeedKey(String),
    /// A function that works on a page, handed something else.
    NotAPage(String),
    /// `url` handed anything but an address from the top of the site.
    NotAnAddress,
    /// A filter that works on a date, handed something else.
    NotADate(String),
    /// A filter that works on a list of pages, handed something else.
    NotAList(String),
    /// A snippet that reaches itself through its own calls.
    CallsItself(String),
    /// A page's writing that asks for itself.
    ContentInItself,
    /// An output whose building took all the engine's steps it may, told
    /// of the template it is built with.
    RanOn {
        template: String,
        steps: u64,
    },
    /// A `!!binary` value whose bytes aren't text, shown or made into words.
    NotText,
    /// `page.card` asked for by a page's writing.
    CardInWriting,
    /// `page.card` asked for by a text of a card.
    CardInCard,
    /// An operator, as written, used on values it can't work on, the left
    /// first.
    Operator {
        op: String,
        left: Shape,
        right: Shape,
    },
    DividedByZero,
    /// A sum, as written, whose answer is too large a number.
    TooLarge(String),
    RepeatNotWhole,
    RepeatedTooLong,
    /// An expression, as written, the engine can't work out.
    CantWorkOut(String),
    /// A function, filter, test or operator handed a value it can't work
    /// on; the value's kind where it is known.
    Needs {
        callee: String,
        wanted: Wanted,
        given: Option<Shape>,
    },
    CantSlice(Shape),
    SliceStep,
    /// What is called, as written, that can't be.
    NotCallable(String),
    /// A method asked of a value with none of that name; the call written
    /// as a filter, where the template language has one of the name.
    NoMethod {
        of: Shape,
        name: String,
        filter: Option<String>,
    },
    /// A function or filter called without a value it needs; for a
    /// filter, its input as written.
    MissingArgument {
        callee: String,
        input: Option<String>,
    },
    TooManyArguments(String),
    NoSuchArgument {
        callee: String,
        name: String,
        nearest: Option<String>,
    },
    GivenTwice {
        name: String,
        callee: String,
    },
    NoLength(Shape),
    CantLoop(Shape),
    /// A filter that makes a list, handed a value it can't make one of.
    CantList {
        callee: String,
        of: Shape,
    },
    /// `int` or `float` handed text that doesn't read as a number.
    NotANumber(String),
    EmptyText(String),
    CantNumber {
        callee: String,
        of: Shape,
    },
    /// A `%` mark of `format`'s, as written, handed a value of the wrong
    /// kind.
    FormatNeeds {
        mark: String,
        given: Shape,
    },
    FormatCharacter(String),
    /// A `%` mark of `format`'s that writes a whole number, handed a
    /// number with a fraction.
    FormatWhole(String),
    /// A `%` mark `format` doesn't have.
    FormatHasNo(String),
    FormatTooFew,
    FormatNamed(String),
    /// `range` asked for more numbers than the engine makes.
    RangeTooLong(u64),
    RangeStep,
    LoopNotRecursive,
    LoopTakesOne,
    /// The names a loop or `set` unpacks each item into, as written.
    Unpack(String),
    UnpackCount {
        names: String,
        wants: usize,
        given: usize,
    },
    NotANamespace(String),
    ExtendsTwice,
    ExtendsName,
    /// A template, by its file, that extends itself.
    ExtendsItself(String),
    /// A template, by its file, reached again through `extends`.
    ExtendsAgain(String),
    SuperOutside,
    /// Calls nested too deeply, by the macro calling itself where it is
    /// known.
    CallsTooDeep(Option<String>),
}

/// The kind of a value a template handed on, as the engine knows it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Shape {
    Text,
    Number,
    TrueFalse,
    None,
    List,
    Map,
    Date,
    Bytes,
    /// Any other value, such as a function.
    Other,
}

/// What a function, filter, test or operator needs to be handed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Wanted {
    Whole,
    WholeFromZero,
    Number,
    TrueFalse,
    Character,
    Text,
    List,
    Map,
    ListOrText,
    Numbers,
    Lists,
    /// Somewhere `in` can look: text, a list or a map.
    LookIn,
    CountAboveZero,
    FilterName,
    TestName,
    FieldName,
    NamedValues,
}

/// How a file outside the site was reached for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Reach {
    /// A Sass load, as written, leading out of `assets/`.
    Load(String),
    /// A file a card description names, as written, outside
    /// `templates/og/`.
    CardFile(String),
    /// A link, by its path in the site, whose target is outside the folder
    /// it must stay `within`, such as `static/`; outside the site's folder
    /// where none is named.
    Link {
        path: String,
        within: Option<String>,
    },
}

/// What might have been meant by a missing file.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Hint {
    /// The nearest name the folder looked in has.
    Closest(String),
    /// The file is in `static/`, published under its own name at this address.
    InStatic(String),
    /// `code.css`, which Statac once wrote beside the site's own files.
    NoLongerWritten,
}

/// What a site writes in place of a retired setting.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InPlace {
    /// Nothing: what it asked for is what Statac does now.
    Nothing,
    /// `code_colours: true`, with a theme's rules in the site's own
    /// stylesheet: the theme `code_style` named, where it named one.
    CodeColours { theme: Option<String> },
}

/// What sort of word a second or unknown spelling stands for.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Word {
    Field,
    Setting,
    /// A kind of value in a field declaration, such as `text`.
    FieldKind,
    /// A piece of an address pattern, such as `{year}`.
    AddressPiece,
    File,
    TimeZone,
    /// A name a template reads, such as `posts`.
    Name,
    Filter,
    Test,
    Snippet,
    Tag,
    Series,
    Folder,
    /// A page, by its name as its address writes it.
    Page,
    /// A table in `data/`, by the name `each:` gives it.
    Table,
    /// A marker in the writing, such as `<!--more-->`.
    Marker,
    /// Where a card draws a text, such as `top left`.
    Place,
    /// A card a page picks with `card:`.
    Card,
    /// A word standing for a date in front matter, such as `=today`.
    Stamp,
    /// A key of a language file, such as `months`.
    Line,
}

/// What a name kept as the author's own is read as.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Own {
    /// A field, by the name a declaration would give it.
    Field(String),
    Setting,
    Type,
}

/// Where an author's own value is read from in a template.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Root {
    Page,
    Site,
}

/// How two pages in one language would be the same translation.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Translated {
    /// Both have this `translation_key`.
    Key(String),
    /// Both stand where the page read from this file does.
    Place(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SetAside {
    /// A name Statac fills on every page itself.
    StatacFills(Root),
    /// A word on a page sending readers on, which publishes only its
    /// redirect.
    SendsOn,
    /// A name a card description doesn't read, with the nearest it does.
    NotOnCard { nearest: Option<String> },
    /// `card` on a page whose `og_image` names its own picture.
    OwnPicture,
    /// One of Statac's words or settings, or a declared field, with nothing
    /// written after its colon, as a field or a setting.
    NothingWritten(Word),
}

/// What is published as text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum PublishedAsText {
    /// A call or tag in another tool's braces, `{{< name >}}`, `{{% name %}}`
    /// or `{% name %}`.
    Call,
    /// A line in another tool's braces with a colon, `{: .class}`.
    Mark,
    /// Front matter with no `---` lines, by the file's first line.
    FrontMatter,
}

/// What a tag in raw HTML leaves unmatched.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Unmatched {
    /// An element opened and never closed, by its name as written.
    NeverClosed(String),
    /// A comment opened and never closed.
    CommentNeverClosed,
    /// An end tag with nothing open for it to close, by its name as
    /// written.
    ClosesNothing(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WrongPlace {
    /// Handed down, though each page has its own.
    EachPagesOwn,
    /// An `order:` list of names handed down, though it names one list's
    /// pages.
    OrderNames,
    /// A site setting written for pages.
    SiteSetting,
    /// Handed down from the top of `content/`, which is the whole site.
    TopFolder,
    /// Handed down from a page with no pages inside it.
    NothingInside,
    /// Handed down from `statac.yaml`, though only a folder says whether a
    /// list has a page.
    WholeSite,
    /// A `site.` value written on a page.
    SiteValue,
    /// A folder's own setting, on a page that isn't a folder's `index.md`.
    FolderOnly,
    /// A word only a page in `content/` can give, written on an item of a
    /// table.
    Item,
    /// The more marker, somewhere it can't end an excerpt.
    More(MoreAt),
    /// A setting about one of the site's own commands, on a site that
    /// doesn't set the command, named by its setting.
    NoCommand(&'static str),
    /// A key only Statac's own language files write, in a site's.
    LanguageFile,
}

/// Words a language file gives that a page's writing is shown with.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum ShownWords {
    /// What a screen reader says for a footnote's way back to the text.
    BackToTheText,
    /// The names of the kinds of callout.
    Callouts,
}

/// Where a more marker stands that can't end an excerpt.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MoreAt {
    List,
    Quotation,
    /// The writing a call wraps, by the snippet's name.
    Call(String),
    Footnote,
    /// On a line of its own inside a block of the author's HTML.
    Html,
    /// Inside an element the author's HTML opened before it and holds
    /// open, by its name as written.
    Element(String),
    /// On a line with other writing.
    Words,
}

/// What holds the names a declared field's value must be one of, by its
/// path in the site.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NamedIn {
    /// A table in `data/`: `data/people.yaml`.
    Data(String),
    /// A folder of pages: `content/books/`.
    Folder(String),
}

/// What a value should have been.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Expected {
    Text,
    TrueFalse,
    Number,
    WholeNumber {
        from: u64,
    },
    List,
    Date,
    /// A date, or a year and month, or a year.
    When,
    /// An address on the web or in the site.
    Address,
    /// The site's own full address.
    SiteAddress,
    OneOf(Vec<String>),
    LanguageCode,
    /// `true`, `false`, an address or a list of addresses, as `feed:` takes.
    TrueFalseOrAddress,
    /// `name: value` lines.
    Fields,
    /// An address pattern ending in `{slug}/`.
    ListAddress,
    /// An address pattern opening with `{list}` and holding `{n}`.
    LaterAddress,
    /// A page's own id.
    Id,
    /// Items, each `name: value` lines, as a list or by name.
    Table,
    /// `data.` and the name of a table in `data/`, as `each:` takes.
    TableName,
    /// A kind of value in a field declaration.
    FieldKind,
    /// A list in a language file.
    Words(WordList),
    /// One character, as each digit or mark is.
    OneCharacter,
    /// One space character.
    Space,
    /// A whole number in a range.
    Between {
        from: u64,
        to: u64,
    },
    /// A number in a range once rounded.
    RoundedBetween {
        from: u64,
        to: u64,
    },
    /// The name of a font file Statac reads.
    FontFile,
    /// A file or a folder in the built site, by its path from the top.
    BuiltPlace,
    /// A file or a folder in the site's folder, by its path from there.
    SitePlace,
    /// A card text's size, from 1 to 630, or `up to` one.
    CardSize,
    /// `#RGB`, `#RRGGBB`, or a template.
    Colour,
    /// `#RGB` or `#RRGGBB`, as a colour's template builds for a page.
    BuiltColour,
    /// A picture of the site's, as `og_image` names.
    OwnPicture,
    /// The name of a face in `fetched/faces/`, as a mention names it.
    FaceName,
}

/// The lists and runs of marks a language file holds.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WordList {
    Months,
    Weekdays,
    Days,
    AmPm,
    Digits,
    Quotes,
    Callouts,
}

/// What a value was instead.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Found {
    /// A single value, exactly as written.
    Written(String),
    List,
    Mapping,
    /// A single value that holds a secret, so is never quoted.
    Hidden,
    /// A list or a run of marks holding this many.
    Items(usize),
    /// A list holding this word twice.
    Twice(String),
    /// Nothing at all, where a template built none.
    Nothing,
}

/// Why a port can't be listened on.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Unheard {
    /// Another program has it.
    InUse,
    /// The system's own reason.
    System(Reason),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Missing {
    Day { day: u8, month: u8, year: i32 },
    Month(u8),
    Time(String),
    Offset(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Requirer {
    EveryPage,
    Type(String),
    /// A page making a page per item of a table, which needs a pattern for
    /// their addresses.
    Each,
    /// A page making a page per post, which needs a pattern for their
    /// addresses.
    EachPost,
    /// A card description, which names its background and fonts.
    Card,
    /// A text on a card, which needs all four of its fields.
    CardText,
    /// A mention in `fetched/mentions/`, which Statac writes whole.
    Mention,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AddressFault {
    /// A `{name}` no pattern fills.
    UnknownPiece {
        written: String,
        nearest: Option<String>,
    },
    /// A `{` nothing closes.
    Unclosed,
    /// A `}` nothing opened.
    Stray,
    /// One of the characters Windows keeps for itself.
    Character(char),
    /// `.` or `..`, the disk's words for this folder and the one above.
    Dots(String),
    /// A name ending in a space or a dot, which Windows takes off.
    TrailingSpaceOrDot(char),
    /// A name Windows keeps for a device, such as `con`.
    Device(String),
    /// A `%`, the web's spelling of another character.
    Percent,
    /// A `#`, where a browser stops reading an address.
    Hash,
    Control,
    /// A pattern asking for a piece of a date, on a page with none.
    NoDate {
        piece: String,
    },
    /// A pattern handed to many pages that asks for no piece at all.
    FillsNothing {
        pattern: String,
    },
    /// An item of a table lacking the field a piece of its pattern asks for.
    ItemLacks {
        /// The item, as `data/people.yaml: ada` or `data/people.yaml: item 3`.
        item: String,
        field: String,
        piece: String,
    },
    /// A post lacking the field a piece of the pattern its page is made by
    /// asks for.
    PostLacks {
        /// The post, as `content/post-1.md`.
        post: String,
        field: String,
        piece: String,
    },
    /// A page lacking the field a piece of its own pattern asks for.
    PageLacks {
        /// The page, as `content/a.md`.
        page: String,
        field: String,
        piece: String,
    },
    /// A feed's address naming a folder, with the nearest that names a
    /// file.
    FeedAtFolder {
        nearest: String,
    },
    /// A feed's name beside its page holding a `/`, with the address it
    /// would take from the top of the site.
    SlashBesidePage {
        nearest: String,
    },
}

/// Why a date format can't be read. Each spelling is ready to quote: the
/// reference moment as the site's words write it, and a corrected format
/// where there is one.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum FormatFault {
    /// `looks` where a template's example could have named a look.
    Empty {
        reference: String,
        examples: Vec<String>,
        looks: bool,
    },
    /// A piece written twice: `first` is the piece already read.
    Twice {
        first: String,
        second: String,
        piece: Piece,
    },
    /// Text that matches nothing in the reference moment.
    NotInReference {
        written: String,
        reference: String,
        corrected: Option<String>,
    },
    /// An hour on a twelve-hour clock, with no word for morning or
    /// afternoon.
    LoneHour {
        written: String,
        am: String,
        pm: String,
    },
    MinuteWithoutHour {
        written: String,
        examples: Vec<String>,
    },
    /// Nothing in the example is part of the reference moment: `looks`
    /// where it is a template's, which could have named a look.
    NoPattern {
        example: String,
        reference: String,
        examples: Vec<String>,
        looks: bool,
    },
    OnlyATime {
        example: String,
        with_date: String,
    },
    OnlyAWeekday {
        example: String,
        with_date: Option<String>,
    },
    /// Pieces of the date every date needs, missing, and how the site's
    /// words write the month.
    Missing {
        example: String,
        pieces: Vec<Piece>,
        months: Vec<String>,
    },
    TwoDigitYear {
        example: String,
    },
    /// Another weekday's word: the reference day, and its weekday in the
    /// site's words.
    OtherWeekday {
        written: String,
        day: String,
        weekday: String,
        corrected: String,
    },
    /// Another month's word: the reference day, and its month in the
    /// site's words.
    OtherMonth {
        written: String,
        day: String,
        month: String,
        corrected: String,
    },
    /// A piece written in words the site's language hasn't got.
    NoWords {
        list: WordsWanted,
        from: WordsFrom,
    },
    /// A month's or weekday's name in English, where the language the date
    /// is written in has words of its own: the example with that
    /// language's word in its place, where it has one.
    English {
        written: String,
        corrected: Option<String>,
    },
}

/// A look a template names a date by, each written as the language's
/// Unicode CLDR data has it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Look {
    /// The CLDR's long date: `31 December 2025`.
    Long,
    /// Its medium date: `31 Dec 2025`.
    Short,
    /// The day and the month: `31 Dec`.
    Day,
    /// The medium date and the short time: `31 Dec 2025, 15:04`.
    Stamp,
}

impl Look {
    pub(crate) const ALL: [Look; 4] = [Look::Long, Look::Short, Look::Day, Look::Stamp];

    /// As a template names it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Look::Long => "long",
            Look::Short => "short",
            Look::Day => "day",
            Look::Stamp => "stamp",
        }
    }

    pub(crate) fn named(name: &str) -> Option<Look> {
        Look::ALL.into_iter().find(|look| look.name() == name)
    }
}

/// A piece of a date as a format writes it.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Piece {
    Weekday,
    Day,
    Month,
    Year,
    Hour,
    Minute,
    AmPm,
}

/// A list of words a format writes from.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum WordsWanted {
    Months,
    MonthsShort,
    Weekdays,
    WeekdaysShort,
    Days,
    AmPm,
}

/// Where a language's words come from, for saying which has none.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WordsFrom {
    /// The site's own file, by its path in the site.
    Site(String),
    /// Statac's own words for the tag.
    Statac(String),
    /// No words at all for the tag.
    Nowhere(String),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Unquietenable {
    Error,
    LeavesAPageOut,
}

/// What can be said to help with an unknown command.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CommandHint {
    /// A command spelt nearly so, or an option such as `--help`.
    Similar(String),
    /// A command Statac once had, and the setting that does its work now.
    Setting(String),
    None,
}

/// What a name given for a kind of problem that names none once was.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Former {
    Nothing,
    /// The name of a kind before it was renamed, which the suggestion
    /// gives now.
    Renamed,
    /// The name of a kind Statac no longer has.
    Retired,
}

/// What can be said to help with an unknown option.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OptionHint {
    /// A known option spelt nearly so, written out whole.
    Similar(String),
    /// An option the command doesn't take, though others do.
    ForOther {
        command: String,
        option: String,
        takers: Box<[&'static str]>,
    },
    /// `--drafts`, asking `statac serve` for what it does unasked.
    ShownAlready,
    /// Nothing close: the options the command does take.
    Takes {
        command: String,
        options: Box<[&'static str]>,
    },
    /// Nothing close, and no command to list the options of.
    None,
    /// An option Statac once had for writing its own files into a site.
    Retired,
    /// `--site`, which Statac once had for the site's folder, with the
    /// command and the folder, where each was written.
    FolderAfterCommand {
        command: Option<String>,
        folder: Option<String>,
    },
}

/// Something published, or an item a template handled, as a problem
/// names it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Named {
    /// By its file, or by the item of a file it was made from:
    /// `content/post-1.md`, `data/people.yaml: ada`.
    File(String),
    /// A list's page Statac made, by its list: a folder by its path under
    /// `content/`, the site's home page by the empty path.
    ListPage(Word, String),
    /// A list's page after its first, counted from 2, by where the first
    /// stands.
    Later { number: u32, list: String },
    /// A list's feed, by its list and where it is published.
    Feed(Box<NamedFeed>),
    /// An item that is no page, by its place: `item 2`.
    Item(String),
    /// What `after_build` makes, as `after_build_outputs` names it.
    MadeAfter(String),
    /// The files a reader's browser searches the site with.
    Search,
    /// An old address a page's `redirect_from` names: the page, and where
    /// the address is written.
    Redirect { page: Box<Named>, at: Place },
    /// A page whose address its own `permalink` gives: the page, and
    /// where the `permalink` is written.
    Permalink { page: Box<Named>, at: Place },
}

impl Named {
    /// Where what gives this its address is written, where a problem with
    /// the address is best told.
    pub(crate) fn given_at(&self) -> Option<Place> {
        match self {
            Named::Redirect { at, .. } | Named::Permalink { at, .. } => Some(*at),
            _ => None,
        }
    }

    /// A list's page `number`, counted from 2, by where its first stands.
    pub(crate) fn later(number: usize, list: String) -> Named {
        Named::Later {
            number: u32::try_from(number).expect("fewer pages than a u32 counts"),
            list,
        }
    }
}

#[cfg(test)]
impl From<&str> for Named {
    fn from(path: &str) -> Named {
        Named::File(path.into())
    }
}

#[cfg(test)]
impl From<&str> for Box<Named> {
    fn from(path: &str) -> Box<Named> {
        Box::new(path.into())
    }
}

#[cfg(test)]
impl From<String> for Named {
    fn from(path: String) -> Named {
        Named::File(path)
    }
}

#[cfg(test)]
impl From<String> for Box<Named> {
    fn from(path: String) -> Box<Named> {
        Box::new(path.into())
    }
}

/// A list's feed as a problem names it.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct NamedFeed {
    pub(crate) word: Word,
    pub(crate) name: String,
    pub(crate) address: String,
}

/// How a second place bears on a problem.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Relation {
    FirstWritten,
    /// What is shown in place of the problem's own.
    Shown,
    /// Where the same field is written nested.
    Nested,
    /// The page whose build met a problem in a template: by the place
    /// drawn, or where that place isn't the page's own file, by name.
    Building(Option<Named>),
    /// A call that led into the file holding the problem, innermost first.
    CalledFrom,
    /// The `markdown` filter that read a value as writing.
    ReadAsMarkdown,
    /// Another thing published at the problem's address.
    AlsoPublished,
}

impl Kind {
    pub fn code(&self) -> Code {
        match self {
            Kind::BrokenLink { .. } => Code::BrokenLink,
            Kind::OtherSpelling { .. } => Code::OtherSpelling,
            Kind::RetiredSetting { .. } => Code::RetiredSetting,
            Kind::CantWrite { .. } => Code::CantWrite,
            Kind::MalformedYaml { .. } => Code::MalformedYaml,
            Kind::MalformedJson { .. } => Code::MalformedJson,
            Kind::DuplicateKey { .. } => Code::DuplicateKey,
            Kind::MoreWrittenTwice { .. } => Code::MoreWrittenTwice,
            Kind::AliasLimit { .. } => Code::AliasLimit,
            Kind::NestedTooDeep => Code::NestedTooDeep,
            Kind::UnknownYamlTag { .. } => Code::UnknownYamlTag,
            Kind::NotData { .. } => Code::NotData,
            Kind::DataNamedTwice { .. } => Code::DataNamedTwice,
            Kind::MergeKey => Code::MergeKey,
            Kind::SameId { .. } => Code::SameId,
            Kind::TwoLists { .. } => Code::TwoLists,
            Kind::UnclosedFrontMatter => Code::UnclosedFrontMatter,
            Kind::UnknownProblemName { .. } => Code::UnknownProblemName,
            Kind::CantQuieten { .. } => Code::CantQuieten,
            Kind::UnknownCommand { .. } => Code::UnknownCommand,
            Kind::MissingCommand => Code::MissingCommand,
            Kind::UnknownOption { .. } => Code::UnknownOption,
            Kind::MissingValue { .. } => Code::MissingValue,
            Kind::OptionGivenTwice { .. } => Code::OptionGivenTwice,
            Kind::UnexpectedArgument { .. } => Code::UnexpectedArgument,
            Kind::MissingName => Code::MissingName,
            Kind::NothingToSearch { .. } => Code::NothingToSearch,
            Kind::UnreadableName { .. } => Code::UnreadableName,
            Kind::UnusableName { .. } => Code::UnusableName,
            Kind::MissingFile { .. } => Code::MissingFile,
            Kind::SassStopped { .. } => Code::SassStopped,
            Kind::WorkerOutsideImportMap { .. } => Code::WorkerOutsideImportMap,
            Kind::SameUrl { .. } | Kind::LaterPageTaken { .. } => Code::SameUrl,
            Kind::UrlTaken { .. } => Code::UrlTaken,
            Kind::UrlGone { .. } | Kind::AddressesGone { .. } => Code::UrlGone,
            Kind::UnwritableName { .. } => Code::UnwritableName,
            Kind::NotStatacsFolder { .. } => Code::NotStatacsFolder,
            Kind::OutputInTheWay { .. } => Code::OutputInTheWay,
            Kind::CantRead { .. } => Code::CantRead,
            Kind::CantRemove { .. } => Code::CantRemove,
            Kind::CantListen { .. } => Code::CantListen,
            Kind::OutsideSite { .. } => Code::OutsideSite,
            Kind::SassWarning { .. } => Code::SassWarning,
            Kind::MalformedCall { .. } => Code::MalformedCall,
            Kind::MalformedMaths { .. } => Code::MalformedMaths,
            Kind::ControlCharacter { .. } => Code::ControlCharacter,
            Kind::ColonInCall { .. } => Code::ColonInCall,
            Kind::PublishedAsText { .. } => Code::PublishedAsText,
            Kind::UnknownWord { .. } | Kind::LoopInForIf { .. } => Code::UnknownWord,
            Kind::AlwaysFalse { .. } => Code::AlwaysFalse,
            Kind::ReadAsCustom { .. } => Code::ReadAsCustom,
            Kind::NestedSetting { .. } => Code::NestedSetting,
            Kind::FieldWrittenTwoWays { .. } => Code::FieldWrittenTwoWays,
            Kind::ReservedName { .. } => Code::ReservedName,
            Kind::SetAside { .. } => Code::SetAside,
            Kind::WrongKind { .. } => Code::WrongKind,
            Kind::NamesNothing { .. } => Code::NamesNothing,
            Kind::NeedsQuotes { .. } => Code::NeedsQuotes,
            Kind::OneTagNotTwo { .. } => Code::OneTagNotTwo,
            Kind::UnclearDate { .. } => Code::UnclearDate,
            Kind::NoSuchDate { .. } => Code::NoSuchDate,
            Kind::DatesDisagree { .. } => Code::DatesDisagree,
            Kind::NotStamped { .. } => Code::NotStamped,
            Kind::MissingField { .. } => Code::MissingField,
            Kind::MissingData { .. } => Code::MissingData,
            Kind::MissingContentFolder { .. } => Code::MissingContentFolder,
            Kind::NoNamesToCheck { .. } => Code::NoNamesToCheck,
            Kind::WrongPlace { .. } => Code::WrongPlace,
            Kind::UnusableUrl { .. } => Code::UnusableUrl,
            Kind::UnusableDateFormat { .. } => Code::UnusableDateFormat,
            Kind::NoTimeZone => Code::NoTimeZone,
            Kind::HookStopped { .. } => Code::HookStopped,
            Kind::HookNotRun { .. } => Code::HookNotRun,
            Kind::NothingMade { .. } => Code::NothingMade,
            Kind::OutputFolderNamed { .. } => Code::OutputFolderNamed,
            Kind::UnknownLanguage { .. } => Code::UnknownLanguage,
            Kind::LanguageWrittenTwice { .. } => Code::LanguageWrittenTwice,
            Kind::NoPagesInLanguage { .. } => Code::NoPagesInLanguage,
            Kind::MissingWords { .. } => Code::MissingWords,
            Kind::LanguageFolderInPattern { .. } => Code::LanguageFolderInPattern,
            Kind::SameTranslation { .. } => Code::SameTranslation,
            Kind::UnmatchedTranslationKey { .. } => Code::UnmatchedTranslationKey,
            Kind::NotLinkedByPlace { .. } => Code::NotLinkedByPlace,
            Kind::UnknownCodeLanguage { .. } => Code::UnknownCodeLanguage,
            Kind::CodeNotColoured { .. } => Code::CodeNotColoured,
            Kind::MalformedTemplate { .. } => Code::MalformedTemplate,
            Kind::MissingTemplate { .. } => Code::MissingTemplate,
            Kind::BuiltWithPageTemplate { .. } => Code::BuiltWithPageTemplate,
            Kind::BuiltWithoutTemplate { .. } => Code::BuiltWithoutTemplate,
            Kind::NoValue { .. } => Code::NoValue,
            Kind::NotSupported { .. } => Code::NotSupported,
            Kind::WrongArgument { .. } => Code::WrongArgument,
            Kind::TemplateStopped { .. } => Code::TemplateStopped,
            Kind::PageInSnippet { .. } => Code::PageInSnippet,
            Kind::TextNeverShown { .. } => Code::TextNeverShown,
            Kind::ContentNeverShown { .. } => Code::ContentNeverShown,
            Kind::SpelledTwoWays { .. } => Code::SpelledTwoWays,
            Kind::DraftLeftOut { .. } => Code::DraftLeftOut,
            Kind::ScheduledPost { .. } => Code::ScheduledPost,
            Kind::NumberedFolder { .. } => Code::NumberedFolder,
            Kind::UnorderedPages { .. } => Code::UnorderedPages,
            Kind::MalformedList { .. } => Code::MalformedList,
            Kind::NotAPost { .. } => Code::NotAPost,
            Kind::FeedNotWritten { .. } => Code::FeedNotWritten,
            Kind::FeedWrittenTwice { .. } => Code::FeedWrittenTwice,
            Kind::FeedFormatMismatch { .. } => Code::FeedFormatMismatch,
            Kind::NoFeedKey { .. } => Code::NoFeedKey,
            Kind::FeedIdChanged { .. } => Code::FeedIdChanged,
            Kind::NoSiteUrl { .. } => Code::NoSiteUrl,
            Kind::UnusedFeedTemplate { .. } => Code::UnusedFeedTemplate,
            Kind::UnusedLanguageFile { .. } => Code::UnusedLanguageFile,
            Kind::NoLanguage => Code::NoLanguage,
            Kind::NoListPage { .. } => Code::NoListPage,
            Kind::NoHomePage { .. } => Code::NoHomePage,
            Kind::RobotsBeneathPath { .. } => Code::RobotsBeneathPath,
            Kind::RedirectToItself { .. }
            | Kind::RedirectFromItself { .. }
            | Kind::RedirectedBack { .. } => Code::RedirectToItself,
            Kind::ContentNotPublished { .. } => Code::ContentNotPublished,
            Kind::SortedLast { .. } => Code::SortedLast,
            Kind::LoopOverEveryPage { .. } => Code::LoopOverEveryPage,
            Kind::CallInHtml { .. } => Code::CallInHtml,
            Kind::UntitledPage { .. } => Code::UntitledPage,
            Kind::MissingDefinition { .. } => Code::MissingDefinition,
            Kind::MissingAnchor { .. } => Code::MissingAnchor,
            Kind::FootnoteNeverShown { .. } => Code::FootnoteNeverShown,
            Kind::DefinitionWrittenTwice { .. } => Code::DefinitionWrittenTwice,
            Kind::CellNeverShown { .. } => Code::CellNeverShown,
            Kind::IdUsedTwice { .. } => Code::IdUsedTwice,
            Kind::UnmatchedTag { .. } => Code::UnmatchedTag,
            Kind::SizeKept { .. } => Code::SizeKept,
            Kind::UnreadableImage { .. } => Code::UnreadableImage,
            Kind::ColourConverted { .. } => Code::ColourConverted,
            Kind::DescribesNoImage { .. } => Code::DescribesNoImage,
            Kind::CacheNotWritten { .. } => Code::CacheNotWritten,
            Kind::MissingAltText { .. } => Code::MissingAltText,
            Kind::MissingLinkText { .. } => Code::MissingLinkText,
            Kind::MissingButtonText { .. } => Code::MissingButtonText,
            Kind::MissingIframeTitle { .. } => Code::MissingIframeTitle,
            Kind::MissingLabel { .. } => Code::MissingLabel,
            Kind::MissingTitle { .. } => Code::MissingTitle,
            Kind::HeicImage { .. } => Code::HeicImage,
            Kind::LocationPublished { .. } => Code::LocationPublished,
            Kind::ImagesOutOfProportion { .. } => Code::ImagesOutOfProportion,
            Kind::UnusableCardFile { .. } => Code::UnusableCardFile,
            Kind::CardNotDrawn { .. } => Code::CardNotDrawn,
            Kind::UnusedCard { .. } => Code::UnusedCard,
            Kind::ImageMayNotShow { .. } => Code::ImageMayNotShow,
            Kind::NoThumbnail { .. } => Code::NoThumbnail,
            Kind::MalformedRecord { .. } => Code::MalformedRecord,
            Kind::RetiredFile { .. } => Code::RetiredFile,
            Kind::TokenInSettings => Code::TokenInSettings,
            Kind::WebmentionsOff => Code::WebmentionsOff,
            Kind::MentionOfNoPage { .. } => Code::MentionOfNoPage,
            Kind::RefusedWebmentions { .. } => Code::RefusedWebmentions,
            Kind::FolderNotEmpty { .. } => Code::FolderNotEmpty,
            Kind::AlreadyASite { .. } => Code::AlreadyASite,
            Kind::NoSiteHere { .. } => Code::NoSiteHere,
            Kind::CantTellWhatMade { .. } => Code::CantTellWhatMade,
            Kind::MissingFolder => Code::MissingFolder,
        }
    }

    /// Stable, readable name: used by `quiet:`, `statac explain` and the explanation link.
    pub fn name(&self) -> &'static str {
        self.code().name()
    }

    pub fn severity(&self) -> Severity {
        self.code().severity()
    }

    /// Whether the problem is about the whole of the file it is placed
    /// in, so it is shown by the file's name with no line of it marked.
    pub(crate) fn of_a_whole_file(&self) -> bool {
        matches!(
            self,
            Kind::UnusedCard { .. }
                | Kind::SassStopped {
                    fault: SassFault::NotCss { placed: false, .. }
                }
        )
    }

    /// Whether an author may quieten this kind by name. Errors, and anything
    /// that leaves a page out, never can be.
    #[cfg(test)]
    pub(crate) fn quietenable(&self) -> bool {
        self.code().quietenable()
    }

    /// Other places that bear on the problem, such as where a duplicate key
    /// was first written.
    pub(crate) fn related(&self) -> Vec<(Relation, Option<Place>)> {
        match self {
            Kind::DuplicateKey { first, .. }
            | Kind::MoreWrittenTwice { first, .. }
            | Kind::SameId { first, .. }
            | Kind::LanguageWrittenTwice { first, .. }
            | Kind::FeedWrittenTwice { first, .. }
            | Kind::TwoLists {
                first_at: first, ..
            } => {
                vec![(Relation::FirstWritten, Some(*first))]
            }
            Kind::SameTranslation { first, .. } => vec![(Relation::FirstWritten, *first)],
            Kind::DefinitionWrittenTwice { shown, .. } => vec![(Relation::Shown, *shown)],
            Kind::FieldWrittenTwoWays { nested, .. } => {
                vec![(Relation::Nested, Some(*nested))]
            }
            Kind::IdUsedTwice {
                taken: Taken::Heading(first) | Taken::Table(first),
                ..
            } => vec![(Relation::FirstWritten, Some(*first))],
            Kind::LaterPageTaken {
                page_at: Some(at), ..
            } => vec![(Relation::AlsoPublished, Some(*at))],
            _ => Vec::new(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Problem {
    pub kind: Kind,
    /// None for a problem in no file.
    pub place: Option<Place>,
    /// Where the problem was met from, where that isn't where it is
    /// written: the page being built, and the calls that led there.
    pub(crate) context: Context,
}

/// The places a problem was met from, held apart since most problems
/// have none.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(crate) struct Context(Option<Box<Places>>);

#[derive(Clone, Default, PartialEq, Eq, Debug)]
struct Places(Vec<(Relation, Option<Place>)>);

impl Context {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &(Relation, Option<Place>)> {
        self.0.iter().flat_map(|places| places.0.iter())
    }

    fn push(&mut self, relation: Relation, place: Option<Place>) {
        self.0.get_or_insert_default().0.push((relation, place));
    }
}

/// A second place drawn under a problem's own, labelled with how it bears
/// on it; a label alone where what it names has no place.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Related {
    pub label: String,
    pub place: Option<Place>,
}

impl Problem {
    pub(crate) fn at(kind: Kind, place: Place) -> Self {
        Problem {
            kind,
            place: Some(place),
            context: Context::default(),
        }
    }

    /// The problem at `place`, or in no file where there is none.
    pub(crate) fn placed(kind: Kind, place: Option<Place>) -> Self {
        Problem {
            kind,
            place,
            context: Context::default(),
        }
    }

    pub(crate) fn nowhere(kind: Kind) -> Self {
        Problem {
            kind,
            place: None,
            context: Context::default(),
        }
    }

    /// The same problem, met from `place` as `relation` says.
    pub(crate) fn met(mut self, relation: Relation, place: Option<Place>) -> Self {
        self.context.push(relation, place);
        self
    }

    /// The same problem with its own place, and each place it was met
    /// from, as `to` gives it.
    pub(crate) fn moved(mut self, to: impl Fn(Place) -> Place) -> Self {
        self.place = self.place.map(&to);
        if let Some(places) = self.context.0.as_mut() {
            for (_, place) in &mut places.0 {
                *place = place.map(&to);
            }
        }
        self
    }

    /// Whether `other` is this problem met building another page: alike in
    /// all but the page it names.
    pub(crate) fn alike_but_for_page(&self, other: &Problem) -> bool {
        let rest = |problem: &Problem| {
            problem
                .context
                .iter()
                .filter(|(relation, _)| !matches!(relation, Relation::Building(_)))
                .cloned()
                .collect::<Vec<_>>()
        };
        self.place == other.place && self.kind == other.kind && rest(self) == rest(other)
    }

    /// The pages whose builds met the problem, the first to meet it first.
    pub(crate) fn built(&self) -> impl Iterator<Item = (Option<&Named>, Option<Place>)> {
        self.context
            .iter()
            .filter_map(|(relation, place)| match relation {
                Relation::Building(named) => Some((named.as_ref(), *place)),
                _ => None,
            })
    }

    /// Adds the pages whose builds met `other`, which is this problem met
    /// building them.
    pub(crate) fn met_building_too(&mut self, other: &Problem) {
        for (relation, place) in other.context.iter() {
            if matches!(relation, Relation::Building(_)) {
                self.context.push(relation.clone(), *place);
            }
        }
    }
}

/// A heading of an explanation without its `{#id}`, which only a page can
/// use.
pub(crate) fn unanchored(heading: &str) -> &str {
    heading
        .rsplit_once(" {#")
        .filter(|(_, id)| id.ends_with('}'))
        .map_or(heading, |(heading, _)| heading)
}

/// Why a file or folder couldn't be read, written or removed, or the
/// system couldn't do what was asked: Statac's own reason, the system's
/// by its kind, or what was said. Statac's own travel inside an
/// `io::Error` as its source.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Reason {
    /// It is a link, never followed there.
    Link,
    /// It is a link that leads outside the old site.
    LinkOutside,
    /// It is a folder or something else, not a file.
    NotAFile,
    /// It is a file, not a folder.
    NotAFolder,
    /// A folder with no `statac.yaml`.
    NotASite,
    /// A path outside the built site.
    OutsideBuilt,
    /// A path outside the new site.
    OutsideNew,
    /// The new site's folder is inside the old one, given here.
    InsideOld(PathBuf),
    /// The old site, given here, is inside the new site's folder.
    HoldsOld(PathBuf),
    /// The top of the disk, which is never removed.
    TopOfDisk,
    /// Something other than what is wanted there has its name.
    Taken,
    /// A file of the author's that no longer holds what was read.
    Changed,
    /// A file with a second name somewhere: one of the author's, which
    /// writing it would part it from, or one Statac keeps, which may be a
    /// file outside the site.
    NamedTwice,
    /// A path that ends in no folder's name.
    NamesNoFolder,
    /// A post whose front matter has no place an `id:` line would be read.
    NoPlaceForId,
    /// A file's address that can't be made, for this fault.
    Address(Box<AddressFault>),
    /// A line of a settings file that doesn't read as TOML.
    NotToml,
    /// A file Sass found not to be text.
    NotText,
    /// A file Sass couldn't read, saying no more.
    NotForSass,
    /// The system is watching as many files as it allows.
    WatchLimit,
    // The system's, by kind.
    Denied,
    FolderNamed,
    FileInTheWay,
    DiskFull,
    Missing,
    ReadOnly,
    NotUtf8,
    /// What the system or a library said, ready to follow a colon.
    Said(String),
}

impl Reason {
    /// The reason `error` gives: Statac's own where it carries one, else
    /// the system's.
    pub(crate) fn of(error: &io::Error) -> Reason {
        if let Some(reason) = error.get_ref().and_then(|inner| inner.downcast_ref()) {
            return Reason::clone(reason);
        }
        match Refused::of(error) {
            Some(Refused::Link) => return Reason::Link,
            Some(Refused::NotAFile) => return Reason::NotAFile,
            Some(Refused::Taken) => return Reason::Taken,
            Some(Refused::NamesNoFolder) => return Reason::NamesNoFolder,
            None => {}
        }
        match error.kind() {
            io::ErrorKind::PermissionDenied => Reason::Denied,
            io::ErrorKind::IsADirectory => Reason::FolderNamed,
            io::ErrorKind::NotADirectory => Reason::FileInTheWay,
            io::ErrorKind::StorageFull => Reason::DiskFull,
            io::ErrorKind::NotFound => Reason::Missing,
            io::ErrorKind::ReadOnlyFilesystem => Reason::ReadOnly,
            io::ErrorKind::InvalidData => Reason::NotUtf8,
            _ => Reason::Said(system_words(&error.to_string())),
        }
    }
}

impl std::error::Error for Reason {}

impl From<Reason> for io::Error {
    fn from(reason: Reason) -> io::Error {
        io::Error::other(reason)
    }
}

/// What the system said, to follow a colon: without its error number, and
/// with no capital to open it.
pub(crate) fn system_words(text: &str) -> String {
    let text = match text.rfind(" (os error ") {
        Some(i) => &text[..i],
        None => text,
    };
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// Why Sass stopped compiling a stylesheet. Names are as the author
/// wrote them.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SassFault {
    /// What an `@error` rule said, without the quotes of a string.
    Raised(String),
    NoVariable(String),
    NoMixin(String),
    NoFunction(String),
    /// A character Sass needed where it stopped.
    Missing(char),
    /// Where Sass needed a `{`, text that reads as a declaration with its
    /// colon left out, written as it should be.
    ReadsAsDeclaration(String),
    /// An opening never closed: a bracket, `#{`, a quote or `/*`.
    Unclosed(&'static str),
    ClosesNothing,
    NoValue,
    NoName,
    NoQuotedText,
    EndsEarly,
    /// A flag Sass took for a misspelt `!important`.
    UnknownFlag(String),
    /// An operator in a calculation without a space on each side.
    Unspaced(String),
    TopLevelParent,
    /// An at-rule where it can't be used.
    NotHere(String),
    ReturnOutsideFunction,
    /// `@use` or `@forward` after other rules.
    LoadAfterRules(String),
    ContentOutsideMixin,
    ExtendOutsideRule,
    /// What a function holds that it can't, in Sass's words.
    InFunction(String),
    /// A mixin handed a content block it doesn't take.
    NoContentBlock(String),
    NoReturn(String),
    MissingArgument {
        callable: String,
        argument: String,
    },
    TooManyArguments {
        callable: String,
        takes: usize,
        given: usize,
    },
    NoArgument {
        callable: String,
        argument: String,
    },
    /// A stylesheet loaded by a name no file has, and the nearest name
    /// one in its folder has.
    NoStylesheet {
        written: String,
        nearest: Option<String>,
    },
    /// A `sass:` module Sass doesn't have.
    NoBuiltInModule(String),
    /// A namespace no `@use` gives, and whether Sass has a module of that
    /// name to give it.
    NoNamespace {
        name: String,
        built_in: bool,
    },
    TwoModules(String),
    Private(String),
    NotDefault(String),
    LoadsItself,
    /// Units that can't be combined, in the order written where the text
    /// shows them.
    Units(String, String),
    /// An operation Sass can't carry out.
    Operation(String),
    /// A value, as Sass shows it, that CSS has no way to write, and
    /// whether Sass said where: met as it writes the stylesheet out, it
    /// gives no place.
    NotCss {
        value: String,
        placed: bool,
    },
    /// An argument handed the wrong kind of value, shown as Sass shows it.
    WrongType {
        argument: String,
        wanted: SassType,
        value: String,
    },
    HasUnits {
        argument: String,
        value: String,
    },
    OutOfRange {
        argument: String,
        index: String,
        items: usize,
    },
    RepeatedKey(String),
    ForUnits {
        from: String,
        to: String,
    },
    /// `@extend` of a compound selector, and the rule that extends each
    /// of its parts.
    CompoundExtend(Option<String>),
    IndentedSemicolon,
    /// Indentation of the kind a `.sass` file doesn't use: a tab where it
    /// indents with spaces, or a space where it indents with tabs.
    MixedIndent {
        tab: bool,
    },
    FirstLineIndented,
    /// grass's own words for anything else, their first line.
    Other(String),
}

/// The kind of value an argument needs.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SassType {
    Number,
    Colour,
    Map,
    String,
    Function,
    Arguments,
    Calculation,
    WholeNumber,
}

/// What stands where an output folder should be, when it isn't Statac's.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Occupant {
    File,
    Link,
    /// A folder without the marker, holding this much at its top.
    Entries {
        files: usize,
        folders: usize,
    },
}

/// What the parser found where the YAML stops making sense.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Malformed {
    /// Placed at the opening quote, not where the parser gave up.
    UnclosedQuote(char),
    /// A later line of a text in quotes that does close, standing too far
    /// left for the reader to take as more of the text. Placed at the
    /// line: the quote opens `back` bytes before it, and `spaces` before
    /// each later line are enough.
    ShallowLine {
        quote: char,
        back: usize,
        spaces: usize,
    },
    UnclosedBracket(char),
    MismatchedBracket {
        open: char,
        close: char,
    },
    /// An escape in double quotes YAML doesn't define, as written (`\q`).
    UnknownEscape(String),
    TextAfterQuote,
    Indentation,
    Tab,
    /// An alias to no anchor, as written (`*name`).
    UnknownAnchor(String),
    SecondDocument,
    TooDeep,
    EndsEarly,
    /// A character YAML allows nowhere, such as a C1 control left by text
    /// saved in an old encoding.
    ControlCharacter(char),
    /// Text the parser couldn't place, as written.
    Unexpected(String),
    /// A value tagged `!!binary` that isn't base64.
    NotBase64,
}

/// What is wrong with some JSON, where the reader stops.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum JsonFault {
    /// Placed at the opening quote.
    UnclosedString,
    UnclosedBracket(char),
    MismatchedBracket {
        open: char,
        close: char,
    },
    /// An escape JSON doesn't define, as written (`\q`).
    UnknownEscape(String),
    /// Half of a pair of `\u` escapes, as written (`\ud83d`).
    HalfCharacter(String),
    /// A character below U+0020 written as itself in a string.
    ControlCharacter(char),
    /// A number JSON doesn't write that way, as written (`01`, `.5`).
    NotANumber(String),
    /// A comma with no value after it, before `]` or `}`.
    TrailingComma,
    TooDeep,
    EndsEarly,
    /// Text the reader couldn't place, as written.
    Unexpected(String),
}

/// Which of the two alias limits was passed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Repeated {
    Values(usize),
    TextBytes(usize),
}

/// A snippet call Statac can't read, placed at the exact text.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CallFault {
    /// `@name(` with no `)` before the paragraph ends, as written.
    Unclosed(String),
    /// A quote opening a value that doesn't close on its line.
    UnclosedQuote(char),
    TextAfterQuote,
    /// A name in letters a snippet's name can't hold, as written.
    Name(String),
    /// A call on a line of its own to a snippet that wraps writing, with no
    /// `@end` after it, by the snippet's name.
    NoEnd(String),
    /// An `@end` with no call open for it to close.
    StrayEnd,
    /// A call to a snippet that wraps writing, somewhere other than on a
    /// line of its own, by the snippet's name.
    WrapsNothing(String),
    /// An `@url(...)` handed anything but one address from the top of the
    /// site.
    NotOneAddress,
}

/// Maths Statac can't make MathML of, placed at the exact text. What is
/// quoted is as written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MathsFault {
    UnknownCommand(String),
    UnknownEnvironment(String),
    UnknownColour(String),
    /// What opens a group the maths never closes, `{` or `\begin{cases}`,
    /// and what would close it. None where the opening can't be found.
    Unclosed {
        opening: Option<String>,
        closing: String,
    },
    /// A `}`, `\end{...}` or `\right` with nothing open for it to close.
    ClosesNothing(String),
    /// An `\end{...}` closing another environment, and the one it should be.
    Mismatched {
        written: String,
        expected: String,
    },
    /// A command short of what it takes, found at what comes next: a `}`,
    /// an `\end{...}`, or the closing dollars.
    MissingBefore(String),
    /// A second `^` or `_` on one thing.
    Twice(char),
    /// An `\eqref` to a label no equation on the page has.
    NoSuchLabel(String),
    /// Anything else, by what stands where Statac stopped.
    Unreadable(String),
}

/// A site that keeps videos.
#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum Host {
    YouTube,
    Vimeo,
}

/// What a build wrote.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Published {
    pub(crate) pages: usize,
    /// The pages left at old addresses, sending readers on.
    pub(crate) redirects: usize,
    pub(crate) feeds: usize,
    /// Everything else: assets, static files and the files beside the writing.
    pub(crate) files: usize,
}

/// Where JavaScript the built pages carry comes from.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(crate) enum Carried {
    /// A script the site publishes, at its address, from its file.
    Published { address: String, from: String },
    /// A script loaded from an address the site doesn't publish.
    Loaded(String),
    /// JavaScript written into pages by this file.
    Written(String),
    /// Something embedded from this address, empty where it has none,
    /// which may run scripts of its own.
    Embedded(String),
}

/// What one round asks of the pages already open.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Told {
    /// No built file changed.
    Nothing,
    /// Only stylesheets changed: the address each was at, and is at now.
    Styles(Vec<(String, String)>),
    Reload,
}

/// Why a folder is polled when polling wasn't asked for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(crate) enum Because {
    /// The folder is on a disk of this kind, which may not report changes.
    Remote(String),
    /// The system's reports couldn't be had, for this reason.
    NoReports(Reason),
}

/// The kind of thing a name holds.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(crate) enum Holds {
    Text,
    Html,
    Number,
    Truth,
    Date,
    Page,
    Pages,
    /// One of the site's lists, as a tag or a folder is.
    List,
    Lists,
    /// Any other list of things.
    Items,
    Fields,
    Mentions,
    Call,
    Unknown,
}

/// The most levels of writing, quotations, lists, emphasis and the like,
/// one inside another, that are read.
pub(crate) const DEEPEST_NESTING: usize = 200;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{Sources, Span};

    #[test]
    fn errors_are_never_quietenable() {
        let kind = Kind::OtherSpelling {
            word: Word::Field,
            written: "template".into(),
            ours: "layout".into(),
        };
        assert!(!kind.quietenable());
    }

    #[test]
    fn a_broken_link_can_be_quietened() {
        let kind = Kind::BrokenLink {
            target: "/about-us/".into(),
            suggestion: None,
            inside: false,
        };
        assert!(kind.quietenable());
    }

    #[test]
    fn names_are_unique_and_found_again() {
        for &code in Code::ALL {
            assert_eq!(Code::from_name(code.name()), Some(code));
        }
        let mut names: Vec<_> = Code::ALL.iter().map(|code| code.name()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), Code::ALL.len());
        assert_eq!(Code::from_name("brokn-link"), None);
    }

    #[test]
    fn every_kind_has_one_explanation_and_every_explanation_a_kind() {
        let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("explanations");
        let mut named = Vec::new();
        for entry in std::fs::read_dir(folder).unwrap() {
            let file = entry.unwrap().file_name().into_string().unwrap();
            let name = file.strip_suffix(".md").unwrap_or_else(|| panic!("{file}"));
            assert!(Code::from_name(name).is_some(), "{file} names no kind");
            named.push(Code::from_name(name).unwrap());
        }
        named.sort_unstable();
        assert_eq!(named, Code::ALL);
    }

    #[test]
    fn a_former_name_is_no_kind_now_and_names_one_kind() {
        let mut formers: Vec<_> = FORMER_NAMES.iter().map(|(former, _)| *former).collect();
        formers.sort_unstable();
        formers.dedup();
        assert_eq!(formers.len(), FORMER_NAMES.len());
        for &(former, code) in FORMER_NAMES {
            assert_eq!(Code::from_name(former), None, "{former}");
            assert_eq!(Code::renamed(former), Some(code));
            assert!(code.former_names().any(|name| name == former));
        }
        assert_eq!(
            Code::unknown("undescribed-picture"),
            Kind::UnknownProblemName {
                written: "undescribed-picture".into(),
                suggestion: Some("missing-alt-text".into()),
                former: Former::Renamed,
            }
        );
    }

    #[test]
    fn a_retired_name_is_no_kind_now_nor_a_former_name() {
        for &(retired, code) in RETIRED_NAMES {
            assert_eq!(Code::from_name(retired), None, "{retired}");
            assert_eq!(Code::renamed(retired), None, "{retired}");
            assert!(code.former_names().any(|name| name == retired));
            assert_eq!(
                Code::unknown(retired),
                Kind::UnknownProblemName {
                    written: retired.into(),
                    suggestion: None,
                    former: Former::Retired,
                }
            );
        }
    }

    #[test]
    fn explanations_follow_the_wording_guide() {
        for &code in Code::ALL {
            let text = code.explanation();
            let name = code.name();
            assert!(text.starts_with("# "), "{name}");
            for part in ["\n## How to fix it\n", "\n## Example\n"] {
                assert!(text.contains(part), "{name} has no {part:?}");
            }
            // What the terminal prints comes straight after the opening.
            let seen = "## What you'll see {#what-youll-see}";
            let first = text.lines().find(|line| line.starts_with("## "));
            assert_eq!(first, Some(seen), "{name}");
            assert_eq!(text.matches(seen).count(), 1, "{name}");
            assert!(!text.contains(['\u{2013}', '\u{2014}']), "{name}");
            assert!(!text.contains("! ") && !text.contains("!\n"), "{name}");
            // The words are prose's to avoid: a setting named in code, such
            // as `url`, is the author's to write.
            let prose: String = text
                .split("```")
                .step_by(2)
                .flat_map(|outside| outside.split('`').step_by(2))
                .collect::<Vec<_>>()
                .join(" ");
            let lower = prose.to_lowercase();
            let words: Vec<_> = lower.split(|c: char| !c.is_alphanumeric()).collect();
            for banned in [
                "invalid",
                "fatal",
                "please",
                "simply",
                "just",
                "oops",
                "parse",
                "render",
                "undefined",
                "picture",
                "pictures",
                "photograph",
                "photographs",
            ] {
                assert!(!words.contains(&banned), "{name} says {banned:?}");
            }
            assert!(!lower.contains("failed to"), "{name}");
        }
    }

    #[test]
    fn a_terminal_is_shown_no_heading_ids() {
        for &code in Code::ALL {
            let (written, shown) = (code.explanation(), code.explained());
            assert!(shown.contains("\n## What you'll see\n"), "{}", code.name());
            assert_eq!(
                shown.replace("\r\n", "\n").len() + " {#what-youll-see}".len(),
                written.replace("\r\n", "\n").len(),
                "{}",
                code.name()
            );
        }
        // A heading inside a code block is the author's example.
        let shown = Code::IdUsedTwice.explained();
        assert!(shown.contains("\n## Getting there {#route}\n"));
    }

    #[test]
    fn a_duplicate_key_points_back_at_its_first() {
        let mut sources = Sources::default();
        let first = Place {
            source: sources.add("content/post-1.md", "title: Post 1\n".into()),
            span: Span::new(0, 5),
        };
        let problem = Problem::at(
            Kind::DuplicateKey {
                key: "title".into(),
                first,
            },
            first,
        );
        let related = problem.related();
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].place, Some(first));
        assert_eq!(related[0].label, "First written");
    }

    #[test]
    fn system_reasons_lose_their_error_numbers() {
        let error = io::Error::from(io::ErrorKind::PermissionDenied);
        assert_eq!(Reason::of(&error), Reason::Denied);
        let error = io::Error::other("Something odd (os error 99)");
        assert_eq!(Reason::of(&error), Reason::Said("something odd".into()));
    }

    #[test]
    fn statacs_own_reason_comes_back_out_of_an_error() {
        let error = io::Error::from(Reason::InsideOld("old".into()));
        assert_eq!(Reason::of(&error), Reason::InsideOld("old".into()));
    }
}
