//! `statac import` reads each made site in `tests/import/*/old` and
//! writes the new site kept beside it in `new/`, file for file and byte
//! for byte. The old site is the same after as before, and the new one
//! passes `statac check` with no error but the templates it lacks.

mod scratch;

use scratch::Scratch;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use statac::build::{self, Mode, Options};
use statac::date::Date;
use statac::import::{self, Tool};
use statac::problem::{Kind, Problem, Severity};

const KEY: &str = "0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20";

fn today() -> Date {
    Date::day_only(2026, 10, 6)
}

/// A folder of its own for each test, named `label`, with nothing there.
fn scratch(label: &str) -> Scratch {
    Scratch::new("import", label)
}

/// Every file under `folder`, by its path in it, with its bytes.
fn files(folder: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut waiting = vec![folder.to_path_buf()];
    while let Some(at) = waiting.pop() {
        for entry in fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                waiting.push(path);
            } else {
                let rel = path.strip_prefix(folder).unwrap();
                let rel = rel.to_string_lossy().replace('\\', "/");
                found.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    found
}

/// Imports `old` into `new` with the day fixed, giving the exit code and
/// what was said.
fn import(old: &Path, new: &Path, from: Option<Tool>) -> (u8, String) {
    let mut said = Vec::new();
    let options = import::Options {
        old: old.to_path_buf(),
        new: Some(new.to_path_buf()),
        from,
        today: Some(today()),
    };
    let code = import::run(&options, None, &mut said, false);
    (code, String::from_utf8(said).unwrap())
}

/// Imports `old` into `new` as [`import`] does, with `typed` there to
/// read, by a person at a terminal where `terminal` says.
fn import_answering(old: &Path, new: &Path, typed: &str, terminal: bool) -> (u8, String) {
    let mut said = Vec::new();
    let options = import::Options {
        old: old.to_path_buf(),
        new: Some(new.to_path_buf()),
        from: None,
        today: Some(today()),
    };
    let mut reader = typed.as_bytes();
    let input = statac::new::Input {
        reader: &mut reader,
        terminal,
    };
    let code = import::run(&options, Some(input), &mut said, false);
    (code, String::from_utf8(said).unwrap())
}

/// The name of each warning and note `statac check` finds in `site`, in
/// order, having found no error but a missing template, which an imported
/// site has none of.
fn checked(site: &Path) -> Vec<String> {
    let outcome = build::run(&Options {
        mode: Mode::Check,
        site: site.to_path_buf(),
        out: None,
        today: Some(today()),
        drafts: false,
        url: None,
    });
    let report = statac::report::Report::new(outcome.sources(), site, false);
    let template = |problem: &&Problem| matches!(problem.kind, Kind::MissingTemplate { .. });
    let errors: Vec<_> = (outcome.problems.iter())
        .filter(|problem| problem.kind.severity() == Severity::Error)
        .filter(|problem| !template(problem))
        .cloned()
        .collect();
    assert!(errors.is_empty(), "{}", report.problems(&errors));
    let mut names: Vec<String> = (outcome.problems.iter())
        .filter(|problem| !template(problem))
        .map(|problem| problem.kind.name().to_owned())
        .collect();
    names.sort();
    names.dedup();
    names
}

/// `made`, the files of a new site, with the `feed_key` the kept ones have.
fn with_key(mut made: BTreeMap<String, Vec<u8>>) -> BTreeMap<String, Vec<u8>> {
    let settings = made.get_mut("statac.yaml").unwrap();
    let text = String::from_utf8(settings.clone()).unwrap();
    let fixed: Vec<String> = text
        .lines()
        .map(|line| match line.starts_with("feed_key: ") {
            true => format!("feed_key: {KEY}"),
            false => line.to_owned(),
        })
        .collect();
    *settings = (fixed.join("\n") + "\n").into_bytes();
    made
}

/// Imports the fixture `name`, compares what it wrote with what is kept,
/// and gives the names of what `check` finds in it.
fn fixture(name: &str) -> Vec<String> {
    imported(name).1
}

/// [`fixture`], giving the new site's folder too.
fn imported(name: &str) -> (Scratch, Vec<String>) {
    let old = PathBuf::from(format!("tests/import/{name}/old"));
    let wanted = Path::new("tests/import").join(name).join("new");
    let before = files(&old);
    let new = scratch(name);
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    assert_eq!(files(&old), before, "{name}: the old site changed");
    let made = with_key(files(&new));
    let kept = files(&wanted);
    for (path, bytes) in &made {
        let shown = String::from_utf8_lossy(bytes);
        assert!(
            kept.get(path) == Some(bytes),
            "{name}: `{path}` is now:\n{shown}"
        );
    }
    for path in kept.keys() {
        assert!(made.contains_key(path), "{name}: `{path}` wasn't written");
    }
    let template = made.keys().find(|path| path.starts_with("templates/"));
    assert_eq!(template, None, "{name}: a template was written");
    let found = checked(&new);
    (new, found)
}

const NO_ZONE: &str = "no-time-zone";
const DRAFT: &str = "draft-left-out";
const AS_TEXT: &str = "published-as-text";
/// Gives the new site in `site` the templates its pages, its lists and its
/// feeds are built with, which an imported site has none of, so that what
/// it publishes can be looked at.
fn templated(site: &Path) {
    let page = "<title>{{ headline(page) }}</title>\n{{ page.content }}\n";
    let list = "{% for post in page.posts %}<a href=\"{{ post.url }}\">{{ post.title }}</a>\n\
                {% endfor %}";
    fs::create_dir_all(site.join("templates")).unwrap();
    fs::write(site.join("templates/page.html"), page).unwrap();
    for name in ["home", "folder"] {
        let path = site.join(format!("templates/{name}.html"));
        fs::write(path, format!("{page}{list}")).unwrap();
    }
    let feed = "<feed>{% for post in posts %}<entry>{{ post.url }}</entry>{% endfor %}</feed>\n";
    fs::write(site.join("templates/feed.xml"), feed).unwrap();
}

const BUILT: &str = "built-without-template";
const NO_FEED: &str = "feed-not-written";
const NO_LABEL: &str = "missing-label";

#[test]
fn each_permalink_style_keeps_every_address() {
    for (name, found) in [
        ("permalink-date", &[BUILT][..]),
        ("permalink-pretty", &[BUILT, NO_ZONE]),
        ("permalink-ordinal", &[BUILT, NO_ZONE]),
        ("permalink-weekdate", &[BUILT, NO_ZONE]),
        ("permalink-none", &[BUILT, NO_ZONE]),
        ("permalink-pattern", &[BUILT, NO_ZONE]),
    ] {
        assert_eq!(fixture(name), found, "{name}");
    }
}

#[test]
fn each_later_page_of_the_old_home_page_sends_readers_on_to_a_page_of_the_posts() {
    let (new, found) = imported("home-pages");
    assert_eq!(found, [BUILT]);
    templated(&new);
    let out = scratch("home-pages-built");
    let outcome = build::run(&Options {
        mode: Mode::Build,
        site: new.to_path_buf(),
        out: Some(out.clone()),
        today: Some(today()),
        drafts: false,
        url: None,
    });
    let stopped = |problem: &&statac::problem::Problem| problem.kind.severity() == Severity::Error;
    assert_eq!(outcome.problems.iter().filter(stopped).count(), 0);
    let read = |path: &str| fs::read_to_string(out.join(path)).unwrap();
    // The old third page held the oldest post, as the list's third does.
    assert!(read("page3/index.html").contains("url=https://example.org/posts/page/3/"));
    assert!(read("posts/page/3/index.html").contains("Post 1"));
    // The old second page held the later post of one day and the list's
    // second holds the earlier, so it is sent on to the page of its number.
    assert!(read("page2/index.html").contains("url=https://example.org/posts/page/2/"));
    assert!(!out.join("page").exists());
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert!(report.contains(
        "redirects from `/page2/`, an old paginated page of the home page, to the page of \
         `content/posts/` with the same number, which doesn't hold exactly the same posts"
    ));
    assert!(!report.contains("Nothing in the new site is published at these old URLs"));
}

#[test]
fn the_home_page_of_a_site_imported_from_any_tool_has_no_later_pages() {
    // Only Hugo paginated every list, so only its site asks for
    // pagination, and its home page says `0`.
    for (name, found, paged) in [
        ("hugo-home-pages", &[BUILT, NO_FEED][..], true),
        ("zola-home-pages", &[BUILT][..], false),
        ("eleventy-home-pages", &[BUILT][..], false),
    ] {
        let (new, problems) = imported(name);
        assert_eq!(problems, found, "{name}");
        templated(&new);
        let out = scratch(&format!("{name}-built"));
        let outcome = build::run(&Options {
            mode: Mode::Build,
            site: new.to_path_buf(),
            out: Some(out.clone()),
            today: Some(today()),
            drafts: false,
            url: None,
        });
        assert!(outcome.published.is_some(), "{name}");
        let home = fs::read_to_string(new.join("content/index.md")).unwrap();
        assert_eq!(
            home.contains("\nposts_per_page: 0\n"),
            paged,
            "{name}: {home}"
        );
        assert!(!out.join("page/2").exists(), "{name}");
        assert_eq!(out.join("posts/page/2").exists(), paged, "{name}");
        // Zola sent `/page/1/` on to the home page, and so does the new
        // site.
        let first = fs::read_to_string(out.join("page/1/index.html")).ok();
        let sent_home = first.is_some_and(|page| page.contains("url=https://example.org/\""));
        assert_eq!(sent_home, name == "zola-home-pages", "{name}");
    }
    let report = |name: &str| {
        fs::read_to_string(Path::new("tests/import").join(name).join("new/IMPORT.md")).unwrap()
    };
    assert!(report("hugo-home-pages").contains(
        "Where the old home page was paginated, nothing is published at page 2 onwards now: \
         `content/index.md` keeps its own list to one page with `posts_per_page: 0`"
    ));
    let zola = report("zola-home-pages");
    assert!(zola.contains(
        "nothing is published at the old home page's paginated pages now: they are listed"
    ));
    assert!(zola.contains("\n- `/page/2/`\n"));
    assert!(!report("eleventy-home-pages").contains("paginated pages"));
}

#[test]
fn a_site_under_a_path_keeps_its_addresses_and_links() {
    assert_eq!(fixture("baseurl"), [BUILT, NO_ZONE]);
}

#[test]
fn dates_come_across_from_names_and_front_matter() {
    assert_eq!(fixture("dates"), [BUILT, DRAFT]);
}

#[test]
fn front_matter_comes_across_in_statacs_words() {
    assert_eq!(fixture("front-matter"), [BUILT, DRAFT, NO_ZONE]);
}

#[test]
fn tags_in_the_writing_convert_or_stay_as_written() {
    assert_eq!(fixture("tags-in-writing"), [BUILT, NO_ZONE, AS_TEXT]);
}

#[test]
fn markdown_that_reads_differently_is_counted_not_rewritten() {
    assert_eq!(fixture("markdown-forms"), [BUILT, NO_ZONE, AS_TEXT]);
}

#[test]
fn words_statac_would_read_as_a_snippet_call_are_escaped() {
    assert_eq!(fixture("calls-in-writing"), [BUILT, NO_ZONE]);
}

#[test]
fn code_inside_html_is_given_a_blank_line_and_stays_code() {
    assert_eq!(fixture("html-writing"), [BUILT, NO_LABEL, NO_ZONE]);
}

#[test]
fn markers_statac_refuses_and_dollars_it_reads_as_maths_are_settled() {
    assert_eq!(fixture("markers-and-dollars"), [BUILT, NO_ZONE]);
}

#[test]
fn maths_the_old_site_set_between_dollar_signs_stays_maths() {
    assert_eq!(fixture("hugo-maths"), [BUILT, NO_FEED]);
    assert_eq!(fixture("eleventy-maths"), [BUILT]);
}

#[test]
fn files_data_and_folders_of_pages_come_across() {
    assert_eq!(fixture("files-and-data"), [BUILT, DRAFT, NO_ZONE]);
}

#[test]
fn settings_in_toml_are_read_as_settings_in_yaml() {
    assert_eq!(fixture("settings-toml"), [BUILT]);
}

#[test]
fn a_site_is_read_from_the_folder_its_settings_name() {
    assert_eq!(fixture("source-folder"), [BUILT, NO_ZONE]);
}

#[test]
fn pages_and_feeds_of_the_old_built_output_that_nothing_answers_are_listed() {
    assert_eq!(fixture("built-output"), [BUILT, NO_ZONE]);
}

#[test]
fn what_is_said_counts_the_old_built_pages_nothing_answers() {
    let old = Path::new("tests/import/built-output/old");
    let new = scratch("said-built");
    let (code, said) = import(old, &new, None);
    assert_eq!(code, 0);
    assert!(
        said.contains("\nURLs: 2 kept, 11 no longer published.\n"),
        "{said}"
    );
}

#[test]
fn a_file_whose_name_cant_be_published_is_given_one_that_can() {
    assert_eq!(fixture("hugo-file-names"), [BUILT, NO_FEED]);
}

#[test]
fn an_old_address_the_new_site_cant_keep_is_left_out_and_listed() {
    assert_eq!(fixture("hugo-aliases-kept"), [BUILT, NO_FEED]);
    let old = Path::new("tests/import/hugo-aliases-kept/old");
    let (code, said) = import(old, &scratch("said-aliases"), None);
    assert_eq!(code, 0);
    assert!(
        said.contains("\nURLs: 8 kept, 1 no longer published, 5 that can't be kept.\n"),
        "{said}"
    );
}

#[test]
fn a_site_with_no_posts_has_no_folder_of_posts() {
    assert_eq!(fixture("hugo-no-posts"), [BUILT, NO_FEED]);
    let new = scratch("no-posts");
    let (code, said) = import(Path::new("tests/import/hugo-no-posts/old"), &new, None);
    assert_eq!(code, 0, "{said}");
    assert!(!new.join("content/posts").exists());
}

#[test]
fn a_pattern_goes_in_the_highest_folders_own_page_and_no_page_is_made_for_it() {
    assert_eq!(fixture("hugo-nested-folders"), [BUILT, NO_FEED]);
    let new = scratch("nested");
    let old = Path::new("tests/import/hugo-nested-folders/old");
    let (code, said) = import(old, &new, None);
    assert_eq!(code, 0, "{said}");
    let made = files(&new);
    let indexes: Vec<&String> = (made.keys())
        .filter(|path| path.starts_with("content/post/") && path.ends_with("index.md"))
        .collect();
    assert_eq!(indexes, ["content/post/index.md"]);
}

#[test]
fn what_is_said_counts_what_came_across() {
    let old = Path::new("tests/import/front-matter/old");
    let new = scratch("said");
    let (code, said) = import(old, &new, None);
    assert_eq!(code, 0);
    let shown = new.to_string_lossy();
    assert_eq!(
        said,
        format!(
            "Imported the Jekyll site `tests/import/front-matter/old` into `{shown}`. This is a \
             best effort: read `{shown}/IMPORT.md` before you publish.\n\
             \n\
             Converted: 2 posts, 1 draft, 2 pages.\n\
             URLs: 6 kept.\n\
             Left for you: 7 things of 3 kinds.\n\
             The new site has no templates, so each page is published as its content alone \
             until you add your own: see https://statac.dev/templates/.\n\
             \n\
             Next: `cd {shown}`, add your own templates to `templates/`, then `statac check`.\n"
        )
    );
}

/// A Jekyll site, times what its feed plugin was given: the new site asks
/// for the site's feed at the URL the old one had, and for none where the
/// old site had none.
#[test]
fn the_feed_jekylls_plugin_made_is_asked_for_at_its_url() {
    let plugin = "plugins: [jekyll-feed]\n";
    // (the case, what the settings add, the line asking, the feed's file)
    let cases = [
        ("feed-plugin", plugin, Some("feed: true"), Some("feed.xml")),
        (
            "feed-plugin-path",
            "plugins: [jekyll-feed]\nfeed:\n  path: atom.xml\n",
            Some("feed: /atom.xml"),
            Some("atom.xml"),
        ),
        ("feed-none", "", None, None),
    ];
    for (label, added, line, file) in cases {
        let top = scratch(label);
        let old = top.join("old");
        fs::create_dir_all(old.join("_posts")).unwrap();
        let settings = format!("title: Site 1\nurl: https://example.org\ntimezone: UTC\n{added}");
        fs::write(old.join("_config.yml"), settings).unwrap();
        fs::write(
            old.join("_posts/2026-01-01-post-1.md"),
            "---\ntitle: Post 1\n---\nText.\n",
        )
        .unwrap();
        let new = top.join("new");
        let (code, said) = import(&old, &new, None);
        assert_eq!(code, 0, "{said}");
        let written = fs::read_to_string(new.join("statac.yaml")).unwrap();
        let asking: Vec<&str> = written.lines().filter(|l| l.starts_with("feed:")).collect();
        assert_eq!(asking, Vec::from_iter(line), "{label}: {written}");
        templated(&new);
        let out = top.join("out");
        let outcome = build::run(&Options {
            mode: Mode::Build,
            site: new.clone(),
            out: Some(out.clone()),
            today: Some(today()),
            drafts: false,
            url: None,
        });
        assert!(
            outcome.published.is_some(),
            "{label}: {:?}",
            outcome.problems
        );
        let unused = (outcome.problems.iter()).any(|p| p.kind.name() == "unused-feed-template");
        assert_eq!(unused, file.is_none(), "{label}");
        for name in ["feed.xml", "atom.xml", "posts/feed.xml"] {
            assert_eq!(
                out.join(name).is_file(),
                file == Some(name),
                "{label}: {name}"
            );
        }
    }
}

/// A site of each tool whose import asks for a feed, times whether its
/// settings give its URL: with no templates yet, the feed's URL is kept,
/// not listed as one nothing is published at, and the report names the
/// template the feed is built with, `templates/feed.json` for a JSON Feed,
/// and says its feeds need the site's URL where it has none.
#[test]
fn a_feed_the_import_asks_for_is_kept_at_its_url_with_no_template_or_site_url() {
    let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nText.\n";
    let package = "{ \"devDependencies\": { \"@11ty/eleventy\": \"^3.0.0\", \
                   \"@11ty/eleventy-plugin-rss\": \"^3.1.0\" } }\n";
    let eleventy = |options: &str| {
        format!(
            "import {{ feedPlugin }} from \"@11ty/eleventy-plugin-rss\";\n\
             export default function (eleventyConfig) {{\n  \
             eleventyConfig.addPlugin(feedPlugin, {options});\n}}\n"
        )
    };
    // (the case, the old site's files, the template named, whether the
    // site has a URL)
    type Case<'a> = (&'a str, Vec<(&'a str, String)>, &'a str, bool);
    let cases: [Case; 6] = [
        (
            "jekyll",
            vec![
                (
                    "_config.yml",
                    "title: Site 1\nplugins: [jekyll-feed]\n".into(),
                ),
                ("_posts/2026-01-01-post-1.md", post.into()),
            ],
            "templates/feed.xml",
            false,
        ),
        (
            "jekyll-url",
            vec![
                (
                    "_config.yml",
                    "title: Site 1\nurl: https://example.org\nplugins: [jekyll-feed]\n".into(),
                ),
                ("_posts/2026-01-01-post-1.md", post.into()),
            ],
            "templates/feed.xml",
            true,
        ),
        (
            "hugo",
            vec![
                ("hugo.toml", "title = \"Site 1\"\n".into()),
                ("content/posts/post-1.md", post.into()),
            ],
            "templates/feed.xml",
            false,
        ),
        (
            "zola",
            vec![
                (
                    "zola.toml",
                    "title = \"Site 1\"\ngenerate_feeds = true\n".into(),
                ),
                (
                    "content/posts/_index.md",
                    "+++\ntitle = \"Posts\"\n+++\n".into(),
                ),
                (
                    "content/posts/post-1.md",
                    "+++\ntitle = \"Post 1\"\ndate = 2026-01-01\n+++\nText.\n".into(),
                ),
            ],
            "templates/feed.xml",
            false,
        ),
        (
            "eleventy",
            vec![
                ("package.json", package.into()),
                ("eleventy.config.js", eleventy("{}")),
                ("posts/post-1.md", post.into()),
            ],
            "templates/feed.xml",
            false,
        ),
        (
            "eleventy-json",
            vec![
                ("package.json", package.into()),
                (
                    "eleventy.config.js",
                    eleventy("{ type: \"json\", outputPath: \"/feed.json\" }"),
                ),
                ("posts/post-1.md", post.into()),
            ],
            "templates/feed.json",
            false,
        ),
    ];
    for (label, files, template, url) in cases {
        let top = scratch(&format!("feed-asked-{label}"));
        let old = top.join("old");
        for (path, text) in &files {
            let path = old.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        let new = top.join("new");
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{label}: {told}");
        assert!(!told.contains("no longer published"), "{label}: {told}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert!(
            !report.contains("Nothing in the new site is published at these old URLs"),
            "{label}: {report}"
        );
        let needs = format!(" Its pages and feeds need `{template}`.");
        assert!(report.contains(&needs), "{label}: {report}");
        let url_needed = " Its feeds also need the site's full URL, as `url` in `statac.yaml`.";
        assert_eq!(report.contains(url_needed), !url, "{label}: {report}");
    }
}

/// An Atom feed with one entry, of the id `id`.
fn atom_feed(id: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <feed xmlns=\"http://www.w3.org/2005/Atom\">\n  <id>https://example.org/</id>\n  \
         <entry>\n    <id>{id}</id>\n  </entry>\n</feed>\n"
    )
}

/// An Eleventy site with its RSS plugin, ones whose own feed template
/// gave each post its full URL as its id, and ones whose ids were other or
/// unknown: the posts from before the import keep their full URL as their
/// id in a feed where the old site gave them that, and keep no other.
#[test]
fn an_eleventy_site_keeps_its_posts_ids_where_they_were_their_urls() {
    let plugin = ", \"@11ty/eleventy-plugin-rss\": \"^3.0.0\"";
    let url = "https://example.org/posts/post-1/";
    let rss = |id: &str| {
        format!(
            "<rss version=\"2.0\"><channel><item><guid isPermaLink=\"false\">{id}</guid></item>\
             </channel></rss>\n"
        )
    };
    let by_plugin = "The old site's `package.json` names Eleventy's RSS plugin, whose feed \
                     templates give each post its full URL as its id, so `feed_ids_before` \
                     and `feed_ids_from: eleventy` keep that id for each post.";
    let by_template = "The old site's feed at `/feed.xml` gave each post its full URL as its id, \
                       so `feed_ids_before` and `feed_ids_from: eleventy` keep that id for each \
                       post.";
    // (the case, what `package.json` adds, the feed the old site's own
    // template built, what is said)
    let cases = [
        ("plugin", plugin, None, Some(by_plugin)),
        ("none", "", None, None),
        ("atom-urls", "", Some(atom_feed(url)), Some(by_template)),
        ("rss-urls", "", Some(rss(url)), Some(by_template)),
        (
            "rss-cdata",
            "",
            Some(rss(&format!("<![CDATA[{url}]]>"))),
            Some(by_template),
        ),
        (
            "atom-uuids",
            "",
            Some(atom_feed("urn:uuid:0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20")),
            None,
        ),
        (
            "atom-other",
            "",
            Some(atom_feed("https://example.org/posts/post-2/")),
            None,
        ),
        (
            "rss-none",
            "",
            Some(rss("").replace("<guid isPermaLink=\"false\"></guid>", "")),
            None,
        ),
    ];
    for (label, needed, built, said) in cases {
        let top = scratch(&format!("eleventy-feed-ids-{label}"));
        let old = top.join("old");
        fs::create_dir_all(old.join("posts")).unwrap();
        let package =
            format!("{{ \"devDependencies\": {{ \"@11ty/eleventy\": \"^3.0.0\"{needed} }} }}\n");
        fs::write(old.join("package.json"), package).unwrap();
        let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nText.\n";
        fs::write(old.join("posts/post-1.md"), post).unwrap();
        if let Some(built) = &built {
            let template = "---\npermalink: /feed.xml\n---\n<feed></feed>\n";
            fs::write(old.join("feed.njk"), template).unwrap();
            fs::create_dir_all(old.join("_site/posts/post-1")).unwrap();
            fs::write(old.join("_site/posts/post-1/index.html"), "<p>Text.</p>\n").unwrap();
            fs::write(old.join("_site/feed.xml"), built).unwrap();
        }
        let new = top.join("new");
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{label}: {told}");
        let settings = fs::read_to_string(new.join("statac.yaml")).unwrap();
        let kept: Vec<&str> = (settings.lines())
            .filter(|line| line.starts_with("feed_ids_"))
            .collect();
        let wanted = Vec::from_iter(
            said.into_iter()
                .flat_map(|_| ["feed_ids_before: 2026-10-06", "feed_ids_from: eleventy"]),
        );
        assert_eq!(kept, wanted, "{label}: {settings}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        for sentence in [by_plugin, by_template] {
            assert_eq!(
                report.contains(sentence),
                said == Some(sentence),
                "{label}: {report}"
            );
        }

        let added = "url: https://example.org\nfeed: true\n";
        fs::write(new.join("statac.yaml"), format!("{settings}{added}")).unwrap();
        templated(&new);
        let feed = "<feed>{% for post in posts %}<id>{{ feed_id(post) }}</id>{% endfor %}</feed>\n";
        fs::write(new.join("templates/feed.xml"), feed).unwrap();
        let out = top.join("out");
        let outcome = build::run(&Options {
            mode: Mode::Build,
            site: new.clone(),
            out: Some(out.clone()),
            today: Some(today()),
            drafts: false,
            url: None,
        });
        assert!(
            outcome.published.is_some(),
            "{label}: {:?}",
            outcome.problems
        );
        let built = fs::read_to_string(out.join("feed.xml")).unwrap();
        assert_eq!(
            built.contains(&format!("<id>{url}</id>")),
            said.is_some(),
            "{label}: {built}"
        );
        assert_eq!(
            built.contains("<id>urn:uuid:"),
            said.is_none(),
            "{label}: {built}"
        );
    }
}

/// An Eleventy site, times where its RSS plugin's feed was: the new site
/// asks for the site's feed at the URL the settings or the old built
/// output show, and for none where neither shows one.
#[test]
fn the_feed_eleventys_rss_plugin_made_is_asked_for_at_its_url() {
    let plugin = |options: &str| format!("eleventyConfig.addPlugin(feedPlugin, {options});");
    let unread = plugin("{ outputPath: process.env.FEED }");
    let asked = "in `statac.yaml` says";
    let none = "so Statac asked for none";
    // (the case, the settings' calls, the feeds in the old built output,
    // the lines asking, what is said, the feed's file)
    type Case<'a> = (
        &'a str,
        String,
        &'a [&'a str],
        &'a [&'a str],
        &'a str,
        Option<&'a str>,
    );
    let cases: [Case; 8] = [
        (
            "path",
            plugin("{ outputPath: \"/feed/feed.xml\", collection: { name: \"posts\" } }"),
            &[],
            &["feed: /feed/feed.xml"],
            asked,
            Some("feed/feed.xml"),
        ),
        (
            "default",
            plugin("{ collection: { name: \"posts\" } }"),
            &[],
            &["feed: true"],
            asked,
            Some("feed.xml"),
        ),
        (
            "json",
            plugin("{ type: \"json\", outputPath: \"/feed.json\" }"),
            &[],
            &["feed: /feed.json"],
            "built with `templates/feed.json` once you add it",
            Some("feed.json"),
        ),
        (
            "two",
            format!(
                "{}\n  {}",
                plugin("{ outputPath: \"/feed.xml\" }"),
                plugin("{ type: \"json\", outputPath: \"/feed.json\" }")
            ),
            &[],
            &["feed:", "  - /feed.xml", "  - /feed.json"],
            "Each stays at the same URL",
            Some("feed.xml"),
        ),
        (
            "built",
            unread.clone(),
            &["blog/atom.xml"],
            &["feed: /blog/atom.xml"],
            "so Statac took it for the plugin's",
            Some("blog/atom.xml"),
        ),
        (
            "built-two",
            unread.clone(),
            &["blog/atom.xml", "notes/atom.xml"],
            &[],
            "so Statac can't tell which the plugin made and asked for none",
            None,
        ),
        ("unknown", unread, &[], &[], none, None),
        (
            "template",
            "let { content, data } = getFeedTemplate({ outputPath: \"/posts.xml\" });\n  \
             eleventyConfig.addTemplate(\"feeds/posts.njk\", content, data);"
                .to_owned(),
            &[],
            &["feed: /posts.xml"],
            "The settings' `getFeedTemplate`, from Eleventy's RSS plugin, made an Atom feed at \
             `/posts.xml`.",
            Some("posts.xml"),
        ),
    ];
    for (label, calls, built, lines, said, file) in cases {
        let top = scratch(&format!("eleventy-feed-{label}"));
        let old = top.join("old");
        fs::create_dir_all(old.join("posts")).unwrap();
        let package = "{ \"devDependencies\": { \"@11ty/eleventy\": \"^3.0.0\", \
                       \"@11ty/eleventy-plugin-rss\": \"^3.1.0\" } }\n";
        fs::write(old.join("package.json"), package).unwrap();
        let settings = format!(
            "import {{ feedPlugin, getFeedTemplate }} from \"@11ty/eleventy-plugin-rss\";\n\n\
             export default function (eleventyConfig) {{\n  {calls}\n}}\n"
        );
        fs::write(old.join("eleventy.config.js"), settings).unwrap();
        let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nText.\n";
        fs::write(old.join("posts/post-1.md"), post).unwrap();
        for path in built.iter() {
            let path = old.join("_site").join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, atom_feed("https://example.org/posts/post-1/")).unwrap();
        }
        let new = top.join("new");
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{label}: {told}");
        let written = fs::read_to_string(new.join("statac.yaml")).unwrap();
        let asking: Vec<&str> = (written.lines())
            .skip_while(|line| !line.starts_with("feed:"))
            .take_while(|line| line.starts_with("feed:") || line.starts_with("  - "))
            .collect();
        assert_eq!(asking, lines, "{label}: {written}");
        assert!(written.contains("feed_ids_before: 2026-10-06"), "{label}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert!(report.contains(said), "{label}: {report}");
        for path in built.iter().filter(|_| lines.is_empty()) {
            let listed = format!("- `_site/{path}`: `/{path}`");
            assert!(report.contains(&listed), "{label}: {report}");
        }

        fs::write(
            new.join("statac.yaml"),
            format!("{written}url: https://example.org\n"),
        )
        .unwrap();
        templated(&new);
        let out = top.join("out");
        let outcome = build::run(&Options {
            mode: Mode::Build,
            site: new.clone(),
            out: Some(out.clone()),
            today: Some(today()),
            drafts: false,
            url: None,
        });
        assert!(
            outcome.published.is_some(),
            "{label}: {:?}",
            outcome.problems
        );
        let unused = (outcome.problems.iter()).any(|p| p.kind.name() == "unused-feed-template");
        assert_eq!(unused, file.is_none(), "{label}");
        for name in [
            "feed.xml",
            "feed/feed.xml",
            "feed.json",
            "blog/atom.xml",
            "posts.xml",
        ] {
            assert_eq!(
                out.join(name).is_file(),
                file == Some(name) || (label == "two" && name == "feed.json"),
                "{label}: {name}"
            );
        }
    }
}

/// A JSON Feed with one entry, of the id `id`.
fn json_feed(id: &str) -> String {
    format!(
        "{{\"version\": \"https://jsonfeed.org/version/1.1\", \"title\": \"Site 1\", \
         \"items\": [{{\"id\": \"{id}\"}}]}}\n"
    )
}

/// An Eleventy site, times a feed in its old built output that a template
/// of the old site made, or that nothing the import found made, as Atom or
/// a JSON Feed, alone or beside the RSS plugin's: each is listed as a feed
/// made by a template, and not as a page, and the posts keep their ids
/// where it gave each its full URL.
#[test]
fn an_eleventy_sites_feeds_the_rss_plugin_didnt_make_are_listed_and_keep_their_ids() {
    let url = "https://example.org/posts/post-1/";
    let plugin = "eleventyConfig.addPlugin(feedPlugin, { outputPath: \"/feed.xml\" });";
    // (the case, the template making the feed, the settings' calls, the
    // feed's path in the old built output and what it holds, the lines
    // asking, the line listing it)
    type Case<'a> = (
        &'a str,
        Option<(&'a str, &'a str)>,
        Option<&'a str>,
        &'a str,
        String,
        &'a [&'a str],
        &'a str,
    );
    let cases: [Case; 5] = [
        (
            "atom",
            Some((
                "atom.njk",
                "---\npermalink: /atom.xml\n---\n<feed></feed>\n",
            )),
            None,
            "atom.xml",
            atom_feed(url),
            &[],
            "- `atom.njk`: `/atom.xml`",
        ),
        (
            "json",
            Some(("json.njk", "---\npermalink: /feed.json\n---\n{}\n")),
            None,
            "feed.json",
            json_feed(url),
            &[],
            "- `json.njk`: `/feed.json`",
        ),
        (
            "unmade",
            None,
            None,
            "atom.xml",
            atom_feed(url),
            &[],
            "- `_site/atom.xml`: `/atom.xml`",
        ),
        (
            "unmade-json",
            None,
            None,
            "feed.json",
            json_feed(url),
            &[],
            "- `_site/feed.json`: `/feed.json`",
        ),
        (
            "beside-plugin",
            None,
            Some(plugin),
            "feed.json",
            json_feed(url),
            &["feed: true"],
            "- `_site/feed.json`: `/feed.json`",
        ),
    ];
    for (label, template, calls, path, built, lines, listed) in cases {
        let top = scratch(&format!("eleventy-feed-left-{label}"));
        let old = top.join("old");
        fs::create_dir_all(old.join("posts")).unwrap();
        fs::create_dir_all(old.join("_site/posts/post-1")).unwrap();
        let package = "{ \"devDependencies\": { \"@11ty/eleventy\": \"^3.0.0\" } }\n";
        fs::write(old.join("package.json"), package).unwrap();
        if let Some(calls) = calls {
            let settings = format!(
                "import {{ feedPlugin }} from \"@11ty/eleventy-plugin-rss\";\n\n\
                 export default function (eleventyConfig) {{\n  {calls}\n}}\n"
            );
            fs::write(old.join("eleventy.config.js"), settings).unwrap();
        }
        if let Some((name, text)) = template {
            fs::write(old.join(name), text).unwrap();
        }
        let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nText.\n";
        fs::write(old.join("posts/post-1.md"), post).unwrap();
        fs::write(old.join("_site/posts/post-1/index.html"), "<p>Text.</p>\n").unwrap();
        fs::write(old.join("_site").join(path), built).unwrap();
        let new = top.join("new");
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{label}: {told}");
        let settings = fs::read_to_string(new.join("statac.yaml")).unwrap();
        let asking: Vec<&str> = (settings.lines())
            .filter(|line| line.starts_with("feed:"))
            .collect();
        assert_eq!(asking, lines, "{label}: {settings}");
        assert!(
            settings.contains("\nfeed_ids_from: eleventy\n"),
            "{label}: {settings}"
        );
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        let feeds = report.split("### Feeds made by templates").nth(1);
        assert!(
            feeds.is_some_and(|feeds| feeds.contains(&format!("\n{listed}\n"))),
            "{label}: {report}"
        );
        assert!(
            !report.contains("### Pages made by templates"),
            "{label}: {report}"
        );
    }
}

/// An Eleventy site with posts whose URLs hold `'` and `&`, times the
/// feeds its RSS plugin made: the report names those posts where a feed
/// was Atom or JSON, whose ids the plugin escaped twice, says they may be
/// where it can't tell the feed's kind, and says nothing of RSS.
#[test]
fn an_eleventy_sites_posts_whose_ids_the_rss_plugin_escaped_twice_are_named() {
    let posts = "2 posts, `/posts/a-'-b/` and `/posts/c-&-d/`, ids escaped twice, as Eleventy's \
                 RSS plugin does with a `'` or `&` in an Atom or JSON feed";
    let sure = |feeds: &str| {
        format!(
            "The old site's {feeds} gave {posts}, so feed readers will show each once as new \
             after the move."
        )
    };
    let unsure = format!(
        "The old site's feed may have given {posts}, so feed readers may show each once as new \
         after the move: Statac couldn't read which kind of feed it was."
    );
    let plugin = |options: &str| format!("eleventyConfig.addPlugin(feedPlugin, {options});");
    // (the case, the settings' calls, what is said)
    let cases = [
        ("atom", plugin("{}"), Some(sure("Atom feed at `/feed.xml`"))),
        (
            "json",
            plugin("{ type: \"json\", outputPath: \"/feed.json\" }"),
            Some(sure("JSON feed at `/feed.json`")),
        ),
        ("rss", plugin("{ type: \"rss\" }"), None),
        (
            "rss-and-atom",
            format!(
                "{}\n  {}",
                plugin("{ type: \"rss\", outputPath: \"/rss.xml\" }"),
                plugin("{ type: \"atom\", outputPath: \"/atom.xml\" }")
            ),
            Some(sure("Atom feed at `/atom.xml`")),
        ),
        (
            "unknown",
            plugin("{ type: process.env.TYPE }"),
            Some(unsure),
        ),
    ];
    for (label, calls, said) in cases {
        let top = scratch(&format!("eleventy-feed-twice-{label}"));
        let old = top.join("old");
        fs::create_dir_all(old.join("posts")).unwrap();
        let package = "{ \"devDependencies\": { \"@11ty/eleventy\": \"^3.0.0\", \
                       \"@11ty/eleventy-plugin-rss\": \"^3.1.0\" } }\n";
        fs::write(old.join("package.json"), package).unwrap();
        let settings = format!(
            "import {{ feedPlugin }} from \"@11ty/eleventy-plugin-rss\";\n\n\
             export default function (eleventyConfig) {{\n  {calls}\n}}\n"
        );
        fs::write(old.join("eleventy.config.js"), settings).unwrap();
        for (n, permalink) in [(1, "/posts/a-'-b/"), (2, "/posts/c-&-d/"), (3, "/posts/e/")] {
            let post = format!(
                "---\ntitle: Post {n}\ndate: 2026-01-0{n}\npermalink: \"{permalink}\"\n---\nText.\n"
            );
            fs::write(old.join(format!("posts/post-{n}.md")), post).unwrap();
        }
        let new = top.join("new");
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{label}: {told}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        match &said {
            Some(said) => assert!(report.contains(said.as_str()), "{label}: {report}"),
            None => assert!(!report.contains("escaped twice"), "{label}: {report}"),
        }
        assert_eq!(
            report.matches("escaped twice").count(),
            usize::from(said.is_some()),
            "{label}: {report}"
        );
    }
}

/// An Eleventy site, times the `metadata.base` its RSS plugin's feed was
/// given and the path the settings put the site under: the new site's
/// `url` is the URL that feed made each post's full URL from, so the new
/// site's feed gives each post the id the old one did.
#[test]
fn an_eleventy_sites_url_is_the_one_its_rss_plugins_feed_made_posts_urls_from() {
    // (the case, the plugin's `metadata`, the settings' `pathPrefix`, the
    // `url` written)
    let cases = [
        (
            "base",
            "{ base: \"https://example.org/\" }",
            None,
            Some("https://example.org"),
        ),
        (
            "base-path",
            "{ base: \"https://example.org/site/\" }",
            Some("/blog/"),
            Some("https://example.org/blog"),
        ),
        ("base-not-url", "{ base: \"/\" }", None, None),
        ("no-base", "{ title: \"Site 1\" }", None, None),
    ];
    for (label, metadata, prefix, url) in cases {
        let top = scratch(&format!("eleventy-feed-url-{label}"));
        let old = top.join("old");
        fs::create_dir_all(old.join("posts")).unwrap();
        let package = "{ \"devDependencies\": { \"@11ty/eleventy\": \"^3.0.0\", \
                       \"@11ty/eleventy-plugin-rss\": \"^3.1.0\" } }\n";
        fs::write(old.join("package.json"), package).unwrap();
        let config = match prefix {
            Some(prefix) => format!("\nexport const config = {{ pathPrefix: \"{prefix}\" }};\n"),
            None => String::new(),
        };
        let settings = format!(
            "import {{ feedPlugin }} from \"@11ty/eleventy-plugin-rss\";\n\n\
             export default function (eleventyConfig) {{\n  \
             eleventyConfig.addPlugin(feedPlugin, {{ metadata: {metadata} }});\n}}\n{config}"
        );
        fs::write(old.join("eleventy.config.js"), settings).unwrap();
        let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nText.\n";
        fs::write(old.join("posts/post-1.md"), post).unwrap();
        let new = top.join("new");
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{label}: {told}");
        let written = fs::read_to_string(new.join("statac.yaml")).unwrap();
        let urls: Vec<&str> = (written.lines())
            .filter_map(|line| line.strip_prefix("url: "))
            .collect();
        assert_eq!(urls, Vec::from_iter(url), "{label}: {written}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        let Some(url) = url else {
            assert!(report.contains("has no `url` yet"), "{label}: {report}");
            continue;
        };
        let said = format!(
            "The new site's `url`, `{url}`, is made from `metadata.base` in the settings' \
             `feedPlugin`, on line 4, {}as Eleventy's RSS plugin made each post's full URL.",
            match prefix {
                Some(_) => "and from `pathPrefix`, ",
                None => "",
            }
        );
        assert!(report.contains(&said), "{label}: {report}");

        templated(&new);
        let feed = "<feed>{% for post in posts %}<id>{{ feed_id(post) }}</id>{% endfor %}</feed>\n";
        fs::write(new.join("templates/feed.xml"), feed).unwrap();
        let out = top.join("out");
        let outcome = build::run(&Options {
            mode: Mode::Build,
            site: new.clone(),
            out: Some(out.clone()),
            today: Some(today()),
            drafts: false,
            url: None,
        });
        assert!(
            outcome.published.is_some(),
            "{label}: {:?}",
            outcome.problems
        );
        let built = fs::read_to_string(out.join("feed.xml")).unwrap();
        let id = format!("<id>{url}/posts/post-1/</id>");
        assert!(built.contains(&id), "{label}: {built}");
    }
}

/// An Eleventy site, times the `metadata` its RSS plugin's feed was
/// given: the new site's title and language are the feed's where it gives
/// them, and the report says where each came from and what it assumed for
/// the rest.
#[test]
fn an_eleventy_sites_title_and_language_are_the_ones_its_rss_plugins_feed_was_given() {
    // (the case, the plugin's `metadata`, the title and language written,
    // what is said)
    let cases: [(&str, &str, [&str; 2], &[&str]); 5] = [
        (
            "both",
            "{ title: \"Site 1\", language: \"fr\", base: \"https://example.org/\" }",
            ["title: Site 1", "language: fr"],
            &[
                "The new site's `title`, `Site 1`, is taken from `metadata.title` in the \
                 settings' `feedPlugin`, on line 4.",
                "The new site's `language`, `fr`, is taken from `metadata.language` in the \
                 settings' `feedPlugin`, on line 4.",
            ],
        ),
        (
            "title",
            "{ title: \"Site 1\" }",
            ["title: Site 1", "language: en"],
            &[
                "Eleventy's settings hold no URL or language for the site, so the new site has \
               no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.",
            ],
        ),
        (
            "language",
            "{ language: \"fr\", base: \"https://example.org/\" }",
            ["title: Old", "language: fr"],
            &[
                "Eleventy's settings hold no title for the site, so the new site's `title` is \
               made from the old folder's name: put it right in `statac.yaml`.",
            ],
        ),
        (
            "not-a-code",
            "{ title: \"Site 1\", language: \"not a code\", base: \"https://example.org/\" }",
            ["title: Site 1", "language: en"],
            &[
                "The settings' `feedPlugin`, on line 4, gives `metadata.language` as \
               `not a code`, which isn't a code Statac reads, so the site is in English, \
               `en`: write its code in `language` in `statac.yaml`.",
            ],
        ),
        (
            "neither",
            "{ title: \"\", base: \"https://example.org/\" }",
            ["title: Old", "language: en"],
            &[
                "Eleventy's settings hold no title or language for the site, so the new site's \
               `title` is made from the old folder's name, and it is in English, `en`: put \
               each right in `statac.yaml`.",
            ],
        ),
    ];
    for (label, metadata, lines, said) in cases {
        let top = scratch(&format!("eleventy-feed-title-{label}"));
        let old = top.join("old");
        fs::create_dir_all(old.join("posts")).unwrap();
        let package = "{ \"devDependencies\": { \"@11ty/eleventy\": \"^3.0.0\", \
                       \"@11ty/eleventy-plugin-rss\": \"^3.1.0\" } }\n";
        fs::write(old.join("package.json"), package).unwrap();
        let settings = format!(
            "import {{ feedPlugin }} from \"@11ty/eleventy-plugin-rss\";\n\n\
             export default function (eleventyConfig) {{\n  \
             eleventyConfig.addPlugin(feedPlugin, {{ metadata: {metadata} }});\n}}\n"
        );
        fs::write(old.join("eleventy.config.js"), settings).unwrap();
        let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nText.\n";
        fs::write(old.join("posts/post-1.md"), post).unwrap();
        let new = top.join("new");
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{label}: {told}");
        let written = fs::read_to_string(new.join("statac.yaml")).unwrap();
        let read: Vec<&str> = (written.lines())
            .filter(|line| line.starts_with("title: ") || line.starts_with("language: "))
            .collect();
        assert_eq!(read, lines, "{label}: {written}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        for said in said {
            assert!(report.contains(said), "{label}: {said}\n{report}");
        }
        let holds_no = report.matches("Eleventy's settings hold no").count();
        let holds = !matches!(label, "both" | "not-a-code");
        assert_eq!(holds_no, usize::from(holds), "{label}: {report}");
    }
}

#[test]
fn every_error_check_finds_in_the_new_site_is_listed_and_said() {
    let top = scratch("errors");
    let old = top.join("old");
    fs::create_dir_all(old.join("_posts")).unwrap();
    fs::write(old.join("_config.yml"), "title: Site 1\n").unwrap();
    fs::write(
        old.join("_posts/2026-01-01-post-1.md"),
        "---\ntitle: Post 1\ndate: 04/10/2026\n---\nA post with a date of two readings.\n",
    )
    .unwrap();
    fs::write(
        old.join("_posts/2026-01-02-post-2.md"),
        "---\ntitle: Post 2\n---\nA post whose maths has a brace never closed.\n\n\
         Half is $$\\frac{1}{2$$.\n",
    )
    .unwrap();
    let new = top.join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    let shown = new.to_string_lossy();
    assert!(
        said.ends_with(&format!(
            "Left for you: 4 things of 3 kinds.\n\
             The new site has no templates, so each page is published as its content alone \
             until you add your own: see https://statac.dev/templates/.\n\
             The new site also has 2 errors, listed in `{shown}/IMPORT.md`.\n\
             \n\
             Next: `cd {shown}`, add your own templates to `templates/`, then `statac check`.\n"
        )),
        "{said}"
    );
    let listed = "### Errors in the new site\n\
                  \n\
                  Statac found these when it read the new site, and each stops it building: \
                  `statac check` shows each one with what to do.\n\
                  \n\
                  - `content/posts/post-1.md`, line 3: ``Unclear date: `04/10/2026` could be 4 \
                  October or 10 April``\n\
                  - `content/posts/post-2.md`, line 8: ``Malformed maths: `{` is never closed``\n\
                  \n";
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert!(report.contains(listed), "{report}");
    assert!(!report.contains("Missing template"), "{report}");
    let outcome = build::run(&Options {
        mode: Mode::Check,
        site: new.clone(),
        out: None,
        today: Some(today()),
        drafts: false,
        url: None,
    });
    let errors = (outcome.problems.iter())
        .filter(|problem| problem.kind.severity() == Severity::Error)
        .filter(|problem| !matches!(problem.kind, Kind::MissingTemplate { .. }))
        .count();
    assert_eq!(errors, 2);
}

#[test]
fn a_site_imported_has_its_settings_and_what_came_across_alone() {
    let addressed = "title: Site 1\nurl: https://example.org\n";
    for (name, settings_start) in [
        ("call-kept", addressed),
        ("hugo-call-kept", addressed),
        ("zola-call-kept", addressed),
        ("eleventy-call-kept", "title: Old\nlanguage: en\n"),
    ] {
        let old = PathBuf::from(format!("tests/import/{name}/old"));
        let before = files(&old);
        let new = scratch(&format!("{name}-without"));
        let (code, said) = import(&old, &new, None);
        assert_eq!(code, 0, "{said}");
        assert_eq!(files(&old), before, "{name}: the old site changed");
        let made = with_key(files(&new));
        let kept = files(&Path::new("tests/import").join(name).join("new"));
        for (path, bytes) in &made {
            let shown = String::from_utf8_lossy(bytes);
            assert!(
                kept.get(path) == Some(bytes),
                "{name}: `{path}` is now:\n{shown}"
            );
        }
        for path in kept.keys() {
            assert!(made.contains_key(path), "{name}: `{path}` wasn't written");
        }
        assert!(made.contains_key(".gitignore"), "{name}");
        for path in made.keys() {
            let came = ["content/", "snippets/", "static/"];
            assert!(
                came.iter().any(|folder| path.starts_with(folder))
                    || [".gitignore", "statac.yaml", "IMPORT.md"].contains(&path.as_str()),
                "{name}: `{path}` was written"
            );
        }
        let text = |path: &str| String::from_utf8(made[path].clone()).unwrap();
        let post = text("content/posts/post-1.md");
        assert!(post.contains("\n@note()\n"), "{name}: {post}");
        assert!(made.contains_key("snippets/note.html"), "{name}");
        assert!(
            post.contains("redirect_from:\n  - /old/post-1/\n"),
            "{name}: {post}"
        );
        assert!(made.contains_key("static/file-1.txt"), "{name}");
        let settings = text("statac.yaml");
        assert!(settings.starts_with(settings_start), "{name}: {settings}");
        let report = text("IMPORT.md");
        let templates = "It has no templates, so each page is published as its content alone \
                         until you add templates of your own: see https://statac.dev/templates/.";
        assert_eq!(report.matches(templates).count(), 1, "{name}: {report}");
        assert!(!report.contains("Missing template"), "{name}: {report}");
        assert!(
            said.contains(
                "The new site has no templates, so each page is published as its content alone \
                 until you add your own: see https://statac.dev/templates/.\n"
            ),
            "{name}: {said}"
        );
        // The only errors in the new site are the templates it lacks.
        let outcome = build::run(&Options {
            mode: Mode::Check,
            site: new.clone(),
            out: None,
            today: Some(today()),
            drafts: false,
            url: None,
        });
        let errors: Vec<&str> = (outcome.problems.iter())
            .filter(|problem| problem.kind.severity() == Severity::Error)
            .map(|problem| problem.kind.name())
            .collect();
        assert!(
            errors.iter().all(|error| *error == "missing-template"),
            "{name}: {errors:?}"
        );
    }
}

/// What importing `old` says, having written nothing.
fn refused(old: &Path, new: &Path, from: Option<Tool>) -> String {
    let (code, said) = import(old, new, from);
    assert_eq!(code, 1, "{said}");
    said.lines().next().unwrap_or_default().to_owned()
}

#[test]
fn a_site_that_cant_be_imported_is_refused_and_nothing_is_written() {
    let top = scratch("refused");
    let old = top.join("old");
    fs::create_dir_all(old.join("_posts")).unwrap();
    fs::write(old.join("_config.yml"), "title: Site 1\n").unwrap();
    let before = files(&old);

    let missing = top.join("missing");
    assert_eq!(
        refused(&missing, &top.join("new-1"), None),
        format!(
            "error  Can't read `{}`: it doesn't exist",
            missing.display()
        )
    );

    let full = top.join("full");
    fs::create_dir_all(&full).unwrap();
    fs::write(full.join("a.txt"), "a\n").unwrap();
    let (_, said) = import(&old, &full, None);
    assert_eq!(
        said,
        format!(
            "error  Folder not empty: `{}` holds `a.txt`\n\
             \x20 Give `statac import` an empty folder, or the name of a new one.\n\
             \x20 More: https://statac.dev/e/folder-not-empty/\n",
            full.display()
        )
    );

    let site = top.join("site");
    fs::create_dir_all(&site).unwrap();
    fs::write(site.join("statac.yaml"), "title: Site 2\n").unwrap();
    let (_, said) = import(&old, &site, None);
    assert_eq!(
        said,
        format!(
            "error  Already a site: `{}` has a `statac.yaml`\n\
             \x20 More: https://statac.dev/e/already-a-site/\n",
            site.display()
        )
    );

    let inside = old.join("new");
    assert_eq!(
        refused(&old, &inside, None),
        format!(
            "error  Can't write `{}`: it is inside the site being imported, `{}`",
            inside.display(),
            old.display()
        )
    );
    assert!(!inside.exists());
    assert_eq!(
        refused(&old, &top, None),
        format!(
            "error  Can't write `{}`: the site being imported, `{}`, is inside it",
            top.display(),
            old.display()
        )
    );

    let nothing = top.join("nothing");
    fs::create_dir_all(&nothing).unwrap();
    fs::write(nothing.join("index.md"), "Home.\n").unwrap();
    let said = {
        let (code, said) = import(&nothing, &top.join("new-2"), None);
        assert_eq!(code, 1);
        said
    };
    assert!(
        said.starts_with(&format!(
            "error  Can't tell what made the site: `{}` has no settings file Statac knows\n  \
             Say what made it with `--from=jekyll`, `--from=hugo`, `--from=zola` or \
             `--from=eleventy`.\n",
            nothing.display()
        )),
        "{said}"
    );

    let toml = top.join("toml");
    fs::create_dir_all(&toml).unwrap();
    fs::write(toml.join("_config.toml"), "title = \"Site 1\"\nurl = \n").unwrap();
    assert_eq!(
        refused(&toml, &top.join("new-3"), None),
        format!(
            "error  Can't read `{}`: this line doesn't read as TOML",
            toml.join("_config.toml").display()
        )
    );

    for name in ["new-1", "new-2", "new-3"] {
        assert!(!top.join(name).exists(), "{name}");
    }
    assert_eq!(files(&old), before);
}

#[test]
fn a_site_whose_tool_isnt_told_is_read_as_the_tool_named() {
    let top = scratch("named");
    let old = top.join("old");
    fs::create_dir_all(old.join("_posts")).unwrap();
    fs::write(
        old.join("_posts/2026-01-01-post-1.md"),
        "---\ntitle: Post 1\n---\nA post in a site with no settings file, read as Jekyll's.\n",
    )
    .unwrap();
    let new = top.join("new");
    let (code, said) = import(&old, &new, Some(Tool::Jekyll));
    assert_eq!(code, 0, "{said}");
    let post = fs::read_to_string(new.join("content/posts/post-1.md")).unwrap();
    assert!(
        post.starts_with("---\ntitle: Post 1\ndate: 2026-01-01\n"),
        "{post}"
    );
}

#[test]
fn the_command_line_asks_for_a_folder_and_a_tool_statac_knows() {
    let run = |args: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(args)
            .env("NO_COLOR", "1")
            .output()
            .unwrap()
    };
    let output = run(&["import"]);
    assert_eq!(output.status.code(), Some(2));
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(
        said.starts_with(
            "error  Missing folder: `statac import` needs the folder of the site to import\n"
        ),
        "{said}"
    );
    let output = run(&["import", "old", "--from=tool-1"]);
    assert_eq!(output.status.code(), Some(2));
    let said = String::from_utf8(output.stderr).unwrap();
    assert!(said.contains("`--from`"), "{said}");
    assert!(said.contains("`jekyll`"), "{said}");
    assert!(said.contains("`hugo`"), "{said}");
}

#[test]
fn hugo_settings_are_read_in_each_format_and_split_over_a_folder() {
    for name in [
        "hugo-settings-yaml",
        "hugo-settings-json",
        "hugo-settings-split",
        "hugo-config-toml",
    ] {
        assert_eq!(fixture(name), [BUILT, NO_FEED], "{name}");
    }
}

#[test]
fn hugo_front_matter_comes_across_in_statacs_words() {
    assert_eq!(fixture("hugo-front-matter"), [BUILT, DRAFT, NO_FEED]);
}

#[test]
fn hugo_pages_kept_as_folders_come_across_whole() {
    assert_eq!(fixture("hugo-bundles"), [BUILT, NO_FEED]);
}

#[test]
fn hugo_addresses_are_kept_by_each_placeholder_and_as_files() {
    assert_eq!(fixture("hugo-permalinks"), [BUILT, NO_FEED]);
    assert_eq!(fixture("hugo-ugly"), [BUILT, NO_FEED]);
}

/// Builds the site in `site` into `out`, having found no error.
fn build_into(site: &Path, out: &Path) {
    templated(site);
    let outcome = build::run(&Options {
        mode: Mode::Build,
        site: site.to_path_buf(),
        out: Some(out.to_path_buf()),
        today: Some(today()),
        drafts: false,
        url: None,
    });
    let stopped = |problem: &&statac::problem::Problem| problem.kind.severity() == Severity::Error;
    assert_eq!(outcome.problems.iter().filter(stopped).count(), 0);
}

/// A list stays at the address Hugo gave it, its later pages and its feed
/// where Hugo's were, and its first later page, which Hugo sent on to the
/// list, sends readers on. Hugo before 0.148 published a list under
/// `uglyURLs` as a file, which sends readers on too; since, it is a folder.
#[test]
fn a_hugo_list_stays_at_the_address_hugo_gave_it() {
    // The settings, the list's own front matter, the `permalink` and the
    // `redirect_from` written on it, and the files its page, its second
    // page and its feed are built at.
    type Case<'a> = (
        &'a str,
        &'a str,
        Option<&'a str>,
        &'a [&'a str],
        [&'a str; 3],
    );
    let cases: [Case; 4] = [
        (
            "[permalinks.section]\nposts = '/blog/'\n",
            "",
            Some("/blog/"),
            &["/blog/page/1/"],
            [
                "blog/index.html",
                "blog/page/2/index.html",
                "blog/index.xml",
            ],
        ),
        (
            "uglyURLs = true\n",
            "",
            None,
            &["/posts.html", "/posts/page/1/"],
            [
                "posts/index.html",
                "posts/page/2/index.html",
                "posts/index.xml",
            ],
        ),
        (
            "",
            "url: /blog/\n",
            Some("/blog/"),
            &["/blog/page/1/"],
            [
                "blog/index.html",
                "blog/page/2/index.html",
                "blog/index.xml",
            ],
        ),
        (
            "",
            "",
            None,
            &["/posts/page/1/"],
            [
                "posts/index.html",
                "posts/page/2/index.html",
                "posts/index.xml",
            ],
        ),
    ];
    let older = "Hugo before version 0.148 published a list under `uglyURLs` as a file, such as \
                 `/posts.html`, which now redirects to the list, and its paginated pages as files \
                 too, such as `/posts/page/2.html`: nothing is published at those now.";
    for (at, (settings, front, permalink, sent, files)) in cases.into_iter().enumerate() {
        let old = scratch(&format!("hugo-list-old-{at}"));
        let new = scratch(&format!("hugo-list-new-{at}"));
        let out = scratch(&format!("hugo-list-out-{at}"));
        fs::create_dir_all(old.join("content/posts")).unwrap();
        let top = "baseURL = 'https://example.org/'\ntitle = 'Lists'\n";
        let paging = "[pagination]\npagerSize = 1\n";
        fs::write(old.join("hugo.toml"), format!("{top}{settings}{paging}")).unwrap();
        let list = format!("---\ntitle: Posts\n{front}---\n");
        fs::write(old.join("content/posts/_index.md"), list).unwrap();
        for n in 1..=2 {
            let post = format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\n---\nA post.\n");
            fs::write(old.join(format!("content/posts/post-{n}.md")), post).unwrap();
        }
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{at}: {told}");
        let index = fs::read_to_string(new.join("content/posts/index.md")).unwrap();
        match permalink {
            Some(permalink) => {
                assert!(
                    index.contains(&format!("\npermalink: {permalink}\n")),
                    "{at}: {index}"
                )
            }
            None => assert!(!index.contains("\npermalink:"), "{at}: {index}"),
        }
        let from: String = sent.iter().map(|old| format!("  - {old}\n")).collect();
        assert!(
            index.contains(&format!("\nredirect_from:\n{from}")),
            "{at}: {index}"
        );
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert!(
            !report.contains("Nothing in the new site is published"),
            "{at}: {report}"
        );
        assert!(
            !report.contains("Feeds made by templates"),
            "{at}: {report}"
        );
        assert_eq!(report.contains(older), at == 1, "{at}: {report}");
        build_into(&new, &out);
        for file in files {
            assert!(out.join(file).is_file(), "{at}: nothing at {file}");
        }
        for old in sent {
            let file = match old.strip_suffix('/') {
                Some(folder) => format!("{}/index.html", &folder[1..]),
                None => old[1..].to_owned(),
            };
            assert!(out.join(&file).is_file(), "{at}: nothing at {file}");
        }
        if permalink.is_some() {
            assert!(!out.join("posts/index.html").exists(), "{at}");
        }
    }
}

/// Each later page of a list or a tag in the old built output, which
/// Hugo before 0.148 published under `uglyURLs` as files, sends readers on
/// to the new list's page of the same number.
#[test]
fn later_pages_in_the_old_built_output_send_readers_on() {
    let old = scratch("hugo-built-later-old");
    let new = scratch("hugo-built-later-new");
    let out = scratch("hugo-built-later-out");
    fs::create_dir_all(old.join("content/posts")).unwrap();
    let settings = "baseURL = 'https://example.org/'\ntitle = 'Built'\nuglyURLs = true\n\
                    [pagination]\npagerSize = 1\n";
    fs::write(old.join("hugo.toml"), settings).unwrap();
    for n in 1..=2 {
        let post = format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\ntags: [a]\n---\nA post.\n");
        fs::write(old.join(format!("content/posts/post-{n}.md")), post).unwrap();
    }
    let built = [
        "index.html",
        "posts.html",
        "posts/post-1.html",
        "posts/post-2.html",
        "posts/page/1.html",
        "posts/page/2.html",
        "tags.html",
        "tags/a.html",
        "tags/a/page/1.html",
        "tags/a/page/2.html",
        // No later page of the new site's stands there.
        "posts/chapter/2.html",
    ];
    for page in built {
        let path = old.join("public").join(page);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "<p>A page.</p>\n").unwrap();
    }
    let (code, told) = import(&old, &new, None);
    assert_eq!(code, 0, "{told}");
    // Each old page, the page written for it, and where it sends readers.
    for (from, page, to) in [
        ("/posts/page/1.html", "posts-page-1.md", "/posts/"),
        ("/posts/page/2.html", "posts-page-2.md", "/posts/page/2/"),
        ("/tags/a/page/1.html", "tags-a-page-1.md", "/tags/a/"),
        ("/tags/a/page/2.html", "tags-a-page-2.md", "/tags/a/page/2/"),
    ] {
        let written = fs::read_to_string(new.join("content").join(page)).unwrap();
        assert_eq!(
            written,
            format!("---\npermalink: {from}\nredirect_to: {to}\n---\n")
        );
    }
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert!(
        report.contains(
            "4 pages at the top of `content/`, such as `content/posts-page-1.md`, redirect from \
             old paginated pages of lists, found in the old site's built output, each to the \
             list's paginated page with the same number"
        ),
        "{report}"
    );
    assert!(!report.contains("/page/2.html`"), "{report}");
    assert!(report.contains("`/posts/chapter/2.html`"), "{report}");
    assert!(!new.join("content/posts-chapter-2.md").exists());
    assert!(
        !report.contains("With the old site's built output"),
        "{report}"
    );
    build_into(&new, &out);
    for file in [
        "posts/page/2.html",
        "tags/a/page/2.html",
        "tags/a/page/2/index.html",
    ] {
        assert!(out.join(file).is_file(), "nothing at {file}");
    }
}

/// Without the old built output, a tag's page Hugo published under
/// `uglyURLs` as a file sends readers on, and the report says what
/// nothing answers: the page listing every tag, which the new site
/// doesn't have, among them.
#[test]
fn hugo_tags_published_as_files_are_kept_without_the_built_output() {
    let old = scratch("hugo-ugly-tags-old");
    let new = scratch("hugo-ugly-tags-new");
    let out = scratch("hugo-ugly-tags-out");
    fs::create_dir_all(old.join("content/posts")).unwrap();
    let settings = "baseURL = 'https://example.org/'\ntitle = 'Tags'\nuglyURLs = true\n\
                    [pagination]\npagerSize = 1\n";
    fs::write(old.join("hugo.toml"), settings).unwrap();
    for n in 1..=2 {
        let post = format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\ntags: [a]\n---\nA post.\n");
        fs::write(old.join(format!("content/posts/post-{n}.md")), post).unwrap();
    }
    let (code, told) = import(&old, &new, None);
    assert_eq!(code, 0, "{told}");
    assert!(!new.join("content/tags.md").exists());
    let tag = fs::read_to_string(new.join("content/tags-a.md")).unwrap();
    assert_eq!(
        tag,
        "---\npermalink: /tags/a.html\nredirect_to: /tags/a/\n---\n"
    );
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert!(
        report.contains(
            "Hugo before version 0.148 published a list under `uglyURLs` as a file, such as \
             `/posts.html`, which now redirects to the list, and its paginated pages as files too, \
             such as `/posts/page/2.html`; and every version published a tag's paginated pages \
             as files, such as `/tags/a/page/2.html`: nothing is published at those now. With \
             the old site's built output in `public/`, `statac import` redirects from each it \
             finds there."
        ),
        "{report}"
    );
    assert!(
        !report.contains("Nothing in the new site is published"),
        "{report}"
    );
    assert!(
        report.contains(
            "The old site published a page listing every one of its `tags` at `/tags/`, and the \
             new site has none."
        ),
        "{report}"
    );
    build_into(&new, &out);
    for file in ["tags/a.html", "posts.html"] {
        assert!(out.join(file).is_file(), "nothing at {file}");
    }
}

/// A tag's page `[permalinks]` put where Statac's isn't sends readers on
/// from the address the pattern gave it, and the report names a later
/// page beneath it that nothing answers.
#[test]
fn a_hugo_tag_moved_by_permalinks_sends_readers_on() {
    // The settings, the page written, the old address and the new, and
    // the later page the report names.
    for (at, (settings, page, from, to, later)) in [
        (
            "[permalinks]\ntags = '/tags/:slug/posts/'\n",
            "tags-a-posts.md",
            "/tags/a/posts/",
            "/tags/a/",
            Some("`/tags/a/posts/page/2/`"),
        ),
        (
            "uglyURLs = true\n[permalinks]\ntags = '/topics/:slug/'\n",
            "topics-a.md",
            "/topics/a.html",
            "/topics/a/",
            None,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let old = scratch(&format!("hugo-moved-tags-old-{at}"));
        let new = scratch(&format!("hugo-moved-tags-new-{at}"));
        let out = scratch(&format!("hugo-moved-tags-out-{at}"));
        fs::create_dir_all(old.join("content/posts")).unwrap();
        let settings = format!(
            "baseURL = 'https://example.org/'\ntitle = 'Tags'\n{settings}\
             [pagination]\npagerSize = 1\n"
        );
        fs::write(old.join("hugo.toml"), settings).unwrap();
        for n in 1..=2 {
            let post =
                format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\ntags: [a]\n---\nA post.\n");
            fs::write(old.join(format!("content/posts/post-{n}.md")), post).unwrap();
        }
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{at}: {told}");
        let written = fs::read_to_string(new.join("content").join(page)).unwrap();
        assert_eq!(
            written,
            format!("---\npermalink: {from}\nredirect_to: {to}\n---\n"),
            "{at}"
        );
        assert!(!new.join("content/tags-a.md").exists(), "{at}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert!(
            !report.contains("Nothing in the new site is published"),
            "{at}: {report}"
        );
        if let Some(later) = later {
            assert!(
                report.contains(&format!(
                    "so each is at Statac's own URL, and a page at the old one redirects to it. \
                     A tag's paginated pages were published beneath its list page, such as \
                     {later}: nothing is published at those now."
                )),
                "{at}: {report}"
            );
        }
        build_into(&new, &out);
        let file = match from.strip_suffix('/') {
            Some(folder) => format!("{}/index.html", &folder[1..]),
            None => from[1..].to_owned(),
        };
        assert!(out.join(&file).is_file(), "{at}: nothing at {file}");
    }
}

/// Each later page of a tag's page `[permalinks]` put where Statac's
/// isn't, in the old built output, sends readers on to the tag's page of
/// the same number.
#[test]
fn later_pages_of_a_moved_hugo_tag_in_the_built_output_send_readers_on() {
    let old = scratch("hugo-moved-later-old");
    let new = scratch("hugo-moved-later-new");
    let out = scratch("hugo-moved-later-out");
    fs::create_dir_all(old.join("content/posts")).unwrap();
    let settings = "baseURL = 'https://example.org/'\ntitle = 'Built'\n\
                    [pagination]\npagerSize = 1\n[permalinks]\ntags = '/tags/:slug/posts/'\n";
    fs::write(old.join("hugo.toml"), settings).unwrap();
    for n in 1..=2 {
        let post = format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\ntags: [a]\n---\nA post.\n");
        fs::write(old.join(format!("content/posts/post-{n}.md")), post).unwrap();
    }
    for page in [
        "index.html",
        "posts/index.html",
        "posts/post-1/index.html",
        "posts/post-2/index.html",
        "tags/index.html",
        "tags/a/posts/index.html",
        "tags/a/posts/page/1/index.html",
        "tags/a/posts/page/2/index.html",
    ] {
        let path = old.join("public").join(page);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "<p>A page.</p>\n").unwrap();
    }
    let (code, told) = import(&old, &new, None);
    assert_eq!(code, 0, "{told}");
    // Each old page, the page written for it, and where it sends readers.
    for (from, page, to) in [
        ("/tags/a/posts/", "tags-a-posts.md", "/tags/a/"),
        (
            "/tags/a/posts/page/1/",
            "tags-a-posts-page-1.md",
            "/tags/a/",
        ),
        (
            "/tags/a/posts/page/2/",
            "tags-a-posts-page-2.md",
            "/tags/a/page/2/",
        ),
    ] {
        let written = fs::read_to_string(new.join("content").join(page)).unwrap();
        assert_eq!(
            written,
            format!("---\npermalink: {from}\nredirect_to: {to}\n---\n")
        );
    }
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert!(!report.contains("`/tags/a/posts/page/2/`"), "{report}");
    assert!(!report.contains("A tag's paginated pages"), "{report}");
    build_into(&new, &out);
    for file in [
        "tags/a/posts/index.html",
        "tags/a/posts/page/1/index.html",
        "tags/a/posts/page/2/index.html",
        "tags/a/page/2/index.html",
    ] {
        assert!(out.join(file).is_file(), "nothing at {file}");
    }
}

/// A Hugo site whose posts are each in the series `a`, with `settings`
/// after the address and the title, showing one post to a page.
fn hugo_series_site(old: &Path, settings: &str) {
    fs::create_dir_all(old.join("content/posts")).unwrap();
    let settings = format!(
        "baseURL = 'https://example.org/'\ntitle = 'Series'\n{settings}\
         [taxonomies]\ntag = 'tags'\nseries = 'series'\n[pagination]\npagerSize = 1\n"
    );
    fs::write(old.join("hugo.toml"), settings).unwrap();
    for n in 1..=2 {
        let post = format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\nseries: [a]\n---\nA post.\n");
        fs::write(old.join(format!("content/posts/post-{n}.md")), post).unwrap();
    }
}

/// A series' page where Statac's isn't, by `[permalinks]` or under
/// `uglyURLs`, sends readers on from the old address, and the report
/// names a later page beneath it that nothing answers.
#[test]
fn a_hugo_series_page_where_statacs_isnt_sends_readers_on() {
    // The settings, the page written, the old address and the new, and
    // the later page the report names.
    for (at, (settings, page, from, to, later)) in [
        (
            "[permalinks]\nseries = '/series/:slug/posts/'\n",
            "series-a-posts.md",
            "/series/a/posts/",
            "/series/a/",
            Some("`/series/a/posts/page/2/`"),
        ),
        (
            "uglyURLs = true\n",
            "series-a.md",
            "/series/a.html",
            "/series/a/",
            None,
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let old = scratch(&format!("hugo-moved-series-old-{at}"));
        let new = scratch(&format!("hugo-moved-series-new-{at}"));
        let out = scratch(&format!("hugo-moved-series-out-{at}"));
        hugo_series_site(&old, settings);
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{at}: {told}");
        let written = fs::read_to_string(new.join("content").join(page)).unwrap();
        assert_eq!(
            written,
            format!("---\npermalink: {from}\nredirect_to: {to}\n---\n"),
            "{at}"
        );
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert!(!report.contains(&format!("- `{from}`")), "{at}: {report}");
        if let Some(later) = later {
            assert!(
                report.contains(&format!(
                    "so each is at Statac's own URL, and a page at the old one redirects to it. \
                     A series' paginated pages were published beneath its list page, such as \
                     {later}: nothing is published at those now."
                )),
                "{at}: {report}"
            );
        }
        build_into(&new, &out);
        let file = match from.strip_suffix('/') {
            Some(folder) => format!("{}/index.html", &folder[1..]),
            None => from[1..].to_owned(),
        };
        assert!(out.join(&file).is_file(), "{at}: nothing at {file}");
    }
}

/// Each later page of a series' page `[permalinks]` put where Statac's
/// isn't, in the old built output, sends readers on to the series' page
/// of the same number.
#[test]
fn later_pages_of_a_moved_hugo_series_in_the_built_output_send_readers_on() {
    let old = scratch("hugo-moved-series-later-old");
    let new = scratch("hugo-moved-series-later-new");
    let out = scratch("hugo-moved-series-later-out");
    hugo_series_site(&old, "[permalinks]\nseries = '/series/:slug/posts/'\n");
    for page in [
        "index.html",
        "posts/index.html",
        "posts/post-1/index.html",
        "posts/post-2/index.html",
        "series/a/posts/index.html",
        "series/a/posts/page/1/index.html",
        "series/a/posts/page/2/index.html",
    ] {
        let path = old.join("public").join(page);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "<p>A page.</p>\n").unwrap();
    }
    let (code, told) = import(&old, &new, None);
    assert_eq!(code, 0, "{told}");
    // Each old page, the page written for it, and where it sends readers.
    for (from, page, to) in [
        ("/series/a/posts/", "series-a-posts.md", "/series/a/"),
        (
            "/series/a/posts/page/1/",
            "series-a-posts-page-1.md",
            "/series/a/",
        ),
        (
            "/series/a/posts/page/2/",
            "series-a-posts-page-2.md",
            "/series/a/page/2/",
        ),
    ] {
        let written = fs::read_to_string(new.join("content").join(page)).unwrap();
        assert_eq!(
            written,
            format!("---\npermalink: {from}\nredirect_to: {to}\n---\n")
        );
    }
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert!(!report.contains("A series' paginated pages"), "{report}");
    build_into(&new, &out);
    for file in [
        "series/a/posts/index.html",
        "series/a/posts/page/1/index.html",
        "series/a/posts/page/2/index.html",
        "series/a/page/2/index.html",
    ] {
        assert!(out.join(file).is_file(), "nothing at {file}");
    }
}

/// Hugo's list held the pages in its folder, those in folders beneath it
/// that are no lists, and each list beneath it, and each of its later
/// pages is looked for in the new site.
#[test]
fn hugo_later_pages_count_what_hugo_listed() {
    let old = scratch("hugo-held-old");
    let new = scratch("hugo-held-new");
    fs::create_dir_all(old.join("content/posts/2026")).unwrap();
    fs::create_dir_all(old.join("content/posts/deep")).unwrap();
    let settings = "baseURL = 'https://example.org/'\ntitle = 'Held'\n\
                    [pagination]\npagerSize = 2\n";
    fs::write(old.join("hugo.toml"), settings).unwrap();
    let post = |n: u32| format!("---\ntitle: Post {n}\ndate: 2026-01-0{n}\n---\nA post.\n");
    fs::write(old.join("content/posts/post-1.md"), post(1)).unwrap();
    fs::write(old.join("content/posts/2026/post-2.md"), post(2)).unwrap();
    fs::write(
        old.join("content/posts/deep/_index.md"),
        "---\ntitle: Deep\n---\n",
    )
    .unwrap();
    fs::write(old.join("content/posts/deep/post-3.md"), post(3)).unwrap();
    let (code, told) = import(&old, &new, None);
    assert_eq!(code, 0, "{told}");
    let index = fs::read_to_string(new.join("content/posts/index.md")).unwrap();
    assert!(
        index.contains("\nredirect_from:\n  - /posts/page/1/\n"),
        "{index}"
    );
    let deep = fs::read_to_string(new.join("content/posts/deep/index.md")).unwrap();
    assert!(!deep.contains("redirect_from"), "{deep}");
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert!(
        !report.contains("Nothing in the new site is published"),
        "{report}"
    );
}

#[test]
fn hugo_site_under_a_path_keeps_its_addresses_and_links() {
    assert_eq!(fixture("hugo-under-path"), [BUILT, NO_FEED, AS_TEXT]);
}

#[test]
fn hugo_later_pages_are_spoken_of_wherever_the_old_site_could_have_had_them() {
    let lists = "Hugo paginated only the lists whose templates asked for it";
    let home = "Where the old home page was paginated, nothing is published at page 2 onwards";
    // How many posts, the settings, and whether later pages are spoken of.
    for (posts, settings, said) in [
        (10, "", false),
        (11, "", true),
        (2, "[pagination]\npagerSize = 5\n", true),
        (6, "[pagination]\npagerSize = 5\n", true),
    ] {
        let old = scratch(&format!("hugo-later-old-{posts}"));
        let new = scratch(&format!("hugo-later-new-{posts}"));
        fs::create_dir_all(old.join("content/posts")).unwrap();
        let top = "baseURL = 'https://example.org/'\ntitle = 'Later pages'\n";
        fs::write(old.join("hugo.toml"), format!("{top}{settings}")).unwrap();
        for n in 1..=posts {
            let post = format!("---\ntitle: Post {n}\ndate: 2026-01-{n:02}\n---\nA post.\n");
            fs::write(old.join(format!("content/posts/post-{n}.md")), post).unwrap();
        }
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{posts}: {told}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert_eq!(report.contains(lists), said, "{posts}: {report}");
        assert_eq!(report.contains(home), said, "{posts}: {report}");
    }
}

#[test]
fn hugo_feeds_keep_the_kinds_and_the_name_the_settings_gave_them() {
    let both = "Hugo's feeds were RSS, at `index.xml` beside the home page and each list page";
    let home = "Hugo's feed was RSS, at `feed.xml` beside the home page. It stays at the same \
                URL, as `feed: feed.xml` on the home page says, built with `templates/feed.xml` \
                once you add it";
    let lists = "Hugo's feeds were RSS, at `index.xml` beside each list page";
    // The settings, whether the old site has a home page of its own, the
    // `feed` of the new home page and of the posts' page, where each has
    // one, and what is said.
    let cases = [
        ("", true, Some("index.xml"), Some("index.xml"), &[both][..]),
        ("", false, Some("index.xml"), Some("index.xml"), &[both][..]),
        (
            "[outputs]\nhome = ['html', 'rss']\nsection = ['html']\n\
             [outputFormats.RSS]\nbaseName = 'feed'\n",
            true,
            Some("feed.xml"),
            None,
            &[home][..],
        ),
        (
            "[outputs]\nhome = ['HTML', 'RSS']\nsection = ['HTML']\n\
             [outputformats.rss]\nbasename = 'feed'\n",
            false,
            Some("feed.xml"),
            None,
            &[home][..],
        ),
        (
            "[outputs]\nhome = ['html']\n",
            true,
            None,
            Some("index.xml"),
            &[lists][..],
        ),
        ("disableKinds = ['rss']\n", true, None, None, &[][..]),
        (
            "[outputs]\nhome = ['html']\nsection = ['html']\n",
            false,
            None,
            None,
            &[][..],
        ),
    ];
    for (at, (settings, has_home, home_feed, posts_feed, said)) in cases.into_iter().enumerate() {
        let old = scratch(&format!("hugo-feeds-old-{at}"));
        let new = scratch(&format!("hugo-feeds-new-{at}"));
        fs::create_dir_all(old.join("content/posts")).unwrap();
        let top = "baseURL = 'https://example.org/'\ntitle = 'Feeds'\ntimeZone = 'Europe/London'\n";
        fs::write(old.join("hugo.toml"), format!("{top}{settings}")).unwrap();
        let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA post.\n";
        fs::write(old.join("content/posts/post-1.md"), post).unwrap();
        if has_home {
            fs::write(old.join("content/_index.md"), "---\ntitle: Home\n---\n").unwrap();
        }
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{settings}: {told}");
        let read = |path: &str| fs::read_to_string(new.join(path)).unwrap();
        let word = |path: &str, feed: Option<&str>| {
            let page = read(path);
            let lines: Vec<&str> = page.lines().filter(|l| l.starts_with("feed:")).collect();
            let wanted = feed.map(|feed| format!("feed: {feed}"));
            assert_eq!(
                lines,
                Vec::from_iter(wanted.as_deref()),
                "{settings}: {path}: {page}"
            );
        };
        word("content/index.md", home_feed);
        word("content/posts/index.md", posts_feed);
        let report = read("IMPORT.md");
        for sentence in said {
            assert!(report.contains(sentence), "{settings}: {report}");
        }
        assert!(!report.contains("`feed: false`"), "{settings}: {report}");
        assert!(!report.contains("`outputs"), "{settings}: {report}");
        assert!(!report.contains("`outputFormats"), "{settings}: {report}");

        templated(&new);
        let out = scratch(&format!("hugo-feeds-built-{at}"));
        let outcome = build::run(&Options {
            mode: Mode::Build,
            site: new.clone(),
            out: Some(out.clone()),
            today: Some(today()),
            drafts: false,
            url: None,
        });
        assert!(outcome.published.is_some(), "{settings}");
        let mut feeds: Vec<String> = (files(&out).into_keys())
            .filter(|path| path.ends_with(".xml") && path != "sitemap.xml")
            .collect();
        feeds.sort();
        let mut wanted = Vec::from_iter(home_feed.map(str::to_owned));
        wanted.extend(posts_feed.map(|feed| format!("posts/{feed}")));
        assert_eq!(feeds, wanted, "{settings}");
    }
}

#[test]
fn hugo_settings_for_formats_statac_has_no_equal_for_are_listed() {
    let old = scratch("hugo-feeds-left-old");
    let new = scratch("hugo-feeds-left-new");
    fs::create_dir_all(old.join("content/posts")).unwrap();
    let settings = "baseURL = 'https://example.org/'\ntitle = 'Feeds'\n\
                    [outputs]\nhome = ['html', 'json', 'rss']\npage = ['html']\n\
                    [outputFormats.rss]\nbaseName = 'feed'\nmediaType = 'application/rss'\n";
    fs::write(old.join("hugo.toml"), settings).unwrap();
    let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA post.\n";
    fs::write(old.join("content/posts/post-1.md"), post).unwrap();
    let (code, told) = import(&old, &new, None);
    assert_eq!(code, 0, "{told}");
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert!(report.contains("`outputs: {home: [json]}`"), "{report}");
    assert!(
        report.contains("`outputFormats: {rss: {mediaType: application/rss}}`"),
        "{report}"
    );
}

#[test]
fn a_hugo_feeds_name_that_isnt_a_plain_one_is_left_and_listed() {
    for name in ["../feed", "a/b", ".hidden", "fe`ed", "two words", ""] {
        let old = scratch("hugo-feeds-name-old");
        let new = scratch("hugo-feeds-name-new");
        fs::create_dir_all(old.join("content/posts")).unwrap();
        let settings = format!(
            "baseURL = 'https://example.org/'\ntitle = 'Feeds'\n\
             [outputFormats.rss]\nbaseName = '{name}'\n"
        );
        fs::write(old.join("hugo.toml"), settings).unwrap();
        let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA post.\n";
        fs::write(old.join("content/posts/post-1.md"), post).unwrap();
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{name}: {told}");
        let page = fs::read_to_string(new.join("content/posts/index.md")).unwrap();
        assert!(page.contains("\nfeed: index.xml\n"), "{name}: {page}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert!(
            report.contains("`outputFormats: {rss: {baseName: "),
            "{name}: {report}"
        );
    }
}

#[test]
fn hugo_tags_keep_the_addresses_hugo_gave_them() {
    assert_eq!(fixture("hugo-tags"), [BUILT, NO_FEED]);
}

#[test]
fn hugo_languages_come_across_by_folder_and_by_name() {
    assert_eq!(fixture("hugo-languages-folders"), [BUILT, NO_FEED]);
    assert_eq!(fixture("hugo-languages-names"), [BUILT, NO_FEED]);
}

#[test]
fn hugo_dates_come_across_from_names_and_front_matter() {
    assert_eq!(fixture("hugo-dates"), [BUILT, NO_FEED]);
}

#[test]
fn hugo_tags_in_the_writing_convert_or_stay_as_written() {
    assert_eq!(
        fixture("hugo-tags-in-writing"),
        [
            "broken-link",
            BUILT,
            "call-in-html",
            NO_FEED,
            "missing-definition",
            AS_TEXT
        ]
    );
}

#[test]
fn hugo_markdown_that_reads_differently_is_counted_not_rewritten() {
    assert_eq!(fixture("hugo-markdown-forms"), [BUILT, NO_FEED]);
}

#[test]
fn hugo_files_data_and_folders_left_come_across_or_are_listed() {
    assert_eq!(fixture("hugo-files-and-data"), [BUILT, NO_FEED]);
}

#[test]
fn a_hugo_site_is_told_by_its_settings() {
    let top = scratch("told");
    let site = |name: &str, files: &[(&str, &str)]| {
        let folder = top.join(name);
        for (path, text) in files {
            let path = folder.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        folder
    };
    for (name, files) in [
        ("hugo-toml", &[("hugo.toml", "title = \"Site 1\"\n")][..]),
        ("hugo-yaml", &[("hugo.yaml", "title: Site 1\n")]),
        ("hugo-yml", &[("hugo.yml", "title: Site 1\n")]),
        ("hugo-json", &[("hugo.json", "{\"title\": \"Site 1\"}\n")]),
        (
            "config-toml",
            &[("config.toml", "BaseURL = \"https://example.org/\"\n")],
        ),
        (
            "config-yaml",
            &[("config.yaml", "baseurl: https://example.org/\n")],
        ),
        ("config-json", &[("config.json", "{\"baseURL\": \"/\"}\n")]),
        ("split", &[("config/_default/params.toml", "a = 1\n")]),
    ] {
        let folder = site(name, files);
        assert!(Tool::Hugo.owns(&folder), "{name}");
        assert!(!Tool::Jekyll.owns(&folder), "{name}");
        assert!(!Tool::Zola.owns(&folder), "{name}");
    }
    let other = site(
        "other-tool",
        &[("config.toml", "base_url = \"https://example.org/\"\n")],
    );
    assert!(!Tool::Hugo.owns(&other));
    assert!(Tool::Zola.owns(&other));

    let both = site(
        "both",
        &[
            ("hugo.toml", "title = \"Site 1\"\n"),
            ("_config.yml", "title: Site 1\n"),
        ],
    );
    assert_eq!(
        refused(&both, &top.join("new-2"), None),
        format!(
            "error  Can't tell what made the site: `{}` has the settings files of more than one tool",
            both.display()
        )
    );
    let new = top.join("new-3");
    let (code, said) = import(&both, &new, Some(Tool::Hugo));
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Imported the Hugo site"), "{said}");

    let broken = site(
        "broken",
        &[("hugo.toml", "title = \"Site 1\"\nbaseURL = \n")],
    );
    assert_eq!(
        refused(&broken, &top.join("new-4"), None),
        format!(
            "error  Can't read `{}`: this line doesn't read as TOML",
            broken.join("hugo.toml").display()
        )
    );
    assert!(!top.join("new-4").exists());
}

const OUTSIDE: &str = "kept outside the old site";

/// An old site at `outside/old` under a new folder for `label`, with one
/// post, and files in `outside/` and `outside/inner/` that are no part of
/// it. Gives the new folder, the old site's folder and `outside/`.
fn beside_files_of_others(label: &str) -> (Scratch, PathBuf, PathBuf) {
    let outside_in = scratch(label);
    let outside = outside_in.join("outside");
    let old = outside.join("old");
    fs::create_dir_all(old.join("_posts")).unwrap();
    fs::write(
        old.join("_posts/2026-01-01-post-1.md"),
        "---\ntitle: Post 1\n---\nA post of the old site's own.\n",
    )
    .unwrap();
    for folder in [outside.clone(), outside.join("inner")] {
        fs::create_dir_all(folder.join("_posts")).unwrap();
        fs::write(folder.join("file-9.txt"), OUTSIDE).unwrap();
        fs::write(folder.join("data-9.yml"), format!("a: {OUTSIDE}\n")).unwrap();
        fs::write(folder.join("page-9.html"), format!("<p>{OUTSIDE}</p>\n")).unwrap();
        fs::write(
            folder.join("post-9.md"),
            format!("---\ntitle: Post 9\ndate: 2026-02-02\n---\n{OUTSIDE}\n"),
        )
        .unwrap();
        fs::write(
            folder.join("_posts/2026-02-02-post-9.md"),
            format!("---\ntitle: Post 9\n---\n{OUTSIDE}\n"),
        )
        .unwrap();
    }
    (outside_in, old, outside)
}

/// Imports `old`, whose settings are `settings`, and gives the report,
/// having seen that the old site's own post came across and nothing from
/// outside it did, by name or by what it holds.
fn nothing_from_outside(old: &Path, settings: &str) -> String {
    nothing_from_outside_of(old, "_config.yml", settings)
}

/// [`nothing_from_outside`], the settings being in the file `named`.
fn nothing_from_outside_of(old: &Path, named: &str, settings: &str) -> String {
    fs::write(old.join(named), settings).unwrap();
    let new = old.parent().unwrap().parent().unwrap().join("new");
    let (code, said) = import(old, &new, None);
    assert_eq!(code, 0, "{settings}: {said}");
    let made = files(&new);
    assert!(made.contains_key("content/posts/post-1.md"), "{settings}");
    for (path, bytes) in &made {
        let text = String::from_utf8_lossy(bytes);
        assert!(!path.contains("-9"), "{settings}: `{path}` was written");
        assert!(!text.contains(OUTSIDE), "{settings}: `{path}`:\n{text}");
        assert!(
            !text.contains("post-9") && !text.contains("file-9"),
            "{settings}: `{path}`:\n{text}"
        );
    }
    let report = String::from_utf8(made["IMPORT.md"].clone()).unwrap();
    fs::remove_dir_all(&new).unwrap();
    report
}

#[cfg(unix)]
const LEADS_OUTSIDE: &str = "a link that leads outside the old site, which Statac doesn't follow";
#[cfg(unix)]
const LEADS_BACK: &str =
    "a link that leads back to a folder it is inside, which Statac doesn't follow";

/// A page of the old site's own at `path`.
#[cfg(unix)]
fn page_at(old: &Path, path: &str, title: &str) {
    fs::create_dir_all(old.join(path).parent().unwrap()).unwrap();
    let text = format!("---\ntitle: {title}\n---\nA page of the old site's own.\n");
    fs::write(old.join(path), text).unwrap();
}

#[cfg(unix)]
#[test]
fn a_link_that_leads_inside_the_old_site_is_followed_where_it_stands() {
    use std::os::unix::fs::symlink;
    let (_folder, old, _) = beside_files_of_others("links-inside");
    page_at(&old, "store/page-2.md", "Page 2");
    symlink("store", old.join("linked")).unwrap();
    symlink("store/page-2.md", old.join("page-3.md")).unwrap();
    fs::write(old.join("_config.yml"), "title: Site 1\n").unwrap();
    let new = old.parent().unwrap().parent().unwrap().join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    let made = files(&new);
    for path in [
        "content/store/page-2.md",
        "content/linked/page-2.md",
        "content/page-3.md",
    ] {
        let text = String::from_utf8_lossy(&made[path]);
        assert!(text.contains("title: Page 2"), "{path}: {text}");
    }
    let report = String::from_utf8_lossy(&made["IMPORT.md"]);
    assert!(
        report.contains(
            "Statac followed 2 links, such as `linked`, which lead to somewhere inside the old \
             site, and read what each leads to as standing where the link is.\n"
        ),
        "{report}"
    );
    assert!(!report.contains("Links Statac didn't follow"), "{report}");
}

#[cfg(unix)]
#[test]
fn a_link_that_leads_outside_back_or_nowhere_is_never_followed() {
    use std::os::unix::fs::symlink;
    let (_folder, old, outside) = beside_files_of_others("links-refused");
    // A folder beside the old one whose name begins with its name.
    let beside = outside.join("old-2");
    fs::create_dir_all(&beside).unwrap();
    fs::write(
        beside.join("page-8.md"),
        format!("---\ntitle: Page 8\n---\n{OUTSIDE}\n"),
    )
    .unwrap();
    page_at(&old, "store/page-2.md", "Page 2");
    page_at(&old, "a/page-4.md", "Page 4");
    page_at(&old, "b/page-5.md", "Page 5");
    symlink("store", old.join("linked")).unwrap();
    // Inside a folder a link leads to, one that leads outside.
    symlink(&outside, old.join("store/out")).unwrap();
    symlink("../old-2", old.join("beside")).unwrap();
    symlink("..", old.join("store/up")).unwrap();
    symlink("../b", old.join("a/x")).unwrap();
    symlink("../a", old.join("b/y")).unwrap();
    symlink("nowhere", old.join("gone")).unwrap();
    let report = nothing_from_outside(&old, "title: Site 1\n");
    assert!(!report.contains("Page 8"), "{report}");
    for (link, why) in [
        ("store/out", LEADS_OUTSIDE),
        ("linked/out", LEADS_OUTSIDE),
        ("beside", LEADS_OUTSIDE),
        ("store/up", LEADS_BACK),
        ("linked/up", LEADS_BACK),
        ("a/x/y", LEADS_BACK),
        ("b/y/x", LEADS_BACK),
        ("gone", "a link to something that isn't there"),
    ] {
        let line = format!("- `{link}`: {why}\n");
        let listed = report.split("### Links Statac didn't follow").nth(1);
        assert!(
            listed.is_some_and(|listed| listed.contains(&line)),
            "{line}{report}"
        );
    }
    assert!(
        report.contains("Statac followed 3 links, such as `a/x`,"),
        "{report}"
    );
}

#[cfg(unix)]
#[test]
fn a_folder_the_settings_name_is_read_through_a_link_that_leads_inside() {
    use std::os::unix::fs::symlink;
    let (_folder, old, _) = beside_files_of_others("links-setting");
    fs::create_dir_all(old.join("kept")).unwrap();
    fs::rename(old.join("_posts"), old.join("kept/_posts")).unwrap();
    symlink("kept", old.join("linked")).unwrap();
    let report = nothing_from_outside(&old, "title: Site 1\nsource: linked\n");
    assert!(!report.contains("Settings naming a folder"), "{report}");
    assert!(
        report.contains("Statac followed 1 link, `linked`, which leads"),
        "{report}"
    );
}

#[cfg(unix)]
const UNPUBLISHED: &str =
    "a link that leads to a place the old site didn't publish, which Statac doesn't follow";

/// Makes an old site of `files`, with a file and a page holding the
/// marker in each of `kept_back`, places the tool reads without
/// publishing or doesn't read, and imports it. Then puts a link to each,
/// and to a hidden file, in the folder `links_in`, where the tool
/// publishes what it finds, imports it again, and sees that the links
/// brought nothing into the new site, and that each is listed as leading
/// to what wasn't published. One more link, there both times, leads to
/// `published`, a folder the tool does publish, which holds a hidden
/// file with the marker: the folder comes across at the link, as
/// `comes_across` in the new site, and the hidden file doesn't.
#[cfg(unix)]
fn nothing_kept_back_is_published_through_a_link(
    label: &str,
    files: &[(&str, &str)],
    kept_back: &[&str],
    links_in: &str,
    published: &str,
    comes_across: &str,
) {
    use std::os::unix::fs::symlink;
    let top = scratch(label);
    let old = top.join("old");
    let write = |path: &str, text: &str| {
        fs::create_dir_all(old.join(path).parent().unwrap()).unwrap();
        fs::write(old.join(path), text).unwrap();
    };
    for (path, text) in files {
        write(path, text);
    }
    let page = format!("---\ntitle: Page 7\ndate: 2026-03-03\n---\n{OUTSIDE}\n");
    write(".env", OUTSIDE);
    write(&format!("{published}/.hidden-7.txt"), OUTSIDE);
    for place in kept_back {
        write(&format!("{place}/file-7.txt"), OUTSIDE);
        write(&format!("{place}/page-7.md"), &page);
        write(&format!("{place}/inner/page-7.html"), &page);
    }
    fs::create_dir_all(old.join(links_in)).unwrap();
    let up = "../".repeat(links_in.split('/').count());
    let link = |to: &str, name: &str| {
        symlink(format!("{up}{to}"), old.join(links_in).join(name)).unwrap();
    };
    link(published, "linked");
    let (code, said) = import(&old, &top.join("new-1"), None);
    assert_eq!(code, 0, "{said}");
    let without = files_of(&top.join("new-1"));
    link(".env", "env.txt");
    for (n, place) in kept_back.iter().enumerate() {
        link(place, &format!("link-{n}"));
    }
    let (code, said) = import(&old, &top.join("new-2"), None);
    assert_eq!(code, 0, "{said}");
    let made = files_of(&top.join("new-2"));
    // The report differs, and the key made fresh for the feed's ids.
    let but_those = |made: &BTreeMap<String, Vec<u8>>| -> Vec<(String, usize)> {
        (made.iter())
            .filter(|(path, _)| !matches!(path.as_str(), "IMPORT.md" | "statac.yaml"))
            .map(|(path, bytes)| (path.clone(), bytes.len()))
            .collect()
    };
    assert_eq!(but_those(&made), but_those(&without), "{label}");
    for (path, bytes) in &made {
        let through =
            path.contains("link-") || path.contains("env.txt") || path.contains("/linked/.");
        assert!(!through, "{label}: `{path}` was written");
        let text = String::from_utf8_lossy(bytes);
        let own = path != "IMPORT.md" && without.get(path) == Some(bytes);
        assert!(own || !text.contains(OUTSIDE), "{label}: `{path}`:\n{text}");
    }
    assert!(
        made.contains_key(comes_across),
        "{label}: {:?}",
        made.keys()
    );
    let report = String::from_utf8_lossy(&made["IMPORT.md"]);
    let listed = report.split("### Links Statac didn't follow").nth(1);
    for name in (0..kept_back.len())
        .map(|n| format!("link-{n}"))
        .chain(["env.txt".to_owned()])
    {
        let line = format!("- `{links_in}/{name}`: {UNPUBLISHED}\n");
        assert!(
            listed.is_some_and(|listed| listed.contains(&line)),
            "{label}: {line}{report}"
        );
    }
}

#[cfg(unix)]
fn files_of(folder: &Path) -> BTreeMap<String, Vec<u8>> {
    files(folder)
}

#[cfg(unix)]
#[test]
fn a_link_never_publishes_what_jekyll_kept_back() {
    nothing_kept_back_is_published_through_a_link(
        "kept-back-jekyll",
        &[
            ("_config.yml", "title: Site 1\nexclude: [private]\n"),
            (
                "_posts/2026-01-01-post-1.md",
                "---\ntitle: Post 1\n---\nA post.\n",
            ),
            ("kept/page-2.md", "---\ntitle: Page 2\n---\nA page.\n"),
        ],
        &[
            ".git",
            "node_modules/package-1",
            "_drafts",
            "_data",
            "_includes",
            "_layouts",
            "private",
            "_site",
        ],
        "assets",
        "kept",
        "content/assets/linked/page-2.md",
    );
}

#[cfg(unix)]
#[test]
fn a_link_never_publishes_what_hugo_kept_back() {
    nothing_kept_back_is_published_through_a_link(
        "kept-back-hugo",
        &[
            (
                "hugo.toml",
                "baseURL = \"https://example.org/\"\ntitle = \"Site 1\"\n",
            ),
            (
                "content/posts/post-1.md",
                "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA post.\n",
            ),
            ("static/kept/file-2.txt", "A file.\n"),
        ],
        &[
            ".git",
            "node_modules/package-1",
            "data",
            "layouts",
            "archetypes",
            "themes/theme-1",
            "public",
        ],
        "static/files",
        "static/kept",
        "static/files/linked/file-2.txt",
    );
}

#[cfg(unix)]
#[test]
fn a_link_never_publishes_what_zola_kept_back() {
    nothing_kept_back_is_published_through_a_link(
        "kept-back-zola",
        &[
            (
                "config.toml",
                "base_url = \"https://example.org\"\ntitle = \"Site 1\"\nignored_content = [\"private/*\"]\n",
            ),
            (
                "content/post-1.md",
                "+++\ntitle = \"Post 1\"\ndate = 2026-01-01\n+++\nA post.\n",
            ),
            ("static/kept/file-2.txt", "A file.\n"),
        ],
        &[
            ".git",
            "node_modules/package-1",
            "templates",
            "sass",
            "themes/theme-1",
            "content/private",
            "public",
        ],
        "static/files",
        "static/kept",
        "static/files/linked/file-2.txt",
    );
}

#[cfg(unix)]
#[test]
fn a_link_never_publishes_what_eleventy_kept_back() {
    nothing_kept_back_is_published_through_a_link(
        "kept-back-eleventy",
        &[
            (
                ".eleventy.js",
                "module.exports = function (eleventyConfig) {\n  eleventyConfig.addPassthroughCopy(\"files\");\n};\n",
            ),
            (".eleventyignore", "private/\n"),
            (
                "posts/post-1.md",
                "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA post.\n",
            ),
            ("kept/page-2.md", "---\ntitle: Page 2\n---\nA page.\n"),
        ],
        &[
            ".git",
            "node_modules/package-1",
            "_data",
            "_includes",
            "private",
            "_site",
        ],
        "files",
        "kept",
        "static/files/linked/page-2.md",
    );
}

#[cfg(unix)]
#[test]
fn an_eleventy_folder_outside_the_input_folder_is_read_through_a_link_into_it() {
    use std::os::unix::fs::symlink;
    let top = scratch("links-beyond");
    let old = top.join("old");
    let write = |path: &str, text: &str| {
        fs::create_dir_all(old.join(path).parent().unwrap()).unwrap();
        fs::write(old.join(path), text).unwrap();
    };
    let settings = "module.exports = function () {\n  return { dir: { input: \"src\" } };\n};\n";
    write(".eleventy.js", settings);
    write(".eleventyignore", "notes/private/\n");
    write("src/index.md", "---\ntitle: Home\n---\nThe home page.\n");
    write(
        "notes/note-1.md",
        "---\ntitle: Note 1\n---\nA note kept beside the input folder.\n",
    );
    let kept_back = format!("---\ntitle: Page 7\n---\n{OUTSIDE}\n");
    write("notes/private/page-7.md", &kept_back);
    write("notes/.hidden/page-7.md", &kept_back);
    write("notes/node_modules/package-1/page-7.md", &kept_back);
    symlink("../notes", old.join("src/notes")).unwrap();
    let new = top.join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    let made = files(&new);
    assert!(
        made.contains_key("content/notes/note-1.md"),
        "{:?}",
        made.keys()
    );
    for (path, bytes) in &made {
        let text = String::from_utf8_lossy(bytes);
        assert!(
            !text.contains(OUTSIDE) && !text.contains("Page 7"),
            "`{path}`:\n{text}"
        );
    }
    let report = String::from_utf8_lossy(&made["IMPORT.md"]);
    assert!(
        report.contains("Statac followed 1 link, `src/notes`,"),
        "{report}"
    );
}

#[cfg(unix)]
#[test]
fn only_so_many_links_are_followed_in_one_import() {
    use std::os::unix::fs::symlink;
    let (_folder, old, _) = beside_files_of_others("links-many");
    page_at(&old, "store/page-2.md", "Page 2");
    for n in 0..70 {
        symlink("store", old.join(format!("linked-{n:02}"))).unwrap();
    }
    fs::write(old.join("_config.yml"), "title: Site 1\n").unwrap();
    let new = old.parent().unwrap().parent().unwrap().join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    let made = files(&new);
    let report = String::from_utf8_lossy(&made["IMPORT.md"]);
    assert!(report.contains("Statac followed 64 links,"), "{report}");
    let past = ": a link past the number Statac follows in one site\n";
    assert_eq!(report.matches(past).count(), 6, "{report}");
    assert!(made.contains_key("content/linked-63/page-2.md"));
    assert!(!made.contains_key("content/linked-64/page-2.md"));
}

#[cfg(unix)]
#[test]
fn an_old_site_given_as_a_link_is_read() {
    use std::os::unix::fs::symlink;
    let (_folder, old, outside) = beside_files_of_others("links-old");
    let linked = outside.parent().unwrap().join("old-linked");
    symlink(&old, &linked).unwrap();
    fs::write(old.join("_config.yml"), "title: Site 1\n").unwrap();
    let new = outside.parent().unwrap().join("new");
    let (code, said) = import(&linked, &new, None);
    assert_eq!(code, 0, "{said}");
    let made = files(&new);
    assert!(made.contains_key("content/posts/post-1.md"), "{said}");
    let report = String::from_utf8_lossy(&made["IMPORT.md"]);
    assert!(!report.contains("Statac followed"), "{report}");
}

const FOLDER_UNREAD: &str = "### Settings naming a folder Statac doesn't read\n\
     \n\
     Each of these settings names a folder outside the old site's own folder, or one reached \
     through a link that leads outside it. Statac reads nothing outside the old site, so each was \
     read as if it weren't set: bring across by hand what the folder holds, if it is wanted.\n\
     \n";

const FOLDER_SETTINGS: [&str; 7] = [
    "source",
    "destination",
    "collections_dir",
    "data_dir",
    "layouts_dir",
    "includes_dir",
    "plugins_dir",
];

#[test]
fn a_folder_the_settings_name_outside_the_old_site_is_never_read() {
    let (_folder, old, outside) = beside_files_of_others("outside");
    fs::create_dir_all(old.join("a")).unwrap();
    let absolute = outside.to_string_lossy().replace('\\', "/");
    for setting in FOLDER_SETTINGS {
        for folder in ["..", "../inner", "a/../..", "a/../../inner", &absolute, "/"] {
            let line = format!("{setting}: {folder}");
            let report = nothing_from_outside(&old, &format!("title: Site 1\n{line}\n"));
            let listed = format!("{FOLDER_UNREAD}- `_config.yml`, line 2: `{line}`\n");
            assert!(report.contains(&listed), "{line}: {report}");
        }
    }
}

#[cfg(unix)]
#[test]
fn a_folder_the_settings_name_through_a_link_is_never_read() {
    use std::os::unix::fs::symlink;
    let (_folder, old, outside) = beside_files_of_others("linked");
    symlink(&outside, old.join("linked")).unwrap();
    for setting in FOLDER_SETTINGS {
        for folder in ["linked", "linked/inner", "./linked/"] {
            let line = format!("{setting}: {folder}");
            let report = nothing_from_outside(&old, &format!("title: Site 1\n{line}\n"));
            let listed = format!("{FOLDER_UNREAD}- `_config.yml`, line 2: `{line}`\n");
            assert!(report.contains(&listed), "{line}: {report}");
        }
    }
    fs::remove_file(old.join("linked")).unwrap();
    symlink(&outside, old.join("_site")).unwrap();
    let report = nothing_from_outside(&old, "title: Site 1\n");
    assert!(!report.contains("built output, `_site/`"), "{report}");
    assert!(
        report.contains(
            "- `_site`: a link that leads outside the old site, which Statac doesn't follow.\n"
        ),
        "{report}"
    );
    assert!(
        report.contains(
            "### Links Statac didn't follow\n\
             \n\
             Statac follows a link in the old site only to something inside it that the old \
             site published. Each of these it didn't follow, for the reason beside it, so what \
             the old site read through it didn't come across: put the folder or file itself \
             where the link is, in a copy of the old site, and import again.\n\
             \n\
             - `_site`: a link that leads outside the old site, which Statac doesn't follow\n"
        ),
        "{report}"
    );
}

#[cfg(unix)]
#[test]
fn a_settings_file_that_is_a_link_isnt_read() {
    use std::os::unix::fs::symlink;
    let (_folder, old, outside) = beside_files_of_others("linked-settings");
    fs::write(
        outside.join("settings-9.yml"),
        format!("title: {OUTSIDE}\n"),
    )
    .unwrap();
    symlink(outside.join("settings-9.yml"), old.join("_config.yml")).unwrap();
    assert!(!Tool::Jekyll.owns(&old));
    let new = outside.parent().unwrap().join("new");
    let (code, said) = import(&old, &new, Some(Tool::Jekyll));
    assert_eq!(code, 0, "{said}");
    for (path, bytes) in files(&new) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(OUTSIDE), "`{path}`:\n{text}");
    }
}

#[test]
fn an_include_or_a_link_named_outside_the_old_site_is_never_read() {
    let (_folder, old, outside) = beside_files_of_others("include");
    fs::write(old.join("_config.yml"), "title: Site 1\n").unwrap();
    fs::write(
        old.join("_posts/2026-01-02-post-2.md"),
        "---\ntitle: Post 2\n---\n{% include ../../file-9.txt %}\n\n\
         {% include ../page-9.html a=\"1\" %}\n\n\
         [A file]({% link ../file-9.txt %}), \
         [a post]({% post_url ../_posts/2026-02-02-post-9 %}).\n",
    )
    .unwrap();
    let before = files(&outside);
    let new = outside.parent().unwrap().join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    let made = files(&new);
    for (path, bytes) in &made {
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains(OUTSIDE), "`{path}`:\n{text}");
    }
    let snippets: Vec<&str> = (made.keys())
        .filter(|path| path.starts_with("snippets/") && path.contains("_9"))
        .map(String::as_str)
        .collect();
    assert_eq!(
        snippets,
        [
            "snippets/include_______file_9.html",
            "snippets/include____page_9.html"
        ]
    );
    assert_eq!(files(&outside), before);
    let beside = fs::read_dir(outside.parent().unwrap()).unwrap().count();
    assert_eq!(beside, 2);
}

#[test]
fn every_place_of_a_kind_is_listed_however_many() {
    let top = scratch("every-place");
    let old = top.join("old");
    fs::create_dir_all(old.join("_posts")).unwrap();
    fs::write(old.join("_config.yml"), "title: Site 1\n").unwrap();
    let lines: String = (1..=60)
        .map(|n| format!("Line {n}.\n{{: .class-{n}}}\n\n"))
        .collect();
    fs::write(
        old.join("_posts/2026-01-01-post-1.md"),
        format!("---\ntitle: Post 1\n---\n{lines}"),
    )
    .unwrap();
    let new = top.join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    for n in 1..=60 {
        let line = 3 + 3 * n - 1;
        let listed = format!("- `_posts/2026-01-01-post-1.md`, line {line}: `{{: .class-{n}}}`\n");
        assert!(report.contains(&listed), "{listed}{report}");
    }
    assert!(!report.contains(" more.\n"), "{report}");
}

const HUGO_FOLDER_SETTINGS: [&str; 10] = [
    "contentDir",
    "staticDir",
    "publishDir",
    "dataDir",
    "layoutDir",
    "assetDir",
    "i18nDir",
    "archetypeDir",
    "themesDir",
    "resourceDir",
];

/// A Hugo site with one post, beside files that are no part of it, as
/// [`beside_files_of_others`] makes them.
fn hugo_beside_files_of_others(label: &str) -> (Scratch, PathBuf, PathBuf) {
    let (folder, old, outside) = beside_files_of_others(label);
    fs::remove_dir_all(old.join("_posts")).unwrap();
    fs::create_dir_all(old.join("content/posts")).unwrap();
    fs::write(
        old.join("content/posts/post-1.md"),
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA post of the old site's own.\n",
    )
    .unwrap();
    (folder, old, outside)
}

#[test]
fn hugo_folders_the_settings_name_outside_the_old_site_are_never_read() {
    let (_folder, old, outside) = hugo_beside_files_of_others("hugo-outside");
    fs::create_dir_all(old.join("a")).unwrap();
    let absolute = outside.to_string_lossy().replace('\\', "/");
    let base = "baseURL: https://example.org/\n";
    for setting in HUGO_FOLDER_SETTINGS {
        for folder in ["..", "../inner", "a/../..", &absolute, "/"] {
            let line = format!("{setting}: {folder}");
            let report = nothing_from_outside_of(&old, "hugo.yaml", &format!("{base}{line}\n"));
            let listed = format!("{FOLDER_UNREAD}- `hugo.yaml`, line 2: `{line}`\n");
            assert!(report.contains(&listed), "{line}: {report}");
        }
    }
    let settings = format!("{base}languages:\n  en:\n    contentDir: ..\n");
    let report = nothing_from_outside_of(&old, "hugo.yaml", &settings);
    let listed = format!("{FOLDER_UNREAD}- `hugo.yaml`, line 2: `languages.en.contentDir: ..`\n");
    assert!(report.contains(&listed), "{report}");
}

#[cfg(unix)]
#[test]
fn hugo_folders_the_settings_name_through_a_link_are_never_read() {
    use std::os::unix::fs::symlink;
    let (_folder, old, outside) = hugo_beside_files_of_others("hugo-linked");
    symlink(&outside, old.join("linked")).unwrap();
    let base = "baseURL: https://example.org/\n";
    for setting in HUGO_FOLDER_SETTINGS {
        for folder in ["linked", "linked/inner"] {
            let line = format!("{setting}: {folder}");
            let report = nothing_from_outside_of(&old, "hugo.yaml", &format!("{base}{line}\n"));
            let listed = format!("{FOLDER_UNREAD}- `hugo.yaml`, line 2: `{line}`\n");
            assert!(report.contains(&listed), "{line}: {report}");
        }
    }
    fs::remove_file(old.join("linked")).unwrap();
    // The folders Hugo reads by their own names, and its folder of
    // settings, each a link.
    fs::create_dir_all(outside.join("config/_default")).unwrap();
    fs::write(
        outside.join("config/_default/params.yaml"),
        format!("author: {OUTSIDE}\n"),
    )
    .unwrap();
    for name in ["static", "data", "public", "layouts", "themes", "config"] {
        let to = match name {
            "config" => outside.join("config"),
            _ => outside.clone(),
        };
        symlink(to, old.join(name)).unwrap();
        let report = nothing_from_outside_of(&old, "hugo.yaml", base);
        let listed = format!(
            "- `{name}`: a link that leads outside the old site, which Statac doesn't follow.\n"
        );
        assert!(report.contains(&listed), "{name}: {report}");
        fs::remove_file(old.join(name)).unwrap();
    }
    fs::rename(old.join("content"), old.join("kept")).unwrap();
    symlink(&outside, old.join("content")).unwrap();
    fs::write(old.join("hugo.yaml"), base).unwrap();
    let new = outside.parent().unwrap().join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    for (path, bytes) in files(&new) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(!path.contains("-9"), "`{path}` was written");
        assert!(!text.contains(OUTSIDE), "`{path}`:\n{text}");
    }
}

#[test]
fn a_picture_a_hugo_tag_names_outside_the_old_site_is_never_looked_for() {
    let (_folder, old, outside) = hugo_beside_files_of_others("hugo-figure");
    fs::write(outside.join("photo-9.jpg"), OUTSIDE).unwrap();
    fs::write(old.join("hugo.yaml"), "baseURL: https://example.org/\n").unwrap();
    fs::write(
        old.join("content/posts/post-1.md"),
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\n\
         {{< figure src=\"/../../photo-9.jpg\" >}}\n\n{{< figure src=\"/../photo-1.jpg\" >}}\n",
    )
    .unwrap();
    let new = outside.parent().unwrap().join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    // Each is a call as written, of a snippet the site supplies.
    let post = fs::read_to_string(new.join("content/posts/post-1.md")).unwrap();
    assert!(post.contains("@figure(src='/../../photo-9.jpg')"), "{post}");
    for (path, bytes) in files(&new) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(OUTSIDE), "`{path}`:\n{text}");
    }
}

#[cfg(unix)]
#[test]
fn an_old_address_that_cant_be_kept_is_told_of_once_though_the_built_output_holds_it() {
    let top = scratch("unkept-built");
    let old = top.join("old");
    fs::create_dir_all(old.join("content/posts")).unwrap();
    fs::create_dir_all(old.join("public/old/post-1-??")).unwrap();
    fs::create_dir_all(old.join("public/posts/post-1")).unwrap();
    fs::write(old.join("hugo.yaml"), "baseURL: https://example.org/\n").unwrap();
    fs::write(
        old.join("content/posts/post-1.md"),
        "---\ntitle: Post 1\ndate: 2026-01-01\naliases:\n  - /old/post-1-??/\n---\nA post.\n",
    )
    .unwrap();
    fs::write(old.join("public/old/post-1-??/index.html"), "<p>Old.</p>\n").unwrap();
    fs::write(
        old.join("public/posts/post-1/index.html"),
        "<p>A post.</p>\n",
    )
    .unwrap();
    let new = top.join("new");
    let (code, said) = import(&old, &new, None);
    assert_eq!(code, 0, "{said}");
    let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
    assert_eq!(report.matches("`/old/post-1-??/`").count(), 1, "{report}");
    assert!(
        report.contains(
            "This old URL can't be kept: the new site can't redirect from it, so it was left \
             out of its page's `redirect_from`.\n\
             \n\
             - `/old/post-1-??/`, for `content/posts/post-1.md`: `?` can't be in a name\n"
        ),
        "{report}"
    );
}

#[test]
fn zola_settings_are_read_from_zola_toml_or_config_toml() {
    assert_eq!(fixture("zola-settings"), [BUILT]);
    assert_eq!(fixture("zola-config-toml"), [BUILT, NO_FEED]);
}

#[test]
fn zola_front_matter_comes_across_in_statacs_words() {
    assert_eq!(fixture("zola-front-matter"), [BUILT, DRAFT]);
}

#[test]
fn zola_lists_keep_their_order_pages_and_redirects() {
    assert_eq!(fixture("zola-lists"), [BUILT]);
}

#[test]
fn zola_lists_answer_the_first_page_zola_sent_on_to_them() {
    assert_eq!(fixture("zola-first-pages"), [BUILT]);
}

/// Zola names a list's address by its folder as written, so a list in a
/// folder Statac's address spells otherwise stays at Zola's, its later
/// pages and its feed beside it, and nothing sends readers on from it; a
/// list at its folder's address is left there.
#[test]
fn a_zola_list_stays_at_the_address_zola_gave_it() {
    for (at, (folder, permalink)) in [("Folder-1", Some("/Folder-1/")), ("folder-1", None)]
        .into_iter()
        .enumerate()
    {
        let old = scratch(&format!("zola-list-old-{at}"));
        let new = scratch(&format!("zola-list-new-{at}"));
        let out = scratch(&format!("zola-list-out-{at}"));
        fs::create_dir_all(old.join(format!("content/{folder}"))).unwrap();
        let settings = "base_url = \"https://example.org\"\ntitle = \"Lists\"\n";
        fs::write(old.join("config.toml"), settings).unwrap();
        let list = "+++\ntitle = \"Folder 1\"\nsort_by = \"date\"\npaginate_by = 1\n\
                    generate_feeds = true\n+++\n";
        fs::write(old.join(format!("content/{folder}/_index.md")), list).unwrap();
        for n in 1..=2 {
            let post = format!("+++\ntitle = \"Post {n}\"\ndate = 2026-01-0{n}\n+++\nA post.\n");
            fs::write(old.join(format!("content/{folder}/post-{n}.md")), post).unwrap();
        }
        let (code, told) = import(&old, &new, None);
        assert_eq!(code, 0, "{at}: {told}");
        let index = fs::read_to_string(new.join(format!("content/{folder}/index.md"))).unwrap();
        match permalink {
            Some(permalink) => {
                assert!(
                    index.contains(&format!("\npermalink: {permalink}\n")),
                    "{at}: {index}"
                )
            }
            None => assert!(!index.contains("permalink:"), "{at}: {index}"),
        }
        // Zola's first later page sends readers on to the list itself.
        assert!(
            !index.contains(&format!("  - /{folder}/\n")),
            "{at}: {index}"
        );
        assert!(
            index.contains(&format!("redirect_from:\n  - /{folder}/page/1/\n")),
            "{at}: {index}"
        );
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert!(
            !report.contains("Nothing in the new site is published"),
            "{at}: {report}"
        );
        build_into(&new, &out);
        // A disk may not tell `Folder-1` from `folder-1`, so the names are
        // read as written.
        let names: Vec<String> = (fs::read_dir(&out).unwrap())
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&folder.to_owned()), "{at}: {names:?}");
        for file in ["index.html", "page/2/index.html", "atom.xml"] {
            let path = out.join(folder).join(file);
            assert!(path.is_file(), "{at}: nothing at {folder}/{file}");
        }
    }
}

#[test]
fn zola_pages_keep_their_names_dates_and_addresses() {
    assert_eq!(fixture("zola-pages"), [BUILT]);
}

#[test]
fn zola_names_are_made_ready_by_each_slugify_setting() {
    assert_eq!(fixture("zola-slugify-on"), [BUILT]);
    assert_eq!(fixture("zola-slugify-safe"), [BUILT]);
    assert_eq!(fixture("zola-slugify-off"), [BUILT]);
}

#[test]
fn zola_tags_keep_the_addresses_zola_gave_them() {
    assert_eq!(fixture("zola-tags"), [BUILT]);
}

#[test]
fn zola_languages_come_across_by_place() {
    assert_eq!(fixture("zola-languages"), [BUILT, NO_FEED]);
}

#[test]
fn zola_tags_in_the_writing_convert_or_stay_as_written() {
    assert_eq!(
        fixture("zola-tags-in-writing"),
        ["broken-link", BUILT, DRAFT, AS_TEXT]
    );
}

/// Zola 0.23's components convert as the tags they replaced do, a
/// component the site defines being its own; one handing what only a
/// template can work out, or named with a `.`, stays as written.
#[test]
fn zola_components_in_the_writing_convert_or_stay_as_written() {
    assert_eq!(fixture("zola-components"), [BUILT, AS_TEXT]);
}

/// Zola 0.23 runs the writing as a template: `raw` is taken out, and the
/// rest of its template code is told, inside code too, but not in a file
/// `skip_content_templating` names. Where nothing in the old site says
/// which version made it, the template code is told as what it may have
/// run.
#[test]
fn zola_template_code_in_the_writing_is_told_by_the_version_that_ran_it() {
    assert_eq!(fixture("zola-templated-writing"), [BUILT, AS_TEXT]);
    assert_eq!(fixture("zola-skip-templating"), [BUILT, AS_TEXT]);
    assert_eq!(fixture("zola-template-code"), [BUILT]);
}

/// Zola 0.23 has no shortcodes, so a call in its writing is to one of
/// Zola's functions: `get_url` gives the address Zola gave, and any other
/// is told rather than made a snippet call.
#[test]
fn zola_functions_in_the_writing_give_their_address_or_are_told() {
    assert_eq!(fixture("zola-functions"), [BUILT]);
}

/// Zola 0.23 lets an `@/` link, or `get_url`, name a file beside a page:
/// it is the address of the page that owns the file, in the language of
/// the page linking where it has one, then the file's path from the
/// page's folder.
#[test]
fn a_zola_link_to_a_file_beside_a_page_reaches_it() {
    assert_eq!(fixture("zola-beside"), [BUILT]);
}

#[test]
fn private_use_characters_come_across_from_hugo_and_zola_as_written() {
    let words = "A \u{E000} and \u{E001}.\n\n<div title=\"\u{E001}\">\u{E000}</div>\n\n\
                 ```\n\u{E001}\u{E000}\n```\n\n";
    let front = "+++\ntitle = \"Post 1\"\ndate = 2026-01-01\n+++\n";
    // The tool, its settings file and what it holds, and a tag that
    // becomes a call.
    let tools = [
        (
            "hugo",
            "hugo.toml",
            "baseURL = \"https://example.org/\"\n",
            "{{< youtube \"\u{E000}\" >}}",
        ),
        (
            "zola",
            "config.toml",
            "base_url = \"https://example.org\"\n",
            "{{ youtube(id=\"\u{E000}\") }}",
        ),
    ];
    for (tool, settings, set, tag) in tools {
        let old = scratch(&format!("{tool}-private-use-old"));
        fs::create_dir_all(old.join("content/posts")).unwrap();
        fs::write(old.join(settings), set).unwrap();
        fs::write(
            old.join("content/posts/post-1.md"),
            format!("{front}{words}{tag}\n"),
        )
        .unwrap();
        let new = scratch(&format!("{tool}-private-use"));
        let (code, said) = import(&old, &new, None);
        assert_eq!(code, 0, "{tool}: {said}");
        let post = fs::read_to_string(new.join("content/posts/post-1.md")).unwrap();
        assert!(
            post.ends_with(&format!("\n---\n{words}@youtube(id='\u{E000}')\n")),
            "{tool}: {post:?}"
        );
    }
}

#[test]
fn zola_site_under_a_path_keeps_its_addresses_and_links() {
    assert_eq!(fixture("zola-under-path"), [BUILT]);
}

#[test]
fn zola_feeds_stay_at_their_old_addresses() {
    assert_eq!(fixture("zola-feeds"), [BUILT, NO_FEED]);
}

#[test]
fn zola_built_output_is_looked_in_for_every_address() {
    assert_eq!(fixture("zola-built-agrees"), [BUILT, NO_FEED]);
    assert_eq!(fixture("zola-built-missing"), [BUILT, NO_FEED]);
}

#[test]
fn a_zola_site_is_told_by_its_settings() {
    let top = scratch("zola-told");
    let site = |name: &str, files: &[(&str, &str)]| {
        let folder = top.join(name);
        for (path, text) in files {
            let path = folder.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        folder
    };
    for (name, file, text) in [
        ("zola-toml", "zola.toml", "title = \"Site 1\"\n"),
        (
            "config-toml",
            "config.toml",
            "base_url = \"https://example.org\"\n",
        ),
    ] {
        let folder = site(name, &[(file, text)]);
        assert!(Tool::Zola.owns(&folder), "{name}");
        assert!(!Tool::Hugo.owns(&folder), "{name}");
        assert!(!Tool::Jekyll.owns(&folder), "{name}");
    }
    let other = site(
        "other-tool",
        &[("config.toml", "baseURL = \"https://example.org/\"\n")],
    );
    assert!(!Tool::Zola.owns(&other));
    let (code, said) = import(&other, &top.join("new-1"), None);
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Imported the Hugo site"), "{said}");
    let neither = site("neither", &[("config.toml", "title = \"Site 1\"\n")]);
    assert!(!Tool::Zola.owns(&neither));
    assert!(!Tool::Hugo.owns(&neither));
    let new = top.join("new-2");
    let (code, said) = import(&neither, &new, Some(Tool::Zola));
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Imported the Zola site"), "{said}");
}

/// A Zola site in `top/old` naming `setting` in its settings, with a
/// post calling `youtube` and a file outside it, at `top/outside`, holding
/// what the setting would read there: built output, or a theme's tag of
/// the same name. Gives the report the import writes and the new site's
/// every file.
fn zola_outside(top: &Path, setting: &str) -> (String, BTreeMap<String, Vec<u8>>) {
    let old = top.join("old");
    let outside = top.join("outside");
    for (path, text) in [
        (
            old.join("config.toml"),
            format!("base_url = \"https://example.org\"\n{setting}\n"),
        ),
        (
            old.join("content/posts/post-1.md"),
            "+++\ntitle = \"Post 1\"\ndate = 2026-01-01\n+++\nA post calling a tag a theme \
             outside the site has.\n\n{{ youtube(id=\"abcdefghijk\") }}\n"
                .to_owned(),
        ),
        (
            outside.join("templates/shortcodes/youtube.html"),
            "outside-1\n".to_owned(),
        ),
        (outside.join("index.html"), "outside-1\n".to_owned()),
    ] {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let new = top.join("new");
    let (code, said) = import(&old, &new, Some(Tool::Zola));
    assert_eq!(code, 0, "{said}");
    let made = files(&new);
    let report = String::from_utf8(made["IMPORT.md"].clone()).unwrap();
    for (path, bytes) in &made {
        assert!(
            !String::from_utf8_lossy(bytes).contains("outside-1"),
            "{path} holds what is outside the site"
        );
    }
    let post = String::from_utf8(made["content/posts/post-1.md"].clone()).unwrap();
    assert!(post.contains("@youtube(id='abcdefghijk')"), "{post}");
    (report, made)
}

#[test]
fn a_zola_folder_named_outside_the_site_is_never_read() {
    let top = scratch("zola-outside");
    let outside = std::path::absolute(top.join("absolute").join("outside")).unwrap();
    let absolute = outside.to_string_lossy().replace('\\', "/");
    for (label, setting, shown) in [
        (
            "up",
            "output_dir = \"../outside\"",
            "output_dir: ../outside",
        ),
        (
            "absolute",
            &format!("output_dir = \"{absolute}\""),
            &format!("output_dir: {absolute}"),
        ),
        (
            "theme-up",
            "theme = \"../../outside\"",
            "theme: ../../outside`",
        ),
        (
            "theme-absolute",
            &format!("theme = \"{absolute}\""),
            &format!("theme: {absolute}`"),
        ),
    ] {
        let (report, _) = zola_outside(&top.join(label), setting);
        assert!(
            report.contains("### Settings naming a folder Statac doesn't read"),
            "{label}: {report}"
        );
        assert!(report.contains(shown), "{label}: {report}");
        assert!(!report.contains("built output, `"), "{label}: {report}");
    }
}

#[cfg(unix)]
#[test]
fn a_zola_folder_reached_through_a_link_is_never_read() {
    let top = scratch("zola-linked");
    for (label, setting, link) in [
        ("output", "output_dir = \"built\"", "built"),
        ("theme", "theme = \"theme-1\"", "themes/theme-1"),
    ] {
        let at = top.join(label);
        let old = at.join("old");
        fs::create_dir_all(old.join("themes")).unwrap();
        std::os::unix::fs::symlink(at.join("outside"), old.join(link)).unwrap();
        let (report, _) = zola_outside(&at, setting);
        assert!(
            report.contains("### Settings naming a folder Statac doesn't read"),
            "{label}: {report}"
        );
        assert!(!report.contains("built output, `"), "{label}: {report}");
    }
    let at = top.join("content");
    let old = at.join("old");
    fs::create_dir_all(old.join("content/posts")).unwrap();
    fs::create_dir_all(old.join("static")).unwrap();
    fs::create_dir_all(at.join("outside")).unwrap();
    fs::write(at.join("outside/post-2.md"), "outside-1\n").unwrap();
    std::os::unix::fs::symlink(
        at.join("outside/post-2.md"),
        old.join("content/posts/post-2.md"),
    )
    .unwrap();
    std::os::unix::fs::symlink(at.join("outside/post-2.md"), old.join("static/file-1.txt"))
        .unwrap();
    let (report, made) = zola_outside(&at, "");
    assert!(!made.contains_key("content/posts/post-2.md"));
    assert!(!made.contains_key("static/file-1.txt"));
    assert!(
        report.contains(
            "- `content/posts/post-2.md`: a link that leads outside the old site, \
             which Statac doesn't follow."
        ),
        "{report}"
    );
}

#[test]
fn a_zola_feed_named_with_a_path_is_not_taken_for_a_feeds_name() {
    let top = scratch("zola-feed-path");
    let setting = "generate_feeds = true\nfeed_filenames = [\"../../outside/feed-9.xml\"]";
    let (report, made) = zola_outside(&top, setting);
    for (path, bytes) in &made {
        let text = String::from_utf8_lossy(bytes);
        assert!(!text.contains("feed-9"), "`{path}`:\n{text}");
    }
    assert!(!report.contains("Errors in the new site"), "{report}");
}

#[cfg(unix)]
#[test]
fn a_zola_settings_file_that_is_a_link_isnt_read() {
    let top = scratch("zola-linked-settings");
    let old = top.join("old");
    fs::create_dir_all(old.join("content")).unwrap();
    fs::write(
        top.join("settings-9.toml"),
        "base_url = \"https://example.org\"\ntitle = \"outside-1\"\n",
    )
    .unwrap();
    std::os::unix::fs::symlink(top.join("settings-9.toml"), old.join("config.toml")).unwrap();
    assert!(!Tool::Zola.owns(&old));
    let new = top.join("new");
    let (code, said) = import(&old, &new, Some(Tool::Zola));
    assert_eq!(code, 1, "{said}");
    assert!(said.contains("it is a link"), "{said}");
    assert!(!new.exists());
}

#[test]
fn eleventy_settings_are_read_in_either_quotes_and_assumed_where_absent() {
    for name in [
        "eleventy-settings-double",
        "eleventy-settings-single",
        "eleventy-settings-absent",
        "eleventy-settings-program",
        "eleventy-package-json",
    ] {
        assert_eq!(fixture(name), [BUILT], "{name}");
    }
}

#[test]
fn eleventy_front_matter_comes_across_in_statacs_words() {
    assert_eq!(fixture("eleventy-front-matter"), [BUILT]);
}

#[test]
fn eleventy_permalinks_are_kept_in_each_form() {
    assert_eq!(fixture("eleventy-permalinks"), [BUILT, DRAFT]);
}

#[test]
fn eleventy_data_is_handed_down_as_statac_hands_it() {
    // Eleventy published no page listing `notes/`, and nothing asks for one.
    assert_eq!(fixture("eleventy-data"), [BUILT]);
}

#[test]
fn eleventy_folders_of_unpublished_files_with_no_title_are_left_behind() {
    assert_eq!(fixture("eleventy-parts"), [BUILT, DRAFT]);
}

#[test]
fn eleventy_permalinks_written_on_every_page_become_one_pattern() {
    assert_eq!(fixture("eleventy-pattern"), [BUILT]);
}

#[test]
fn eleventy_addresses_through_the_url_filter_carry_the_path_the_settings_give() {
    assert_eq!(fixture("eleventy-under-path"), ["broken-link", BUILT]);
}

const ADD_URL: &str = "Add the full URL as `url` in `statac.yaml`";

#[test]
fn an_address_given_for_a_site_whose_settings_give_none_is_its_url() {
    let old = Path::new("tests/import/eleventy-under-path/old");
    let new = scratch("address-given");
    let (code, said) = import_answering(old, &new, "https://example.org\n", false);
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Imported the Eleventy site"), "{said}");
    let read = |path: &str| fs::read_to_string(new.join(path)).unwrap();
    assert!(read("statac.yaml").contains("\nurl: https://example.org/blog\n"));
    let report = read("IMPORT.md");
    assert!(report.contains("its `url` is the URL you gave"), "{report}");
    assert!(!report.contains(ADD_URL), "{report}");
    assert!(report.contains("The site is under a path, `/blog`, as its `url` says"));
    assert_eq!(checked(&new), ["broken-link", BUILT, "missing-anchor"]);

    let new = scratch("address-given-whole");
    let (code, said) = import_answering(old, &new, "https://example.org/blog/\n", false);
    assert_eq!(code, 0, "{said}");
    let settings = fs::read_to_string(new.join("statac.yaml")).unwrap();
    assert!(settings.contains("\nurl: https://example.org/blog\n"));
}

#[test]
fn a_site_whose_settings_give_no_address_has_no_url_where_none_is_given() {
    let old = Path::new("tests/import/eleventy-under-path/old");
    for (label, typed) in [("address-enter", "\n"), ("address-nothing", "")] {
        let new = scratch(label);
        let (code, said) = import_answering(old, &new, typed, false);
        assert_eq!(code, 0, "{said}");
        assert!(said.starts_with("Imported the Eleventy site"), "{said}");
        let made = files(&new);
        let kept = files(Path::new("tests/import/eleventy-under-path/new"));
        assert_eq!(made["IMPORT.md"], kept["IMPORT.md"], "{label}");
        let settings = String::from_utf8(made["statac.yaml"].clone()).unwrap();
        assert!(!settings.contains("url:"), "{settings}");
        assert!(String::from_utf8_lossy(&made["IMPORT.md"]).contains(ADD_URL));
    }
}

/// A Jekyll, Hugo or Zola site, times the URL its settings give, and
/// asked for one with Enter pressed: the report says the new site has no
/// `url` wherever it has none.
#[test]
fn a_site_whose_settings_give_no_url_is_said_to_have_none() {
    let post = "---\ntitle: Post 1\ndate: 2026-01-01\n---\nText.\n";
    let none = "so the new site has no `url` yet: add it to `statac.yaml` once it is known.";
    // (the case, the settings file and what it holds, the post's file, the
    // reason said)
    let cases = [
        (
            "jekyll",
            "_config.yml",
            "",
            "_posts/2026-01-01-post-1.md",
            "The settings give no `url`",
        ),
        (
            "jekyll-host",
            "_config.yml",
            "url: example.org\n",
            "_posts/2026-01-01-post-1.md",
            "The settings' `url`, `example.org`, isn't an `https://` or `http://` URL",
        ),
        (
            "jekyll-url",
            "_config.yml",
            "url: https://example.org\n",
            "_posts/2026-01-01-post-1.md",
            "",
        ),
        (
            "hugo",
            "hugo.toml",
            "",
            "content/posts/post-1.md",
            "The settings give no `baseURL`",
        ),
        (
            "hugo-path",
            "hugo.toml",
            "baseURL = \"/\"\n",
            "content/posts/post-1.md",
            "The settings' `baseURL`, `/`, isn't an `https://` or `http://` URL",
        ),
        (
            "hugo-empty",
            "hugo.toml",
            "baseURL = \"\"\n",
            "content/posts/post-1.md",
            "The settings give no `baseURL`",
        ),
        (
            "hugo-url",
            "hugo.toml",
            "baseURL = \"https://example.org/\"\n",
            "content/posts/post-1.md",
            "",
        ),
        (
            "zola",
            "zola.toml",
            "",
            "content/post-1.md",
            "The settings give no `base_url`",
        ),
        (
            "zola-path",
            "zola.toml",
            "base_url = \"/\"\n",
            "content/post-1.md",
            "The settings' `base_url`, `/`, isn't an `https://` or `http://` URL",
        ),
        (
            "zola-url",
            "zola.toml",
            "base_url = \"https://example.org\"\n",
            "content/post-1.md",
            "",
        ),
    ];
    for (label, settings, given, page, why) in cases {
        for (asked, typed) in [("", None), ("-asked", Some("\n"))] {
            let label = format!("{label}{asked}");
            let top = scratch(&format!("no-url-{label}"));
            let old = top.join("old");
            fs::create_dir_all(old.join(page).parent().unwrap()).unwrap();
            let title = match settings.ends_with(".yml") {
                true => "title: Site 1\n",
                false => "title = \"Site 1\"\n",
            };
            fs::write(old.join(settings), format!("{given}{title}")).unwrap();
            fs::write(old.join(page), post).unwrap();
            let new = top.join("new");
            let (code, told) = match typed {
                Some(typed) => import_answering(&old, &new, typed, false),
                None => import(&old, &new, None),
            };
            assert_eq!(code, 0, "{label}: {told}");
            let written = fs::read_to_string(new.join("statac.yaml")).unwrap();
            assert_eq!(
                written.contains("\nurl: "),
                why.is_empty(),
                "{label}: {written}"
            );
            let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
            let said = format!("{why}, {none}");
            assert_eq!(report.contains(&said), !why.is_empty(), "{label}: {report}");
            assert_eq!(report.contains(none), !why.is_empty(), "{label}: {report}");
        }
    }
}

#[test]
fn the_sites_address_is_asked_for_at_a_terminal_until_it_is_one_or_enter() {
    let old = Path::new("tests/import/eleventy-under-path/old");
    let question = "What is the site's URL?\n\x20 The old site's settings don't say.\n\
                    \x20 Enter for none yet.\n";
    let new = scratch("address-asked");
    let typed = "example\nhttps://example.org/blog\n";
    let (code, said) = import_answering(old, &new, typed, true);
    assert_eq!(code, 0, "{said}");
    let asked = format!(
        "{question}\x20 > \x20 `example` isn't a full URL. Write it as \
         `https://example.org`.\n\x20 > \nImported the Eleventy site"
    );
    assert!(said.starts_with(&asked), "{said}");
    let settings = fs::read_to_string(new.join("statac.yaml")).unwrap();
    assert!(settings.contains("\nurl: https://example.org/blog\n"));

    let new = scratch("address-asked-enter");
    let (code, said) = import_answering(old, &new, "\n", true);
    assert_eq!(code, 0, "{said}");
    assert!(
        said.starts_with(&format!("{question}\x20 > \nImported")),
        "{said}"
    );
    assert!(
        !fs::read_to_string(new.join("statac.yaml"))
            .unwrap()
            .contains("url:")
    );
}

#[test]
fn an_unusable_address_read_with_no_terminal_stops_the_import() {
    let old = Path::new("tests/import/eleventy-under-path/old");
    let new = scratch("address-unusable");
    let (code, said) = import_answering(old, &new, "example\n", false);
    assert_eq!(code, 1);
    assert_eq!(
        said,
        "`example` isn't a full URL. Write it as `https://example.org`.\n"
    );
    assert!(!new.exists());
}

#[test]
fn the_sites_address_is_asked_for_of_any_tool_and_only_where_the_settings_give_none() {
    let hugo = scratch("address-hugo-old");
    fs::create_dir_all(hugo.join("content/posts")).unwrap();
    fs::write(hugo.join("hugo.toml"), "title = \"Site 1\"\n").unwrap();
    let post = "+++\ntitle = \"Post 1\"\ndate = 2026-01-01\n+++\nA post.\n";
    fs::write(hugo.join("content/posts/post-1.md"), post).unwrap();
    let zola = scratch("address-zola-old");
    fs::create_dir_all(zola.join("content")).unwrap();
    fs::write(
        zola.join("config.toml"),
        "base_url = \"/\"\ntitle = \"Site 1\"\n",
    )
    .unwrap();
    fs::write(zola.join("content/post-1.md"), post).unwrap();
    let jekyll = Path::new("tests/import/permalink-pretty/old");
    for (label, old) in [
        ("hugo", hugo.as_ref()),
        ("zola", zola.as_ref()),
        ("jekyll", jekyll),
    ] {
        let new = scratch(&format!("address-{label}"));
        let (code, said) = import_answering(old, &new, "https://example.org/\n", true);
        assert_eq!(code, 0, "{said}");
        assert!(said.starts_with("What is the site's URL?\n"), "{said}");
        let settings = fs::read_to_string(new.join("statac.yaml")).unwrap();
        assert!(settings.contains("\nurl: https://example.org\n"), "{label}");
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert!(!report.contains("no `url`"), "{label}: {report}");
    }
    let old = Path::new("tests/import/hugo-home-pages/old");
    let new = scratch("address-known");
    let (code, said) = import_answering(old, &new, "https://example.com\n", true);
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Imported the Hugo site"), "{said}");
    let settings = fs::read_to_string(new.join("statac.yaml")).unwrap();
    assert!(
        settings.contains("\nurl: https://example.org\n"),
        "{settings}"
    );
}

#[test]
fn eleventy_folders_given_by_a_name_said_once_are_read() {
    assert_eq!(fixture("eleventy-settings-named"), [BUILT]);
}

#[test]
fn eleventy_markdown_options_are_read_in_a_file_the_settings_fetch() {
    assert_eq!(fixture("eleventy-settings-fetched"), [BUILT]);
}

#[test]
fn eleventy_categories_become_tags_and_their_old_pages_send_readers_on() {
    assert_eq!(fixture("eleventy-categories"), [BUILT]);
}

#[test]
fn eleventy_pages_are_dated_and_named_by_their_files() {
    assert_eq!(fixture("eleventy-dates"), [BUILT]);
}

#[test]
fn eleventy_redirects_become_the_pages_old_addresses() {
    assert_eq!(fixture("eleventy-redirects"), [BUILT]);
}

#[test]
fn eleventy_tags_in_the_writing_convert_or_stay_as_written() {
    assert_eq!(
        fixture("eleventy-tags-in-writing"),
        ["broken-link", BUILT, AS_TEXT]
    );
}

#[test]
fn eleventy_line_ends_are_line_breaks_where_the_settings_said() {
    assert_eq!(fixture("eleventy-breaks"), [BUILT]);
}

#[test]
fn eleventy_templates_ignored_files_and_files_beside_pages() {
    assert_eq!(fixture("eleventy-templates"), [BUILT, AS_TEXT]);
}

#[test]
fn eleventy_built_output_is_looked_in_for_every_address() {
    assert_eq!(fixture("eleventy-built-agrees"), [BUILT]);
    assert_eq!(fixture("eleventy-built-missing"), [BUILT]);
}

/// A folder under `top` holding `files`.
fn made_site(top: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let folder = top.join(name);
    for (path, text) in files {
        let path = folder.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    folder
}

#[test]
fn an_eleventy_site_is_told_by_its_settings_or_its_package() {
    let top = scratch("eleventy-told");
    for (name, files) in [
        (
            "dot-js",
            &[(".eleventy.js", "module.exports = function () {};\n")][..],
        ),
        (
            "config-js",
            &[("eleventy.config.js", "export default {};\n")],
        ),
        (
            "config-mjs",
            &[("eleventy.config.mjs", "export default {};\n")],
        ),
        (
            "config-cjs",
            &[("eleventy.config.cjs", "module.exports = {};\n")],
        ),
        (
            "package",
            &[(
                "package.json",
                "{\"dependencies\": {\"@11ty/eleventy\": \"^3.0.0\"}}\n",
            )],
        ),
    ] {
        let folder = made_site(&top, name, files);
        assert!(Tool::Eleventy.owns(&folder), "{name}");
        for other in [Tool::Jekyll, Tool::Hugo, Tool::Zola] {
            assert!(!other.owns(&folder), "{name}");
        }
    }
    for (name, text) in [
        (
            "other-package",
            "{\"dependencies\": {\"other-tool\": \"1\"}}\n",
        ),
        ("broken-package", "{\"dependencies\": \n"),
    ] {
        let folder = made_site(&top, name, &[("package.json", text)]);
        assert!(!Tool::Eleventy.owns(&folder), "{name}");
    }
    let neither = made_site(
        &top,
        "neither",
        &[(
            "posts/post-1.md",
            "---\ntitle: Post 1\n---\nA post read as Eleventy's.\n",
        )],
    );
    let (code, said) = import(&neither, &top.join("new-1"), Some(Tool::Eleventy));
    assert_eq!(code, 0, "{said}");
    assert!(said.starts_with("Imported the Eleventy site"), "{said}");
    let report = fs::read_to_string(top.join("new-1/IMPORT.md")).unwrap();
    assert!(
        report.starts_with("This site was imported from an Eleventy site"),
        "{report}"
    );
}

/// An Eleventy site with one post beside files that are no part of it, as
/// [`beside_files_of_others`] makes them.
fn eleventy_beside_files_of_others(label: &str) -> (Scratch, PathBuf, PathBuf) {
    let (folder, old, outside) = beside_files_of_others(label);
    fs::remove_dir_all(old.join("_posts")).unwrap();
    fs::create_dir_all(old.join("posts")).unwrap();
    fs::write(
        old.join("posts/post-1.md"),
        "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA post of the old site's own.\n",
    )
    .unwrap();
    (folder, old, outside)
}

const COPY_UNREAD: &str = "### Copies Statac doesn't make\n";

/// Imports `old` with `settings` as its `.eleventy.js`, and gives the
/// report, having seen that the old site's own post came across and that
/// nothing from outside it did. The report may name what it didn't read.
fn eleventy_nothing_from_outside(old: &Path, settings: &str) -> String {
    fs::write(old.join(".eleventy.js"), settings).unwrap();
    let new = old.parent().unwrap().parent().unwrap().join("new");
    let (code, said) = import(old, &new, None);
    assert_eq!(code, 0, "{settings}: {said}");
    let made = files(&new);
    assert!(made.contains_key("content/posts/post-1.md"), "{settings}");
    for (path, bytes) in &made {
        let text = String::from_utf8_lossy(bytes);
        // A copy named by a whole path lands under that path, which holds
        // the scratch folder's own name, so only the file's name is asked.
        let name = path.rsplit('/').next().unwrap_or_default();
        assert!(!name.contains("-9"), "{settings}: `{path}` was written");
        assert!(!text.contains(OUTSIDE), "{settings}: `{path}`:\n{text}");
    }
    let report = String::from_utf8(made["IMPORT.md"].clone()).unwrap();
    fs::remove_dir_all(&new).unwrap();
    report
}

/// Settings whose second line is `line`.
fn eleventy_settings(line: &str) -> String {
    format!("module.exports = function (eleventyConfig) {{\n  {line}\n}};\n")
}

#[test]
fn an_eleventy_folder_or_copy_named_outside_the_old_site_is_never_read() {
    let (_folder, old, outside) = eleventy_beside_files_of_others("eleventy-outside");
    fs::create_dir_all(old.join("a")).unwrap();
    let absolute = outside.to_string_lossy().replace('\\', "/");
    for setting in ["input", "output", "includes", "data", "layouts"] {
        for folder in ["..", "../inner", "a/../..", &absolute, "/"] {
            let line = format!("return {{ dir: {{ {setting}: \"{folder}\" }} }};");
            let report = nothing_from_outside_of(&old, ".eleventy.js", &eleventy_settings(&line));
            let listed =
                format!("{FOLDER_UNREAD}- `.eleventy.js`, line 2: `{setting}: {folder}`\n");
            assert!(report.contains(&listed), "{line}: {report}");
        }
    }
    for copy in [
        "\"..\"".to_owned(),
        "\"../inner\"".to_owned(),
        "\"../file-9.txt\"".to_owned(),
        "'a/../../file-9.txt'".to_owned(),
        format!("\"{absolute}/file-9.txt\""),
        "\"../*.txt\"".to_owned(),
        "{ \"..\": \"copied\" }".to_owned(),
        "{ \"a\": \"../../escaped\" }".to_owned(),
    ] {
        let line = format!("eleventyConfig.addPassthroughCopy({copy});");
        let report = eleventy_nothing_from_outside(&old, &eleventy_settings(&line));
        let listed = format!("- `.eleventy.js`, line 2: `addPassthroughCopy({copy})`\n");
        assert!(report.contains(COPY_UNREAD), "{line}: {report}");
        assert!(report.contains(&listed), "{line}: {report}");
        assert!(!outside.parent().unwrap().join("escaped").exists());
    }
    // Eleventy writes a copy into its output folder, however its place
    // there is written, and Statac into `static/`. A place that names a
    // drive is no place in a site, and is told.
    fs::write(old.join("a/file-1.txt"), "file-1\n").unwrap();
    let line = format!("eleventyConfig.addPassthroughCopy({{ \"a\": \"{absolute}/copied\" }});");
    let report = eleventy_nothing_from_outside(&old, &eleventy_settings(&line));
    assert_eq!(report.contains(COPY_UNREAD), cfg!(windows), "{report}");
    assert!(!outside.join("copied").exists());
    fs::remove_file(old.join("a/file-1.txt")).unwrap();
}

#[cfg(unix)]
#[test]
fn an_eleventy_folder_or_copy_reached_through_a_link_is_never_read() {
    use std::os::unix::fs::symlink;
    let (_folder, old, outside) = eleventy_beside_files_of_others("eleventy-linked");
    symlink(&outside, old.join("linked")).unwrap();
    for setting in ["input", "output", "includes", "data", "layouts"] {
        for folder in ["linked", "linked/inner", "./linked/"] {
            let line = format!("return {{ dir: {{ {setting}: \"{folder}\" }} }};");
            let report = nothing_from_outside_of(&old, ".eleventy.js", &eleventy_settings(&line));
            let listed =
                format!("{FOLDER_UNREAD}- `.eleventy.js`, line 2: `{setting}: {folder}`\n");
            assert!(report.contains(&listed), "{line}: {report}");
        }
    }
    for copy in [
        "\"linked\"",
        "\"linked/file-9.txt\"",
        "{ \"linked\": \"copied\" }",
    ] {
        let line = format!("eleventyConfig.addPassthroughCopy({copy});");
        let report = eleventy_nothing_from_outside(&old, &eleventy_settings(&line));
        assert!(report.contains(COPY_UNREAD), "{line}: {report}");
    }
    // A pattern finds nothing through the link, and the walk tells of it.
    let line = "eleventyConfig.addPassthroughCopy(\"**/*.txt\");";
    let report = eleventy_nothing_from_outside(&old, &eleventy_settings(line));
    assert!(
        report.contains(
            "- `linked`: a link that leads outside the old site, which Statac doesn't follow.\n"
        ),
        "{report}"
    );
    fs::remove_file(old.join("linked")).unwrap();
    // The files read by their own names, each a link.
    fs::write(outside.join("_redirects"), "/post-9/ /posts/post-1/\n").unwrap();
    fs::write(outside.join("ignore-9"), "posts/\n").unwrap();
    for (name, to) in [
        ("_redirects", outside.join("_redirects")),
        (".eleventyignore", outside.join("ignore-9")),
        ("_data", outside.clone()),
        ("_includes", outside.clone()),
        ("_site", outside.clone()),
        ("img", outside.clone()),
    ] {
        symlink(&to, old.join(name)).unwrap();
        let settings = eleventy_settings("eleventyConfig.addPassthroughCopy(\"img\");");
        let report = eleventy_nothing_from_outside(&old, &settings);
        assert!(!report.contains("post-9"), "{name}: {report}");
        fs::remove_file(old.join(name)).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn an_eleventy_settings_file_that_is_a_link_isnt_read() {
    use std::os::unix::fs::symlink;
    let (_folder, old, outside) = eleventy_beside_files_of_others("eleventy-linked-settings");
    fs::write(
        outside.join("settings-9.js"),
        format!("module.exports = () => ({{ dir: {{ input: \"..\" }} }}); // {OUTSIDE}\n"),
    )
    .unwrap();
    symlink(outside.join("settings-9.js"), old.join(".eleventy.js")).unwrap();
    assert!(!Tool::Eleventy.owns(&old));
    let new = outside.parent().unwrap().join("new");
    let (code, said) = import(&old, &new, Some(Tool::Eleventy));
    assert_eq!(code, 0, "{said}");
    for (path, bytes) in files(&new) {
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(OUTSIDE), "`{path}`:\n{text}");
        assert!(!text.contains("settings-9"), "`{path}`:\n{text}");
    }
}

#[test]
fn eleventy_reads_gitignore_unless_the_settings_turn_it_off() {
    let top = scratch("eleventy-gitignore");
    let files = [
        (".gitignore", "drafts/\n"),
        (
            "posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\nA post.\n",
        ),
        (
            "drafts/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-01-02\n---\nA post `.gitignore` leaves out.\n",
        ),
    ];
    for (name, settings, kept) in [
        ("on", "module.exports = function () {};\n", false),
        (
            "off",
            "module.exports = function (c) { c.setUseGitIgnore(false); };\n",
            true,
        ),
    ] {
        let mut all = files.to_vec();
        all.push((".eleventy.js", settings));
        let old = made_site(&top, name, &all);
        let new = top.join(format!("{name}-new"));
        let (code, said) = import(&old, &new, None);
        assert_eq!(code, 0, "{said}");
        assert_eq!(
            new.join("content/drafts/post-2.md").exists(),
            kept,
            "{name}"
        );
        let report = fs::read_to_string(new.join("IMPORT.md")).unwrap();
        assert_eq!(
            report.contains("- `drafts/`: a folder `.gitignore` leaves out"),
            !kept,
            "{report}"
        );
    }
}

#[test]
fn front_matter_holding_a_private_use_character_is_mended_as_any_other() {
    let post = "---\ntitle: Post 1\nicon: \u{E000}\nsummary: 'One\ntwo'\n---\nWords.\n";
    // The tool, and the files of a site with one post.
    let tools = [
        (
            Tool::Hugo,
            "hugo",
            "hugo.toml",
            "baseURL = \"https://example.org/\"\n",
            "content/posts/post-1.md",
        ),
        (
            Tool::Zola,
            "zola",
            "config.toml",
            "base_url = \"https://example.org\"\n",
            "content/posts/post-1.md",
        ),
        (
            Tool::Jekyll,
            "jekyll",
            "_config.yml",
            "title: Old\n",
            "_posts/2026-01-01-post-1.md",
        ),
        (
            Tool::Eleventy,
            "eleventy",
            ".eleventy.js",
            "module.exports = function () {};\n",
            "posts/post-1.md",
        ),
    ];
    for (tool, name, settings, set, path) in tools {
        let old = scratch(&format!("{name}-mend-private-use-old"));
        fs::create_dir_all(old.join(path).parent().unwrap()).unwrap();
        fs::write(old.join(settings), set).unwrap();
        fs::write(old.join(path), post).unwrap();
        let new = scratch(&format!("{name}-mend-private-use"));
        let (code, said) = import(&old, &new, Some(tool));
        assert_eq!(code, 0, "{name}: {said}");
        let made = fs::read_to_string(new.join("content/posts/post-1.md")).unwrap();
        assert!(made.contains("summary: One two\n"), "{name}: {made}");
        assert!(made.contains("icon: \u{E000}\n"), "{name}: {made}");
    }
}
