//! crates.io shows `crates-io.md` in place of `README.md`, and the two say
//! the same of what Statac is and how Cargo installs it.

fn read(name: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(name)).unwrap()
}

/// The paragraph or fenced block that opens with `start`, up to the blank line
/// after it.
fn passage<'a>(text: &'a str, start: &str, name: &str) -> &'a str {
    let at = text
        .find(&format!("\n{start}"))
        .unwrap_or_else(|| panic!("{name} has no passage opening with {start:?}"))
        + 1;
    let rest = &text[at..];
    rest.find("\n\n").map_or(rest, |end| &rest[..end])
}

#[test]
fn the_readme_for_crates_io_says_what_the_readme_does() {
    let manifest = read("Cargo.toml");
    let tagline = manifest
        .lines()
        .find_map(|line| line.strip_prefix("description = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("Cargo.toml has a description");
    let readme = read("README.md");
    let crates_io = read("crates-io.md");
    for start in [
        &format!("<p align=\"center\">{tagline}</p>"),
        "Statac makes a website",
        "It is one program",
        "While `statac serve` runs",
        "```sh\ncargo install",
        "Cargo builds Statac",
    ] {
        assert_eq!(
            passage(&crates_io, start, "crates-io.md"),
            passage(&readme, start, "README.md"),
            "crates-io.md and README.md differ in the passage opening with {start:?}"
        );
    }
}
