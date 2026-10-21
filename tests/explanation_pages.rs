//! The documentation's generated pages: a page for each kind of problem,
//! written into the folder given, a page naming every file of a new site,
//! and the list of the addresses Statac prints.

use std::path::PathBuf;

#[allow(dead_code, reason = "the tests call what `main` calls")]
#[path = "../examples/printed_addresses.rs"]
mod printed_addresses;

use printed_addresses::explanation_pages;

/// An empty folder of this test's own, with a site's settings where
/// `settings` asks.
fn folder(name: &str, settings: bool) -> PathBuf {
    let folder = std::env::temp_dir().join(format!("statac-pages-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    if settings {
        std::fs::write(folder.join("statac.yaml"), "title: Site\n").unwrap();
    }
    folder
}

#[test]
fn the_pages_are_written_into_the_folder_given_and_one_of_no_kind_is_removed() {
    let site = folder("written", true);
    let pages = explanation_pages::folder(&site);
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(pages.join("no-such-kind.md"), "---\ntitle: Gone\n---\n").unwrap();
    explanation_pages::write(&site);

    let wanted = explanation_pages::pages();
    assert_eq!(wanted.len(), statac::problem::Code::ALL.len() + 1);
    for (name, text) in &wanted {
        assert_eq!(std::fs::read_to_string(pages.join(name)).unwrap(), *text);
        assert!(
            text.starts_with("---\n# Written from explanations/"),
            "{name}"
        );
        assert!(text.contains("\ntitle: "), "{name}");
        assert!(
            name == "index.md" || text.contains("\n## Its name\n"),
            "{name}"
        );
    }
    assert_eq!(std::fs::read_dir(&pages).unwrap().count(), wanted.len());
    assert_eq!(
        std::fs::read_to_string(explanation_pages::starter_path(&site)).unwrap(),
        explanation_pages::starter_page()
    );
    let _ = std::fs::remove_dir_all(&site);
}

#[test]
fn only_one_folder_holding_a_sites_settings_is_taken() {
    let given = |arguments: &[&PathBuf]| {
        explanation_pages::site_given(arguments.iter().map(|path| path.as_os_str().to_owned()))
    };
    let site = folder("given", true);
    let bare = folder("bare", false);
    assert_eq!(given(&[&site]), Some(site.clone()));
    assert_eq!(given(&[]), None);
    assert_eq!(given(&[&bare]), None);
    assert_eq!(given(&[&site, &site]), None);
    let _ = std::fs::remove_dir_all(&site);
    let _ = std::fs::remove_dir_all(&bare);
}

#[test]
fn the_starter_page_names_every_file_of_a_new_site() {
    let page = explanation_pages::starter_page();
    let (always, with_cards) = page
        .split_once("\n## What `statac new --cards` adds {#cards}\n")
        .expect("the page says what `--cards` adds");
    let plain = statac::new::files_of_a_new_site(false);
    assert!(plain.len() > 50);
    for (path, _) in &plain {
        let folder = explanation_pages::listed_under(path);
        let heading = match folder {
            "" => "\n## The site's folder ".to_owned(),
            folder => format!("\n## `{folder}` "),
        };
        let (_, after) = always.split_once(&heading).expect(path);
        let rows = after.split_once("\n## ").map_or(after, |(rows, _)| rows);
        let name = path.strip_prefix(folder).unwrap();
        assert!(rows.contains(&format!("| `{name}` |")), "{path}");
    }
    let mut added = 0;
    for (path, _) in statac::new::files_of_a_new_site(true) {
        if !plain.iter().any(|(known, _)| *known == path) {
            assert!(with_cards.contains(&format!("| `{path}` |")), "{path}");
            added += 1;
        }
    }
    assert!(added > 0);
    assert!(with_cards.contains("| `languages/<code>.yaml` |"));
    for (name, _) in statac::new::words() {
        assert!(with_cards.contains(&format!("| `{name}` |")), "{name}");
    }
    let tables: Vec<&str> = page
        .split("| File | What it is | Described in |\n")
        .collect();
    assert!(tables.len() > 10);
    for table in &tables[1..] {
        let (rows, _) = table.split_once("\n\n").unwrap_or((table, ""));
        assert!(rows.ends_with("\n{.ref}"), "{rows}");
    }
}

#[test]
fn the_addresses_printed_are_whole_pages_sorted_with_their_anchors() {
    let addresses = printed_addresses::addresses();
    assert!(addresses.is_sorted());
    for address in &addresses {
        let page = address
            .split_once('#')
            .map_or(address.as_str(), |(page, _)| page);
        let path = page.strip_prefix("https://statac.dev");
        assert!(
            path.is_some_and(|path| path.starts_with('/') && path.ends_with('/')),
            "{address}"
        );
    }
    let holds = |address: &str| addresses.iter().any(|known| known == address);
    assert!(addresses.iter().any(|address| address.contains('#')));
    // One the starter's files name, and one kind of problem's page.
    assert!(holds("https://statac.dev/lists/"));
    assert!(holds("https://statac.dev/e/broken-link/"));
    for address in explanation_pages::starter_addresses() {
        assert!(holds(&format!("https://statac.dev{address}")), "{address}");
    }
}
