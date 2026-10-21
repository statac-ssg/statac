use super::*;

use std::io::Cursor;

use crate::testing::TempSite;

fn framed(message: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{message}", message.len())
}

/// Runs a session of `messages` and gives what the server sent, each read
/// back, and its exit code.
fn session(messages: &[String]) -> (Vec<Message>, u8) {
    let input: String = messages.iter().map(|message| framed(message)).collect();
    let mut output = Vec::new();
    let code = run(&mut Cursor::new(input.into_bytes()), &mut output);
    let mut sent = Cursor::new(output);
    let mut read = Vec::new();
    while let Some(text) = wire::read(&mut sent).unwrap() {
        read.push(Message::read(text).expect("the server sends JSON"));
    }
    (read, code)
}

fn initialize(encodings: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\
         \"params\":{{\"capabilities\":{{\"general\":{{\"positionEncodings\":[{encodings}]}}}}}}}}"
    )
}

fn opened(method: &str, file: &Path) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/{method}\",\
         \"params\":{{\"textDocument\":{{\"uri\":{}}}}}}}",
        json::string(&uri::of(file))
    )
}

const SHUT_DOWN: &str = "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"shutdown\"}";
const EXIT: &str = "{\"jsonrpc\":\"2.0\",\"method\":\"exit\"}";

/// Each diagnostic published for `file`, in the last message about it, as
/// its code, line and character.
fn told(sent: &[Message], file: &Path) -> Vec<(String, String, String)> {
    let uri = uri::of(file);
    let Some(last) = (sent.iter().rev()).find(|message| {
        message.text(&["method"]) == Some("textDocument/publishDiagnostics")
            && message.text(&["params", "uri"]) == Some(uri.as_str())
    }) else {
        return Vec::new();
    };
    let Some(Value::Sequence(diagnostics)) = last.at(&["params", "diagnostics"]).map(|n| &n.value)
    else {
        panic!("no list of diagnostics");
    };
    let text = |node: &Node, path: &[&str]| {
        let found = path.iter().try_fold(node, |node, key| node.get(key));
        match found.map(|node| &node.value) {
            Some(Value::Scalar(text)) => text.clone(),
            _ => String::new(),
        }
    };
    (diagnostics.iter())
        .map(|d| {
            (
                text(d, &["code"]),
                text(d, &["range", "start", "line"]),
                text(d, &["range", "start", "character"]),
            )
        })
        .collect()
}

fn site() -> TempSite {
    TempSite::site(&[
        ("statac.yaml", "language: en\ntags.permalnk: /a/\n"),
        (
            "content/post-1.md",
            "---\ntitle: \"Café 😀\"\ndate: 3/4/2026\n---\nA post.\n",
        ),
    ])
}

#[test]
fn a_message_is_read_after_its_header_and_a_blank_line_before_it_is_passed_over() {
    let text = format!("\r\n{}{}", framed("{\"a\":1}"), framed("[]"));
    let mut input = Cursor::new(text.into_bytes());
    assert_eq!(
        wire::read(&mut input).unwrap().as_deref(),
        Some("{\"a\":1}")
    );
    assert_eq!(wire::read(&mut input).unwrap().as_deref(), Some("[]"));
    assert_eq!(wire::read(&mut input).unwrap(), None);
}

#[test]
fn a_length_that_isnt_a_number_or_is_too_long_ends_the_reading() {
    for header in ["Content-Length: many", "Content-Length: 999999999999"] {
        let mut input = Cursor::new(format!("{header}\r\n\r\n{{}}").into_bytes());
        assert!(wire::read(&mut input).is_err(), "{header}");
    }
}

#[test]
fn a_position_counts_characters_as_the_editor_asked() {
    let mut sources = Sources::default();
    let id = sources.add("a.md", "\u{feff}é😀x\nb".to_owned());
    let x = "\u{feff}é😀".len();
    assert_eq!(
        diagnostics::position(&sources, id, x, Encoding::Utf8),
        (0, 6)
    );
    assert_eq!(
        diagnostics::position(&sources, id, x, Encoding::Utf16),
        (0, 3)
    );
    let b = "\u{feff}é😀x\n".len();
    assert_eq!(
        diagnostics::position(&sources, id, b, Encoding::Utf16),
        (1, 0)
    );
}

#[test]
fn opening_a_file_tells_every_problem_in_the_site_where_it_is_written() {
    let site = site();
    let page = site.root.join("content/post-1.md");
    let settings = site.root.join("statac.yaml");
    for encodings in ["\"utf-16\"", "\"utf-8\",\"utf-16\""] {
        let (sent, code) = session(&[
            initialize(encodings),
            opened("didOpen", &page),
            SHUT_DOWN.into(),
            EXIT.into(),
        ]);
        assert_eq!(code, 0);
        let utf8 = encodings.contains("utf-8");
        let chosen = sent[0].text(&["result", "capabilities", "positionEncoding"]);
        assert_eq!(chosen, Some(if utf8 { "utf-8" } else { "utf-16" }));
        let on_page = told(&sent, &page);
        assert!(
            on_page.contains(&("unclear-date".into(), "2".into(), "6".into())),
            "{on_page:?}"
        );
        let in_settings = told(&sent, &settings);
        assert!(
            in_settings.contains(&("unknown-word".into(), "1".into(), "0".into())),
            "{in_settings:?}"
        );
    }
}

#[test]
fn a_problem_gone_on_saving_is_taken_away() {
    let site = site();
    let page = site.root.join("content/post-1.md");
    let mut output = Vec::new();
    let mut server = Server {
        output: &mut output,
        encoding: Encoding::Utf16,
        initialised: false,
        shut_down: false,
        sites: HashMap::new(),
    };
    let mut answer = |message: String| {
        let message = Message::read(message).unwrap();
        server.answer(&message).unwrap();
    };
    answer(initialize("\"utf-16\""));
    answer(opened("didOpen", &page));
    std::fs::write(
        &page,
        "---\ntitle: Post 1\ndate: 2026-04-03\n---\nA post.\n",
    )
    .unwrap();
    answer(opened("didSave", &page));
    let mut sent = Cursor::new(output);
    let mut read = Vec::new();
    while let Some(text) = wire::read(&mut sent).unwrap() {
        read.push(Message::read(text).unwrap());
    }
    let uri = uri::of(&page);
    let about_page: Vec<usize> = (0..read.len())
        .filter(|&at| read[at].text(&["params", "uri"]) == Some(uri.as_str()))
        .collect();
    assert_eq!(about_page.len(), 2, "told of its problems, then again");
    let dated = |up_to: usize| {
        let told = told(&read[..=up_to], &page);
        told.iter().any(|(code, ..)| code == "unclear-date")
    };
    assert!(dated(about_page[0]));
    assert!(!dated(about_page[1]));
}

#[test]
fn what_the_server_cant_answer_is_answered_with_the_protocols_error() {
    let unknown = "{\"jsonrpc\":\"2.0\",\"id\":\"a\",\"method\":\"textDocument/hover\"}";
    let (sent, code) = session(&[
        unknown.into(),
        "{not json".into(),
        initialize("\"utf-16\""),
        unknown.into(),
        EXIT.into(),
    ]);
    // Exit without shutting down first.
    assert_eq!(code, 1);
    let errors: Vec<_> = (sent.iter())
        .filter_map(|m| Some((m.id()?.to_owned(), m.text(&["error", "code"])?.to_owned())))
        .collect();
    assert_eq!(
        errors,
        [
            ("\"a\"".to_owned(), "-32002".to_owned()),
            ("null".to_owned(), "-32700".to_owned()),
            ("\"a\"".to_owned(), "-32601".to_owned()),
        ]
    );
}

#[test]
fn a_file_in_no_site_is_left_alone() {
    let folder = crate::testing::Scratch::new("lsp-no-site");
    let file = folder.write("a.md", "# A\n");
    let (sent, code) = session(&[
        initialize("\"utf-16\""),
        opened("didOpen", &file),
        SHUT_DOWN.into(),
        EXIT.into(),
    ]);
    assert_eq!(code, 0);
    assert_eq!(sent.len(), 2, "the answers to initialize and shutdown only");
}
