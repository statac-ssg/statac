//! `statac lsp`: a language server an editor starts, speaking the Language
//! Server Protocol on standard input and output. It keeps each site the
//! editor opens a file of checked as `statac check` checks it, reading the
//! text the editor holds for each file it has open in place of the file,
//! and tells the editor every problem at the place it is written: as the
//! site is first opened, once typing pauses, and as files of the site
//! change on disk. Each check builds again only what changed since the
//! last. It completes what is typed in the site's settings, in front
//! matter, in the writing and in templates from what Statac takes there
//! and what the site holds.
//!
//! Nothing but the protocol may be written to standard output, which the
//! editor reads as messages.

mod complete;
mod diagnostics;
#[cfg(test)]
mod tests;
mod wire;

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use diagnostics::Encoding;

use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::json;
use crate::serve::watch::{self, How, Settle, Watching};
use crate::source::Sources;
use crate::unsaved::Unsaved;
use crate::yaml::{Node, Value};

/// The protocol's codes for a request it can't answer.
const PARSE_ERROR: i32 = -32700;
const METHOD_NOT_FOUND: i32 = -32601;
const NOT_INITIALISED: i32 = -32002;

/// How long typing pauses before the site is checked.
const PAUSE: Duration = Duration::from_millis(330);

/// The protocol's word for the editor sending a file's whole text with
/// each change.
const WHOLE_TEXT: u8 = 1;

/// What, typed, has the editor ask what may follow, besides the letters
/// of a word: a value's space after its colon, and a list's bracket and
/// comma; in the writing, a call's `@`, a bracket or quote opening a
/// value or an address, and the `#` and `/` within an address; in a
/// template, the dot before a field and the bar before a filter.
const TRIGGERS: &str = "[\" \",\"[\",\",\",\"@\",\"(\",\"\\\"\",\"'\",\"#\",\"/\",\".\",\"|\"]";

/// Answers the editor's messages on `input` with its own on `output` until
/// the editor says to stop, and gives the exit code: 0 where it asked to
/// shut down first, as the protocol has it, and 1 otherwise.
pub(crate) fn run(input: impl BufRead + Send + 'static, output: &mut dyn Write) -> u8 {
    run_with(input, output, PAUSE, How::Auto)
}

/// [`run`], checking a site once typing pauses for `pause`, and watching
/// each site's folder as `watching` says.
fn run_with(
    mut input: impl BufRead + Send + 'static,
    output: &mut dyn Write,
    pause: Duration,
    watching: How,
) -> u8 {
    let (events, heard) = mpsc::channel();
    let reading = events.clone();
    // Never joined: once the editor says to stop, the server ends whether
    // or not its input has.
    let read = std::thread::Builder::new().name("lsp-input".into());
    let read = read.spawn(move || {
        loop {
            let event = match wire::read(&mut input) {
                Ok(Some(text)) => Event::Message(text),
                Ok(None) | Err(_) => Event::Ended,
            };
            let ended = matches!(event, Event::Ended);
            if reading.send(event).is_err() || ended {
                return;
            }
        }
    });
    if read.is_err() {
        return 1;
    }
    let mut server = Server {
        output,
        encoding: Encoding::Utf16,
        snippets: false,
        initialised: false,
        shut_down: false,
        pause,
        watching,
        events,
        open: HashMap::new(),
        sites: HashMap::new(),
    };
    server.serve(&heard)
}

/// What the server hears, from the editor and from each site's folder.
enum Event {
    /// A message from the editor.
    Message(String),
    /// The editor's messages ended, or could no longer be read.
    Ended,
    /// Files of the site in this folder changed on disk.
    Changed(PathBuf),
}

struct Server<'a> {
    output: &'a mut dyn Write,
    encoding: Encoding,
    /// Whether the editor fills in places in what a completion writes.
    snippets: bool,
    initialised: bool,
    shut_down: bool,
    pause: Duration,
    watching: How,
    /// Where each site's folder sends word of its changes.
    events: Sender<Event>,
    /// The text the editor holds for each file it has open, by the file's
    /// real path.
    open: HashMap<PathBuf, Arc<str>>,
    /// Each site a file was opened in, by its folder.
    sites: HashMap<PathBuf, Site>,
}

/// A site kept checked.
struct Site {
    checking: Checking,
    /// Where a problem in no file is told.
    nowhere: PathBuf,
    /// The diagnostics the editor was last sent for each file.
    told: BTreeMap<PathBuf, Vec<String>>,
    /// When the site is to be checked next, where it is to be.
    due: Option<Instant>,
    /// Held while the site's folder is watched.
    _watching: Option<Watching>,
}

/// What answering one message came to: the exit code where the editor
/// said to stop, and an error where the editor can no longer be written to.
type Answered = std::io::Result<Option<u8>>;

impl Server<'_> {
    /// Hears the editor and each site's folder until the editor says to
    /// stop, checking each site as it falls due.
    fn serve(&mut self, heard: &Receiver<Event>) -> u8 {
        loop {
            let waited = match self.next_due() {
                Some(due) => heard.recv_timeout(due.saturating_duration_since(Instant::now())),
                None => heard.recv().map_err(|_| RecvTimeoutError::Disconnected),
            };
            match waited {
                Ok(event) => {
                    if let Some(code) = ended(self.hear(event)) {
                        return code;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return 1,
            }
            if self.next_due().is_none_or(|due| due > Instant::now()) {
                continue;
            }
            // What arrived meanwhile is heard first, so a check reads the
            // latest text, and a site whose typing goes on waits.
            while let Ok(event) = heard.try_recv() {
                if let Some(code) = ended(self.hear(event)) {
                    return code;
                }
            }
            let now = Instant::now();
            let mut due: Vec<PathBuf> = (self.sites.iter())
                .filter(|(_, site)| site.due.is_some_and(|due| due <= now))
                .map(|(folder, _)| folder.clone())
                .collect();
            due.sort();
            for folder in due {
                if self.check(&folder).is_err() {
                    return 1;
                }
            }
        }
    }

    /// When the next site is due to be checked, where one is.
    fn next_due(&self) -> Option<Instant> {
        self.sites.values().filter_map(|site| site.due).min()
    }

    fn hear(&mut self, event: Event) -> Answered {
        match event {
            Event::Message(text) => match Message::read(text) {
                Some(message) => self.answer(&message),
                None => self.fail("null", PARSE_ERROR, "The message isn't JSON."),
            },
            Event::Ended => Ok(Some(1)),
            Event::Changed(folder) => {
                self.put_off(&folder);
                Ok(None)
            }
        }
    }

    fn answer(&mut self, message: &Message) -> Answered {
        let id = message.id();
        let method = message.text(&["method"]).unwrap_or_default();
        if !self.initialised && method != "initialize" && method != "exit" {
            return match id {
                Some(id) => self.fail(id, NOT_INITIALISED, "The server isn't initialised."),
                None => Ok(None),
            };
        }
        match method {
            "initialize" => {
                self.initialised = true;
                let offered =
                    message.texts(&["params", "capabilities", "general", "positionEncodings"]);
                if offered.contains(&"utf-8") {
                    self.encoding = Encoding::Utf8;
                }
                let item = ["params", "capabilities", "textDocument", "completion"];
                let item = [&item[..], &["completionItem", "snippetSupport"]].concat();
                self.snippets = message.truth(&item);
                let result = format!(
                    "{{\"capabilities\":{{\"positionEncoding\":\"{}\",\
                     \"textDocumentSync\":{{\"openClose\":true,\"change\":{WHOLE_TEXT},\
                     \"save\":{{\"includeText\":false}}}},\
                     \"completionProvider\":{{\"triggerCharacters\":{TRIGGERS}}}}},\
                     \"serverInfo\":{{\"name\":\"statac\",\"version\":{}}}}}",
                    self.encoding.name(),
                    json::string(env!("CARGO_PKG_VERSION")),
                );
                self.respond(id.unwrap_or("null"), &result)
            }
            "textDocument/completion" => {
                let result = self.complete(message)?;
                self.respond(id.unwrap_or("null"), &result)
            }
            "shutdown" => {
                self.shut_down = true;
                self.respond(id.unwrap_or("null"), "null")
            }
            "exit" => Ok(Some(u8::from(!self.shut_down))),
            "textDocument/didOpen" | "textDocument/didChange" => {
                let text = match method {
                    "textDocument/didOpen" => message.text(&["params", "textDocument", "text"]),
                    _ => message.last_text(&["params", "contentChanges"]),
                };
                let Some((file, folder)) = self.file_and_site(message) else {
                    return Ok(None);
                };
                if let Some(text) = text {
                    self.open.insert(real(&file), Arc::from(text));
                }
                match self.sites.contains_key(&folder) {
                    true => self.put_off(&folder),
                    false => self.begin(folder, file),
                }
                Ok(None)
            }
            "textDocument/didSave" | "textDocument/didClose" => {
                let Some((file, folder)) = self.file_and_site(message) else {
                    return Ok(None);
                };
                if method == "textDocument/didClose" {
                    self.open.remove(&real(&file));
                }
                if self.sites.contains_key(&folder) {
                    self.put_off(&folder);
                }
                Ok(None)
            }
            _ => match id {
                Some(id) => self.fail(id, METHOD_NOT_FOUND, "Statac doesn't answer this."),
                None => Ok(None),
            },
        }
    }

    /// The file a message is about, and the folder of the site it is in,
    /// where it is in one.
    fn file_and_site(&self, message: &Message) -> Option<(PathBuf, PathBuf)> {
        let uri = message.text(&["params", "textDocument", "uri"]);
        let file = uri.and_then(uri::path)?;
        let site = site_of(&file)?;
        Some((file, site))
    }

    /// Begins keeping the site at `folder` checked, `file` being the one
    /// opened in it: checked at once, and watched.
    fn begin(&mut self, folder: PathBuf, file: PathBuf) {
        let settings = folder.join(crate::config::FILE_NAME);
        let nowhere = match settings.is_file() {
            true => settings,
            false => file,
        };
        let (batches, changes) = mpsc::channel();
        // A folder that can't be watched even by polling can't be read,
        // which the check says.
        let watching = watch::start(&folder, self.watching, Settle::default(), batches).ok();
        if watching.is_some() {
            let events = self.events.clone();
            let changed = folder.clone();
            // Ends once watching does, or the server has.
            let forward = std::thread::Builder::new().name("lsp-watch".into());
            let _ = forward.spawn(move || {
                for _ in changes {
                    if events.send(Event::Changed(changed.clone())).is_err() {
                        return;
                    }
                }
            });
        }
        let site = Site {
            checking: Checking::new(),
            nowhere,
            told: BTreeMap::new(),
            due: Some(Instant::now()),
            _watching: watching,
        };
        self.sites.insert(folder, site);
    }

    /// What may be written where the cursor stands, as the protocol's
    /// list of completions, or `null` where nothing is offered there.
    fn complete(&mut self, message: &Message) -> std::io::Result<String> {
        let at = |name| message.text(&["params", "position", name])?.parse().ok();
        let uri = message.text(&["params", "textDocument", "uri"]);
        let (Some(file), Some(line), Some(character)) =
            (uri.and_then(uri::path), at("line"), at("character"))
        else {
            return Ok("null".to_owned());
        };
        let text = match self.open.get(&real(&file)) {
            Some(text) => Arc::clone(text),
            None => match std::fs::read_to_string(&file) {
                Ok(text) => Arc::from(text),
                Err(_) => return Ok("null".to_owned()),
            },
        };
        let (Some(folder), Some(offset)) = (
            site_of(&file),
            diagnostics::offset_in(&text, line, character, self.encoding),
        ) else {
            return Ok("null".to_owned());
        };
        // What the site holds is offered once it has been read.
        let unread = (self.sites.get(&folder))
            .is_some_and(|site| site.checking.renderer().is_none() && site.due.is_some());
        if unread {
            self.check(&folder)?;
        }
        let known = (self.sites.get(&folder)).and_then(|site| site.checking.renderer());
        let completion = complete::complete(&folder, &file, &text, offset, known, self.snippets);
        let Some(completion) = completion else {
            return Ok("null".to_owned());
        };
        let position = |offset| {
            let (line, character) = diagnostics::position_in(&text, offset, self.encoding);
            format!("{{\"line\":{line},\"character\":{character}}}")
        };
        let range = format!(
            "{{\"start\":{},\"end\":{}}}",
            position(completion.typed.start),
            position(completion.typed.end)
        );
        let items: Vec<String> = (completion.items.iter())
            .map(|item| {
                let detail = match &item.detail {
                    Some(detail) => format!(",\"detail\":{}", json::string(detail)),
                    None => String::new(),
                };
                let filter = match &item.filter {
                    Some(filter) => format!(",\"filterText\":{}", json::string(filter)),
                    None => String::new(),
                };
                let format = match item.kind.fills() && self.snippets {
                    true => ",\"insertTextFormat\":2",
                    false => "",
                };
                format!(
                    "{{\"label\":{},\"kind\":{}{detail}{filter}{format},\"sortText\":{},\
                     \"textEdit\":{{\"range\":{range},\"newText\":{}}}}}",
                    json::string(&item.label),
                    item.kind.number(),
                    json::string(&format!("{}{}", item.group, item.label)),
                    json::string(&item.insert),
                )
            })
            .collect();
        Ok(format!(
            "{{\"isIncomplete\":{},\"items\":[{}]}}",
            completion.incomplete,
            items.join(",")
        ))
    }

    /// Puts off checking the site at `folder` until typing has paused.
    fn put_off(&mut self, folder: &Path) {
        if let Some(site) = self.sites.get_mut(folder) {
            site.due = Some(Instant::now() + self.pause);
        }
    }

    /// Checks the site at `folder` and tells the editor of each file whose
    /// problems differ from what it was last told.
    fn check(&mut self, folder: &Path) -> std::io::Result<()> {
        let encoding = self.encoding;
        let Some(site) = self.sites.get_mut(folder) else {
            return Ok(());
        };
        site.due = None;
        site.checking.set_unsaved(Unsaved::new(self.open.clone()));
        let options = Options {
            mode: Mode::Check,
            site: folder.to_path_buf(),
            out: None,
            today: None,
            drafts: false,
            url: None,
        };
        let checking = &mut site.checking;
        let checked =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| checking.check(&options)));
        let Ok(outcome) = checked else {
            // What a check that stopped part way kept may be anything.
            site.checking = Checking::new();
            let params = format!(
                "{{\"type\":1,\"message\":{}}}",
                json::string(crate::wording::CHECK_STOPPED)
            );
            return self.notify("window/showMessage", &params).map(|_| ());
        };
        let found = diagnostics::by_file(&outcome, folder, &site.nowhere, encoding);
        let before = std::mem::replace(&mut site.told, found);
        let found = &self.sites[folder].told;
        let gone: Vec<PathBuf> = (before.keys())
            .filter(|file| !found.contains_key(*file))
            .cloned()
            .collect();
        let changed: Vec<(PathBuf, Vec<String>)> = (found.iter())
            .filter(|(file, told)| before.get(*file) != Some(told))
            .map(|(file, told)| (file.clone(), told.clone()))
            .collect();
        for file in gone {
            self.publish(&file, &[])?;
        }
        for (file, told) in changed {
            self.publish(&file, &told)?;
        }
        Ok(())
    }

    fn publish(&mut self, file: &Path, diagnostics: &[String]) -> Answered {
        let params = format!(
            "{{\"uri\":{},\"diagnostics\":[{}]}}",
            json::string(&uri::of(file)),
            diagnostics.join(",")
        );
        self.notify("textDocument/publishDiagnostics", &params)
    }

    fn respond(&mut self, id: &str, result: &str) -> Answered {
        let message = format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}");
        wire::write(self.output, &message).map(|()| None)
    }

    fn fail(&mut self, id: &str, code: i32, said: &str) -> Answered {
        let message = format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{code},\"message\":{}}}}}",
            json::string(said)
        );
        wire::write(self.output, &message).map(|()| None)
    }

    fn notify(&mut self, method: &str, params: &str) -> Answered {
        let message =
            format!("{{\"jsonrpc\":\"2.0\",\"method\":\"{method}\",\"params\":{params}}}");
        wire::write(self.output, &message).map(|()| None)
    }
}

/// The exit code where `answered` ends the server: where the editor said
/// to stop, or can no longer be written to.
fn ended(answered: Answered) -> Option<u8> {
    match answered {
        Ok(code) => code,
        Err(_) => Some(1),
    }
}

/// The folder of the site `file` is in: the nearest folder above it that
/// is a site, as `statac check` finds one.
fn site_of(file: &Path) -> Option<PathBuf> {
    file.ancestors()
        .skip(1)
        .find(|folder| matches!(crate::new::site_mark(folder), Ok(Some(_))))
        .map(Path::to_path_buf)
}

/// Where `file` really is, past every link, as a reading of a site knows
/// it; as it is named, where it isn't there.
fn real(file: &Path) -> PathBuf {
    std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf())
}

/// One message from the editor, read as JSON.
struct Message {
    sources: Sources,
    node: Node,
}

impl Message {
    fn read(text: String) -> Option<Message> {
        let mut sources = Sources::default();
        let id = sources.add("message", text);
        let node = crate::yaml::read_json(&sources, id).ok()?;
        Some(Message { sources, node })
    }

    /// The request's id as it is written, to answer with; none for a
    /// notification, which has none and wants no answer.
    fn id(&self) -> Option<&str> {
        let id = self.node.get("id")?;
        Some(self.sources.spelling(id.place))
    }

    fn at(&self, path: &[&str]) -> Option<&Node> {
        path.iter().try_fold(&self.node, |node, key| node.get(key))
    }

    /// The text at `path`, where it is a string.
    fn text(&self, path: &[&str]) -> Option<&str> {
        match &self.at(path)?.value {
            Value::Scalar(text) => Some(text),
            _ => None,
        }
    }

    /// The `text` of the last item of the list at `path`.
    fn last_text(&self, path: &[&str]) -> Option<&str> {
        let Value::Sequence(items) = &self.at(path)?.value else {
            return None;
        };
        match &items.last()?.get("text")?.value {
            Value::Scalar(text) => Some(text),
            _ => None,
        }
    }

    /// Whether the value at `path` is `true`.
    fn truth(&self, path: &[&str]) -> bool {
        let Some(node) = self.at(path) else {
            return false;
        };
        matches!(node.value, Value::Scalar(_)) && self.sources.spelling(node.place) == "true"
    }

    /// The texts in the list at `path`.
    fn texts(&self, path: &[&str]) -> Vec<&str> {
        match self.at(path).map(|node| &node.value) {
            Some(Value::Sequence(items)) => (items.iter())
                .filter_map(|item| match &item.value {
                    Value::Scalar(text) => Some(text.as_str()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }
}

/// The protocol names a file by a `file:` address.
mod uri {
    use std::path::{Path, PathBuf};

    pub(super) fn of(path: &Path) -> String {
        url::Url::from_file_path(path)
            .map(String::from)
            .unwrap_or_else(|()| format!("file://{}", path.display()))
    }

    pub(super) fn path(uri: &str) -> Option<PathBuf> {
        url::Url::parse(uri).ok()?.to_file_path().ok()
    }
}
