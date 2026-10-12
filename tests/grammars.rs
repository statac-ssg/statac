//! The colours Statac's styles give code are the ones their themes give, and
//! every grammar and theme carried has its notice.

#[allow(dead_code, reason = "the tests use what is written, not `main`")]
#[path = "../examples/grammars.rs"]
mod grammars;

#[test]
fn the_styles_written_are_the_ones_the_themes_give() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/highlight/styles.rs");
    let written = std::fs::read_to_string(path).unwrap();
    assert!(
        written.replace("\r\n", "\n") == grammars::styles_rs(),
        "src/highlight/styles.rs differs from its themes; \
         run `cargo run --example grammars -- <tm-grammars>` to write it again"
    );
}

#[test]
fn every_grammar_and_theme_carried_has_its_notice() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let licences = std::fs::read_to_string(root.join("src/highlight/licences.md")).unwrap();
    let notices = std::fs::read_to_string(grammars::notices_path()).unwrap();
    let texts = grammars::texts(&notices);
    let again = "run `cargo run --example grammars -- <tm-grammars>` to write it again";
    for heading in grammars::LISTS {
        let carried = grammars::listed(&licences, heading);
        let noticed = grammars::listed(&notices, heading);
        assert!(!carried.is_empty(), "nothing under {heading}");
        assert!(
            carried.len() == noticed.len(),
            "THIRD-PARTY-GRAMMARS.md lists {} under {heading} and licences.md {}; {again}",
            noticed.len(),
            carried.len()
        );
        for (line, with) in carried.iter().zip(noticed) {
            let numbers: Option<Vec<usize>> = with
                .strip_prefix(line)
                .and_then(|rest| rest.strip_prefix(' '))
                .map(|rest| rest.split(' ').collect::<Vec<_>>())
                .and_then(|said| {
                    said.iter()
                        .map(|said| said.strip_prefix('[')?.strip_suffix(']')?.parse().ok())
                        .collect()
                });
            assert!(
                numbers.is_some_and(|numbers| numbers.iter().all(|number| texts.contains(number))),
                "THIRD-PARTY-GRAMMARS.md gives no notice for `{line}`; {again}"
            );
        }
    }
}
