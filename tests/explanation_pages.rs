//! The documentation's generated pages: a page for each kind of problem,
//! written into the folder given, and the list of the addresses Statac
//! prints.

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
    let _ = std::fs::remove_dir_all(&site);
}

#[test]
fn the_index_says_what_each_kind_is() {
    let pages = explanation_pages::pages();
    let (_, index) = pages.iter().find(|(name, _)| name == "index.md").unwrap();
    let kinds: Vec<&str> = index
        .lines()
        .filter(|line| line.starts_with("- ["))
        .collect();
    assert_eq!(kinds.len(), statac::problem::Code::ALL.len());
    for line in &kinds {
        assert!(line.contains("/): ") && line.ends_with('.'), "{line}");
    }
    // The sentence ends where a colon opens its detail, and a name keeps
    // its capital.
    for line in [
        "- [Malformed YAML](/e/malformed-yaml/): Statac couldn't read some YAML.",
        "- [Same id](/e/same-id/): two pages have the same `id:`.",
    ] {
        assert!(kinds.contains(&line), "{line}");
    }
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
    // One Statac prints, and one kind of problem's page.
    assert!(holds("https://statac.dev/templates/"));
    assert!(holds("https://statac.dev/e/broken-link/"));
}
