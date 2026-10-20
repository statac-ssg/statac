//! The search index a build publishes under `search/`, end to end.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

struct Site {
    root: PathBuf,
}

impl Site {
    /// A site under a path with a home page and two posts, whose search
    /// page shows a box reading the site's search index.
    fn new(name: &str) -> Site {
        let root =
            std::env::temp_dir().join(format!("statac-search-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let site = Site { root };
        site.write(
            "statac.yaml",
            "title: Site\nlanguage: en\ntimezone: UTC\nurl: https://example.org/blog/\n\
             pages.feed: false\n",
        );
        site.write("templates/home.html", "{{ page.content }}\n");
        site.write("templates/page.html", "{{ page.content }}\n");
        site.write(
            "snippets/find.html",
            "{% set box = page.search_box() %}\
             <search id=\"{{ box.id }}\" data-index=\"{{ box.index }}\"></search>\n",
        );
        site.write(
            "content/index.md",
            "---\ntitle: Home\n---\nThe home page.\n",
        );
        site.write(
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\nThe first post, on running.\n\n\
             ## Later {#later}\n\nIt runs on.\n",
        );
        site.write(
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-01-02\n---\nThe second post.\n",
        );
        site.write("content/search.md", "---\ntitle: Search\n---\n@find()\n");
        site
    }

    /// The site in English and French at `url`, whose French search page
    /// shows a box too.
    fn in_two_languages(name: &str, url: &str) -> Site {
        let site = Site::new(name);
        site.write(
            "statac.yaml",
            &format!(
                "title: Site\nlanguage: [en, fr]\ntimezone: UTC\nurl: {url}\n\
                 pages.feed: false\n"
            ),
        );
        site.write(
            "content/fr/index.md",
            "---\ntitle: Accueil\n---\nLa page d'accueil.\n",
        );
        site.write(
            "content/fr/article-1.md",
            "---\ntitle: Article 1\ndate: 2026-01-01\n---\n\
             Le premier article, sur les chanteuses du café.\n\n\
             ## Plus tard {#plus-tard}\n\nUne chanteuse, running.\n",
        );
        site.write(
            "content/fr/article-2.md",
            "---\ntitle: Article 2\ndate: 2026-01-02\n---\nLe second article, et un post.\n",
        );
        site.write(
            "content/fr/chercher.md",
            "---\ntitle: Chercher\n---\n@find()\n",
        );
        site
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// Runs statac with `args` and the site's folder, and gives what it
    /// printed and its exit code.
    fn run(&self, args: &[&str]) -> (String, i32) {
        let output = Command::new(env!("CARGO_BIN_EXE_statac"))
            .args(args)
            .arg(&self.root)
            .env("NO_COLOR", "1")
            .output()
            .unwrap();
        assert!(output.stderr.is_empty(), "{output:?}");
        (
            String::from_utf8(output.stdout).unwrap(),
            output.status.code().unwrap(),
        )
    }

    /// Builds the site, and gives each file under `search/` by its path
    /// there, unpacked where it is packed.
    fn index(&self) -> Vec<(String, String)> {
        self.index_in("search")
    }

    /// Builds the site, and gives each file of the index in `folder` of the
    /// built site by its path there, unpacked where it is packed.
    fn index_in(&self, folder: &str) -> Vec<(String, String)> {
        let (said, code) = self.run(&["build", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{said}");
        let folder = self.root.join("_site").join(folder);
        let mut found = Vec::new();
        let mut folders = vec![folder.clone()];
        while let Some(next) = folders.pop() {
            for entry in fs::read_dir(&next).into_iter().flatten() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    folders.push(path);
                    continue;
                }
                let rel = path.strip_prefix(&folder).unwrap();
                let rel = rel.to_string_lossy().replace('\\', "/");
                if rel == "index.html" {
                    continue;
                }
                let bytes = fs::read(&path).unwrap();
                let text = match rel.ends_with(".bin") {
                    true => miniz_oxide::inflate::decompress_to_vec_zlib(&bytes).unwrap(),
                    false => bytes,
                };
                found.push((rel, String::from_utf8(text).unwrap()));
            }
        }
        found.sort();
        found
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn text<'a>(index: &'a [(String, String)], name: &str) -> &'a str {
    let found = index.iter().find(|(rel, _)| rel == name);
    &found.unwrap_or_else(|| panic!("{name} in {index:?}")).1
}

/// The name a file of `index` starting `before` is published under.
fn named<'a>(index: &'a [(String, String)], before: &str) -> Vec<&'a str> {
    let found = index.iter().filter(|(rel, _)| rel.starts_with(before));
    found.map(|(rel, _)| rel.as_str()).collect()
}

#[test]
fn a_site_no_page_of_which_names_the_index_has_none() {
    let site = Site::new("unasked");
    site.write("content/search.md", "---\ntitle: Search\n---\nNo box.\n");
    assert!(site.index().is_empty());
    // A box for another search tool asks for none either.
    site.write(
        "snippets/find.html",
        "{% set box = page.search_box() %}<div id=\"{{ box.id }}\"></div>\n",
    );
    site.write("content/search.md", "---\ntitle: Search\n---\n@find()\n");
    assert!(site.index().is_empty());
}

#[test]
fn a_page_asking_for_a_search_box_publishes_the_index_its_files_named_by_content() {
    let site = Site::new("asked");
    let index = site.index();
    let records = named(&index, "records.");
    let words = named(&index, "words.");
    let pages = named(&index, "pages/");
    assert_eq!(
        (records.len(), words.len(), pages.len()),
        (1, 1, 4),
        "{index:?}"
    );
    // The script reading them, after their names.
    let script = text(&index, "index.js");
    let names = format!(
        "const FILES = {{\"records\":\"{}\",\"words\":[[\"\",\"{}\"]]}};\n\n",
        records[0], words[0]
    );
    assert!(script.starts_with(&names), "{script}");
    assert!(script.contains("export async function search("), "{script}");
    // The pages with no date come first, then the dated from the oldest.
    let order = ["Home", "Search", "Post 1", "Post 2"];
    let shown: Vec<&str> = order
        .iter()
        .map(|title| {
            let title = format!("\"title\":\"{title}\"");
            let page = pages
                .iter()
                .find(|page| text(&index, page).contains(&title));
            page.unwrap().trim_start_matches("pages/")
        })
        .collect();
    assert_eq!(
        text(&index, records[0]),
        format!(
            "{{\"pages\":[[\"{}\",1],[\"{}\",1],[\"{}\",2],[\"{}\",1]],\
             \"lengths\":[8,5,15,10,13]}}",
            shown[0], shown[1], shown[2], shown[3]
        )
    );
    let built = fs::read_to_string(site.root.join("_site/search/index.html")).unwrap();
    assert!(
        built.contains("data-index=\"/blog/search/index.js\""),
        "{built}"
    );
    let post = text(&index, &format!("pages/{}", shown[2]));
    assert_eq!(
        post,
        "{\"url\":\"/blog/post-1/\",\"title\":\"Post 1\",\"date\":\"2026-01-01\",\"day\":\"1 January 2026\",\
         \"parts\":[[null,null,\"The first post, on running.\"],\
         [\"later\",\"Later\",\"It runs on.\"]]}"
    );
    // Both forms of the word stand with their one stem's records.
    let list = text(&index, words[0]);
    assert!(list.contains("\"running\",\"runs\""), "{list}");
    // Built again, every file is the same.
    assert_eq!(site.index(), index);
}

#[test]
fn a_page_edited_changes_its_own_text_and_leaves_every_other_pages() {
    let site = Site::new("edited");
    let before = site.index();
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\n---\nThe second post, edited.\n",
    );
    let after = site.index();
    let gone: Vec<&str> = (before.iter())
        .filter(|file| !after.contains(file))
        .map(|(rel, _)| rel.as_str())
        .collect();
    let new: Vec<&str> = (after.iter())
        .filter(|file| !before.contains(file))
        .map(|(rel, _)| rel.as_str())
        .collect();
    let kinds = |names: &[&str]| -> Vec<String> {
        let kinds = names
            .iter()
            .map(|name| name.split(['.', '/']).next().unwrap());
        kinds.map(str::to_owned).collect()
    };
    assert_eq!(kinds(&gone), ["index", "pages", "records", "words"]);
    assert_eq!(kinds(&new), ["index", "pages", "records", "words"]);
}

#[test]
fn a_file_at_an_address_of_the_index_is_told_as_a_clash() {
    // The French index alone is at `fr/search`: a file there, and a
    // folder spelt otherwise, meet it and nothing else.
    for (file, named) in [
        ("static/search/index.js", "static/search/index.js"),
        ("static/fr/search/index.js", "static/fr/search/index.js"),
        ("static/fr/search", "static/fr/search"),
        ("static/fr/Search/notes.txt", "static/fr/Search"),
    ] {
        let site = Site::in_two_languages("clash", "https://example.org/blog/");
        site.write(file, "\n");
        let (said, code) = site.run(&["check", "--today=2026-10-04"]);
        assert_eq!(code, 1, "{file}: {said}");
        let told = said.lines().find(|line| line.contains("Same address"));
        let told = told.unwrap_or_else(|| panic!("{file}: {said}"));
        assert!(told.contains(&format!("`{named}`")), "{told}");
        assert!(told.contains("the site's search index"), "{told}");
    }
}

#[test]
fn each_language_a_page_asks_for_has_an_index_of_its_own_pages_at_its_top() {
    // The site's path, the pages whose box reads the index, and the
    // folders of the indexes built.
    let cases: &[(&str, &[&str], &[&str])] = &[
        (
            "/blog",
            &["search.md", "fr/chercher.md"],
            &["search", "fr/search"],
        ),
        ("", &["fr/chercher.md"], &["fr/search"]),
        ("", &["search.md"], &["search"]),
        ("", &[], &[]),
    ];
    for &(path, asking, indexed) in cases {
        let site = Site::in_two_languages("languages", &format!("https://example.org{path}/"));
        for page in ["search.md", "fr/chercher.md"] {
            if !asking.contains(&page) {
                site.write(
                    &format!("content/{page}"),
                    "---\ntitle: Search\n---\nNo box.\n",
                );
            }
        }
        for (folder, page) in [
            ("search", "search/index.html"),
            ("fr/search", "fr/chercher/index.html"),
        ] {
            let index = site.index_in(folder);
            if !indexed.contains(&folder) {
                assert!(index.is_empty(), "{path} {asking:?} {folder}: {index:?}");
                continue;
            }
            // Each language's index holds its own pages, and no other's.
            let pages = named(&index, "pages/");
            assert_eq!(pages.len(), 4, "{path} {folder}: {index:?}");
            let home = format!("{path}/{}", folder.trim_end_matches("search"));
            for page in pages {
                let text = text(&index, page);
                let at = format!("{{\"url\":\"{home}");
                assert!(text.starts_with(&at), "{path} {folder}: {text}");
                if folder == "search" {
                    assert!(!text.contains("/fr/"), "{text}");
                }
            }
            let built = fs::read_to_string(site.root.join("_site").join(page)).unwrap();
            let address = format!("data-index=\"{path}/{folder}/index.js\"");
            assert!(built.contains(&address), "{path} {page}: {built}");
        }
    }
}

/// `text` as `JSON.stringify` writes it.
fn json(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if u32::from(c) < 0x20 => out.push_str(&format!("\\u{:04x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn json_or_null(text: Option<&str>) -> String {
    text.map_or("null".into(), json)
}

#[test]
fn the_browsers_script_finds_what_statac_search_finds() {
    let site = Site::new("reader");
    let long = "Words before the match go on for a while, and then some more. ".repeat(3);
    site.write(
        "content/post-3.md",
        &format!(
            "---\ntitle: Café notes\ndate: 2026-01-03\n---\n\
             A naïve start, about cafés.\n\n## Brewing {{#brewing}}\n\n\
             {long}Brewing runs best slowly. {long}\n\n## Running {{#running}}\n\n\
             Runners run, and running is fun.\n\n- one way to brew\n- another\n  - set in\n"
        ),
    );
    site.write(
        "content/post-4.md",
        "---\ntitle: 日本語のブログ\ndate: 2026-01-04\n---\n日本語のブログを書く。ログを読む。\n",
    );
    // Pages with no title, named as a list names them.
    site.write(
        "content/notes/index.md",
        "---\ntitle: Notes\nlayout: page\npages.untitled: true\n---\n",
    );
    site.write(
        "content/notes/note-1.md",
        "---\ndate: 2026-01-05\n---\nAn untitled note.\n",
    );
    site.write(
        "content/plain.md",
        "An untitled page, called by its opening words.\n",
    );
    site.write(
        "content/post-5.md",
        "---\ntitle: Post 5\ndate: 2026-01-06\n---\n𐌰𐌱 alpha5 card cart bard barn\n",
    );
    site.write(
        "content/post-6.md",
        "---\ntitle: Post 6\ndate: 2026-01-07\n---\ncart\n",
    );
    let built = site.index();
    assert!(!built.is_empty());
    let asked: &[(&str, bool)] = &[
        ("running", false),
        ("run", false),
        ("cafe", false),
        ("CAFÉ NAIVE", false),
        ("brewing slowly", false),
        ("post", false),
        ("running nowhere", false),
        ("nowhere", false),
        ("ru", true),
        ("caf", true),
        ("r", true),
        ("the first ", true),
        ("ブログ", false),
        ("ログ", false),
        ("日本", true),
        ("untitled", false),
        ("another", false),
        ("", false),
        // Words the site doesn't write, respelt as the nearest it does.
        ("brewinf", false),
        ("runnign", false),
        ("brewnig slowley", false),
        ("brewnig slowly", false),
        ("Brewnig OK", false),
        ("brewnig", true),
        ("ブロゲ", false),
        ("brewnig zzzzzz", false),
        ("𐌰𐌲", false),
        ("alpah 7", false),
        ("alpah.7", false),
        ("carx", false),
        ("barx", false),
    ];
    let rust = statac_finds(&site, 0, asked);
    assert!(rust.contains("\"Running\""), "{rust}");
    // A note called by its date isn't dated again; a titled page is.
    assert!(
        rust.contains("\"/blog/notes/note-1/\",\"5 January 2026\",null,"),
        "{rust}"
    );
    assert!(
        rust.contains("\"An untitled page, called by its opening words.\",null,"),
        "{rust}"
    );
    assert!(rust.contains("\"Post 1\",\"1 January 2026\","), "{rust}");
    // Each item of a list stands on a line of its own.
    assert!(
        rust.contains("way to brew\\n\",false],[\"another\",true],[\"\\nset in"),
        "{rust}"
    );
    let meant: Vec<String> = (split(&rust).iter().skip(18))
        .map(|found| split(found).pop().unwrap())
        .collect();
    assert_eq!(
        meant,
        [
            "\"brewing\"",
            "\"running\"",
            "\"brewing slowly\"",
            "null",
            "\"brewing OK\"",
            "\"brewing\"",
            "null",
            "null",
            "null",
            "\"alpha5 7\"",
            "null",
            "\"cart\"",
            "\"bard\"",
        ]
    );
    let Some(browser) = browser_finds(&site.root.join("_site/search/index.js"), asked, None) else {
        return;
    };
    for ((words, _), (rust, browser)) in asked.iter().zip(split(&rust).iter().zip(split(&browser)))
    {
        assert_eq!(rust, &browser, "{words}");
    }
    assert_eq!(rust, browser);
}

#[test]
fn a_suggestion_comes_from_the_piece_of_the_word_list_the_word_falls_in() {
    let site = Site::new("reader-pieces");
    // Enough words packing poorly that the word list is cut into pieces.
    let mut state: u64 = 1;
    let mut letter = || {
        state =
            (state.wrapping_mul(6_364_136_223_846_793_005)).wrapping_add(1_442_695_040_888_963_407);
        char::from(b'a' + ((state >> 33) % 26) as u8)
    };
    for page in 0..30 {
        let words: Vec<String> = (0..1000)
            .map(|_| (0..8).map(|_| letter()).collect())
            .collect();
        site.write(
            &format!("content/many-{page}.md"),
            &format!("---\ntitle: Many\n---\n{}\n", words.join(" ")),
        );
    }
    let index = site.index();
    let script = text(&index, "index.js");
    let named = script.lines().next().unwrap().split("[\"").skip(1);
    let firsts: Vec<&str> = named.map(|item| &item[..item.find('"').unwrap()]).collect();
    assert!(firsts.len() > 2, "{firsts:?}");
    // One edit from the first word of a piece, before it, and so in the
    // piece before, and after it, in its own.
    let first = firsts[2];
    let before = &first[..first.len() - 1];
    let after = format!("{first}q");
    let asked = &[(before, false), (after.as_str(), false)];
    let rust = statac_finds(&site, 0, asked);
    assert_eq!(rust, format!("[[0,true,[],null],[0,true,[],\"{first}\"]]"));
    let Some(browser) = browser_finds(&site.root.join("_site/search/index.js"), asked, None) else {
        return;
    };
    assert_eq!(rust, browser);
}

#[test]
fn a_run_of_japanese_finds_the_same_pages_however_the_browser_cuts_it_into_words() {
    let site = Site::new("reader-unspaced");
    site.write(
        "content/post-3.md",
        "---\ntitle: 静的サイト\ndate: 2026-01-03\n---\n静的サイトをビルドする。ビルドは速い。\n",
    );
    assert!(!site.index().is_empty());
    let asked: &[(&str, bool)] = &[
        ("ビルド", false),
        ("ビルドする", false),
        ("サイトをビルド", false),
        ("ビル", true),
        ("ビル", false),
    ];
    let rust = statac_finds(&site, 0, asked);
    for found in &split(&rust)[..4] {
        assert!(found.starts_with("[1,false,[[\"/blog/post-3/\""), "{rust}");
    }
    let Some(browser) = browser_finds(&site.root.join("_site/search/index.js"), asked, None) else {
        return;
    };
    for ((words, _), (rust, browser)) in asked.iter().zip(split(&rust).iter().zip(split(&browser)))
    {
        assert_eq!(rust, &browser, "{words}");
    }
}

#[test]
fn each_languages_script_finds_what_statac_search_finds_in_that_language() {
    let site = Site::in_two_languages("reader-languages", "https://example.org/blog/");
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\n---\nThe second post, on a café and a chanteuse.\n",
    );
    for folder in ["search", "fr/search"] {
        assert!(!site.index_in(folder).is_empty(), "{folder}");
    }
    let asked: &[(&str, bool)] = &[
        ("running", false),
        ("runs", false),
        ("chanteuse", false),
        ("chanteuses", false),
        ("cafe", false),
        ("post", false),
        ("article", false),
        ("premier post", false),
        ("nowhere", false),
        ("ch", true),
        ("ru", true),
        ("p", true),
        ("chanteuze", false),
        ("artcile", false),
    ];
    let cases = [
        (0, "_site/search/index.js"),
        (1, "_site/fr/search/index.js"),
    ];
    let found: Vec<(String, Option<String>)> = (cases.iter())
        .map(|&(language, script)| {
            let rust = statac_finds(&site, language, asked);
            (rust, browser_finds(&site.root.join(script), asked, None))
        })
        .collect();
    // Each language finds its own pages for a word both write.
    let (english, french) = (&found[0].0, &found[1].0);
    let cafe = |found: &str| split(found)[4].clone();
    assert!(cafe(english).contains("/blog/post-2/"), "{english}");
    assert!(!cafe(english).contains("/fr/"), "{english}");
    assert!(cafe(french).contains("/blog/fr/article-1/"), "{french}");
    assert!(!cafe(french).contains("/blog/post-"), "{french}");
    // Each respells from its own words.
    let meant = |found: &str, at: usize| split(&split(found)[at]).pop().unwrap();
    assert_eq!(meant(english, 12), "\"chanteuse\"");
    assert_eq!(meant(english, 13), "null");
    assert_eq!(meant(french, 13), "\"article\"");
    for ((language, _), (rust, browser)) in cases.iter().zip(&found) {
        let Some(browser) = browser else {
            return;
        };
        for ((words, _), (rust, browser)) in
            asked.iter().zip(split(rust).iter().zip(split(browser)))
        {
            assert_eq!(rust, &browser, "{language} {words}");
        }
        assert_eq!(rust, browser, "{language}");
    }
}

#[test]
fn a_browser_holding_the_script_from_an_earlier_build_reads_the_later_one() {
    let site = Site::new("earlier");
    site.index();
    let held = site.root.join("held");
    fs::create_dir_all(&held).unwrap();
    fs::copy(
        site.root.join("_site/search/index.js"),
        held.join("index.js"),
    )
    .unwrap();
    site.write(
        "content/post-2.md",
        "---\ntitle: Post 2\ndate: 2026-01-02\n---\nThe second post, on running too.\n",
    );
    let later = site.index();
    // The later build's files, beside the earlier build's script.
    for (rel, _) in &later {
        if rel != "index.js" {
            let to = held.join(rel);
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(site.root.join("_site/search").join(rel), to).unwrap();
        }
    }
    let asked = &[("running", false)];
    let Some(browser) = browser_finds(
        &held.join("index.js"),
        asked,
        Some(&site.root.join("_site/search/index.js")),
    ) else {
        return;
    };
    assert_eq!(browser, statac_finds(&site, 0, asked));
    assert!(browser.contains("post-2"), "{browser}");
}

/// What `statac search` finds in the pages of `site`'s language at
/// `language` in its order for each of `asked`, written as the browser's
/// script is by `search_reader.mjs`.
fn statac_finds(site: &Site, language: usize, asked: &[(&str, bool)]) -> String {
    use statac::site::Id;
    let today = statac::date::read("2026-10-04").ok();
    let (renderer, _) = statac::build::renderer(&site.root, today);
    let language = statac::site::LanguageId::from_index(language);
    let records = statac::search::records(&renderer, language);
    let index = statac::search::Index::of(renderer.site(), language, records);
    let found: Vec<String> = asked
        .iter()
        .map(|&(words, typing)| {
            let results = statac::search::results(&renderer, &index, words, typing, 100);
            let shown: Vec<String> = (results.shown.iter())
                .map(|shown| {
                    let excerpt: Vec<String> = (shown.excerpt.iter())
                        .map(|(text, marked)| format!("[{},{marked}]", json(text)))
                        .collect();
                    format!(
                        "[{},{},{},{},[{}]]",
                        json(&format!("/blog{}", shown.address)),
                        json(&shown.title),
                        json_or_null(shown.day.as_deref()),
                        json_or_null(shown.heading.as_deref()),
                        excerpt.join(",")
                    )
                })
                .collect();
            format!(
                "[{},{},[{}],{}]",
                results.pages,
                results.some,
                shown.join(","),
                json_or_null(results.suggestion.as_deref())
            )
        })
        .collect();
    format!("[{}]", found.join(","))
}

/// What the browser's script at `script` finds for each of `asked`, run by
/// `node`, with `later` written over it once it is read; none where `node`
/// is not installed.
fn browser_finds(
    script: &std::path::Path,
    asked: &[(&str, bool)],
    later: Option<&std::path::Path>,
) -> Option<String> {
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("node is not installed: the browser's script is not run");
        return None;
    }
    let questions: Vec<String> = (asked.iter())
        .map(|&(words, typing)| format!("[{},{typing}]", json(words)))
        .collect();
    let harness = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/search_reader.mjs");
    let mut node = Command::new("node");
    node.arg(harness)
        .arg(script)
        .arg(format!("[{}]", questions.join(",")));
    node.args(later);
    let output = node.output().unwrap();
    assert!(output.status.success(), "{output:?}");
    Some(String::from_utf8(output.stdout).unwrap())
}

/// The items of a JSON list, split at its top level.
fn split(list: &str) -> Vec<String> {
    let inner = &list[1..list.len() - 1];
    let (mut items, mut depth, mut quoted, mut escaped, mut from) =
        (Vec::new(), 0, false, false, 0);
    for (at, c) in inner.char_indices() {
        match c {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '[' | '{' if !quoted => depth += 1,
            ']' | '}' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                items.push(inner[from..at].to_owned());
                from = at + 1;
            }
            _ => {}
        }
    }
    items.push(inner[from..].to_owned());
    items
}

#[test]
fn a_link_to_the_script_reading_an_index_some_page_asks_for_lands() {
    let linking = "{% set box = page.search_box() %}\
                   <link rel=\"modulepreload\" href=\"{{ box.index }}\">\
                   <script type=\"module\" src=\"{{ box.index }}\"></script>\n";
    for (name, languages, url) in [
        ("linked", false, "https://example.org/blog/"),
        ("linked-top", true, "https://example.org/"),
        ("linked-languages", true, "https://example.org/blog/"),
    ] {
        let site = match languages {
            true => Site::in_two_languages(name, url),
            false => Site::new(name),
        };
        site.write("snippets/find.html", linking);
        let (said, _) = site.run(&["check", "--today=2026-10-04"]);
        assert!(!said.contains("broken-link"), "{name}: {said}");
    }
    // A link to the script with no page asking for its index lands nowhere.
    let site = Site::new("linked-unasked");
    site.write("content/search.md", "---\ntitle: Search\n---\nNo box.\n");
    site.write(
        "templates/page.html",
        "<script type=\"module\" src=\"{{ url(\"/search/index.js\") }}\"></script>\
         {{ page.content }}\n",
    );
    let (said, _) = site.run(&["check", "--today=2026-10-04"]);
    assert!(
        said.contains("nothing is published at `/blog/search/index.js`"),
        "{said}"
    );
}
