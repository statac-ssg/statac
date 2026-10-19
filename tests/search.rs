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
        let (said, code) = self.run(&["build", "--today=2026-10-04"]);
        assert_eq!(code, 0, "{said}");
        let folder = self.root.join("_site/search");
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
    assert_eq!(
        text(&index, "index.json"),
        format!(
            "{{\"format\":1,\"records\":\"{}\",\"words\":[[\"\",\"{}\"]]}}\n",
            records[0], words[0]
        )
    );
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
        built.contains("data-index=\"/blog/search/index.json\""),
        "{built}"
    );
    let post = text(&index, &format!("pages/{}", shown[2]));
    assert_eq!(
        post,
        "{\"url\":\"/blog/post-1/\",\"title\":\"Post 1\",\"date\":\"2026-01-01\",\
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
    let site = Site::new("clash");
    site.write("static/search/index.json", "{}\n");
    let (said, code) = site.run(&["check", "--today=2026-10-04"]);
    assert_eq!(code, 1, "{said}");
    assert!(
        said.contains("`static/search/index.json` and the site's search index"),
        "{said}"
    );
}
