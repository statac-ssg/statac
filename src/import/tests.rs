use std::path::Path;

use super::report::Shown;
use super::write::{self, Undo};
use super::yaml::{self, Value};
use super::{Field, Import, Page, PageKind};
use crate::date::Date;

fn text(t: &str) -> Value {
    Value::Text(t.into())
}

#[test]
fn every_value_reads_back_as_written() {
    let awkward = [
        text("yes"),
        text("No"),
        text("on"),
        text("y"),
        text("1984"),
        text("3.10"),
        text("010"),
        text("0x10"),
        text("1e3"),
        text(".inf"),
        text("true"),
        text("null"),
        text("~"),
        text(""),
        text(" leading space"),
        text("trailing space "),
        text("a: b"),
        text("a:b"),
        text("@name"),
        text("`code`"),
        text("#F2E9D3"),
        text("a # b"),
        text("- item"),
        text("-"),
        text("- - item"),
        text("? item"),
        text("? a: b"),
        text("[1, 2]"),
        text("{a: 1}"),
        text("'quoted'"),
        text("\"quoted\""),
        text("back\\slash"),
        text("two\nlines"),
        text("tab\there"),
        text("*star"),
        text("&anchor"),
        text("!tag"),
        text("%percent"),
        text("|"),
        text(">"),
        text("?"),
        text("2026-10-04"),
        text("Post 1"),
        text("café"),
        text("\u{7}bell"),
        Value::Number("1984".into()),
        Value::Number("3.10".into()),
        Value::Number("-7".into()),
        Value::Truth(true),
        Value::Truth(false),
        Value::Nothing,
        Value::List(vec![]),
        Value::Fields(vec![]),
        Value::List(vec![text("yes"), Value::Number("2".into()), Value::Nothing]),
        Value::List(vec![Value::List(vec![text("a"), text("b: c")])]),
        Value::Fields(vec![
            ("author".into(), text("Name")),
            ("isbn".into(), text("0-000-00000-0")),
            ("links".into(), Value::List(vec![text("a"), text("b")])),
        ]),
        Value::List(vec![
            Value::Fields(vec![
                ("title".into(), text("Link 1")),
                ("tags".into(), Value::List(vec![text("x")])),
            ]),
            Value::Fields(vec![("title".into(), text("yes"))]),
        ]),
    ];
    for value in awkward {
        // Alone, as one of a list, and as a field of one of a list: what
        // reads as text after `field:` can read as more after a `-`.
        let listed = Value::List(vec![value.clone()]);
        let nested = Value::List(vec![Value::Fields(vec![("name".into(), value.clone())])]);
        for value in [value, listed, nested] {
            let written = yaml::fields([("field", &value)]);
            let read = yaml::read_back(&written);
            assert_eq!(
                read,
                Some(vec![("field".to_owned(), value.clone())]),
                "{written}"
            );
        }
    }
}

#[test]
fn every_name_reads_back_as_written() {
    for name in [
        "title",
        "pages.permalink",
        "1984",
        "yes",
        "a b",
        "a: b",
        "@name",
        "#name",
        "-name",
        "name:",
    ] {
        let value = text("x");
        let written = yaml::fields([(name, &value)]);
        assert_eq!(
            yaml::read_back(&written),
            Some(vec![(name.to_owned(), value.clone())]),
            "{written}"
        );
    }
}

#[test]
fn text_is_quoted_only_where_it_must_be() {
    let cases = [
        ("Post 1", "Post 1"),
        ("a:b", "a:b"),
        ("2026-10-04", "2026-10-04"),
        ("yes", "\"yes\""),
        ("1984", "\"1984\""),
        ("3.10", "\"3.10\""),
        ("a: b", "\"a: b\""),
        ("@name", "\"@name\""),
        ("", "\"\""),
    ];
    for (given, written) in cases {
        assert_eq!(yaml::text_value(given), written, "{given}");
    }
}

fn scratch(label: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir()
        .join(format!("statac-import-unit-{}", std::process::id()))
        .join(label);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn a_site() -> Import {
    let page = |n: usize| Page {
        from: format!("_posts/2026-10-0{n}-post-{n}.md"),
        to: format!("posts/post-{n}.md"),
        kind: PageKind::Post,
        front: vec![Field::new("title", text(&format!("Post {n}")), Some(2))],
        writing: "A post that is never written.\n".into(),
        fences: Vec::new(),
        terms: Vec::new(),
        calls: Vec::new(),
        address: Some(format!("/post-{n}/")),
        own_address: false,
        other_addresses: Vec::new(),
    };
    Import {
        pages: vec![page(1), page(2)],
        ..Import::default()
    }
}

/// Imports `a_site` into `folder`, with the starter where `starter` says,
/// with every file after the first `allowed` refused, and undoes what it
/// wrote, as `write::site` does.
fn stopped_after(old: &Path, folder: &Path, allowed: usize, starter: bool) -> bool {
    let shown = Shown {
        tool: "Jekyll",
        old: "old",
        new: "new",
    };
    let undo = Undo::before(folder);
    let mut written = 0;
    let mut refusing = |root: &Path, rel: &str, bytes: &[u8]| {
        written += 1;
        match written > allowed {
            true => Err(std::io::Error::from(std::io::ErrorKind::StorageFull)),
            false => crate::platform::make_file(root, rel, bytes),
        }
    };
    let made = write::write(
        old,
        folder,
        a_site(),
        Date::day_only(2026, 10, 6),
        &shown,
        starter,
        &mut refusing,
    );
    if made.is_err() {
        undo.undo(folder);
    }
    made.is_err()
}

#[test]
fn a_site_stopped_part_way_leaves_nothing() {
    let top = scratch("stopped");
    let old = top.join("old");
    std::fs::create_dir_all(&old).unwrap();
    for starter in [true, false] {
        for allowed in [0, 1, 2, 3] {
            let outer = top.join(format!("new-{allowed}-{starter}"));
            assert!(
                stopped_after(&old, &outer.join("site"), allowed, starter),
                "{allowed}"
            );
            assert!(!outer.exists(), "{allowed}");
        }
        let empty = top.join(format!("empty-{starter}"));
        std::fs::create_dir_all(&empty).unwrap();
        assert!(stopped_after(&old, &empty, 2, starter));
        assert_eq!(std::fs::read_dir(&empty).unwrap().count(), 0);
    }
}
