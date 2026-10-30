//! The message each explanation shows under "What you'll see", held to
//! what Statac prints. Where a small site can meet the kind, the message
//! is the one a run of the command prints for the site, which follows the
//! explanation's Example. Where none can, such as for a full disk, it is
//! a fixture of the kind drawn as Statac draws it, at the Example's text
//! where the kind has a place.

use std::ffi::OsString;
use std::path::Path;

use super::tests::kinds;
use super::{explanation_link, more, severity};
use crate::album::Progress;
use crate::cli::Terminal;
use crate::problem::{Code, Kind, Maker, Problem, Reason, Severity, Unheard};
use crate::report::Report;
use crate::source::{Place, Sources, Span};
use crate::testing::Scratch;

/// A site, and the command that meets a kind of problem in it.
struct Run {
    code: Code,
    /// The command line, with `{site}` standing for the site's folder.
    args: &'static [&'static str],
    /// The site's files by their paths in it, with an empty `statac.yaml`
    /// where they hold none.
    files: &'static [(&'static str, &'static str)],
    /// Images of the site, made as their names' extensions say, so many
    /// pixels wide and tall.
    images: &'static [(&'static str, u32, u32)],
    /// Images among `images` cut short, as a copy stopped part way leaves
    /// one.
    cut: &'static [&'static str],
    /// Files of the site copied from the repository, by their paths in
    /// each.
    copies: &'static [(&'static str, &'static str)],
    /// Files the site is built with before `files` are written over them.
    first: &'static [(&'static str, &'static str)],
    /// Folders of the site made read-only for the run.
    read_only: &'static [&'static str],
    /// Whether the run stands for one in the site's folder, which names
    /// the site's files from there, not from the folder the command names.
    inside: bool,
    /// Whether the run meets the kind as on Unix alone: Windows can't hold
    /// some of its files, spells a shell's words its own way, or writes
    /// into a read-only folder.
    unix: bool,
}

impl Run {
    const fn new(
        code: Code,
        args: &'static [&'static str],
        files: &'static [(&'static str, &'static str)],
    ) -> Run {
        Run {
            code,
            args,
            files,
            images: &[],
            cut: &[],
            copies: &[],
            first: &[],
            read_only: &[],
            inside: false,
            unix: false,
        }
    }

    const fn with_images(self, images: &'static [(&'static str, u32, u32)]) -> Run {
        Run { images, ..self }
    }

    const fn cutting(self, cut: &'static [&'static str]) -> Run {
        Run { cut, ..self }
    }

    const fn with_copies(self, copies: &'static [(&'static str, &'static str)]) -> Run {
        Run { copies, ..self }
    }

    const fn built_first(self, first: &'static [(&'static str, &'static str)]) -> Run {
        Run { first, ..self }
    }

    const fn read_only(self, read_only: &'static [&'static str]) -> Run {
        Run { read_only, ..self }
    }

    const fn inside(self) -> Run {
        Run {
            inside: true,
            ..self
        }
    }

    const fn on_unix(self) -> Run {
        Run { unix: true, ..self }
    }
}

/// `statac check`, run in the site.
const CHECK: &[&str] = &["check", "{site}"];

const RUNS: &[Run] = &[
    Run::new(
        Code::BrokenLink,
        CHECK,
        &[
            ("content/post-1.md", "Read [Post 2](/post-2/) next.\n"),
            ("content/posts/post-2.md", "Post 2.\n"),
        ],
    ),
    Run::new(
        Code::OtherSpelling,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ntemplate: post\n---\n",
        )],
    ),
    Run::new(
        Code::RetiredSetting,
        CHECK,
        &[("statac.yaml", "title: Site\ncode_style: ocean\n")],
    ),
    Run::new(Code::CantWrite, &["build", "{site}"], &[])
        .built_first(&[("content/post-1.md", "Post 1.\n")])
        .read_only(&["_site"])
        .inside()
        .on_unix(),
    Run::new(
        Code::MalformedYaml,
        CHECK,
        &[("content/post-1.md", "---\ntitle: \"Post 1\n---\n")],
    ),
    Run::new(
        Code::MalformedJson,
        CHECK,
        &[(
            "data/people.json",
            "[\n  {\"name\": \"Person 1\"},\n  {\"name\": \"Person 2\"},\n]\n",
        )],
    ),
    Run::new(
        Code::DuplicateKey,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-04\ntitle: Post 2\n---\n",
        )],
    ),
    Run::new(
        Code::AliasLimit,
        CHECK,
        &[(
            "data/values.yaml",
            "a: &a [one, one, one, one, one, one, one, one, one, one]\n\
             b: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a]\n\
             c: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]\n\
             d: [*c, *c, *c, *c, *c, *c, *c, *c, *c, *c]\n",
        )],
    ),
    Run::new(
        Code::UnknownYamlTag,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\nversion: !local 3.10\n---\n",
        )],
    ),
    Run::new(
        Code::NotData,
        CHECK,
        &[("data/people.txt", "Person 1\nPerson 2\n")],
    ),
    Run::new(
        Code::DataNamedTwice,
        CHECK,
        &[
            ("data/people.yaml", "- Person 1\n"),
            ("data/people.json", "[\"Person 1\"]\n"),
        ],
    ),
    Run::new(
        Code::MergeKey,
        CHECK,
        &[(
            "data/people.yaml",
            "- &first\n  name: Person 1\n  role: Role 1\n- <<: *first\n  name: Person 2\n",
        )],
    ),
    Run::new(
        Code::SameId,
        CHECK,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\nid: k3m9x2\n---\n"),
            ("content/post-2.md", "---\ntitle: Post 2\nid: k3m9x2\n---\n"),
        ],
    ),
    Run::new(
        Code::UnclosedFrontMatter,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-04\n\nThe first paragraph.\n",
        )],
    ),
    Run::new(
        Code::UnknownProblemName,
        CHECK,
        &[("statac.yaml", "quiet: [brokn-link]\n")],
    ),
    Run::new(
        Code::CantQuieten,
        CHECK,
        &[("statac.yaml", "quiet: [broken-link, duplicate-key]\n")],
    ),
    Run::new(Code::UnknownCommand, &["biuld"], &[]),
    Run::new(Code::MissingCommand, &["--out=public"], &[]),
    Run::new(Code::UnknownOption, &["check", "--out=public"], &[]),
    Run::new(Code::MissingValue, &["build", "--out", "public"], &[]),
    Run::new(
        Code::OptionGivenTwice,
        &["build", "--out=public", "--out=dist"],
        &[],
    ),
    Run::new(Code::UnexpectedArgument, &["build", "docs", "public"], &[]),
    Run::new(Code::MissingName, &["explain"], &[]),
    Run::new(Code::NothingToSearch, &["search"], &[]),
    Run::new(Code::UnusableName, &["new", "site-1\u{1b}"], &[]),
    Run::new(
        Code::MissingFile,
        CHECK,
        &[
            ("templates/base.html", "{{ css(\"sit.css\") }}\n"),
            ("assets/site.css", "p {}\n"),
            ("content/post-1.md", "---\nlayout: base\n---\n"),
        ],
    ),
    Run::new(
        Code::SassStopped,
        CHECK,
        &[
            ("assets/_parts.scss", "a {\n  color: $link;\n}\n"),
            ("assets/site.scss", "@use \"parts\";\n"),
        ],
    ),
    Run::new(
        Code::WorkerOutsideImportMap,
        CHECK,
        &[
            (
                "assets/app.js",
                "const worker = new Worker(\"./worker.js\", { type: \"module\" });\n",
            ),
            ("assets/worker.js", "postMessage(1);\n"),
        ],
    ),
    Run::new(
        Code::SameUrl,
        CHECK,
        &[
            ("content/about.md", "About.\n"),
            (
                "content/about-us.md",
                "---\nredirect_from: [/about/]\n---\nAbout us.\n",
            ),
        ],
    ),
    Run::new(
        Code::UrlTaken,
        CHECK,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            (
                "content/post-2.md",
                "---\ntitle: Post 2\ndraft: true\npermalink: /post-1/\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::UrlGone,
        CHECK,
        &[
            ("published.txt", "/about/\n"),
            ("content/about-us.md", "---\ntitle: About us\n---\n"),
        ],
    ),
    Run::new(
        Code::UnwritableName,
        CHECK,
        &[("static/con.txt", "Contact.\n")],
    )
    .on_unix(),
    Run::new(
        Code::OutsideSite,
        CHECK,
        &[("assets/site.scss", "@use \"../../shared/colours\";\n")],
    ),
    Run::new(
        Code::SassWarning,
        CHECK,
        &[
            ("assets/_grid.scss", "@warn \"$columns is deprecated\";\n"),
            ("assets/site.scss", "@use \"grid\";\n"),
        ],
    ),
    Run::new(
        Code::MalformedCall,
        CHECK,
        &[("content/post-1.md", "A photograph: @image('photo-1.jpg'\n")],
    ),
    Run::new(
        Code::MalformedMaths,
        CHECK,
        &[("content/post-1.md", "The area is $\\frac{1}{2 b h$.\n")],
    ),
    Run::new(
        Code::ControlCharacter,
        CHECK,
        &[("content/post-1.md", "Qu\u{92}un\n")],
    ),
    Run::new(
        Code::ColonInCall,
        CHECK,
        &[(
            "content/post-1.md",
            "@image('photo-1.jpg', caption: 'Post 1')\n",
        )],
    ),
    Run::new(
        Code::PublishedAsText,
        CHECK,
        &[("content/post-1.md", "{{< figure src=\"photo-1.jpg\" >}}\n")],
    ),
    Run::new(
        Code::UnknownWord,
        CHECK,
        &[("statac.yaml", "tags.permalnk: /topics/{slug}/\n")],
    ),
    Run::new(
        Code::AlwaysFalse,
        CHECK,
        &[
            (
                "templates/post.html",
                "{% if page.subtitel is defined %}\n  <p>{{ page.subtitel }}</p>\n{% endif %}\n",
            ),
            (
                "content/post-1.md",
                "---\ntitle: Post 1\nsubtitle: The first\nlayout: post\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::ReadAsCustom,
        CHECK,
        &[("content/post-1.md", "---\ntitel: Post 1\n---\n")],
    ),
    Run::new(
        Code::NestedSetting,
        CHECK,
        &[(
            "content/notes/index.md",
            "---\ntitle: Notes\npages:\n  layout: note\n---\n",
        )],
    ),
    Run::new(
        Code::FieldWrittenTwoWays,
        CHECK,
        &[
            (
                "content/post-1.md",
                "---\ntitle: Post 1\nbook.author: Author 1\n---\n",
            ),
            (
                "content/post-2.md",
                "---\ntitle: Post 2\nbook:\n  author: Author 2\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::ReservedName,
        CHECK,
        &[("statac.yaml", "title: Site 1\nposts: 12\n")],
    ),
    Run::new(
        Code::SetAside,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\nsource: notes/post-1.txt\n---\n",
        )],
    ),
    Run::new(
        Code::WrongKind,
        CHECK,
        &[("content/post-1.md", "---\ntitle: Post 1\ndraft: yes\n---\n")],
    ),
    Run::new(
        Code::NamesNothing,
        CHECK,
        &[
            ("statac.yaml", "fields.author: one of data.people\n"),
            (
                "data/people.yaml",
                "person-1:\n  name: Person 1\nperson-2:\n  name: Person 2\n",
            ),
            (
                "content/post-1.md",
                "---\ntitle: Post 1\nauthor: persn-1\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::NeedsQuotes,
        CHECK,
        &[
            ("statac.yaml", "fields.version: text\n"),
            (
                "content/post-1.md",
                "---\ntitle: Version 3.10\nversion: 3.10\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::OneTagNotTwo,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ntags: hills, water\n---\n",
        )],
    ),
    Run::new(
        Code::UnclearDate,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 3/4/2026\n---\n",
        )],
    ),
    Run::new(
        Code::NoSuchDate,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-02-30\n---\n",
        )],
    ),
    Run::new(
        Code::DatesDisagree,
        CHECK,
        &[(
            "content/2026-10-04-post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-05\n---\n",
        )],
    ),
    Run::new(
        Code::NotStamped,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: =monday 09:00\n---\n",
        )],
    ),
    Run::new(
        Code::MissingField,
        CHECK,
        &[
            ("statac.yaml", "types.book.isbn: required text\n"),
            (
                "content/books/book-1.md",
                "---\ntitle: Book 1\ntype: book\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::MissingData,
        CHECK,
        &[
            ("statac.yaml", "fields.author: one of data.poeple\n"),
            ("data/people.yaml", "person-1:\n  name: Person 1\n"),
        ],
    ),
    Run::new(
        Code::MissingContentFolder,
        CHECK,
        &[
            ("statac.yaml", "fields.review_of: one of folder.boks\n"),
            ("content/books/book-1.md", "---\ntitle: Book 1\n---\n"),
        ],
    ),
    Run::new(
        Code::NoNamesToCheck,
        CHECK,
        &[
            ("statac.yaml", "fields.author: one of data.people\n"),
            ("data/people.yaml", "- name: Person 1\n- name: Person 2\n"),
        ],
    ),
    Run::new(
        Code::WrongPlace,
        CHECK,
        &[(
            "content/notes/index.md",
            "---\ntitle: Notes\npages.title: Note\n---\n",
        )],
    ),
    Run::new(
        Code::UnusableUrl,
        CHECK,
        &[(
            "content/notes/index.md",
            "---\npages.permalink: /notes/{yaer}/{name}/\n---\n",
        )],
    ),
    Run::new(
        Code::UnusableDateFormat,
        CHECK,
        &[("statac.yaml", "date_format: 1 January 2020\n")],
    ),
    Run::new(
        Code::NoTimeZone,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-04 09:30\n---\n",
        )],
    ),
    Run::new(
        Code::HookStopped,
        &["build", "{site}"],
        &[
            (
                "statac.yaml",
                "before_build: cat styles/base.css styles/print.css > assets/site.css\n",
            ),
            ("styles/base.css", "body { margin: 0; }\n"),
            ("assets/site.css", ""),
        ],
    )
    .on_unix(),
    Run::new(
        Code::HookNotRun,
        CHECK,
        &[("statac.yaml", "after_build: gzip -k -r \"$STATAC_OUT\"\n")],
    ),
    Run::new(
        Code::NothingMade,
        &["build", "{site}"],
        &[(
            "statac.yaml",
            "after_build: echo built\nafter_build_outputs: [search/]\n",
        )],
    ),
    Run::new(
        Code::OutputFolderNamed,
        CHECK,
        &[("statac.yaml", "after_build: ./index.sh _site\n")],
    )
    .on_unix(),
    Run::new(
        Code::UnknownLanguage,
        CHECK,
        &[("statac.yaml", "language: sw\n")],
    ),
    Run::new(
        Code::LanguageWrittenTwice,
        CHECK,
        &[("statac.yaml", "language: [en, ar, EN]\n")],
    ),
    Run::new(
        Code::NoPagesInLanguage,
        CHECK,
        &[
            ("statac.yaml", "language: [ar, en]\n"),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("content/english/index.md", "---\ntitle: Home\n---\n"),
        ],
    ),
    Run::new(
        Code::MissingTranslation,
        CHECK,
        &[
            ("statac.yaml", "language: fr\n"),
            (
                "languages/fr.yaml",
                "callouts: [Note, Tip, Important, Warning, Caution]\n",
            ),
            ("content/post-1.md", "Une phrase.[^1]\n\n[^1]: Une note.\n"),
        ],
    ),
    Run::new(
        Code::LanguageFolderInPattern,
        CHECK,
        &[
            ("statac.yaml", "language: [en, fr]\n"),
            (
                "content/en/index.md",
                "---\ntitle: Home\npages.permalink: /en/{year}/{slug}/\n---\n",
            ),
            (
                "content/en/post-1.md",
                "---\ntitle: Post 1\ndate: 2026-10-04\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::SameTranslation,
        CHECK,
        &[
            ("statac.yaml", "language: [fr, en]\n"),
            ("content/en/about.md", "---\ntranslation_key: about\n---\n"),
            (
                "content/en/about-us.md",
                "---\ntranslation_key: about\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::UnmatchedTranslationKey,
        CHECK,
        &[
            ("statac.yaml", "language: [fr, en]\n"),
            ("content/about.md", "---\ntranslation_key: about\n---\n"),
            (
                "content/en/about-us.md",
                "---\ntranslation_key: abuot\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::NotLinkedByPlace,
        CHECK,
        &[
            ("statac.yaml", "language: [fr, en]\n"),
            ("content/about.md", "---\ntranslation_key: about\n---\n"),
            ("content/en/about.md", "---\ntitle: About\n---\n"),
        ],
    ),
    Run::new(
        Code::UnknownCodeLanguage,
        CHECK,
        &[
            ("statac.yaml", "code_colours: true\n"),
            ("content/post-1.md", "```cosole\n$ statac build\n```\n"),
        ],
    ),
    Run::new(
        Code::MalformedTemplate,
        CHECK,
        &[
            (
                "templates/post.html",
                "{% if page.date %}\n<p>{{ page.date }}</p>\n",
            ),
            ("content/post-1.md", "---\nlayout: post\n---\n"),
        ],
    ),
    Run::new(
        Code::MissingTemplate,
        CHECK,
        &[
            ("content/posts/index.md", "---\npages.layout: psot\n---\n"),
            ("content/posts/post-1.md", "---\ntitle: Post 1\n---\n"),
            ("templates/post.html", "{{ page.content }}\n"),
        ],
    ),
    Run::new(
        Code::BuiltWithPageTemplate,
        CHECK,
        &[
            ("templates/page.html", "{{ page.content }}\n"),
            ("content/index.md", "---\ntitle: Home\n---\n"),
        ],
    ),
    Run::new(
        Code::BuiltWithoutTemplate,
        CHECK,
        &[
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("content/hello.md", "# Hello\n\nThe first page.\n"),
        ],
    ),
    Run::new(
        Code::NoValue,
        CHECK,
        &[
            ("templates/post.html", "<p>{{ page.summary }}</p>\n"),
            (
                "content/post-1.md",
                "---\ntitle: Post 1\nlayout: post\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::NotSupported,
        CHECK,
        &[
            (
                "templates/folder.html",
                "{% for post in page.posts %}\n{% include \"card.html\" %}\n{% endfor %}\n",
            ),
            ("content/posts/index.md", "---\ntitle: Posts\n---\n"),
        ],
    ),
    Run::new(
        Code::WrongArgument,
        CHECK,
        &[
            ("snippets/card.html", "<h2>{{ post.title }}</h2>\n"),
            (
                "templates/folder.html",
                "{% for post in page.posts %}\n{{ card(pots=post) }}\n{% endfor %}\n",
            ),
            ("content/posts/index.md", "---\ntitle: Posts\n---\n"),
            ("content/posts/post-1.md", "---\ntitle: Post 1\n---\n"),
        ],
    ),
    Run::new(
        Code::TemplateStopped,
        CHECK,
        &[
            ("templates/post.html", "<p>{{ page.title | date }}</p>\n"),
            (
                "content/post-1.md",
                "---\ntitle: Post 1\nlayout: post\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::PageInSnippet,
        CHECK,
        &[
            (
                "snippets/card.html",
                "<h2>{{ page.title }}</h2>\n<a href=\"{{ post.url }}\">Read it</a>\n",
            ),
            (
                "templates/folder.html",
                "{% for post in page.posts %}\n{{ card(post) }}\n{% endfor %}\n",
            ),
            ("content/posts/index.md", "---\ntitle: Posts\n---\n"),
            ("content/posts/post-1.md", "---\ntitle: Post 1\n---\n"),
        ],
    ),
    Run::new(
        Code::TextNeverShown,
        CHECK,
        &[
            ("templates/base.html", "{% block main %}{% endblock %}\n"),
            (
                "templates/post.html",
                "{% extends \"base\" %}\n{% block content %}{{ page.content }}{% endblock %}\n",
            ),
            ("content/post-1.md", "---\nlayout: post\n---\n"),
        ],
    ),
    Run::new(
        Code::ContentNeverShown,
        CHECK,
        &[
            ("templates/hollow.html", "<h1>{{ page.title }}</h1>\n"),
            (
                "content/post-1.md",
                "---\ntitle: Post 1\nlayout: hollow\n---\nPost 1 says this.\n",
            ),
        ],
    ),
    Run::new(
        Code::SpelledTwoWays,
        CHECK,
        &[
            ("content/post-1.md", "---\ntags: [Rust]\n---\n"),
            ("content/post-2.md", "---\ntags: [Rust]\n---\n"),
            ("content/post-3.md", "---\ntags: [rust]\n---\n"),
        ],
    ),
    Run::new(
        Code::DraftLeftOut,
        CHECK,
        &[(
            "content/post-1.md",
            "---\ntitle: Post 1\ndraft: true\n---\n",
        )],
    ),
    Run::new(
        Code::ScheduledPost,
        &["check", "{site}", "--today=2026-10-04"],
        &[(
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-05\n---\n",
        )],
    ),
    Run::new(
        Code::NumberedFolder,
        CHECK,
        &[
            ("content/days/01-day-one.md", "---\ntitle: Day one\n---\n"),
            ("content/days/02-day-two.md", "---\ntitle: Day two\n---\n"),
            (
                "content/days/03-day-three.md",
                "---\ntitle: Day three\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::UnorderedPages,
        CHECK,
        &[
            (
                "content/days/index.md",
                "---\ntitle: Days\norder: [day-one, day-two]\n---\n",
            ),
            ("content/days/day-one.md", "---\ntitle: Day one\n---\n"),
            ("content/days/day-two.md", "---\ntitle: Day two\n---\n"),
            ("content/days/day-three.md", "---\ntitle: Day three\n---\n"),
        ],
    ),
    Run::new(
        Code::MalformedList,
        CHECK,
        &[(
            "content/rust.md",
            "---\ntitle: Rust\nposts: site.posts | tagged(Rust\n---\n",
        )],
    ),
    Run::new(
        Code::NotAPost,
        CHECK,
        &[
            ("statac.yaml", "tags.pages: true\n"),
            (
                "content/tags/rust.md",
                "---\ntitle: Rust\ndate: 2026-10-04\n---\n",
            ),
            (
                "content/post-1.md",
                "---\ntitle: Post 1\ntags: [Rust]\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::TwoLists,
        CHECK,
        &[(
            "content/rust.md",
            "---\ntitle: Rust\nposts: site.posts | tagged(\"Rust\")\neach: site.posts | tagged(\"Rust\")\npermalink: /cards/{name}/\n---\n",
        )],
    ),
    Run::new(
        Code::FeedNotWritten,
        CHECK,
        &[
            ("statac.yaml", "feed: true\n"),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            (
                "content/post-1.md",
                "---\ntitle: Post 1\ndate: 2026-10-04\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::FeedWrittenTwice,
        CHECK,
        &[
            (
                "statac.yaml",
                "url: https://example.org/\nfeed_key: 0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20\n",
            ),
            (
                "content/posts/index.md",
                "---\ntitle: Posts\nfeed: [feed.xml, rss.xml, feed.xml]\n---\n",
            ),
            (
                "content/posts/post-1.md",
                "---\ntitle: Post 1\ndate: 2026-10-04\n---\n",
            ),
            (
                "templates/feed.xml",
                "<feed xmlns=\"http://www.w3.org/2005/Atom\"></feed>\n",
            ),
            ("templates/rss.xml", "<rss version=\"2.0\"></rss>\n"),
        ],
    ),
    Run::new(
        Code::FeedFormatMismatch,
        CHECK,
        &[
            (
                "statac.yaml",
                "title: Site\nurl: https://example.org/\nfeed_key: 0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20\n",
            ),
            (
                "content/index.md",
                "---\ntitle: Home\nfeed: [atom.xml, rss.xml]\n---\n",
            ),
            (
                "content/post-1.md",
                "---\ntitle: Post 1\ndate: 2026-10-04\n---\n",
            ),
            (
                "templates/feed.xml",
                "<feed xmlns=\"http://www.w3.org/2005/Atom\"></feed>\n",
            ),
        ],
    ),
    Run::new(
        Code::NoSiteUrl,
        CHECK,
        &[
            (
                "content/sitemap.md",
                "---\npermalink: /sitemap.xml\nlayout: sitemap.xml\nneeds: url\n---\n",
            ),
            ("templates/sitemap.xml", "<urlset></urlset>\n"),
        ],
    ),
    Run::new(
        Code::UnusedFeedTemplate,
        CHECK,
        &[(
            "templates/feed.xml",
            "<feed xmlns=\"http://www.w3.org/2005/Atom\"></feed>\n",
        )],
    ),
    Run::new(
        Code::UnusedLanguageFile,
        CHECK,
        &[
            ("statac.yaml", "language: en-GB\n"),
            (
                "languages/en-US.yaml",
                "back_to_the_text: back to the text\n",
            ),
        ],
    ),
    Run::new(
        Code::NoLanguage,
        CHECK,
        &[
            ("statac.yaml", "title: Site 1\n"),
            (
                "templates/page.html",
                "<html lang=\"{{ page.language }}\">\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
        ],
    ),
    Run::new(
        Code::NoListPage,
        CHECK,
        &[
            ("content/index.md", "---\ntitle: Home\n---\n"),
            (
                "content/notes/note-1.md",
                "---\ndate: 2026-10-04\n---\nNote 1.\n",
            ),
            (
                "content/notes/note-2.md",
                "---\ndate: 2026-10-05\n---\nNote 2.\n",
            ),
            ("templates/folder.html", "{{ page.content }}\n"),
        ],
    ),
    Run::new(
        Code::NoHomePage,
        CHECK,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
            ("templates/page.html", "{{ page.content }}\n"),
        ],
    ),
    Run::new(
        Code::RobotsBeneathPath,
        CHECK,
        &[
            ("statac.yaml", "url: https://example.org/blog/\n"),
            (
                "content/robots.md",
                "---\npermalink: /robots.txt\nlayout: robots.txt\n---\n",
            ),
            ("templates/robots.txt", "User-agent: *\n"),
        ],
    ),
    Run::new(
        Code::RedirectToItself,
        CHECK,
        &[("content/post-1.md", "---\nredirect_to: /post-1/\n---\n")],
    ),
    Run::new(
        Code::ContentNotPublished,
        CHECK,
        &[
            (
                "content/post-1.md",
                "---\nredirect_to: /posts/post-1/\n---\nThe first post.\n",
            ),
            ("content/posts/post-1.md", "The first post.\n"),
        ],
    ),
    Run::new(
        Code::SortedLast,
        CHECK,
        &[
            (
                "templates/page.html",
                "{% for post in site.pages | sort(attribute=\"rank\") %}\n<p>{{ post.title }}</p>\n{% endfor %}\n",
            ),
            ("content/post-1.md", "---\ntitle: Post 1\nrank: 1\n---\n"),
            ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
        ],
    ),
    Run::new(
        Code::CallInHtml,
        CHECK,
        &[
            ("snippets/book.html", "<cite>{{ title }}</cite>\n"),
            (
                "content/post-1.md",
                "<div class=\"aside\">@book('Post 1')</div>\n",
            ),
        ],
    ),
    Run::new(
        Code::MoreWrittenTwice,
        CHECK,
        &[(
            "content/post-1.md",
            "The part a list shows.\n\n<!--more-->\n\nMore of the post.\n\n<!--more-->\n\nThe rest.\n",
        )],
    ),
    Run::new(
        Code::UntitledPage,
        CHECK,
        &[
            (
                "templates/page.html",
                "<title>{{ headline(page) }}</title>\n",
            ),
            (
                "content/post-1.md",
                "---\ndate: 2026-10-04\n---\nPost 1 opens with this paragraph.\n",
            ),
        ],
    ),
    Run::new(
        Code::MissingDefinition,
        CHECK,
        &[(
            "content/post-1.md",
            "The figures are from 2026.[^source]\n\n[^sources]: The yearly report.\n",
        )],
    ),
    Run::new(
        Code::MissingAnchor,
        CHECK,
        &[
            (
                "content/post-1.md",
                "See [how to get there](/about/#getting-here).\n",
            ),
            ("content/about.md", "## Getting there\n"),
        ],
    ),
    Run::new(
        Code::FootnoteNeverShown,
        CHECK,
        &[(
            "content/post-1.md",
            "The figures are from 2026.[^source]\n\n[^sources]: The yearly report.\n",
        )],
    ),
    Run::new(
        Code::DefinitionWrittenTwice,
        CHECK,
        &[(
            "content/post-1.md",
            "The figures are from 2026.[^1] The photograph is from 2025.[^1]\n\n[^1]: The yearly report.\n\n[^1]: Photograph by the author.\n",
        )],
    ),
    Run::new(
        Code::CellNeverShown,
        CHECK,
        &[(
            "content/post-1.md",
            "| Name   | Count |\n|--------|-------|\n| Post 1 | 1     | Extra |\n",
        )],
    ),
    Run::new(
        Code::IdUsedTwice,
        CHECK,
        &[(
            "content/post-1.md",
            "## Getting there {#route}\n\n## Coming back {#route}\n",
        )],
    ),
    Run::new(
        Code::UnmatchedTag,
        CHECK,
        &[(
            "content/post-1.md",
            "<div class=\"box\">\n\nA note in a box.\n",
        )],
    ),
    Run::new(Code::NestedTooDeep, CHECK, &[("content/post-1.md", DEEP)]),
    Run::new(
        Code::FeedIdChanged,
        CHECK,
        &[(
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\npermalink: /writing/post-1/\n---\n",
        )],
    )
    .built_first(&[
        (
            "statac.yaml",
            "title: Site\nurl: https://example.org/\n\
             feed_key: 0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20\nfeed: true\ntimezone: UTC\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\n",
        ),
        (
            "templates/feed.xml",
            "<feed xmlns=\"http://www.w3.org/2005/Atom\"></feed>\n",
        ),
    ]),
    Run::new(
        Code::UnreadableImage,
        &["build", "{site}"],
        &[(
            "content/post-1.md",
            "![A gradient from blue to orange](photo-1.jpg)\n",
        )],
    )
    .with_images(&[("content/photo-1.jpg", 64, 48)])
    .cutting(&["content/photo-1.jpg"]),
    Run::new(
        Code::DescribesNoImage,
        CHECK,
        &[(
            "content/photo-1.jpg.yaml",
            "alt: A gradient from blue to orange\n",
        )],
    )
    .with_images(&[("content/photo-1.jpeg", 64, 48)]),
    Run::new(
        Code::MissingAltText,
        CHECK,
        &[("content/post-1.md", "![](photo-1.jpg)\n")],
    )
    .with_images(&[("content/photo-1.jpg", 64, 48)]),
    Run::new(
        Code::MissingLinkText,
        CHECK,
        &[
            (
                "templates/page.html",
                "<a href=\"https://example.org/\"><img src=\"/icon.svg\" alt=\"\"></a>\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
        ],
    ),
    Run::new(
        Code::MissingButtonText,
        CHECK,
        &[
            (
                "templates/page.html",
                "<button class=\"menu\"><img src=\"/menu.svg\" alt=\"\"></button>\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
        ],
    ),
    Run::new(
        Code::MissingIframeTitle,
        CHECK,
        &[(
            "content/post-1.md",
            "<iframe src=\"https://example.org/video/1\"></iframe>\n",
        )],
    ),
    Run::new(
        Code::MissingLabel,
        CHECK,
        &[
            (
                "templates/search.html",
                "<input type=\"search\" name=\"q\">\n",
            ),
            (
                "content/search.md",
                "---\ntitle: Search\nlayout: search\n---\n",
            ),
        ],
    ),
    Run::new(
        Code::MissingTitle,
        CHECK,
        &[
            (
                "templates/page.html",
                "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n</head>\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
        ],
    ),
    Run::new(
        Code::HeicImage,
        CHECK,
        &[
            (
                "content/post-1.md",
                "![A gradient from blue to orange](photo-1.heic)\n",
            ),
            ("content/photo-1.heic", "\0\0\0\x18ftypheic\0\0\0\0mif1heic"),
        ],
    ),
    Run::new(
        Code::ImagesOutOfProportion,
        CHECK,
        &[
            ("assets/site.css", "img {\n  max-width: 100%;\n}\n"),
            (
                "templates/page.html",
                "{{ css(\"site.css\") }}\n{{ page.content }}\n",
            ),
            (
                "content/post-1.md",
                "![A gradient from blue to orange](photo-1.jpg)\n",
            ),
        ],
    )
    .with_images(&[("content/photo-1.jpg", 64, 48)]),
    Run::new(
        Code::UnusedCard,
        CHECK,
        &[
            ("statac.yaml", "title: Site\nurl: https://example.org/\n"),
            ("templates/og.wide.yaml", WIDE_CARD),
            ("templates/page.html", CARD_LAYOUT),
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ],
    )
    .with_images(&[("templates/og/wide.png", 1200, 630)])
    .with_copies(FONT),
    Run::new(
        Code::ImageMayNotShow,
        CHECK,
        &[
            ("statac.yaml", "title: Site\nurl: https://example.org/\n"),
            (
                "content/post-1/index.md",
                "---\nog_image: cover.webp\n---\n",
            ),
            ("templates/page.html", CARD_LAYOUT),
        ],
    )
    .with_images(&[("content/post-1/cover.webp", 64, 48)]),
    Run::new(
        Code::NoThumbnail,
        CHECK,
        &[
            (
                "content/post-1.md",
                "@youtube(\"https://youtu.be/abcdefghijk\")\n",
            ),
            (
                "snippets/youtube.html",
                "{# takes address #}\n{% set video = page.youtube(address) %}\n\
                 {% if video.thumbnail %}<img src=\"{{ video.thumbnail.src }}\">{% endif %}\n",
            ),
        ],
    ),
    Run::new(
        Code::MalformedRecord,
        CHECK,
        &[(
            "fetched/record",
            "statac fetched 1\nstills/youtube-abcdefghijk.jpg\t0f3a9c1e5b7d2468\t6 October\n",
        )],
    ),
    Run::new(
        Code::TokenInSettings,
        CHECK,
        &[(
            "statac.yaml",
            "url: https://example.org/\nwebmention_token: abcdefghijklmnop\n",
        )],
    ),
    Run::new(
        Code::WebmentionsOff,
        CHECK,
        &[
            (
                "templates/page.html",
                "<head>\n  <title>{{ page.title }}</title>\n  {{ webmention() }}\n</head>\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
        ],
    ),
    Run::new(
        Code::MentionOfNoPage,
        CHECK,
        &[
            (
                "statac.yaml",
                "url: https://example.org/\nwebmentions: true\n",
            ),
            ("fetched/mentions/2028899.yaml", MENTION),
        ],
    ),
    Run::new(
        Code::RefusedWebmentions,
        CHECK,
        &[
            (
                "statac.yaml",
                "url: https://example.org/\nwebmentions: true\nrefuse_webmentions_from:\n  \
                 - https://a.example/post-1/\n  - https://b.example/\n",
            ),
            ("fetched/mentions/2028899.yaml", MENTION),
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ],
    ),
    Run::new(
        Code::RetiredFile,
        CHECK,
        &[
            (
                "statac.yaml",
                "title: Site\nurl: https://example.org/\nwebmentions: true\n",
            ),
            (
                "fetched/refused",
                "https://a.example/post-1/\nhttps://b.example/\n",
            ),
        ],
    ),
    Run::new(Code::MissingFolder, &["import"], &[]),
    Run::new(
        Code::CardNotDrawn,
        CHECK,
        &[
            ("statac.yaml", "title: Site\nurl: https://example.org/\n"),
            ("templates/og.yaml", CARD),
            ("templates/page.html", CARD_LAYOUT),
            ("content/post-1.md", LONG_TITLE),
        ],
    )
    .with_images(&[("templates/og/background.png", 1200, 630)])
    .with_copies(FONT),
    Run::new(
        Code::SizeKept,
        CHECK,
        &[
            ("statac.yaml", "title: Site\nurl: https://example.org/\n"),
            (
                "templates/og.yaml",
                "background: background.png\nfont: text.ttf\n\n\
                 footer.text: \"{{ site.title }}\"\nfooter.place: bottom\n\
                 footer.size: up to 36\nfooter.colour: \"#B9A88F\"\n",
            ),
            ("templates/page.html", CARD_LAYOUT),
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ],
    )
    .with_images(&[("templates/og/background.png", 1200, 630)])
    .with_copies(FONT),
    Run::new(
        Code::UnusableCardFile,
        CHECK,
        &[
            ("statac.yaml", "title: Site\nurl: https://example.org/\n"),
            ("templates/og.yaml", CARD),
            ("templates/page.html", CARD_LAYOUT),
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ],
    )
    .with_images(&[("templates/og/background.png", 1600, 900)])
    .with_copies(FONT),
];

/// A card with the page's title in the middle and the site's at the
/// bottom.
const CARD: &str = "background: background.png
font: text.ttf

title.text: \"{{ page.og_title }}\"
title.place: centre
title.size: up to 75
title.colour: \"#F2E9D3\"

footer.text: \"{{ site.title }}\"
footer.place: bottom
footer.size: 36
footer.colour: \"#B9A88F\"
";

/// A card with the page's title in the middle, named `wide`.
const WIDE_CARD: &str = "background: wide.png
font: text.ttf

title.text: \"{{ page.og_title }}\"
title.place: centre
title.size: up to 75
title.colour: \"#F2E9D3\"
";

/// The font a card is drawn with.
const FONT: &[(&str, &str)] = &[(
    "templates/og/text.ttf",
    "src/cards/fonts/test/test-latin.ttf",
)];

/// A layout writing a page's social image.
const CARD_LAYOUT: &str = "<!doctype html>
<title>{{ page.og_title }}</title>
{% if page.card is defined %}
<meta property=\"og:image\" content=\"{{ page.card.url | absolute }}\">
{% endif %}
{{ page.content }}
";

/// A post whose title no card has room for.
const LONG_TITLE: &str = concat!(
    "---\ntitle: ",
    "word word word word word word word word word word word word word word word word word word word word ",
    "word word word word word word word word word word word word word word word word word word word word ",
    "word word word word word word word word word word word word word word word word word word word word ",
    "word word word word word word word word word word word word word word word word word word word word ",
    "word word word word word word word word word word word word word word word word word word word word ",
    "word word word word word word word word word word word word word word word word word word word word\n",
    "---\n",
);

/// A webmention for a URL where the site publishes nothing.
const MENTION: &str = "kind: \"liked\"
author: \"A\"
author_url: \"https://a.example/\"
url: \"https://a.example/post-1/\"
target: \"https://example.org/post-1/\"
published: \"2026-09-04\"
fetched: \"2026-10-06\"
";

/// A line of 201 `>` marks, one more than Statac reads.
const DEEP: &str = concat!(
    ">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>",
    ">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>",
    "> The words.\n"
);

/// A kind no run in a test can meet, drawn at the first `at` in `text`,
/// written in the site's file `path`, or in no file where there is no
/// place.
struct Drawn {
    kind: Kind,
    place: Option<(&'static str, &'static str, &'static str)>,
}

/// The kinds drawn where their first fixture would not show what the
/// explanation's Example meets.
fn drawn() -> Vec<Drawn> {
    vec![
        // A line of 52,310 characters.
        Drawn {
            kind: Kind::CodeNotColoured {
                characters: 52310,
                block: true,
            },
            place: Some((
                "content/post-1.md",
                "```js\n!function(){var a=1,b=2;a+=b}();\n```\n",
                "js",
            )),
        },
        // Over 2,000 pages.
        Drawn {
            kind: Kind::LoopOverEveryPage {
                written: Some("site.pages".into()),
                pages: 2301,
                instead: Some("site.pages | selectattr(\"menu\")".into()),
            },
            place: Some((
                "templates/base.html",
                "{% for entry in site.pages if entry.menu %}\n\
                 <li><a href=\"{{ entry.url }}\">{{ entry.title }}</a></li>\n\
                 {% endfor %}\n",
                "site.pages",
            )),
        },
        // Port 80 is refused only where an administrator alone may open
        // it, and a port in use moves `serve` on to the next.
        Drawn {
            kind: Kind::CantListen {
                port: 80,
                last: 80,
                why: Unheard::System(Reason::Denied),
            },
            place: None,
        },
        // Each names a folder as the command line gave it, and a run in a
        // test names the folder by its whole path.
        Drawn {
            kind: Kind::FolderNotEmpty {
                by: Maker::New,
                folder: "notes".into(),
                names: vec!["a.txt".into()],
                more: 0,
            },
            place: None,
        },
        Drawn {
            kind: Kind::NoSiteHere {
                folder: "notes".into(),
            },
            place: None,
        },
        Drawn {
            kind: Kind::CantTellWhatMade {
                folder: "old".into(),
                several: false,
                tools: ["jekyll", "hugo", "zola", "eleventy"]
                    .map(Into::into)
                    .into(),
            },
            place: None,
        },
        // A run offers a new key each time.
        Drawn {
            kind: Kind::NoFeedKey {
                key: "0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20".into(),
            },
            place: None,
        },
    ]
}

/// The file a kind's message is drawn at, where its run prints more than
/// one of the kind.
const AT: &[(Code, &str)] = &[(Code::UnmatchedTranslationKey, "content/en/about-us.md")];

/// What `run` prints, on either stream.
fn printed(run: &Run) -> String {
    let site = Scratch::new("sample");
    site.write("statac.yaml", "");
    for (path, text) in run.first {
        site.write(path, text);
    }
    if !run.first.is_empty() {
        ran(&["build", "{site}"], &site);
    }
    for (path, text) in run.files {
        site.write(path, text);
    }
    for &(path, width, height) in run.images {
        let at = site.write(path, "");
        let image = image::RgbImage::from_pixel(width, height, image::Rgb([40, 90, 160]));
        image.save(&at).unwrap();
        if run.cut.contains(&path) {
            let bytes = std::fs::read(&at).unwrap();
            std::fs::write(&at, &bytes[..bytes.len() / 2]).unwrap();
        }
    }
    for (path, from) in run.copies {
        let from = Path::new(env!("CARGO_MANIFEST_DIR")).join(from);
        site.write(path, std::fs::read(from).unwrap());
    }
    let kept: Vec<_> = (run.read_only.iter())
        .map(|folder| {
            let path = site.join(folder);
            let kept = std::fs::metadata(&path).unwrap().permissions();
            let mut read_only = kept.clone();
            read_only.set_readonly(true);
            std::fs::set_permissions(&path, read_only).unwrap();
            (path, kept)
        })
        .collect();
    let printed = ran(run.args, &site);
    for (path, kept) in kept {
        std::fs::set_permissions(path, kept).unwrap();
    }
    match run.inside {
        true => printed.replace(&format!("{}/", site.to_string_lossy()), ""),
        false => printed,
    }
}

/// What a run of `args` prints, on either stream, with `{site}` standing
/// for `site`.
fn ran(args: &[&str], site: &Path) -> String {
    let folder = site.to_string_lossy();
    let args = args
        .iter()
        .map(|arg| OsString::from(arg.replace("{site}", &folder)));
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut terminal = Terminal {
        out: &mut out,
        err: &mut err,
        colour_out: false,
        colour_err: false,
        progress: Progress::Silent,
    };
    crate::cli::run(args, &mut terminal);
    out.extend(err);
    String::from_utf8(out).unwrap()
}

/// The messages of kind `code` in `printed`, each from its headline to
/// its explanation's link.
fn messages(printed: &str, code: Code) -> Vec<String> {
    let link = format!("  {}\n", more(&explanation_link(code.name())));
    let headlines = [Severity::Error, Severity::Warning, Severity::Note]
        .map(|level| format!("{}  ", severity(level)));
    let mut found = Vec::new();
    let mut message: Option<String> = None;
    for line in printed.split_inclusive('\n') {
        if headlines.iter().any(|headline| line.starts_with(headline)) {
            message = Some(String::new());
        }
        let Some(lines) = &mut message else {
            continue;
        };
        lines.push_str(line);
        if line.starts_with("  More: ") {
            if line == link {
                found.push(lines.clone());
            }
            message = None;
        }
    }
    found
}

/// The message shown for `code`, or what went wrong finding it.
fn message(code: Code) -> Result<String, String> {
    if let Some(run) = RUNS.iter().find(|run| run.code == code) {
        let printed = printed(run);
        let mut found = messages(&printed, code);
        if let Some(&(_, file)) = AT.iter().find(|&&(kind, _)| kind == code) {
            let at = format!("\n  {file}, line ");
            found.retain(|message| message.contains(&at));
        }
        return match found.as_slice() {
            [one] => Ok(one.clone()),
            found => Err(format!(
                "the run printed {} messages of the kind:\n{printed}",
                found.len()
            )),
        };
    }
    let drawn = drawn().into_iter().find(|drawn| drawn.kind.code() == code);
    let (kind, place) = match drawn {
        Some(drawn) => (drawn.kind, drawn.place),
        None => {
            let kind = kinds().into_iter().find(|kind| kind.code() == code);
            (kind.ok_or("there is no run or fixture of the kind")?, None)
        }
    };
    if kind.related().iter().any(|(_, place)| place.is_some()) {
        return Err("its fixture names places, which only a run can draw".to_owned());
    }
    let mut sources = Sources::default();
    let problem = match place {
        Some((path, text, at)) => {
            let source = sources.add(path, text.to_owned());
            let start = text.find(at).expect("the text holds the place");
            let span = Span::new(start, start + at.len());
            Problem::at(kind, Place { source, span })
        }
        None => Problem::nowhere(kind),
    };
    Ok(Report::new(&sources, Path::new(""), false).problem(&problem))
}

#[test]
fn each_kind_has_one_sample_at_most() {
    let drawn: Vec<Code> = drawn().iter().map(|drawn| drawn.kind.code()).collect();
    for (at, run) in RUNS.iter().enumerate() {
        assert!(
            !RUNS[..at].iter().any(|earlier| earlier.code == run.code),
            "{}",
            run.code.name()
        );
        assert!(!drawn.contains(&run.code), "{}", run.code.name());
    }
}

/// Each explanation shows its kind's message, straight after its opening.
#[test]
fn every_explanation_shows_what_statac_prints() {
    let mut wrong = Vec::new();
    for &code in Code::ALL {
        let unix = RUNS.iter().any(|run| run.code == code && run.unix);
        if unix && cfg!(windows) {
            continue;
        }
        let message = match message(code) {
            Ok(message) => message,
            Err(why) => {
                wrong.push(format!("{}: {why}", code.name()));
                continue;
            }
        };
        let section =
            format!("\n## What you'll see {{#what-youll-see}}\n\n```text\n{message}```\n\n## ");
        let text = code.explanation().replace("\r\n", "\n");
        let sections = text.find("\n## ").map_or("", |at| &text[at..]);
        if !sections.starts_with(&section) {
            wrong.push(format!(
                "explanations/{}.md should open its sections with{section}",
                code.name()
            ));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
