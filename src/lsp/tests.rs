use super::*;

use std::io::{BufReader, Cursor};
use std::thread::JoinHandle;

use crate::testing::TempSite;

/// How long typing pauses before a site is checked, in these tests.
const TEST_PAUSE: Duration = Duration::from_millis(100);

/// The longest a test waits for the server.
const BOUND: Duration = Duration::from_secs(20);

fn framed(message: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{message}", message.len())
}

/// An editor, talking to a server on a thread of its own through pipes.
struct Editor {
    to_server: Option<std::io::PipeWriter>,
    heard: Receiver<Message>,
    /// Everything the server has sent, in order.
    sent: Vec<Message>,
    server: JoinHandle<u8>,
}

impl Editor {
    fn start() -> Editor {
        let (from_editor, to_server) = std::io::pipe().unwrap();
        let (from_server, mut to_editor) = std::io::pipe().unwrap();
        let every = Duration::from_millis(50);
        let server = std::thread::spawn(move || {
            let input = BufReader::new(from_editor);
            run_with(input, &mut to_editor, TEST_PAUSE, How::Poll { every })
        });
        let (tell, heard) = mpsc::channel();
        std::thread::spawn(move || {
            let mut from_server = BufReader::new(from_server);
            while let Ok(Some(text)) = wire::read(&mut from_server) {
                let message = Message::read(text).expect("the server sends JSON");
                if tell.send(message).is_err() {
                    return;
                }
            }
        });
        Editor {
            to_server: Some(to_server),
            heard,
            sent: Vec::new(),
            server,
        }
    }

    fn send(&mut self, message: &str) {
        self.send_all(&[message]);
    }

    /// Sends `messages` in one write, as an editor sends a burst.
    fn send_all(&mut self, messages: &[&str]) {
        let to_server = self.to_server.as_mut().expect("not ended");
        let framed: String = messages.iter().map(|message| framed(message)).collect();
        to_server.write_all(framed.as_bytes()).unwrap();
    }

    /// Waits for the server to send a message for which `wanted` holds,
    /// from the `since`th it sent on, and gives where it is among those
    /// sent.
    fn until(&mut self, since: usize, wanted: impl Fn(&Message) -> bool) -> usize {
        let began = Instant::now();
        let mut at = since;
        loop {
            if let Some(found) = self
                .sent
                .get(at..)
                .and_then(|sent| sent.iter().position(&wanted))
            {
                return at + found;
            }
            at = self.sent.len().max(since);
            let left = BOUND.saturating_sub(began.elapsed());
            match self.heard.recv_timeout(left) {
                Ok(message) => self.sent.push(message),
                Err(_) => panic!("never sent; sent:\n{}", self.shown()),
            }
        }
    }

    /// How many messages the server has sent so far, as heard.
    fn mark(&self) -> usize {
        self.sent.len()
    }

    /// Waits until the server sends diagnostics for `file` that `wanted`
    /// holds of, from the `since`th message it sent on, and gives them.
    fn told_until(
        &mut self,
        since: usize,
        file: &Path,
        wanted: impl Fn(&[Told]) -> bool,
    ) -> Vec<Told> {
        let uri = uri::of(file);
        let at = self.until(since, |message| {
            diagnostics_for(message, &uri).is_some_and(|told| wanted(&told))
        });
        diagnostics_for(&self.sent[at], &uri).unwrap()
    }

    /// Everything the server sends for `quiet`.
    fn hear_for(&mut self, quiet: Duration) -> Vec<&Message> {
        let from = self.sent.len();
        let began = Instant::now();
        while let Some(left) = quiet.checked_sub(began.elapsed()) {
            match self.heard.recv_timeout(left) {
                Ok(message) => self.sent.push(message),
                Err(_) => break,
            }
        }
        self.sent[from..].iter().collect()
    }

    /// Asks the server to shut down and exit, and gives its exit code.
    fn end(mut self, shut_down: bool) -> (Vec<Message>, u8) {
        if shut_down {
            self.send(SHUT_DOWN);
        }
        self.send(EXIT);
        drop(self.to_server.take());
        let code = self.server.join().unwrap();
        while let Ok(message) = self.heard.recv_timeout(Duration::from_millis(200)) {
            self.sent.push(message);
        }
        (self.sent, code)
    }

    fn shown(&self) -> String {
        let said = self.sent.iter().map(|message| {
            let method = message.text(&["method"]).unwrap_or("(an answer)");
            let uri = message.text(&["params", "uri"]).unwrap_or_default();
            format!("{method} {uri}\n")
        });
        said.collect()
    }
}

/// A diagnostic as its code, line and character.
type Told = (String, String, String);

/// The diagnostics `message` publishes for the file at `uri`, where it
/// publishes that file's.
fn diagnostics_for(message: &Message, uri: &str) -> Option<Vec<Told>> {
    if message.text(&["method"]) != Some("textDocument/publishDiagnostics")
        || message.text(&["params", "uri"]) != Some(uri)
    {
        return None;
    }
    let Some(Value::Sequence(diagnostics)) =
        message.at(&["params", "diagnostics"]).map(|n| &n.value)
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
    let told = diagnostics.iter().map(|d| {
        (
            text(d, &["code"]),
            text(d, &["range", "start", "line"]),
            text(d, &["range", "start", "character"]),
        )
    });
    Some(told.collect())
}

fn has(told: &[Told], code: &str) -> bool {
    told.iter().any(|(said, ..)| said == code)
}

fn initialize(encodings: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\
         \"params\":{{\"capabilities\":{{\"general\":{{\"positionEncodings\":[{encodings}]}}}}}}}}"
    )
}

fn opened(file: &Path) -> String {
    let text = std::fs::read_to_string(file).unwrap();
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didOpen\",\
         \"params\":{{\"textDocument\":{{\"uri\":{},\"languageId\":\"markdown\",\
         \"version\":1,\"text\":{}}}}}}}",
        json::string(&uri::of(file)),
        json::string(&text)
    )
}

fn changed(file: &Path, text: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/didChange\",\
         \"params\":{{\"textDocument\":{{\"uri\":{},\"version\":2}},\
         \"contentChanges\":[{{\"text\":{}}}]}}}}",
        json::string(&uri::of(file)),
        json::string(text)
    )
}

fn about(method: &str, file: &Path) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/{method}\",\
         \"params\":{{\"textDocument\":{{\"uri\":{}}}}}}}",
        json::string(&uri::of(file))
    )
}

const SHUT_DOWN: &str = "{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"shutdown\"}";
const EXIT: &str = "{\"jsonrpc\":\"2.0\",\"method\":\"exit\"}";

const DATED: &str = "---\ntitle: \"Café 😀\"\ndate: 3/4/2026\n---\nA post.\n";
const FIXED: &str = "---\ntitle: Post 1\ndate: 2026-04-03\n---\nA post.\n";

fn site() -> TempSite {
    TempSite::site(&[
        ("statac.yaml", "language: en\ntags.permalnk: /a/\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/post-1.md", DATED),
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
fn a_position_the_editor_gives_is_found_in_the_text_and_given_back() {
    let text = "\u{feff}é😀x\nb";
    let x = "\u{feff}é😀".len();
    for (encoding, character) in [(Encoding::Utf8, 6), (Encoding::Utf16, 3)] {
        assert_eq!(
            diagnostics::offset_in(text, 0, character, encoding),
            Some(x)
        );
        assert_eq!(diagnostics::position_in(text, x, encoding), (0, character));
    }
    let b = "\u{feff}é😀x\n".len();
    assert_eq!(diagnostics::offset_in(text, 1, 0, Encoding::Utf16), Some(b));
    assert_eq!(diagnostics::position_in(text, b, Encoding::Utf16), (1, 0));
    assert_eq!(
        diagnostics::offset_in(text, 0, 99, Encoding::Utf16),
        Some(b - 1),
        "past the end of its line"
    );
    assert_eq!(diagnostics::offset_in(text, 2, 0, Encoding::Utf16), None);
}

#[test]
fn opening_a_file_tells_every_problem_in_the_site_where_it_is_written() {
    let site = site();
    let page = site.root.join("content/post-1.md");
    let settings = site.root.join("statac.yaml");
    for encodings in ["\"utf-16\"", "\"utf-8\",\"utf-16\""] {
        let mut editor = Editor::start();
        editor.send(&initialize(encodings));
        editor.send(&opened(&page));
        let on_page = editor.told_until(0, &page, |told| has(told, "unclear-date"));
        let in_settings = editor.told_until(0, &settings, |told| has(told, "unknown-word"));
        let (sent, code) = editor.end(true);
        assert_eq!(code, 0);
        let utf8 = encodings.contains("utf-8");
        let chosen = sent[0].text(&["result", "capabilities", "positionEncoding"]);
        assert_eq!(chosen, Some(if utf8 { "utf-8" } else { "utf-16" }));
        let sync = sent[0].text(&["result", "capabilities", "textDocumentSync", "change"]);
        assert_eq!(sync, Some("1"), "the whole text with each change");
        assert!(
            on_page.contains(&("unclear-date".into(), "2".into(), "6".into())),
            "{on_page:?}"
        );
        assert!(
            in_settings.contains(&("unknown-word".into(), "1".into(), "0".into())),
            "{in_settings:?}"
        );
    }
}

#[test]
fn a_problem_typed_or_typed_away_is_told_once_typing_pauses_without_saving() {
    let site = site();
    let page = site.root.join("content/post-1.md");
    let mut editor = Editor::start();
    editor.send(&initialize("\"utf-16\""));
    editor.send(&opened(&page));
    editor.told_until(0, &page, |told| has(told, "unclear-date"));

    let mark = editor.mark();
    editor.send(&changed(&page, FIXED));
    editor.told_until(mark, &page, |told| !has(told, "unclear-date"));
    assert_eq!(std::fs::read_to_string(&page).unwrap(), DATED, "unsaved");

    // Placed in the text as typed, not as saved.
    let moved = "---\ntitle: Post 1\n\n\ndate: 4/3/2026\n---\nA post.\n";
    let mark = editor.mark();
    editor.send(&changed(&page, moved));
    let told = editor.told_until(mark, &page, |told| has(told, "unclear-date"));
    assert!(
        told.contains(&("unclear-date".into(), "4".into(), "6".into())),
        "{told:?}"
    );

    // Closed without saving, the file is as the disk has it.
    let mark = editor.mark();
    editor.send(&about("didClose", &page));
    let told = editor.told_until(mark, &page, |told| has(told, "unclear-date"));
    assert!(
        told.contains(&("unclear-date".into(), "2".into(), "6".into())),
        "{told:?}"
    );
    let (_, code) = editor.end(true);
    assert_eq!(code, 0);
}

#[test]
fn what_is_typed_before_a_pause_is_checked_once_as_it_stands_at_the_pause() {
    let site = site();
    let page = site.root.join("content/post-1.md");
    let mut editor = Editor::start();
    editor.send(&initialize("\"utf-16\""));
    editor.send(&opened(&page));
    editor.told_until(0, &page, |told| has(told, "unclear-date"));
    let broken = "---\ntitle: Post 1\ndate: 2026-04-03\n---\n[A link](/nowhere/).\n";
    editor.send_all(&[&changed(&page, broken), &changed(&page, FIXED)]);
    let heard = editor.hear_for(TEST_PAUSE * 10);
    let uri = uri::of(&page);
    let told: Vec<Vec<Told>> = (heard.iter())
        .filter_map(|message| diagnostics_for(message, &uri))
        .collect();
    assert_eq!(
        told,
        [Vec::<Told>::new()],
        "told once, of the text at the pause"
    );
    // And a broken link, found only by building the page, as typed.
    let mark = editor.mark();
    editor.send(&changed(&page, broken));
    let told = editor.told_until(mark, &page, |told| has(told, "broken-link"));
    assert_eq!(told, [("broken-link".into(), "4".into(), "9".into())]);
    editor.end(true);
}

#[test]
fn a_file_changed_on_disk_is_told_and_a_file_whose_problems_are_the_same_is_not_told_again() {
    let site = site();
    let page = site.root.join("content/post-1.md");
    let other = site.root.join("content/post-2.md");
    let mut editor = Editor::start();
    editor.send(&initialize("\"utf-16\""));
    editor.send(&opened(&page));
    editor.told_until(0, &page, |told| has(told, "unclear-date"));

    // Saved by another program: the site's folder is watched.
    let broken = "---\ntitle: Post 2\n---\n[A link](/nowhere/).\n";
    std::fs::write(&other, broken).unwrap();
    let told = editor.told_until(0, &other, |told| has(told, "broken-link"));
    assert_eq!(told.len(), 1, "{told:?}");

    // Saved as typed: nothing has changed to tell.
    editor.send(&about("didSave", &page));
    let heard = editor.hear_for(TEST_PAUSE * 10);
    let published = (heard.iter())
        .filter(|message| message.text(&["method"]) == Some("textDocument/publishDiagnostics"));
    assert_eq!(published.count(), 0);
    editor.end(true);
}

#[test]
fn what_the_server_cant_answer_is_answered_with_the_protocols_error() {
    let unknown = "{\"jsonrpc\":\"2.0\",\"id\":\"a\",\"method\":\"textDocument/formatting\"}";
    let mut editor = Editor::start();
    editor.send(unknown);
    editor.send("{not json");
    editor.send(&initialize("\"utf-16\""));
    editor.send(unknown);
    // Exit without shutting down first.
    let (sent, code) = editor.end(false);
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
    let mut editor = Editor::start();
    editor.send(&initialize("\"utf-16\""));
    editor.until(0, |message| message.id() == Some("1"));
    editor.send(&opened(&file));
    editor.send(&changed(&file, "# B\n"));
    let heard = editor.hear_for(TEST_PAUSE * 5).len();
    let (sent, code) = editor.end(true);
    assert_eq!(code, 0);
    assert_eq!(heard, 0, "nothing after the answer to initialize");
    assert_eq!(sent.len(), 2, "the answers to initialize and shutdown only");
}

#[test]
fn input_that_ends_ends_the_server() {
    let input = Cursor::new(framed(&initialize("\"utf-16\"")).into_bytes());
    let mut output = Vec::new();
    assert_eq!(run(input, &mut output), 1);
}

fn completion(id: u32, file: &Path, line: usize, character: usize) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"textDocument/completion\",\
         \"params\":{{\"textDocument\":{{\"uri\":{}}},\
         \"position\":{{\"line\":{line},\"character\":{character}}}}}}}",
        json::string(&uri::of(file))
    )
}

/// The completions an answer gives, each as its label, the characters its
/// edit starts and ends at on their line, and what it writes.
fn completions(answer: &Message) -> Vec<(String, String, String, String)> {
    let Some(Value::Sequence(items)) = answer.at(&["result", "items"]).map(|n| &n.value) else {
        return Vec::new();
    };
    let text = |node: &Node, path: &[&str]| {
        let found = path.iter().try_fold(node, |node, key| node.get(key));
        match found.map(|node| &node.value) {
            Some(Value::Scalar(text)) => text.clone(),
            _ => String::new(),
        }
    };
    let edit = ["textEdit", "range"];
    (items.iter())
        .map(|item| {
            (
                text(item, &["label"]),
                text(item, &[edit[0], edit[1], "start", "character"]),
                text(item, &[edit[0], edit[1], "end", "character"]),
                text(item, &["textEdit", "newText"]),
            )
        })
        .collect()
}

#[test]
fn completion_offers_what_the_site_holds_where_the_cursor_stands_in_unsaved_text() {
    let site = TempSite::site(&[
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ntags: [rust]\n---\n",
        ),
        ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
    ]);
    let page = site.root.join("content/post-2.md");
    let typed = "tags: [\"😀\", r";
    let text = format!("---\ntitle: Post 2\n{typed}\n---\n");
    let mut editor = Editor::start();
    editor.send(&initialize("\"utf-16\""));
    // Asked before the site has been read.
    let mark = editor.mark();
    editor.send_all(&[
        &opened(&page),
        &changed(&page, &text),
        &completion(2, &page, 2, typed.encode_utf16().count()),
        &completion(3, &page, 1, 2),
        &completion(4, &page, 4, 0),
    ]);
    let mut answers = Vec::new();
    for id in ["2", "3", "4"] {
        let at = editor.until(mark, |message| message.id() == Some(id));
        answers.push(at);
    }
    let (sent, code) = editor.end(true);
    assert_eq!(code, 0);
    let end = typed.encode_utf16().count().to_string();
    let start = (typed.encode_utf16().count() - 1).to_string();
    assert_eq!(
        completions(&sent[answers[0]]),
        [("rust".into(), start, end, "rust".into())]
    );
    // In the name `title`, typed as far as `ti`.
    let names = completions(&sent[answers[1]]);
    let tags = names.iter().find(|(label, ..)| label == "tags");
    assert_eq!(tags, None, "written already");
    let date = names.iter().find(|(label, ..)| label == "date").unwrap();
    assert_eq!(
        date,
        &("date".into(), "0".into(), "2".into(), "date".into())
    );
    let nothing = &sent[answers[2]];
    let result = nothing.at(&["result"]).unwrap();
    assert_eq!(
        nothing.sources.spelling(result.place),
        "null",
        "nothing offered in the writing"
    );
    let triggers = sent[0].at(&["result", "capabilities", "completionProvider"]);
    assert!(triggers.is_some());
}

/// The `initialize` request of an editor that fills in places in what a
/// completion writes, or one that doesn't.
fn initialize_filling(fills: bool) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\
         \"params\":{{\"capabilities\":{{\"textDocument\":{{\"completion\":\
         {{\"completionItem\":{{\"snippetSupport\":{fills}}}}}}}}}}}}}"
    )
}

#[test]
fn a_call_is_written_with_places_to_fill_only_for_an_editor_that_fills_them() {
    let site = TempSite::site(&[
        ("templates/page.html", "{{ page.content }}\n"),
        ("templates/list.html", "{{ ur }}\n"),
        (
            "snippets/figure.html",
            "{# takes photo #}\n{{ page.photo(photo).alt }}\n",
        ),
        ("content/post-1.md", "---\ntitle: Post 1\n---\n@fi\n"),
    ]);
    let page = site.root.join("content/post-1.md");
    let template = site.root.join("templates/list.html");
    let asked = [
        (
            &page,
            (3, 3),
            "figure",
            "\"figure(photo)\"",
            ["figure(\"$1\")", "figure("],
        ),
        (&template, (0, 5), "url", "\"url\"", ["url($1)", "url("]),
    ];
    for (file, (line, character), name, label, written) in asked {
        for (fills, written, format) in [(true, written[0], "2"), (false, written[1], "")] {
            let mut editor = Editor::start();
            editor.send(&initialize_filling(fills));
            let mark = editor.mark();
            editor.send_all(&[&opened(file), &completion(2, file, line, character)]);
            let at = editor.until(mark, |message| message.id() == Some("2"));
            let (sent, _) = editor.end(true);
            let Some(Value::Sequence(items)) = sent[at].at(&["result", "items"]).map(|n| &n.value)
            else {
                panic!("no items");
            };
            let call = (items.iter())
                .find(|item| {
                    let filter = item.get("filterText").or_else(|| item.get("label"));
                    filter.map(|n| &n.value) == Some(&Value::Scalar(name.into()))
                })
                .expect("the call");
            let said = |key: &str| sent[at].sources.spelling(call.get(key).unwrap().place);
            assert_eq!(said("label"), label);
            let edit = call.get("textEdit").unwrap().get("newText").unwrap();
            assert_eq!(edit.value, Value::Scalar(written.into()), "{name} {fills}");
            let given = call
                .get("insertTextFormat")
                .map(|n| sent[at].sources.spelling(n.place));
            assert_eq!(given.unwrap_or_default(), format, "{name} {fills}");
        }
    }
}

/// The `initialize` request of an editor that reads hovers in `formats`.
fn initialize_hovering(formats: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\
         \"params\":{{\"capabilities\":{{\"textDocument\":{{\"hover\":\
         {{\"contentFormat\":[{formats}]}}}}}}}}}}"
    )
}

fn hover(id: u32, file: &Path, line: usize, character: usize) -> String {
    at_position("textDocument/hover", id, file, line, character)
}

/// A request by `method` about the place at `line` and `character` in
/// `file`.
fn at_position(method: &str, id: u32, file: &Path, line: usize, character: usize) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"{method}\",\
         \"params\":{{\"textDocument\":{{\"uri\":{}}},\
         \"position\":{{\"line\":{line},\"character\":{character}}}}}}}",
        json::string(&uri::of(file))
    )
}

#[test]
fn a_hover_says_what_the_word_is_in_markdown_where_the_editor_reads_it() {
    let site = TempSite::site(&[
        ("statac.yaml", "title: Site 1\ncode_style: ocean\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/post-1.md", "---\ntitle: Post 1\n---\nA post.\n"),
    ]);
    let settings = site.root.join("statac.yaml");
    let page = site.root.join("content/post-1.md");
    let link = "https://statac.dev/settings/#writing-and-code";
    let more = [
        (
            "\"markdown\",\"plaintext\"",
            "markdown",
            format!("[More on statac.dev]({link})"),
        ),
        ("\"plaintext\"", "plaintext", format!("More: {link}")),
    ];
    for (formats, kind, more) in more {
        let mut editor = Editor::start();
        editor.send(&initialize_hovering(formats));
        let mark = editor.mark();
        editor.send_all(&[
            &opened(&settings),
            &hover(2, &settings, 1, 3),
            &hover(3, &page, 3, 2),
        ]);
        let at = editor.until(mark, |message| message.id() == Some("3"));
        let (sent, _) = editor.end(true);
        let offered = sent[0].at(&["result", "capabilities", "hoverProvider"]);
        assert_eq!(sent[0].sources.spelling(offered.unwrap().place), "true");
        let answer = (sent.iter())
            .find(|message| message.id() == Some("2"))
            .expect("the hover");
        assert_eq!(answer.text(&["result", "contents", "kind"]), Some(kind));
        let value = answer.text(&["result", "contents", "value"]).unwrap();
        let first = "`code_style`, a setting\n\nThe colours of code in the writing:";
        assert!(value.starts_with(first), "{value}");
        assert!(value.ends_with(&format!(".\n\n{more}")), "{value}");
        let range = ["result", "range"];
        let end = [&range[..], &["end", "character"]].concat();
        let end = answer.at(&end).unwrap();
        assert_eq!(answer.sources.spelling(end.place), "10");
        assert_eq!(
            sent[at]
                .sources
                .spelling(sent[at].at(&["result"]).unwrap().place),
            "null"
        );
    }
}

#[test]
fn going_to_a_definition_opens_the_file_and_place_what_is_under_the_cursor_names() {
    let site = TempSite::site(&[
        ("statac.yaml", "title: Site 1\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "content/about.md",
            "---\ntitle: About\n---\nA page.\n\n## The team\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\nSee [the team](/about/#the-team).\n",
        ),
    ]);
    let page = site.root.join("content/post-1.md");
    let mut editor = Editor::start();
    editor.send(&initialize(""));
    let mark = editor.mark();
    editor.send_all(&[
        &opened(&page),
        &at_position("textDocument/definition", 2, &page, 3, 6),
        &at_position("textDocument/definition", 3, &page, 3, 1),
    ]);
    let at = editor.until(mark, |message| message.id() == Some("3"));
    let (sent, _) = editor.end(true);
    let offered = sent[0].at(&["result", "capabilities", "definitionProvider"]);
    assert_eq!(sent[0].sources.spelling(offered.unwrap().place), "true");
    let answer = (sent.iter())
        .find(|message| message.id() == Some("2"))
        .expect("the definition");
    let about = uri::of(&site.root.join("content/about.md"));
    assert_eq!(answer.text(&["result", "uri"]), Some(about.as_str()));
    let start = |name| {
        let node = answer.at(&["result", "range", "start", name]).unwrap();
        answer.sources.spelling(node.place).to_owned()
    };
    assert_eq!(
        (start("line"), start("character")),
        ("5".into(), "0".into())
    );
    let none = sent[at].at(&["result"]).unwrap();
    assert_eq!(sent[at].sources.spelling(none.place), "null");
}

/// The `initialize` request of an editor that does, or doesn't, show a
/// page's headings each beneath the one above it.
fn initialize_outlining(nested: bool) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\
         \"params\":{{\"capabilities\":{{\"textDocument\":{{\"documentSymbol\":\
         {{\"hierarchicalDocumentSymbolSupport\":{nested}}}}}}}}}}}"
    )
}

#[test]
fn a_pages_headings_are_listed_as_the_editor_shows_them_and_the_site_is_searched_by_name() {
    let site = TempSite::site(&[
        ("statac.yaml", "title: Site 1\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n# One\n\n## Two\n\n# Three\n",
        ),
    ]);
    let page = site.root.join("content/post-1.md");
    let outline = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/documentSymbol\",\
         \"params\":{{\"textDocument\":{{\"uri\":{}}}}}}}",
        json::string(&uri::of(&page))
    );
    let search = "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"workspace/symbol\",\
                  \"params\":{\"query\":\"post\"}}";
    for nested in [true, false] {
        let mut editor = Editor::start();
        editor.send(&initialize_outlining(nested));
        let mark = editor.mark();
        editor.send_all(&[&opened(&page), &outline, search]);
        editor.until(mark, |message| message.id() == Some("3"));
        let (sent, _) = editor.end(true);
        for offered in ["documentSymbolProvider", "workspaceSymbolProvider"] {
            let offered = sent[0].at(&["result", "capabilities", offered]).unwrap();
            assert_eq!(sent[0].sources.spelling(offered.place), "true");
        }
        let answer = |id| {
            (sent.iter())
                .find(|message| message.id() == Some(id))
                .expect("an answer")
        };
        let outline = answer("2");
        let shown = outline
            .sources
            .spelling(outline.at(&["result"]).unwrap().place);
        let at = json::string(&uri::of(&page));
        let range = |line: usize, start: usize, end_line: usize, end: usize| {
            format!(
                "{{\"start\":{{\"line\":{line},\"character\":{start}}},\
                 \"end\":{{\"line\":{end_line},\"character\":{end}}}}}"
            )
        };
        let wanted = match nested {
            true => format!(
                "[{{\"name\":\"One\",\"kind\":15,\"range\":{},\"selectionRange\":{},\
                 \"children\":[{{\"name\":\"Two\",\"kind\":15,\"range\":{},\
                 \"selectionRange\":{},\"children\":[]}}]}},\
                 {{\"name\":\"Three\",\"kind\":15,\"range\":{},\"selectionRange\":{},\
                 \"children\":[]}}]",
                range(3, 0, 7, 0),
                range(3, 0, 3, 5),
                range(5, 0, 7, 0),
                range(5, 0, 5, 6),
                range(7, 0, 8, 0),
                range(7, 0, 7, 7),
            ),
            false => format!(
                "[{{\"name\":\"One\",\"kind\":15,\"location\":{{\"uri\":{at},\"range\":{}}}}},\
                 {{\"name\":\"Two\",\"kind\":15,\"location\":{{\"uri\":{at},\"range\":{}}}}},\
                 {{\"name\":\"Three\",\"kind\":15,\"location\":{{\"uri\":{at},\"range\":{}}}}}]",
                range(3, 0, 3, 5),
                range(5, 0, 5, 6),
                range(7, 0, 7, 7),
            ),
        };
        assert_eq!(shown, wanted);
        let found = answer("3");
        let found = found.sources.spelling(found.at(&["result"]).unwrap().place);
        let wanted = format!(
            "[{{\"name\":\"Post 1\",\"kind\":1,\"location\":{{\"uri\":{at},\"range\":{}}}}}]",
            range(0, 0, 0, 0)
        );
        assert_eq!(found, wanted);
    }
}

#[test]
fn finding_references_lists_every_link_to_the_page_the_cursor_is_in() {
    let site = TempSite::site(&[
        ("statac.yaml", "title: Site 1\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "content/about.md",
            "---\ntitle: About\n---\nA page.\n\n## The team\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\nSee [the team](/about/#the-team) and [about](/about/).\n",
        ),
    ]);
    let about = site.root.join("content/about.md");
    let post = site.root.join("content/post-1.md");
    let references = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/references\",\
         \"params\":{{\"textDocument\":{{\"uri\":{}}},\
         \"position\":{{\"line\":3,\"character\":2}},\
         \"context\":{{\"includeDeclaration\":true}}}}}}",
        json::string(&uri::of(&about))
    );
    let mut editor = Editor::start();
    editor.send(&initialize(""));
    let mark = editor.mark();
    editor.send_all(&[&opened(&about), &references]);
    editor.until(mark, |message| message.id() == Some("2"));
    let (sent, _) = editor.end(true);
    let offered = sent[0].at(&["result", "capabilities", "referencesProvider"]);
    assert_eq!(sent[0].sources.spelling(offered.unwrap().place), "true");
    let answer = (sent.iter())
        .find(|message| message.id() == Some("2"))
        .expect("the references");
    let found = answer
        .sources
        .spelling(answer.at(&["result"]).unwrap().place);
    let location = |file: &Path, line: usize, start: usize, end: usize| {
        format!(
            "{{\"uri\":{},\"range\":{{\"start\":{{\"line\":{line},\"character\":{start}}},\
             \"end\":{{\"line\":{line},\"character\":{end}}}}}}}",
            json::string(&uri::of(file))
        )
    };
    let wanted = [
        location(&about, 0, 0, 0),
        location(&post, 3, 15, 31),
        location(&post, 3, 45, 52),
    ];
    assert_eq!(found, format!("[{}]", wanted.join(",")));
}

fn actions(id: u32, file: &Path, line: usize, character: usize, only: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"textDocument/codeAction\",\
         \"params\":{{\"textDocument\":{{\"uri\":{}}},\
         \"range\":{{\"start\":{{\"line\":{line},\"character\":{character}}},\
         \"end\":{{\"line\":{line},\"character\":{character}}}}},\
         \"context\":{{\"diagnostics\":[]{only}}}}}}}",
        json::string(&uri::of(file))
    )
}

#[test]
fn a_problem_with_one_exact_answer_is_fixed_by_the_edit_while_its_text_stands() {
    let site = site();
    let post = site.root.join("content/post-1.md");
    let initialize = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\
                      \"params\":{\"capabilities\":{\"textDocument\":{\"codeAction\":\
                      {\"codeActionLiteralSupport\":{\"codeActionKind\":\
                      {\"valueSet\":[\"quickfix\"]}}}}}}}";
    let mut editor = Editor::start();
    editor.send(initialize);
    let mark = editor.mark();
    editor.send(&opened(&post));
    editor.told_until(mark, &post, |told| has(told, "unclear-date"));
    let typed = DATED.replace("A post.", "A post, typed on.");
    let moved = DATED.replace("---\ntitle", "---\n\ntitle");
    editor.send_all(&[
        &actions(2, &post, 2, 8, ""),
        &actions(3, &post, 2, 0, ""),
        &actions(4, &post, 2, 8, ",\"only\":[\"refactor\"]"),
        &changed(&post, &typed),
        &actions(5, &post, 2, 14, ""),
        &changed(&post, &moved),
        &actions(6, &post, 2, 8, ""),
        &actions(7, &post, 3, 8, ""),
    ]);
    editor.until(mark, |message| message.id() == Some("7"));
    let (sent, _) = editor.end(true);
    let offered = sent[0].at(&["result", "capabilities", "codeActionProvider"]);
    assert_eq!(
        sent[0].sources.spelling(offered.unwrap().place),
        "{\"codeActionKinds\":[\"quickfix\"]}"
    );
    // Each fix as its title, its kind, the problem it fixes, whether it is
    // preferred, and its edit's file, place and text.
    let fixes = |id: &str| -> Vec<[String; 7]> {
        let answer = (sent.iter())
            .find(|message| message.id() == Some(id))
            .expect("an answer");
        let Some(Value::Sequence(fixes)) = answer.at(&["result"]).map(|n| &n.value) else {
            panic!("no list of fixes");
        };
        let text = |node: &Node, path: &[&str]| {
            let found = path.iter().try_fold(node, |node, key| node.get(key));
            answer
                .sources
                .spelling(found.expect("there").place)
                .to_owned()
        };
        (fixes.iter())
            .map(|fix| {
                let Some(Value::Sequence(problems)) = fix.get("diagnostics").map(|n| &n.value)
                else {
                    panic!("no problems");
                };
                let Some(Value::Mapping(changes)) = fix
                    .get("edit")
                    .and_then(|edit| edit.get("changes"))
                    .map(|n| &n.value)
                else {
                    panic!("no changes");
                };
                let entry = changes.first().expect("a file");
                let Value::Sequence(edits) = &entry.value.value else {
                    panic!("no edits");
                };
                [
                    text(fix, &["title"]),
                    text(fix, &["kind"]),
                    text(&problems[0], &["code"]),
                    text(fix, &["isPreferred"]),
                    answer.sources.spelling(entry.key.place).to_owned(),
                    text(&edits[0], &["range"]),
                    text(&edits[0], &["newText"]),
                ]
            })
            .collect()
    };
    let at = "{\"start\":{\"line\":2,\"character\":6},\"end\":{\"line\":2,\"character\":14}}";
    let fix = |date: &str| {
        [
            format!("\"Write it as `{date}`\""),
            "\"quickfix\"".to_owned(),
            "\"unclear-date\"".to_owned(),
            "false".to_owned(),
            json::string(&uri::of(&post)),
            at.to_owned(),
            format!("\"{date}\""),
        ]
    };
    // Two readings, so two fixes, neither preferred.
    assert_eq!(fixes("2"), [fix("2026-04-03"), fix("2026-03-04")]);
    // Off the problem, or asked for other kinds of action, nothing is
    // offered.
    assert!(fixes("3").is_empty());
    assert!(fixes("4").is_empty());
    // Typed on elsewhere, the date stands where it was.
    assert_eq!(fixes("5"), [fix("2026-04-03"), fix("2026-03-04")]);
    // Moved before the site is checked again, it is no longer there.
    assert!(fixes("6").is_empty());
    assert!(fixes("7").is_empty());
}

#[test]
fn renaming_a_pages_file_answers_with_the_edits_that_keep_its_links() {
    let site = TempSite::site(&[
        ("statac.yaml", "title: Site 1\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        ("content/about.md", "---\ntitle: About\n---\nA page.\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\nSee [about](/about/).\n",
        ),
    ]);
    let about = site.root.join("content/about.md");
    let post = site.root.join("content/post-1.md");
    let rename = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"workspace/willRenameFiles\",\
         \"params\":{{\"files\":[{{\"oldUri\":{},\"newUri\":{}}}]}}}}",
        json::string(&uri::of(&about)),
        json::string(&uri::of(&site.root.join("content/about-us.md")))
    );
    let mut editor = Editor::start();
    editor.send(&initialize(""));
    let mark = editor.mark();
    editor.send_all(&[&opened(&post), &rename]);
    editor.until(mark, |message| message.id() == Some("2"));
    let (sent, _) = editor.end(true);
    let offered = sent[0].at(&["result", "capabilities", "workspace", "fileOperations"]);
    assert_eq!(
        sent[0].sources.spelling(offered.unwrap().place),
        "{\"willRename\":{\"filters\":[{\"scheme\":\"file\",\"pattern\":{\"glob\":\"**/content/**\"}}]}}"
    );
    let answer = (sent.iter())
        .find(|message| message.id() == Some("2"))
        .expect("the edits");
    let edits = answer
        .sources
        .spelling(answer.at(&["result"]).unwrap().place);
    let edit = |file: &Path, line: usize, start: usize, end: usize, text: &str| {
        format!(
            "{}:[{{\"range\":{{\"start\":{{\"line\":{line},\"character\":{start}}},\
             \"end\":{{\"line\":{line},\"character\":{end}}}}},\"newText\":{}}}]",
            json::string(&uri::of(file)),
            json::string(text)
        )
    };
    assert_eq!(
        edits,
        format!(
            "{{\"changes\":{{{},{}}}}}",
            edit(&about, 2, 0, 0, "redirect_from: [/about/]\n"),
            edit(&post, 3, 12, 19, "/about-us/"),
        )
    );
}

#[test]
fn two_files_published_at_one_address_are_each_told_it_at_their_start() {
    let site = TempSite::site(&[
        ("statac.yaml", "title: Site 1\nlanguage: en\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        ("templates/home.html", "{{ page.content }}\n"),
        ("content/about.md", "---\ntitle: About\n---\n"),
        ("content/team.md", "---\ntitle: Team\n---\n"),
        ("static/team/index.html", "<p>The team</p>\n"),
    ]);
    let about = site.root.join("content/about.md");
    let mut editor = Editor::start();
    editor.send(&initialize(""));
    editor.send(&opened(&about));
    for file in ["content/team.md", "static/team/index.html"] {
        let told = editor.told_until(0, &site.root.join(file), |told| !told.is_empty());
        assert_eq!(
            told,
            [("same-address".into(), "0".into(), "0".into())],
            "{file}"
        );
    }
    let (sent, _) = editor.end(true);
    let settings = uri::of(&site.root.join("statac.yaml"));
    let told: Vec<Told> = (sent.iter())
        .filter_map(|message| diagnostics_for(message, &settings))
        .flatten()
        .collect();
    assert!(told.is_empty(), "{told:?}");
}

const DRAFT: &str = "---\ntitle: Draft 1\ndraft: true\nlayout: pots\n---\n\
                     See [the team](/about/#the-team).\n";

fn drafting() -> TempSite {
    TempSite::site(&[
        ("statac.yaml", "title: Site 1\nlanguage: en\n"),
        ("templates/page.html", "{{ page.content }}\n"),
        (
            "content/about.md",
            "---\ntitle: About\n---\nSee [the draft](/draft-1/).\n\n## The team\n",
        ),
        ("content/draft-1.md", DRAFT),
    ])
}

#[test]
fn a_draft_is_told_what_it_would_be_once_published_and_the_rest_as_published() {
    let site = drafting();
    let draft = site.root.join("content/draft-1.md");
    let about = site.root.join("content/about.md");
    let mut editor = Editor::start();
    editor.send(&initialize(""));
    editor.send(&opened(&draft));
    let told = editor.told_until(0, &draft, |told| has(told, "missing-template"));
    assert!(!has(&told, "draft-left-out"), "{told:?}");
    let told = editor.told_until(0, &about, |told| has(told, "broken-link"));
    assert_eq!(told, [("broken-link".into(), "3".into(), "16".into())]);
    // Published, it is the rest of the site's like any page.
    let mark = editor.mark();
    let published = DRAFT.replace("draft: true\n", "");
    editor.send(&changed(&draft, &published));
    editor.told_until(mark, &about, <[Told]>::is_empty);
    let told = editor.told_until(mark, &draft, |told| !told.is_empty());
    assert!(has(&told, "missing-template"), "{told:?}");
    editor.end(true);
}

#[test]
fn a_draft_the_editor_doesnt_hold_open_is_told_only_that_it_is_left_out() {
    let site = drafting();
    let draft = site.root.join("content/draft-1.md");
    let page = site.root.join("content/about.md");
    let mut editor = Editor::start();
    editor.send(&initialize(""));
    editor.send(&opened(&page));
    let told = editor.told_until(0, &draft, |told| !told.is_empty());
    assert_eq!(told, [("draft-left-out".into(), "2".into(), "7".into())]);
    // Opened, it is told what it would be once published, and closed,
    // that it is left out again.
    let mark = editor.mark();
    editor.send(&opened(&draft));
    editor.told_until(mark, &draft, |told| has(told, "missing-template"));
    let mark = editor.mark();
    editor.send(&about("didClose", &draft));
    editor.told_until(mark, &draft, |told| has(told, "draft-left-out"));
    editor.end(true);
}

#[test]
fn what_is_asked_in_a_draft_is_answered_as_of_a_page() {
    let site = drafting();
    let draft = site.root.join("content/draft-1.md");
    let about = site.root.join("content/about.md");
    let references = format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"textDocument/references\",\
         \"params\":{{\"textDocument\":{{\"uri\":{}}},\
         \"position\":{{\"line\":5,\"character\":0}},\
         \"context\":{{\"includeDeclaration\":false}}}}}}",
        json::string(&uri::of(&draft))
    );
    let mut editor = Editor::start();
    editor.send(&initialize(""));
    let mark = editor.mark();
    editor.send_all(&[&opened(&draft), &references]);
    editor.until(mark, |message| message.id() == Some("2"));
    let (sent, _) = editor.end(true);
    let answer = (sent.iter())
        .find(|message| message.id() == Some("2"))
        .expect("the references");
    let found = answer
        .sources
        .spelling(answer.at(&["result"]).unwrap().place);
    assert_eq!(
        found,
        format!(
            "[{{\"uri\":{},\"range\":{{\"start\":{{\"line\":3,\"character\":16}},\
             \"end\":{{\"line\":3,\"character\":25}}}}}}]",
            json::string(&uri::of(&about))
        )
    );
}

#[test]
fn renaming_finds_the_links_drafts_write_and_sends_no_one_on_from_where_a_draft_was() {
    let site = drafting();
    let draft = site.root.join("content/draft-1.md");
    let about = site.root.join("content/about.md");
    let rename = |id: u32, from: &str, to: &str| {
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"workspace/willRenameFiles\",\
             \"params\":{{\"files\":[{{\"oldUri\":{},\"newUri\":{}}}]}}}}",
            json::string(&uri::of(&site.root.join(from))),
            json::string(&uri::of(&site.root.join(to)))
        )
    };
    let mut editor = Editor::start();
    editor.send(&initialize(""));
    let mark = editor.mark();
    editor.send_all(&[
        &opened(&about),
        &rename(2, "content/about.md", "content/about-us.md"),
        &rename(3, "content/draft-1.md", "content/draft-2.md"),
    ]);
    editor.until(mark, |message| message.id() == Some("3"));
    let (sent, _) = editor.end(true);
    let edits = |id: &str| {
        let answer = (sent.iter())
            .find(|message| message.id() == Some(id))
            .expect("the edits");
        answer
            .sources
            .spelling(answer.at(&["result"]).unwrap().place)
            .to_owned()
    };
    let edit = |file: &Path, line: usize, start: usize, end: usize, text: &str| {
        format!(
            "{}:[{{\"range\":{{\"start\":{{\"line\":{line},\"character\":{start}}},\
             \"end\":{{\"line\":{line},\"character\":{end}}}}},\"newText\":{}}}]",
            json::string(&uri::of(file)),
            json::string(text)
        )
    };
    assert_eq!(
        edits("2"),
        format!(
            "{{\"changes\":{{{},{}}}}}",
            edit(&about, 2, 0, 0, "redirect_from: [/about/]\n"),
            edit(&draft, 5, 15, 31, "/about-us/#the-team"),
        )
    );
    assert_eq!(
        edits("3"),
        format!(
            "{{\"changes\":{{{}}}}}",
            edit(&about, 3, 16, 25, "/draft-2/")
        )
    );
}
