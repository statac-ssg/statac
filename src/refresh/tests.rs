use super::*;

#[test]
fn vimeo_names_its_still_in_a_list_of_one() {
    let named = |json: &str| named_still(json.as_bytes());
    assert_eq!(
        named(r#"[{"id": 123456789, "thumbnail_large": "https://i.vimeocdn.com/video/1-d_640"}]"#),
        Some("https://i.vimeocdn.com/video/1-d_640".into())
    );
    for json in [
        r#"[{"id": 123456789}]"#,
        r#"[{"thumbnail_large": null}]"#,
        r#"[{"thumbnail_large": ""}]"#,
        r#"[{"thumbnail_large": 640}]"#,
        r#"{"thumbnail_large": "https://i.vimeocdn.com/video/1-d_640"}"#,
        r#"[{"thumbnail_large": "a"}, {"thumbnail_large": "b"}]"#,
        "[",
        "",
    ] {
        assert_eq!(named(json), None, "{json}");
    }
    assert_eq!(named_still(b"\xff[]"), None);
}

/// A site holding `token` as its `.statac/token`, where it is given.
fn site_with_token(label: &str, token: Option<&[u8]>) -> std::path::PathBuf {
    let folder = std::env::temp_dir()
        .join(format!("statac-token-{}", std::process::id()))
        .join(label);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(folder.join(".statac")).unwrap();
    if let Some(token) = token {
        std::fs::write(folder.join(TOKEN_FILE), token).unwrap();
    }
    folder
}

fn text_of(token: Option<Token>) -> Option<String> {
    token.map(|token| token.text().to_owned())
}

#[test]
fn the_token_is_the_variables_where_it_is_set_and_else_the_files_one_line() {
    let folder = site_with_token("both", Some(b"from-the-file\n"));
    let (given, said) = super::token(&folder, Some("from-the-variable".into()));
    assert_eq!(text_of(given).as_deref(), Some("from-the-variable"));
    assert_eq!(said, None);

    for written in [
        &b"from-the-file"[..],
        b"  from-the-file \r\n",
        b"\nfrom-the-file\n\n",
    ] {
        let folder = site_with_token("file", Some(written));
        for variable in [None, Some(String::new())] {
            let (given, said) = super::token(&folder, variable);
            assert_eq!(
                text_of(given).as_deref(),
                Some("from-the-file"),
                "{written:?}"
            );
            assert_eq!(said, None);
        }
    }

    let folder = site_with_token("neither", None);
    let (given, said) = super::token(&folder, None);
    assert_eq!(text_of(given), None);
    assert_eq!(said, None);
}

#[test]
fn a_token_file_empty_or_of_more_than_one_line_is_said_and_not_used() {
    let cases = [
        (&b""[..], "`.statac/token` wasn't used: it is empty."),
        (b" \n\n", "`.statac/token` wasn't used: it is empty."),
        (
            b"secret-1\nsecret-2\n",
            "`.statac/token` wasn't used: it holds more than one line.",
        ),
        (
            b"\xffsecret-1",
            "`.statac/token` wasn't used: it isn't UTF-8 text.",
        ),
    ];
    for (written, line) in cases {
        let folder = site_with_token("unused", Some(written));
        let (given, said) = super::token(&folder, None);
        assert_eq!(text_of(given), None, "{written:?}");
        assert_eq!(said.as_deref(), Some(line), "{written:?}");
        assert!(!line.contains("secret"));
    }
}

#[cfg(unix)]
#[test]
fn a_token_file_that_is_a_link_or_behind_one_is_said_and_not_used() {
    use std::os::unix::fs::symlink;
    let outside = site_with_token("outside", Some(b"secret-1\n"));
    let folder = site_with_token("linked-file", None);
    symlink(outside.join(TOKEN_FILE), folder.join(TOKEN_FILE)).unwrap();
    let (given, said) = super::token(&folder, None);
    assert_eq!(text_of(given), None);
    assert_eq!(
        said.as_deref(),
        Some("`.statac/token` wasn't used: it is a link.")
    );

    let folder = site_with_token("linked-folder", None);
    std::fs::remove_dir(folder.join(".statac")).unwrap();
    symlink(outside.join(".statac"), folder.join(".statac")).unwrap();
    let (given, said) = super::token(&folder, None);
    assert_eq!(text_of(given), None);
    assert_eq!(
        said.as_deref(),
        Some("`.statac/token` wasn't used: `.statac` is a link.")
    );
}
