//! The list of other people's work a release carries names the crates
//! `Cargo.lock` does, each at its version, and the standard library of the
//! Rust `Cargo.toml` asks for.

#[allow(dead_code, reason = "the test reads the list, and `main` writes it")]
#[path = "../examples/third_party.rs"]
mod third_party;

#[test]
fn the_list_names_every_crate_the_lock_file_does_at_its_version() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap();
    let written = std::fs::read_to_string(third_party::path()).unwrap();
    let locked = third_party::locked(&lock);
    let listed = third_party::listed(&written);
    assert!(locked.len() > 100, "{} crates read", locked.len());
    let missing: Vec<_> = locked.difference(&listed).collect();
    let gone: Vec<_> = listed.difference(&locked).collect();
    assert!(
        missing.is_empty() && gone.is_empty(),
        "THIRD-PARTY.md differs from Cargo.lock: it lacks {missing:?} and still names {gone:?}; \
         run `cargo run --example third_party` to write it again"
    );
}

#[test]
fn the_list_names_the_standard_library_of_the_rust_the_manifest_asks_for() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let asked = manifest
        .lines()
        .find_map(|line| line.strip_prefix("rust-version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("Cargo.toml names a version of Rust");
    let written = std::fs::read_to_string(third_party::path()).unwrap();
    assert!(
        written.contains(&format!("- Rust's standard library {asked}.")),
        "THIRD-PARTY.md does not name the standard library of Rust {asked}; read what it \
         carries again, into `licences/rust-std/`, and run `cargo run --example third_party`"
    );
}
