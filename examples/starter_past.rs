//! Keeps each starter file as a past version, in `starter/past/<release>/`
//! with this release's number, where no past version of it has its bytes,
//! and writes the table of every past version, `src/new/past.rs`:
//! `cargo run --example starter_past`, run as a release is cut. Running it
//! again changes nothing.

use std::path::{Path, PathBuf};

/// Every file under `folder`, by its path in it with forward slashes.
fn files_under(folder: &Path) -> Vec<String> {
    fn walk(root: &Path, at: &Path, into: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(at) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("the folder can be listed").path();
            if path.is_dir() {
                walk(root, &path, into);
            } else {
                let rel = path
                    .strip_prefix(root)
                    .expect("the path is under the folder");
                let pieces: Vec<String> = rel
                    .components()
                    .map(|piece| piece.as_os_str().to_string_lossy().into_owned())
                    .collect();
                into.push(pieces.join("/"));
            }
        }
    }
    let mut paths = Vec::new();
    walk(folder, folder, &mut paths);
    paths.sort();
    paths
}

/// A release's number as numbers, so that `0.10.0` comes after `0.9.0`.
fn numbered(release: &str) -> Vec<u64> {
    release
        .split(['.', '-', '+'])
        .map(|piece| piece.parse().unwrap_or(0))
        .collect()
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let starter = root.join("starter");
    let past = starter.join("past");
    let release = env!("CARGO_PKG_VERSION");

    // Each past version as (release, path).
    let mut kept: Vec<(String, String)> = Vec::new();
    for path in files_under(&past) {
        let Some((release, path)) = path.split_once('/') else {
            continue;
        };
        kept.push((release.to_owned(), path.to_owned()));
    }
    let at = |release: &str, path: &str| -> PathBuf { past.join(release).join(path) };

    let site = starter.join("site");
    for path in files_under(&site) {
        let bytes = std::fs::read(site.join(&path)).expect("readable");
        let had = (kept.iter())
            .any(|(r, p)| *p == path && std::fs::read(at(r, p)).ok() == Some(bytes.clone()));
        if had {
            continue;
        }
        let to = at(release, &path);
        assert!(
            !to.exists(),
            "starter/past/{release}/{path} holds other bytes: raise the version in Cargo.toml \
             before keeping this starter"
        );
        std::fs::create_dir_all(to.parent().expect("a file has a folder")).expect("made");
        std::fs::write(&to, &bytes).expect("written");
        println!("Kept starter/site/{path} as starter/past/{release}/{path}");
        kept.push((release.to_owned(), path));
    }

    kept.sort_by(|a, b| (numbered(&a.0), &a.0, &a.1).cmp(&(numbered(&b.0), &b.0, &b.1)));
    let entries: String = kept
        .iter()
        .map(|(release, path)| format!("    {release:?} {path:?},\n"))
        .collect();
    let table = match entries.is_empty() {
        true => "past! {}\n".to_owned(),
        false => format!("past! {{\n{entries}}}\n"),
    };
    let file = root.join("src/new/past.rs");
    let text = std::fs::read_to_string(&file).expect("the table can be read");
    let start = text.rfind("past! {").expect("the table is there");
    std::fs::write(&file, format!("{}{table}", &text[..start])).expect("the table is written");
}
