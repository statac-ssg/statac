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
    let unknown = "{\"jsonrpc\":\"2.0\",\"id\":\"a\",\"method\":\"textDocument/hover\"}";
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
        (
            "snippets/figure.html",
            "{# takes photo #}\n{{ page.photo(photo).alt }}\n",
        ),
        ("content/post-1.md", "---\ntitle: Post 1\n---\n@fi\n"),
    ]);
    let page = site.root.join("content/post-1.md");
    for (fills, written, format) in [(true, "figure(\"$1\")", "2"), (false, "figure(", "")] {
        let mut editor = Editor::start();
        editor.send(&initialize_filling(fills));
        let mark = editor.mark();
        editor.send_all(&[&opened(&page), &completion(2, &page, 3, 3)]);
        let at = editor.until(mark, |message| message.id() == Some("2"));
        let (sent, _) = editor.end(true);
        let Some(Value::Sequence(items)) = sent[at].at(&["result", "items"]).map(|n| &n.value)
        else {
            panic!("no items");
        };
        let figure = (items.iter())
            .find(|item| {
                item.get("filterText").map(|n| &n.value) == Some(&Value::Scalar("figure".into()))
            })
            .expect("the snippet");
        let said = |key: &str| sent[at].sources.spelling(figure.get(key).unwrap().place);
        assert_eq!(said("label"), "\"figure(photo)\"");
        let edit = figure.get("textEdit").unwrap().get("newText").unwrap();
        assert_eq!(edit.value, Value::Scalar(written.into()), "{fills}");
        let given = figure
            .get("insertTextFormat")
            .map(|n| sent[at].sources.spelling(n.place));
        assert_eq!(given.unwrap_or_default(), format, "{fills}");
    }
}
