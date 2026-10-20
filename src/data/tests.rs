use super::*;
use crate::testing::TempSite;

fn read_data(files: &[(&str, &str)]) -> (Sources, Data, Vec<Problem>) {
    let site = TempSite::new(files);
    let mut sources = Sources::default();
    let (data, problems) = read(&site.root, &mut sources);
    (sources, data, problems)
}

/// The text a name leads to, as written.
fn written(sources: &Sources, data: &Data, names: &str) -> Option<String> {
    let names: Vec<&str> = names.split('.').collect();
    match data.reach(&names)? {
        Reached::Value { node, .. } => Some(sources.spelling(node.place).to_owned()),
        Reached::Folder(folder) => Some(folder.path.clone()),
        Reached::Unread(id) => Some(format!("unread {}", data.file(id).path)),
    }
}

#[test]
fn files_are_named_by_their_folders_and_stems() {
    let (sources, data, problems) = read_data(&[
        ("data/people.yaml", "- name: Person 1\n"),
        ("data/lists/staff.yml", "lead: Person 2\n"),
        ("data/feeds.json", "{\"feed-1\": {\"title\": \"Feed 1\"}}\n"),
        ("data/.hidden.yaml", "a: 1\n"),
    ]);
    assert_eq!(problems, []);
    let cases = [
        ("people", Some("- name: Person 1")),
        ("lists", Some("data/lists/")),
        ("lists.staff.lead", Some("Person 2")),
        ("feeds.feed-1.title", Some("\"Feed 1\"")),
        ("people.name", None),
        ("staff", None),
        ("hidden", None),
        (".hidden", None),
    ];
    for (names, expected) in cases {
        assert_eq!(
            written(&sources, &data, names).as_deref(),
            expected,
            "{names}"
        );
    }
    assert_eq!(
        data.names(),
        [
            "feeds",
            "feeds.feed-1",
            "lists",
            "lists.staff",
            "lists.staff.lead",
            "people"
        ]
    );
}

#[test]
fn json_reads_as_its_own_reader_reads_it() {
    let (sources, data, problems) = read_data(&[(
        "data/feeds.json",
        "{\n\t\"a\":1,\n\t\"b\": [true, null, 2.5e3],\n\t\"c\": \"\\u00e9\\/\\ud83d\\ude00\",\n\t\"d\": {}\n}\n",
    )]);
    assert_eq!(problems, []);
    let Some(Reached::Value { node, .. }) = data.reach(&["feeds"]) else {
        panic!("no feeds");
    };
    use crate::yaml::Resolved;
    assert_eq!(
        node.get("a").unwrap().resolved(),
        Some(Resolved::Int(Some(1)))
    );
    let Yaml::Sequence(b) = &node.get("b").unwrap().value else {
        panic!("b is a list");
    };
    let read: Vec<_> = b.iter().map(Node::resolved).collect();
    assert_eq!(
        read,
        [
            Some(Resolved::Bool(true)),
            Some(Resolved::Null),
            Some(Resolved::Float(2500.0))
        ]
    );
    assert_eq!(
        node.get("c").unwrap().value,
        Yaml::Scalar("é/\u{1f600}".into())
    );
    assert_eq!(sources.spelling(node.get("d").unwrap().place), "{}");
}

#[test]
fn what_isnt_yaml_or_json_is_said() {
    let (_, data, problems) = read_data(&[
        ("data/notes.txt", "Notes\n"),
        ("data/README", "Notes\n"),
        ("data/people.yaml", "a: 1\n"),
    ]);
    let not_data = |path: &str| Problem::nowhere(Kind::NotData { path: path.into() });
    assert_eq!(
        problems,
        [not_data("data/README"), not_data("data/notes.txt")]
    );
    assert!(data.reach(&["people"]).is_some());
    assert!(data.reach(&["notes"]).is_none());
}

#[test]
fn two_things_at_one_name_are_refused() {
    let twice = |first: &str, second: &str, name: &str| {
        Problem::nowhere(Kind::DataNamedTwice {
            first: first.into(),
            second: second.into(),
            name: name.into(),
        })
    };
    type Files<'a> = &'a [(&'a str, &'a str)];
    let cases: [(Files, Vec<Problem>); 3] = [
        (
            &[("data/people.json", "{}"), ("data/people.yaml", "a: 1")],
            vec![twice(
                "data/people.json",
                "data/people.yaml",
                "site.data.people",
            )],
        ),
        (
            &[
                ("data/people.yaml", "a: 1"),
                ("data/people/staff.yaml", "a: 1"),
                ("data/people/other.yaml", "a: 1"),
            ],
            vec![twice(
                "data/people.yaml",
                "data/people/",
                "site.data.people",
            )],
        ),
        (
            &[
                ("data/lists/people.yml", "a: 1"),
                ("data/lists/people/staff.yaml", "a: 1"),
            ],
            vec![twice(
                "data/lists/people.yml",
                "data/lists/people/",
                "site.data.lists.people",
            )],
        ),
    ];
    for (files, expected) in cases {
        let (_, _, problems) = read_data(files);
        assert_eq!(problems, expected, "{files:?}");
    }
}

#[test]
fn a_merge_key_is_refused_where_it_is() {
    let (sources, data, problems) = read_data(&[(
        "data/people.yaml",
        "base: &base\n  role: Role 1\npeople:\n  - name: Person 1\n    <<: *base\n  - \"<<\": kept\n",
    )]);
    let found: Vec<_> = problems
        .iter()
        .map(|p| {
            (
                p.kind.clone(),
                sources.spelling(p.place.unwrap()).to_owned(),
            )
        })
        .collect();
    assert_eq!(found, [(Kind::MergeKey, "<<".to_owned())]);
    assert_eq!(
        written(&sources, &data, "people.base.role").as_deref(),
        Some("Role 1")
    );
}

#[test]
fn a_file_that_cant_be_read_keeps_its_name() {
    let (_, data, problems) = read_data(&[("data/people.yaml", "a: [1\n")]);
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].kind.name(), "malformed-yaml");
    assert!(matches!(
        data.reach(&["people", "a"]),
        Some(Reached::Unread(_))
    ));
}

#[test]
fn a_site_with_no_data_folder_has_no_data() {
    let (_, data, problems) = read_data(&[("statac.yaml", "title: Site 1\n")]);
    assert_eq!(problems, []);
    assert!(data.top.entries.is_empty());
    assert!(data.files.is_empty());
}

#[test]
fn a_json_file_takes_controls_in_its_strings() {
    let (_, data, problems) = read_data(&[
        ("data/comments.json", "{\"a\": \"x\u{92}y\"}"),
        ("data/comments-2.yaml", "a: \"x\u{92}y\"\n"),
    ]);
    assert_eq!(problems.len(), 1);
    assert_eq!(
        problems[0].kind,
        Kind::MalformedYaml {
            found: crate::yaml::Malformed::ControlCharacter('\u{92}')
        }
    );
    let Some(Reached::Value { node, .. }) = data.reach(&["comments", "a"]) else {
        panic!("no comments");
    };
    assert_eq!(node.value, Yaml::Scalar("x\u{92}y".into()));
}
