//! Prints every address of the documentation that Statac prints or writes
//! into a new site, sorted, one to a line, for the docs to check each
//! answers: `cargo run --example printed_addresses`.

use std::path::Path;

#[allow(dead_code, reason = "only what the starter's files name is read")]
#[path = "explanation_pages.rs"]
pub mod explanation_pages;

const DOCS: &str = "https://statac.dev";

/// Every address under [`DOCS`] written anywhere in `folder`, without the
/// domain: `/feeds/#entry-ids`.
fn written(folder: &Path, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(folder).expect("the folder can be read") {
        let path = entry.expect("the folder can be read").path();
        if path.is_dir() {
            written(&path, found);
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for (at, _) in text.match_indices(DOCS) {
            found.push(
                text[at + DOCS.len()..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || "/#-_".contains(*c))
                    .collect(),
            );
        }
    }
}

/// Every address, whole and sorted, each once: those written in `src/`,
/// the page of each kind of problem, and those the starter's files name.
pub fn addresses() -> Vec<String> {
    let mut found = Vec::new();
    written(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    for code in statac::problem::Code::ALL {
        found.push(format!("/e/{}/", code.name()));
    }
    found.extend(explanation_pages::starter_addresses());
    found.sort();
    found.dedup();
    found
        .iter()
        .map(|address| format!("{DOCS}{address}"))
        .collect()
}

fn main() {
    for address in addresses() {
        println!("{address}");
    }
}
