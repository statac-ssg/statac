use super::*;

/// What a run says on each stream, and its exit code.
fn ran(args: &[&str]) -> (String, String, u8) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut terminal = Terminal {
        out: &mut out,
        err: &mut err,
        colour_out: false,
        colour_err: false,
        progress: Progress::Silent,
    };
    let code = run(args.iter().map(OsString::from), &mut terminal);
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        code,
    )
}

/// The mistakes a command line is met with, drawn, and the exit code.
fn refused(args: &[&str]) -> String {
    let (out, err, code) = ran(args);
    assert_eq!(out, "");
    assert_eq!(code, 2, "{args:?}");
    err
}

/// The site as the command line leaves it where no folder is named.
const UNNAMED: &str = "";

fn parsed(args: &[&str]) -> Result<Request, Vec<Kind>> {
    let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    parse(&args).map_err(|problems| problems.into_iter().map(|p| p.kind).collect())
}

fn options(mode: Mode, site: &str, out: Option<&str>) -> Options {
    Options {
        mode,
        site: site.into(),
        out: out.map(PathBuf::from),
        today: None,
        drafts: false,
        url: None,
    }
}

#[test]
fn commands_are_read_with_their_options() {
    assert_eq!(
        parsed(&["build"]),
        Ok(Request::Build {
            options: options(Mode::Build, UNNAMED, None),
            timings: false,
        })
    );
    assert_eq!(
        parsed(&["build", "--site=docs", "--out=public"]),
        Ok(Request::Build {
            options: options(Mode::Build, "docs", Some("public")),
            timings: false,
        })
    );
    assert_eq!(
        parsed(&["--site=docs", "check"]),
        Ok(Request::Check {
            options: options(Mode::Check, "docs", None),
            json: false,
        })
    );
    assert_eq!(
        parsed(&["explain", "duplicate-key"]),
        Ok(Request::Explain(Code::DuplicateKey))
    );
    assert_eq!(parsed(&[]), Ok(Request::Help(None)));
    assert_eq!(
        parsed(&["build", "--sit=docs", "--help"]),
        Ok(Request::Help(Some("build")))
    );
    assert_eq!(parsed(&["--version"]), Ok(Request::Version));
}

#[test]
fn an_unknown_command_is_offered_the_nearest() {
    assert_eq!(
        refused(&["biuld"]),
        "\
error  Unknown command: `biuld`
  Did you mean `statac build`?
  More: https://statac.dev/e/unknown-command/
"
    );
    assert_eq!(
        refused(&["help"]),
        "\
error  Unknown command: `help`
  Did you mean `statac --help`?
  More: https://statac.dev/e/unknown-command/
"
    );
    assert_eq!(
        refused(&["post-1.md"]),
        "\
error  Unknown command: `post-1.md`
  Run `statac --help` to see the commands.
  More: https://statac.dev/e/unknown-command/
"
    );
}

#[test]
fn an_unknown_option_is_offered_the_nearest() {
    assert_eq!(
        refused(&["build", "--sit=docs"]),
        "\
error  Unknown option: `--sit=docs`
  Did you mean `--site=docs`?
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["check", "-h"]),
        "\
error  Unknown option: `-h`
  Did you mean `--help`?
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["build", "--publish"]),
        "\
error  Unknown option: `--publish`
  `statac build` takes `--site`, `--out`, `--url`, `--today`, `--drafts` and `--timings`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["build", "--draft"]),
        "\
error  Unknown option: `--draft`
  Did you mean `--drafts`?
  More: https://statac.dev/e/unknown-option/
"
    );
}

#[test]
fn drafts_are_asked_for_with_no_value() {
    let wanted = Options {
        drafts: true,
        ..options(Mode::Build, UNNAMED, None)
    };
    assert_eq!(
        parsed(&["build", "--drafts"]),
        Ok(Request::Build {
            options: wanted,
            timings: false,
        })
    );
    let checked = Options {
        drafts: true,
        ..options(Mode::Check, "docs", None)
    };
    assert_eq!(
        parsed(&["--drafts", "check", "docs"]),
        Ok(Request::Check {
            options: checked,
            json: false,
        })
    );
    assert_eq!(
        refused(&["build", "--drafts=yes"]),
        "\
error  Unknown option: `--drafts=yes`
  Did you mean `--drafts`?
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["build", "--drafts", "--drafts"]),
        "\
error  Option given twice: `--drafts`
  Keep one of them.
  More: https://statac.dev/e/option-given-twice/
"
    );
    assert_eq!(
        refused(&["explain", "draft-left-out", "--drafts"]),
        "\
error  Unknown option: `statac explain` has no `--drafts`
  `--drafts` is for `statac build`, `statac check` and `statac search`.
  More: https://statac.dev/e/unknown-option/
"
    );
}

#[test]
fn json_is_asked_of_check_alone_with_no_value() {
    assert_eq!(
        parsed(&["check", "--json", "docs"]),
        Ok(Request::Check {
            options: options(Mode::Check, "docs", None),
            json: true,
        })
    );
    assert_eq!(
        refused(&["build", "--json"]),
        "\
error  Unknown option: `statac build` has no `--json`
  `--json` is for `statac check`.
  More: https://statac.dev/e/unknown-option/
"
    );
}

#[test]
fn timings_are_asked_of_build_alone_with_no_value() {
    assert_eq!(
        parsed(&["build", "--timings", "docs"]),
        Ok(Request::Build {
            options: options(Mode::Build, "docs", None),
            timings: true,
        })
    );
    assert_eq!(
        refused(&["check", "--timings"]),
        "\
error  Unknown option: `statac check` has no `--timings`
  `--timings` is for `statac build`.
  More: https://statac.dev/e/unknown-option/
"
    );
}

#[test]
fn an_option_for_another_command_says_which() {
    assert_eq!(
        refused(&["check", "--out=public"]),
        "\
error  Unknown option: `statac check` has no `--out`
  `--out` is for `statac build`.
  More: https://statac.dev/e/unknown-option/
"
    );
}

#[test]
fn a_value_given_apart_is_offered_joined() {
    assert_eq!(
        refused(&["build", "--site", "docs"]),
        "\
error  Missing value: `--site` needs a folder
  Write it as `--site=docs`.
  More: https://statac.dev/e/missing-value/
"
    );
    assert_eq!(
        parsed(&["build", "--out="]),
        Err(vec![Kind::MissingValue {
            written: "--out=".into(),
            option: "--out".into(),
            value: None,
        }])
    );
    assert_eq!(
        parsed(&["--site", "docs", "build"]),
        Err(vec![Kind::MissingValue {
            written: "--site".into(),
            option: "--site".into(),
            value: Some("docs".into()),
        }])
    );
}

#[test]
fn the_folder_after_the_command_is_the_site() {
    assert_eq!(
        parsed(&["build", "docs"]),
        Ok(Request::Build {
            options: options(Mode::Build, "docs", None),
            timings: false,
        })
    );
    assert_eq!(
        parsed(&["check", "docs"]),
        Ok(Request::Check {
            options: options(Mode::Check, "docs", None),
            json: false,
        })
    );
    assert_eq!(
        parsed(&["build", "docs", "--out=public"]),
        Ok(Request::Build {
            options: options(Mode::Build, "docs", Some("public")),
            timings: false,
        })
    );
}

#[test]
fn the_site_is_given_once() {
    assert_eq!(
        refused(&["build", "docs", "--site=site"]),
        "\
error  Option given twice: `--site`
  Keep one of them.
  More: https://statac.dev/e/option-given-twice/
"
    );
    assert_eq!(
        parsed(&["build", "--site=site", "docs"]),
        Err(vec![Kind::OptionGivenTwice {
            option: "--site".into()
        }])
    );
    assert_eq!(
        parsed(&["build", "docs", "public"]),
        Err(vec![Kind::UnexpectedArgument {
            written: "public".into(),
            suggestion: None,
        }])
    );
}

#[test]
fn today_is_a_day_written_as_iso() {
    let today = |args: &[&str]| match parsed(args) {
        Ok(Request::Build { options, .. } | Request::Check { options, .. }) => options.today,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        today(&["build", "--today=2026-10-04"]),
        Some(crate::date::Date::day_only(2026, 10, 4))
    );
    assert_eq!(
        today(&["check", "--today=2026-02-28"]),
        Some(crate::date::Date::day_only(2026, 2, 28))
    );
    assert_eq!(
        refused(&["build", "--today=2026-02-30"]),
        "\
error  No such date: there is no 30 February in 2026
  More: https://statac.dev/e/no-such-date/
"
    );
    assert_eq!(
        refused(&["build", "--today=04/10/2026"]),
        "\
error  Unclear date: `04/10/2026` could be 4 October or 10 April
  Write it as `2026-10-04` or `2026-04-10`.
  More: https://statac.dev/e/unclear-date/
"
    );
    assert_eq!(
        parsed(&["build", "--today=2026-10-04T09:00"]),
        Err(vec![Kind::WrongKind {
            field: "--today".into(),
            expected: Expected::Date,
            found: Found::Written("2026-10-04T09:00".into()),
            respelt: Some("2026-10-04".into()),
        }])
    );
    assert_eq!(
        parsed(&["build", "--today"]),
        Err(vec![Kind::MissingValue {
            written: "--today".into(),
            option: "--today".into(),
            value: None,
        }])
    );
}

#[test]
fn the_address_for_a_run_is_a_whole_address() {
    let url = |args: &[&str]| match parsed(args) {
        Ok(Request::Build { options, .. } | Request::Check { options, .. }) => options.url,
        other => panic!("{other:?}"),
    };
    assert_eq!(url(&["build"]), None);
    for address in [
        "https://preview.example.org",
        "https://preview.example.org/",
        "https://example.org/preview/42/",
        "http://localhost:8080",
    ] {
        for command in ["build", "check"] {
            let option = format!("--url={address}");
            assert_eq!(url(&[command, &option]).as_deref(), Some(address));
        }
    }
    assert_eq!(
        refused(&["build", "--url=preview.example.org/42/"]),
        "\
error  Wrong kind of value: `--url` is a full URL, such as `https://example.org`, \
not `preview.example.org/42/`
  Write it as `https://preview.example.org/42/`.
  More: https://statac.dev/e/wrong-kind-of-value/
"
    );
    for written in [
        "ftp://example.org",
        "https://example",
        "https://",
        "/preview/",
    ] {
        assert_eq!(
            parsed(&["check", &format!("--url={written}")]),
            Err(vec![Kind::WrongKind {
                field: "--url".into(),
                expected: Expected::SiteAddress,
                found: Found::Written(written.into()),
                respelt: None,
            }]),
        );
    }
    assert_eq!(
        refused(&["build", "--url="]),
        "\
error  Missing value: `--url=` needs a full URL
  Write it as `--url=<url>`.
  More: https://statac.dev/e/missing-value/
"
    );
    assert_eq!(
        refused(&["build", "--url", "https://preview.example.org"]),
        "\
error  Missing value: `--url` needs a full URL
  Write it as `--url=https://preview.example.org`.
  More: https://statac.dev/e/missing-value/
"
    );
    assert_eq!(
        refused(&[
            "check",
            "--url=https://a.example",
            "--url=https://b.example"
        ]),
        "\
error  Option given twice: `--url`
  Keep one of them.
  More: https://statac.dev/e/option-given-twice/
"
    );
}

#[test]
fn only_build_and_check_take_an_address_for_the_run() {
    for command in ["serve", "stamp", "refresh"] {
        assert_eq!(
            refused(&[command, "--url=https://preview.example.org"]),
            format!(
                "\
error  Unknown option: `statac {command}` has no `--url`
  `--url` is for `statac build` and `statac check`.
  More: https://statac.dev/e/unknown-option/
"
            ),
        );
    }
}

#[test]
fn every_mistake_is_said_in_order() {
    assert_eq!(
        refused(&["--sit=docs", "--out=a", "--out=b"]),
        "\
error  Missing command: no `build`, `check`, `serve`, `stamp`, `refresh`, `new`, `import` or \
`explain`
  Run `statac --help` to see the commands.
  More: https://statac.dev/e/missing-command/

error  Unknown option: `--sit=docs`
  Did you mean `--site=docs`?
  More: https://statac.dev/e/unknown-option/

error  Option given twice: `--out`
  Keep one of them.
  More: https://statac.dev/e/option-given-twice/
"
    );
}

#[test]
fn explain_needs_a_known_name() {
    assert_eq!(
        refused(&["explain"]),
        "\
error  Missing name: `statac explain` needs the name of a problem
  The name is the last part of a problem's `More:` link.
  More: https://statac.dev/e/missing-name/
"
    );
    assert_eq!(
        refused(&["explain", "brokn-link"]),
        "\
error  Unknown problem name: `brokn-link`
  Did you mean `broken-link`?
  More: https://statac.dev/e/unknown-problem-name/
"
    );
    assert_eq!(
        parsed(&["explain", "--site=docs", "broken-link"]),
        Err(vec![Kind::UnknownOption {
            written: "--site=docs".into(),
            hint: OptionHint::ForOther {
                command: "explain".into(),
                option: "--site".into(),
                takers: Box::new(["build", "check", "serve", "stamp", "refresh", "search"]),
            },
        }])
    );
}

#[test]
fn explain_prints_the_explanation() {
    let (out, err, code) = ran(&["explain", "broken-link"]);
    assert_eq!(out, Code::BrokenLink.explained());
    assert!(out.contains(
        "\n## What you'll see\n\n```text\n\
         warning  Broken link: nothing is published at `/about-us/`\n  \
         Did you mean `/about/`?\n  \
         More: https://statac.dev/e/broken-link/\n```\n"
    ));
    assert_eq!((err.as_str(), code), ("", 0));
}

#[test]
fn help_and_version_answer_on_standard_output() {
    let (out, err, code) = ran(&["--help"]);
    assert!(out.starts_with("Statac builds a website"));
    assert_eq!((err.as_str(), code), ("", 0));
    let (out, _, code) = ran(&["--version"]);
    assert_eq!(out, format!("Statac {}\n", env!("CARGO_PKG_VERSION")));
    assert_eq!(code, 0);
}

const WHOLE_SCREEN: &str = "Statac builds a website from a folder of Markdown.

Usage:
  statac build [folder]  Read the site and write it out, ready to publish.
  statac check [folder]  Read the site and show every problem with it.
  statac serve [folder]  Build the site and serve it on this computer,
                         building it again each time you save.
  statac stamp [folder]  Write the date over every stamp, such as =today, in
                         the front matter of the site's pages.
  statac refresh [folder]
                         Fetch what the site keeps from other sites, such
                         as a video's thumbnail or a webmention, into
                         fetched/. Webmentions are asked for with the
                         token in STATAC_WEBMENTION_TOKEN, or else in
                         the site's .statac/token.
  statac new [folder]    Ask five questions and write a new site from the
                         answers into the folder, which must be empty or
                         not there yet.
  statac new --cards [folder]
                         Write the sharing card's settings, background
                         and fonts into the site, so a card is drawn
                         for each page.
  statac new --update [folder]
                         Put the newest version in place of each file
                         statac new wrote that was never changed, and
                         show what is new in the others.
  statac import <folder> [new folder]
                         Write a new site from a site another tool made,
                         into the new folder, which must be empty or not
                         there yet; without one, beside the old site, its
                         name ending -statac. The old site is only read.
  statac search <words> [folder]
                         Show the pages a reader of the site would find
                         for the words, best first, each language's
                         apart on a site in several.
  statac explain <name>  Explain a kind of problem. Its name is the last part
                         of the problem's More: link.
  statac lsp             Tell an editor the site's problems as you type. The
                         editor starts it.

Options:
  --site=<folder>        The site's folder, as the folder after the command
                         gives it. Without either, the site the current
                         folder is in.
  --out=<folder>         Where statac build writes the built site. Without
                         it, _site in the site's folder.
  --url=<url>            The site's full URL for this run, in place of url
                         in statac.yaml, as a preview of the site needs.
  --today=<date>         The day the site is built on, as 2026-10-04, for
                         site.today, the day stamps are counted from, and
                         the day statac refresh records. Without it, today
                         in the site's zone.
  --drafts               Build drafts and scheduled posts as if they were
                         published.
  --json                 Show every problem as JSON, for an editor or another
                         program to read, and nothing else.
  --timings              Show where statac build spent its time, once it
                         has finished.
  --no-drafts            Leave drafts and scheduled posts out of what
                         statac serve shows, as statac build does.
  --offline              Fetch nothing as statac serve starts. Without it,
                         statac serve fetches what is new once, as statac
                         refresh does, after the first build with no
                         errors.
  --yes                  Take every answer statac new offers without asking.
  --missing              For statac new --update, write the files statac
                         new writes that the site doesn't have.
  --from=<tool>          The tool that made the site statac import reads:
                         jekyll, hugo, zola or eleventy. Without it, told
                         from the site's files.
  --no-starter           For statac new and statac import, leave out the
                         starter's templates, snippets, stylesheet and
                         pages, for a site you design yourself.
  --port=<number>        The port statac serve answers on. Without it,
                         3000, or the next port that is free.
  --watch=<way>          How statac serve learns of a save: native, by
                         the system's own reports; poll, by looking at
                         every file twice a second; or auto, which
                         polls only where reports can't be relied on.
                         Without it, auto.
  --language=<code>      Search only the pages in this language, named
                         by its code in statac.yaml.
  --help                 Show this.
  --version              Show which version of Statac this is.
";

#[test]
fn help_alone_or_before_a_command_shows_the_whole_screen() {
    for args in [
        &[][..],
        &["--help"],
        &["--help", "build"],
        &["--site=docs", "--help", "check"],
        &["biuld", "--help"],
        &["post-1.md", "--help"],
    ] {
        let (out, err, code) = ran(args);
        assert_eq!(out, WHOLE_SCREEN, "{args:?}");
        assert_eq!((err.as_str(), code), ("", 0), "{args:?}");
    }
}

#[test]
fn help_after_a_command_shows_its_entry_and_the_options_it_takes() {
    let (out, err, code) = ran(&["build", "--help"]);
    assert_eq!(
        out,
        "Statac builds a website from a folder of Markdown.

Usage:
  statac build [folder]  Read the site and write it out, ready to publish.

Options:
  --site=<folder>        The site's folder, as the folder after the command
                         gives it. Without either, the site the current
                         folder is in.
  --out=<folder>         Where statac build writes the built site. Without
                         it, _site in the site's folder.
  --url=<url>            The site's full URL for this run, in place of url
                         in statac.yaml, as a preview of the site needs.
  --today=<date>         The day the site is built on, as 2026-10-04, for
                         site.today, the day stamps are counted from, and
                         the day statac refresh records. Without it, today
                         in the site's zone.
  --drafts               Build drafts and scheduled posts as if they were
                         published.
  --timings              Show where statac build spent its time, once it
                         has finished.
  --help                 Show this.
  --version              Show which version of Statac this is.
"
    );
    assert_eq!((err.as_str(), code), ("", 0));
    let (out, _, _) = ran(&["explain", "--help"]);
    assert!(out.ends_with(
        "\
Usage:
  statac explain <name>  Explain a kind of problem. Its name is the last part
                         of the problem's More: link.

Options:
  --help                 Show this.
  --version              Show which version of Statac this is.
"
    ));
}

#[test]
fn each_commands_screen_is_the_whole_screens_lines_for_it() {
    for command in COMMANDS {
        let (screen, _, _) = ran(&[command, "--help"]);
        assert!(screen.starts_with(INTRO), "{command}");
        let mut lines = WHOLE_SCREEN.lines();
        for line in screen.lines() {
            assert!(lines.any(|whole| whole == line), "{command}: {line:?}");
        }
        for (of, entry) in USAGE {
            assert_eq!(screen.contains(entry), of == command, "{command}: {entry}");
        }
        for (option, entry) in OPTION_LINES {
            let takes = QUESTIONS.contains(&option)
                || OPTIONS
                    .iter()
                    .chain(&FLAGS)
                    .any(|(taken, takers)| *taken == option && takers.contains(&command));
            assert_eq!(screen.contains(entry), takes, "{command}: {option}");
        }
    }
    let (new, _, _) = ran(&["new", "--help"]);
    assert!(new.contains("  statac new --update [folder]\n"), "{new}");
    assert!(new.contains("  statac new --cards [folder]\n"), "{new}");
    assert!(new.contains("  --missing  "), "{new}");
    assert!(!new.contains("  --site="), "{new}");
}

#[test]
fn help_after_a_command_answers_before_any_other_mistake() {
    let (screen, _, _) = ran(&["serve", "--help"]);
    for args in [
        &["serve", "--port=http", "--help"][..],
        &["--sit=docs", "serve", "post-1.md", "post-2.md", "--help"],
        &["serve", "--help", "--out=public"],
    ] {
        let (out, err, code) = ran(args);
        assert_eq!(out, screen, "{args:?}");
        assert_eq!((err.as_str(), code), ("", 0), "{args:?}");
    }
}

#[test]
fn a_site_that_isnt_there_is_said_to_be_missing() {
    let missing = "\
error  Can't read `site-that-isnt-there`: it doesn't exist
  More: https://statac.dev/e/cant-read/
";
    let (out, err, code) = ran(&["check", "site-that-isnt-there"]);
    assert_eq!(
        (out.as_str(), err.as_str(), code),
        (
            format!("{missing}\n1 error: 1 cant-read.\n").as_str(),
            "",
            1
        )
    );
    let (out, _, code) = ran(&["build", "site-that-isnt-there"]);
    assert_eq!(
        (out.as_str(), code),
        (
            format!("{missing}\nWrote nothing: 1 error stopped the build.\n").as_str(),
            1
        )
    );
    // Serve builds first, and with nothing to watch it ends there.
    let (out, err, code) = ran(&["serve", "site-that-isnt-there", "--offline"]);
    assert_eq!(
        (out.as_str(), err.as_str(), code),
        (
            format!("{missing}\nWrote nothing: 1 error stopped the build.\n").as_str(),
            "",
            1
        )
    );
}

fn serving(site: &str) -> serve::Options {
    serve::Options {
        site: site.into(),
        port: 3000,
        watch: How::Auto,
        settle: watch::Settle::default(),
        today: None,
        drafts: true,
        fetcher: None,
        token: None,
    }
}

fn online(options: serve::Options) -> Request {
    Request::Serve {
        options,
        offline: false,
    }
}

#[test]
fn serve_is_read_with_its_options() {
    assert_eq!(parsed(&["serve"]), Ok(online(serving(UNNAMED))));
    assert_eq!(parsed(&["serve", "docs"]), Ok(online(serving("docs"))));
    assert_eq!(
        parsed(&["--site=docs", "serve", "--port=8080", "--watch=native"]),
        Ok(online(serve::Options {
            port: 8080,
            watch: How::Native,
            ..serving("docs")
        }))
    );
    assert_eq!(
        parsed(&["serve", "--watch=poll", "--today=2026-10-04"]),
        Ok(online(serve::Options {
            watch: How::Poll {
                every: Duration::from_millis(500)
            },
            today: Some(crate::date::Date::day_only(2026, 10, 4)),
            ..serving(UNNAMED)
        }))
    );
    assert_eq!(
        parsed(&["serve", "--watch=auto"]),
        Ok(online(serving(UNNAMED)))
    );
    assert_eq!(
        parsed(&["serve", "--no-drafts"]),
        Ok(online(serve::Options {
            drafts: false,
            ..serving(UNNAMED)
        }))
    );
    assert_eq!(
        parsed(&["serve", "--offline"]),
        Ok(Request::Serve {
            options: serving(UNNAMED),
            offline: true
        })
    );
}

#[test]
fn only_serve_can_be_asked_to_fetch_nothing() {
    assert_eq!(
        refused(&["refresh", "--offline"]),
        "\
error  Unknown option: `statac refresh` has no `--offline`
  `--offline` is for `statac serve`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["build", "--offline"]),
        "\
error  Unknown option: `statac build` has no `--offline`
  `--offline` is for `statac serve`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["serve", "--offline=yes"]),
        "\
error  Unknown option: `--offline=yes`
  Did you mean `--offline`?
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        parsed(&["serve", "--offline", "--offline"]),
        Err(vec![Kind::OptionGivenTwice {
            option: "--offline".into()
        }])
    );
}

#[test]
fn serve_shows_drafts_unasked_and_only_it_can_leave_them_out() {
    assert_eq!(
        refused(&["serve", "--drafts"]),
        "\
error  Unknown option: `statac serve` has no `--drafts`
  `statac serve` shows drafts and scheduled posts already. `--no-drafts` leaves them \
out.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["build", "--no-drafts"]),
        "\
error  Unknown option: `statac build` has no `--no-drafts`
  `--no-drafts` is for `statac serve`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        parsed(&["serve", "--no-drafts", "--no-drafts"]),
        Err(vec![Kind::OptionGivenTwice {
            option: "--no-drafts".into()
        }])
    );
}

#[test]
fn a_port_is_a_whole_number_a_machine_has() {
    for written in ["0", "65536", "-1", "http", "30 00"] {
        assert_eq!(
            parsed(&["serve", &format!("--port={written}")]),
            Err(vec![Kind::WrongKind {
                field: "--port".into(),
                expected: Expected::Between { from: 1, to: 65535 },
                found: Found::Written(written.into()),
                respelt: None,
            }]),
            "{written}"
        );
    }
    assert_eq!(
        refused(&["serve", "--port=http"]),
        "\
error  Wrong kind of value: `--port` is a whole number from 1 to 65535, not `http`
  More: https://statac.dev/e/wrong-kind-of-value/
"
    );
    assert_eq!(
        refused(&["serve", "--port"]),
        "\
error  Missing value: `--port` needs a number
  Write it as `--port=<number>`.
  More: https://statac.dev/e/missing-value/
"
    );
    assert_eq!(
        refused(&["serve", "--port=3000", "--port=3001"]),
        "\
error  Option given twice: `--port`
  Keep one of them.
  More: https://statac.dev/e/option-given-twice/
"
    );
}

#[test]
fn new_takes_update_for_and_missing_alone_and_together() {
    let new = |folder: Option<&str>, update| {
        Ok(Request::New(new::Options {
            folder: folder.map(PathBuf::from),
            yes: false,
            update,
            cards: false,
            no_starter: false,
        }))
    };
    let update = |missing| Some(new::update::Update { missing });
    assert_eq!(parsed(&["new", "--update"]), new(None, update(false)));
    assert_eq!(
        parsed(&["new", "site-1", "--update"]),
        new(Some("site-1"), update(false))
    );
    assert_eq!(
        parsed(&["new", "--update", "--missing"]),
        new(None, update(true))
    );
    assert_eq!(
        parsed(&["--update", "new", "--missing", "site-1"]),
        new(Some("site-1"), update(true))
    );
    assert_eq!(parsed(&["new", "site-1"]), new(Some("site-1"), None));
}

#[test]
fn missing_without_update_and_yes_with_it_are_refused() {
    assert_eq!(
        refused(&["new", "--missing", "--yes"]),
        "\
error  Unknown option: `statac new` takes `--missing` only with `--update`
  Add `--update`, or leave `--missing` out.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["new", "--update", "--yes"]),
        "\
error  Unknown option: `statac new --update` has no `--yes`
  `--update` asks nothing, so leave `--yes` out.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        parsed(&["new", "--update", "--update"]),
        Err(vec![Kind::OptionGivenTwice {
            option: "--update".into()
        }])
    );
    for (option, takers) in [
        ("--update", ["new"]),
        ("--missing", ["new"]),
        ("--cards", ["new"]),
    ] {
        assert_eq!(
            parsed(&["build", option]),
            Err(vec![Kind::UnknownOption {
                written: option.into(),
                hint: OptionHint::ForOther {
                    command: "build".into(),
                    option: option.into(),
                    takers: Box::new(takers),
                },
            }])
        );
    }
}

#[test]
fn new_takes_cards_with_a_folder_or_none_and_never_with_update() {
    let new = |folder: Option<&str>, yes| {
        Ok(Request::New(new::Options {
            folder: folder.map(PathBuf::from),
            yes,
            update: None,
            cards: true,
            no_starter: false,
        }))
    };
    assert_eq!(parsed(&["new", "--cards"]), new(None, false));
    assert_eq!(
        parsed(&["--cards", "new", "site-1", "--yes"]),
        new(Some("site-1"), true)
    );
    assert_eq!(
        refused(&["new", "--cards", "--update"]),
        "\
error  Unknown option: `statac new --update` has no `--cards`
  Run `statac new --cards` by itself.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["new", "--cards", "--missing"]),
        "\
error  Unknown option: `statac new` takes `--missing` only with `--update`
  Add `--update`, or leave `--missing` out.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        parsed(&["new", "--cards", "--cards"]),
        Err(vec![Kind::OptionGivenTwice {
            option: "--cards".into()
        }])
    );
}

#[test]
fn new_and_import_take_no_starter_and_no_other_command_does() {
    let import = |starter| {
        Ok(Request::Import(import::Options {
            old: PathBuf::from("old-1"),
            new: Some(PathBuf::from("new-1")),
            from: None,
            today: None,
            starter,
        }))
    };
    assert_eq!(parsed(&["import", "old-1", "new-1"]), import(true));
    assert_eq!(
        parsed(&["import", "--no-starter", "old-1", "new-1"]),
        import(false)
    );
    assert_eq!(
        parsed(&["--no-starter", "import", "old-1", "new-1"]),
        import(false)
    );
    assert_eq!(
        parsed(&["import", "old-1", "--no-starter", "--no-starter"]),
        Err(vec![Kind::OptionGivenTwice {
            option: "--no-starter".into()
        }])
    );
    assert_eq!(
        parsed(&["new", "--no-starter", "site-1"]),
        Ok(Request::New(new::Options {
            folder: Some(PathBuf::from("site-1")),
            yes: false,
            update: None,
            cards: false,
            no_starter: true,
        }))
    );
    assert_eq!(
        refused(&["build", "--no-starter"]),
        "\
error  Unknown option: `statac build` has no `--no-starter`
  `--no-starter` is for `statac new` and `statac import`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["new", "--update", "--no-starter"]),
        "\
error  Unknown option: `--no-starter`
  `--update` works on the site that is here, so leave `--no-starter` out.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["import", "old-1", "--no-starter=true"]),
        "\
error  Unknown option: `--no-starter=true`
  Did you mean `--no-starter`?
  More: https://statac.dev/e/unknown-option/
"
    );
}

#[test]
fn a_way_of_watching_is_one_of_three() {
    assert_eq!(
        refused(&["serve", "--watch=fast"]),
        "\
error  Wrong kind of value: `--watch` is `auto`, `native` or `poll`, not `fast`
  More: https://statac.dev/e/wrong-kind-of-value/
"
    );
    assert_eq!(
        refused(&["serve", "--watch", "poll"]),
        "\
error  Missing value: `--watch` needs a way
  Write it as `--watch=poll`.
  More: https://statac.dev/e/missing-value/
"
    );
    assert_eq!(
        parsed(&["serve", "--watch=poll", "--watch=native"]),
        Err(vec![Kind::OptionGivenTwice {
            option: "--watch".into()
        }])
    );
}

#[test]
fn stamp_is_read_with_its_options() {
    let stamping = |site: &str, today: Option<&str>| {
        Ok(Request::Stamp(stamp::Options {
            site: site.into(),
            today: today.map(|day| date::read(day).unwrap()),
        }))
    };
    assert_eq!(parsed(&["stamp"]), stamping(UNNAMED, None));
    assert_eq!(parsed(&["stamp", "docs"]), stamping("docs", None));
    assert_eq!(
        parsed(&["--site=docs", "stamp", "--today=2026-10-04"]),
        stamping("docs", Some("2026-10-04"))
    );
    assert_eq!(
        refused(&["stamp", "--drafts"]),
        "\
error  Unknown option: `statac stamp` has no `--drafts`
  `--drafts` is for `statac build`, `statac check` and `statac search`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["stamp", "--out=public"]),
        "\
error  Unknown option: `statac stamp` has no `--out`
  `--out` is for `statac build`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["stamp", "docs", "content/post-1.md"]),
        "\
error  Unexpected argument: `content/post-1.md`
  More: https://statac.dev/e/unexpected-argument/
"
    );
    assert_eq!(
        refused(&["stmap"]),
        "\
error  Unknown command: `stmap`
  Did you mean `statac stamp`?
  More: https://statac.dev/e/unknown-command/
"
    );
}

#[test]
fn refresh_is_read_with_its_options() {
    let refreshing = |site: &str, today: Option<&str>| {
        Ok(Request::Refresh(refresh::Options {
            site: site.into(),
            today: today.map(|day| date::read(day).unwrap()),
            token: None,
        }))
    };
    assert_eq!(parsed(&["refresh"]), refreshing(UNNAMED, None));
    assert_eq!(parsed(&["refresh", "docs"]), refreshing("docs", None));
    assert_eq!(
        parsed(&["--site=docs", "refresh", "--today=2026-10-06"]),
        refreshing("docs", Some("2026-10-06"))
    );
    assert_eq!(
        refused(&["refresh", "--drafts"]),
        "\
error  Unknown option: `statac refresh` has no `--drafts`
  `--drafts` is for `statac build`, `statac check` and `statac search`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["refresh", "--port=8080"]),
        "\
error  Unknown option: `statac refresh` has no `--port`
  `--port` is for `statac serve`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["refresh", "docs", "fetched"]),
        "\
error  Unexpected argument: `fetched`
  More: https://statac.dev/e/unexpected-argument/
"
    );
    assert_eq!(
        refused(&["refrsh"]),
        "\
error  Unknown command: `refrsh`
  Did you mean `statac refresh`?
  More: https://statac.dev/e/unknown-command/
"
    );
    assert_eq!(
        refused(&["refresh", "--fetch"]),
        "\
error  Unknown option: `--fetch`
  `statac refresh` takes `--site` and `--today`.
  More: https://statac.dev/e/unknown-option/
"
    );
}

#[test]
fn a_site_to_refresh_that_isnt_there_is_said_to_be_missing() {
    let (out, err, code) = ran(&["refresh", "site-that-isnt-there"]);
    assert_eq!(
        (out.as_str(), err.as_str(), code),
        (
            "\
error  Can't read `site-that-isnt-there`: it doesn't exist
  More: https://statac.dev/e/cant-read/

1 error: 1 cant-read.
",
            "",
            1
        )
    );
}

#[test]
fn refuse_is_refused_with_the_setting_that_does_its_work() {
    assert_eq!(
        refused(&["refuse"]),
        "\
error  Unknown command: `refuse`
  Statac does this with `refuse_webmentions_from:` in `statac.yaml`.
  More: https://statac.dev/e/unknown-command/
"
    );
    // A command spelt nearly so is still offered for a near spelling.
    assert_eq!(
        parsed(&["refesh"]),
        Err(vec![Kind::UnknownCommand {
            written: "refesh".into(),
            hint: CommandHint::Similar("refresh".into()),
        }])
    );
}

#[test]
fn a_site_to_search_that_isnt_there_is_said_to_be_missing() {
    for args in [
        &["search", "post", "site-that-isnt-there"][..],
        &["search", "post", "--site=site-that-isnt-there"],
    ] {
        let (out, err, code) = ran(args);
        assert_eq!(
            (out.as_str(), err.as_str(), code),
            (
                "\
error  Can't read `site-that-isnt-there`: it doesn't exist
  More: https://statac.dev/e/cant-read/
",
                "",
                1
            ),
            "{args:?}"
        );
    }
}

#[test]
fn serve_takes_only_its_own_options() {
    assert_eq!(
        refused(&["serve", "--out=public"]),
        "\
error  Unknown option: `statac serve` has no `--out`
  `--out` is for `statac build`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["build", "--port=8080"]),
        "\
error  Unknown option: `statac build` has no `--port`
  `--port` is for `statac serve`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["serve", "--prot=8080"]),
        "\
error  Unknown option: `--prot=8080`
  Did you mean `--port=8080`?
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["serve", "--open"]),
        "\
error  Unknown option: `--open`
  `statac serve` takes `--site`, `--today`, `--port`, `--watch`, `--no-drafts` and `--offline`.
  More: https://statac.dev/e/unknown-option/
"
    );
    assert_eq!(
        refused(&["sreve"]),
        "\
error  Unknown command: `sreve`
  Did you mean `statac serve`?
  More: https://statac.dev/e/unknown-command/
"
    );
}

/// What a run says on each stream, and its exit code, given arguments the
/// system can hold that aren't text.
#[cfg(unix)]
fn ran_with(args: &[OsString]) -> (String, String, u8) {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let mut terminal = Terminal {
        out: &mut out,
        err: &mut err,
        colour_out: false,
        colour_err: false,
        progress: Progress::Silent,
    };
    let code = run(args.iter().cloned(), &mut terminal);
    (
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
        code,
    )
}

#[cfg(unix)]
#[test]
fn a_name_that_isnt_utf8_is_refused_before_anything_is_made() {
    use std::os::unix::ffi::OsStringExt;
    let name = OsString::from_vec(b"folder-1-\xff".to_vec());
    let said = "\
error  Unreadable name: `folder-1-\u{FFFD}` holds bytes that aren't UTF-8
  More: https://statac.dev/e/unreadable-name/
";
    let option = |option: &str| {
        let mut written = OsString::from(option);
        written.push(&name);
        written
    };
    let commands: [&[OsString]; 9] = [
        &["build".into(), name.clone()],
        &["check".into(), option("--site=")],
        &["serve".into(), name.clone()],
        &["stamp".into(), name.clone()],
        &["refresh".into(), name.clone()],
        &["new".into(), name.clone()],
        &["import".into(), name.clone()],
        &["import".into(), "old-1".into(), name.clone()],
        &["build".into(), option("--out=")],
    ];
    for args in commands {
        assert_eq!(
            ran_with(args),
            (String::new(), said.to_owned(), 2),
            "{args:?}"
        );
    }
    assert!(!Path::new(&name).exists());
}

#[cfg(unix)]
#[test]
fn a_command_that_isnt_utf8_is_not_a_name() {
    use std::os::unix::ffi::OsStringExt;
    let command = OsString::from_vec(b"buil\xff".to_vec());
    let (_, err, code) = ran_with(&[command]);
    assert_eq!(code, 2);
    assert!(err.starts_with("error  Unknown command: `buil\u{FFFD}`"));
}

#[test]
fn a_name_with_a_control_character_is_refused_before_any_folder_is_made() {
    let said = "\
error  Unusable name: `folder-1\u{241b}` holds a character that can't be shown
  Name the folder without it.
  More: https://statac.dev/e/unusable-name/
";
    let name = "folder-1\u{1b}";
    let commands: [&[&str]; 9] = [
        &["build", name],
        &["check", "--site=folder-1\u{1b}"],
        &["serve", name],
        &["stamp", name],
        &["refresh", name],
        &["new", name],
        &["import", name],
        &["import", "old-1", name],
        &["build", "--out=folder-1\u{1b}"],
    ];
    for args in commands {
        assert_eq!(refused(args), said, "{args:?}");
    }
    assert!(!Path::new(name).exists());
}

#[test]
fn a_mistake_in_a_name_comes_with_the_rest_in_order() {
    let said = refused(&[
        "build",
        "folder-1\u{1b}",
        "--out=folder-3\u{7}",
        "--sit=folder-2",
    ]);
    assert_eq!(said.matches("error  ").count(), 3, "{said}");
    assert!(said.contains("`folder-1\u{241b}`"));
    assert!(said.contains("Unknown option: `--sit=folder-2`"));
    assert!(said.contains("`folder-3\u{2407}`"));
}

#[test]
fn a_fault_of_statacs_own_is_told_after_rusts_report() {
    let ran = |command, body: fn(&mut Terminal) -> u8| {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let mut terminal = Terminal {
            out: &mut out,
            err: &mut err,
            colour_out: false,
            colour_err: false,
            progress: Progress::Silent,
        };
        let code = guarded(command, &mut terminal, body);
        (
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
            code,
        )
    };
    let fault = |_: &mut Terminal| -> u8 { panic!("a fault") };
    let report =
        "Report it at https://github.com/statac-ssg/statac/issues, with the lines above.\n";
    assert_eq!(
        ran(Some("build"), fault),
        (
            String::new(),
            format!("Build stopped: Statac met a fault of its own.\nWrote nothing.\n{report}"),
            101
        )
    );
    let fault_after = |_: &mut Terminal| -> u8 {
        crate::publish::mark_in_place();
        panic!("a fault")
    };
    assert_eq!(
        ran(Some("build"), fault_after),
        (
            String::new(),
            format!("Build stopped: Statac met a fault of its own.\n{report}"),
            101
        )
    );
    assert_eq!(
        ran(Some("check"), fault),
        (
            String::new(),
            format!("Check stopped: Statac met a fault of its own.\n{report}"),
            101
        )
    );
    for other in [Some("serve"), Some("new"), None] {
        assert_eq!(
            ran(other, fault),
            (String::new(), report.to_owned(), 101),
            "{other:?}"
        );
    }
    assert_eq!(ran(Some("build"), |_| 1), (String::new(), String::new(), 1));
}

#[test]
fn search_is_read_with_its_words_and_options() {
    let searching = |words: &str, site: &str, drafts: bool, today: Option<&str>| {
        Ok(Request::Search(search::Options {
            site: site.into(),
            words: words.into(),
            today: today.map(|day| date::read(day).unwrap()),
            drafts,
            language: None,
        }))
    };
    let in_language = |language: &str| {
        Ok(Request::Search(search::Options {
            site: UNNAMED.into(),
            words: "zfs".into(),
            today: None,
            drafts: false,
            language: Some(language.into()),
        }))
    };
    assert_eq!(
        parsed(&["search", "raspberry pi"]),
        searching("raspberry pi", UNNAMED, false, None)
    );
    assert_eq!(
        parsed(&["search", "zfs", "docs", "--drafts"]),
        searching("zfs", "docs", true, None)
    );
    assert_eq!(
        parsed(&["--site=docs", "search", "zfs", "--today=2026-10-06"]),
        searching("zfs", "docs", false, Some("2026-10-06"))
    );
    // The code is read against the site's once the site is read.
    for (args, language) in [
        (&["search", "zfs", "--language=fr"][..], "fr"),
        (&["search", "--language=pt-BR", "zfs"], "pt-BR"),
        (&["search", "zfs", "--language=xx"], "xx"),
    ] {
        assert_eq!(parsed(args), in_language(language), "{args:?}");
    }
    assert_eq!(
        refused(&["search", "zfs", "--language=en", "--language=fr"]),
        "\
error  Option given twice: `--language`
  Keep one of them.
  More: https://statac.dev/e/option-given-twice/
"
    );
    assert_eq!(
        refused(&["build", "--language=fr"]),
        "\
error  Unknown option: `statac build` has no `--language`
  `--language` is for `statac search`.
  More: https://statac.dev/e/unknown-option/
"
    );
    for words in [
        &["search"][..],
        &["search", " "],
        &["search", "--site=docs"],
    ] {
        assert_eq!(
            refused(words),
            "\
error  Nothing to search for: `statac search` needs the words to look for
  Give them after the command, as `statac search \"raspberry pi\"`.
  More: https://statac.dev/e/nothing-to-search/
"
        );
    }
    assert_eq!(
        refused(&["search", "zfs", "docs", "fetched"]),
        "\
error  Unexpected argument: `fetched`
  More: https://statac.dev/e/unexpected-argument/
"
    );
}
