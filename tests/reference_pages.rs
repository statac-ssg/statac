//! The documentation's reference pages: a page for each group of names
//! Statac describes, written into the folder given, and the page listing
//! them.

use std::path::PathBuf;

use statac::described::Group;

#[allow(dead_code, reason = "the tests call what `main` calls")]
#[path = "../examples/reference_pages.rs"]
mod reference_pages;

/// An empty folder of this test's own, holding a site's settings.
fn folder(name: &str) -> PathBuf {
    let folder =
        std::env::temp_dir().join(format!("statac-reference-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("statac.yaml"), "title: Site\n").unwrap();
    folder
}

#[test]
fn the_pages_are_written_into_the_folder_given_and_one_of_no_group_is_removed() {
    let site = folder("written");
    let pages = reference_pages::folder(&site);
    std::fs::create_dir_all(&pages).unwrap();
    std::fs::write(
        pages.join("no-such-group.md"),
        "---\n# Written from descriptions/no-such-group.md by `cargo run --example \
         reference_pages`.\ntitle: Gone\n---\n",
    )
    .unwrap();
    // A page written by hand beside them stays.
    let by_hand = "---\ntitle: Templates\n---\nWhich template builds a page.\n";
    std::fs::write(pages.join("templates.md"), by_hand).unwrap();
    reference_pages::write(&site);
    assert_eq!(
        std::fs::read_to_string(pages.join("templates.md")).unwrap(),
        by_hand
    );
    std::fs::remove_file(pages.join("templates.md")).unwrap();

    let wanted = reference_pages::pages();
    assert_eq!(wanted.len(), Group::ALL.len() + 1);
    for (name, text) in &wanted {
        assert_eq!(std::fs::read_to_string(pages.join(name)).unwrap(), *text);
        assert!(
            text.starts_with("---\n# Written from descriptions/"),
            "{name}"
        );
        assert!(text.contains("\ntitle: "), "{name}");
    }
    assert_eq!(std::fs::read_dir(&pages).unwrap().count(), wanted.len());
    let _ = std::fs::remove_dir_all(&site);
}

#[test]
fn each_name_has_a_heading_of_its_own_and_each_group_is_listed() {
    let pages = reference_pages::pages();
    let text = |name: &str| {
        let found = pages.iter().find(|(file, _)| file == name);
        &found.unwrap_or_else(|| panic!("{name}")).1
    };
    let index = text("index.md");
    for &group in Group::ALL {
        let page = text(&format!("{}.md", group.name()));
        let summary = format!("\nsummary: {}\n", group.summary());
        assert!(page.contains(&summary), "{group:?}");
        assert!(page.contains(group.about()), "{group:?}");
        for described in group.described() {
            let id = reference_pages::id(&described.name);
            let heading = format!(
                "\n## `{}` {{#{id}}}\n\n{}\n",
                described.name, described.text
            );
            assert!(page.contains(&heading), "{heading}");
            if let Some(more) = &described.more {
                let at = more.strip_prefix("https://statac.dev").unwrap();
                assert!(page.contains(&format!("({at})")), "{more}");
            }
        }
        let link = format!("- [{}](/reference/{}/): ", group.title(), group.name());
        assert!(index.contains(&link), "{link}");
    }
    assert_eq!(
        reference_pages::id("types.<type>.<name>"),
        "types-type-name"
    );
    assert_eq!(reference_pages::id("og_title"), "og_title");
}
