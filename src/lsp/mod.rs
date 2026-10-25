//! `statac lsp`: a language server an editor starts, speaking the Language
//! Server Protocol on standard input and output. It keeps each site the
//! editor opens a file of checked as `statac check` checks it, reading the
//! text the editor holds for each file it has open in place of the file,
//! and tells the editor every problem at the place it is written: as the
//! site is first opened, once typing pauses, and as files of the site
//! change on disk, with the edit that fixes it where Statac's suggestion
//! is exact. Each check builds again only what changed since the last. It
//! completes what is typed in the site's settings, in front matter, in
//! the writing and in templates from what Statac takes there and what the
//! site holds, and says what the word under the pointer is in the site's
//! settings, in front matter, in the writing and in templates, and goes
//! to where what it names is written. It lists a
//! page's headings, and finds the site's pages by title and its snippets
//! and templates by name. Renaming a page's file, or a folder of pages,
//! it keeps every link to the pages moved leading to them, and sends
//! readers on from the addresses they leave.
//!
//! Nothing but the protocol may be written to standard output, which the
//! editor reads as messages.

mod complete;
mod diagnostics;
mod fixes;
mod hover;
mod references;
mod rename;
mod symbols;
#[cfg(test)]
mod tests;
mod wire;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{BufRead, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use diagnostics::{Encoding, Told};
use hover::Goes;
use references::Written;

use crate::build::incremental::Checking;
use crate::build::{Mode, Options};
use crate::json;
use crate::problem::Kind;
use crate::serve::watch::{self, How, Settle, Watching};
use crate::source::Sources;
use crate::templates::Renderer;
use crate::unsaved::Unsaved;
use crate::yaml::{Node, Value};

/// The protocol's codes for a request it can't answer.
const PARSE_ERROR: i32 = -32700;
const METHOD_NOT_FOUND: i32 = -32601;
const NOT_INITIALISED: i32 = -32002;

/// How long typing pauses before the site is checked.
const PAUSE: Duration = Duration::from_millis(330);

/// The protocol's kind of symbol a heading is told as: a string, as
/// editors' own Markdown support tells one.
const HEADING: u8 = 15;

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
        markdown: false,
        described_in_markdown: false,
        nested: false,
        actions: false,
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
    /// Whether the editor reads Markdown in what a hover says.
    markdown: bool,
    /// Whether it reads Markdown in what a completion's item is.
    described_in_markdown: bool,
    /// Whether the editor shows a page's headings each beneath the one
    /// above it.
    nested: bool,
    /// Whether the editor takes a fix as the edit it makes.
    actions: bool,
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
    /// The site read with its drafts and posts waiting for their day as if
    /// published, where it has any, for what is told and asked of their
    /// files: what they would be once published.
    drafts: Checking,
    /// The paths in the site of the files of the pages the last check left
    /// out.
    left_out: HashSet<String>,
    /// Whether the site has been checked since it was last checked with
    /// the pages left out, and has any.
    drafts_stale: bool,
    /// The files of pages left out that the editor holds open, as the last
    /// check found them: told what they would be once published, the rest
    /// being told only that they are left out.
    open_left_out: HashSet<PathBuf>,
    /// The problems the last check found, by the file each is told in, but
    /// for the notes that a page is left out told in its own file.
    published: BTreeMap<PathBuf, Vec<Told>>,
    /// Those notes, by file.
    notes: BTreeMap<PathBuf, Vec<Told>>,
    /// The problems the last check with the pages left out found in each
    /// of their files the editor holds open.
    drafts_told: BTreeMap<PathBuf, Vec<Told>>,
    /// Where a problem in no file is told.
    nowhere: PathBuf,
    /// The problems the editor was last told of in each file.
    told: BTreeMap<PathBuf, Vec<Told>>,
    /// When the site is to be checked next, where it is to be.
    due: Option<Instant>,
    /// Held while the site's folder is watched.
    _watching: Option<Watching>,
}

impl Site {
    /// The site as last read for what is asked of `file`, a file in the
    /// site at `folder`: with the pages left out where it is one of theirs.
    fn known_for(&self, folder: &Path, file: &Path) -> Option<&Renderer> {
        match self.left_out.contains(&in_site(folder, file)) {
            true => self.known_whole(),
            false => self.checking.renderer(),
        }
    }

    /// The site as last read for what is asked of it whole, links and
    /// names written in pages left out among the rest.
    fn known_whole(&self) -> Option<&Renderer> {
        (self.drafts.renderer()).or_else(|| self.checking.renderer())
    }

    /// The paths of the files of the pages left out, where
    /// [`Site::known_whole`] holds them.
    fn left_out(&self) -> Option<&HashSet<String>> {
        (self.drafts.renderer()).map(|_| &self.left_out)
    }

    /// Whether the check with the pages left out is to follow the site's
    /// own, to tell the files of theirs the editor holds open.
    fn drafts_wanted(&self) -> bool {
        self.drafts_stale && !self.open_left_out.is_empty()
    }

    /// The problems to tell the editor of now, by file, for the site at
    /// `folder`: what its last check found, each file of a page left out
    /// that the editor holds open told what the last check with it found
    /// there, and the rest that they are left out.
    fn told_now(&self, folder: &Path) -> BTreeMap<PathBuf, Vec<Told>> {
        let mut found = self.published.clone();
        for path in &self.left_out {
            let file = folder.join(path);
            let more = (self.drafts_told.get(&file)).or_else(|| self.notes.get(&file));
            let Some(more) = more.filter(|more| !more.is_empty()) else {
                continue;
            };
            let had = found.entry(file).or_default();
            for told in more {
                if !had.iter().any(|was| was.diagnostic == told.diagnostic) {
                    had.push(told.clone());
                }
            }
        }
        found
    }
}

/// How the server checks the site at `folder`: with its drafts and posts
/// waiting for their day as if published where `drafts` holds.
fn options(folder: &Path, drafts: bool) -> Options {
    Options {
        mode: Mode::Check,
        site: folder.to_path_buf(),
        out: None,
        today: None,
        drafts,
        url: None,
    }
}

/// `file`'s path in the site at `folder`, with `/` between its parts.
fn in_site(folder: &Path, file: &Path) -> String {
    let rel = file.strip_prefix(folder).unwrap_or(file);
    rel.to_string_lossy().replace('\\', "/")
}

/// Where a request about a place in a file stands.
struct Placed {
    folder: PathBuf,
    file: PathBuf,
    text: Arc<str>,
    offset: usize,
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
            // Typing heard meanwhile puts the check with drafts off until
            // the site's own check is done again.
            while let Ok(event) = heard.try_recv() {
                if let Some(code) = ended(self.hear(event)) {
                    return code;
                }
            }
            let mut drafts: Vec<PathBuf> = (self.sites.iter())
                .filter(|(_, site)| site.drafts_wanted() && site.due.is_none())
                .map(|(folder, _)| folder.clone())
                .collect();
            drafts.sort();
            for folder in drafts {
                if self.check_drafts(&folder).is_err() {
                    return 1;
                }
            }
        }
    }

    /// When the next site is due to be checked, where one is. The check
    /// with a site's pages left out follows its own check as soon as
    /// nothing else is waiting, where the editor holds one of their files
    /// open, and waits while typing goes on.
    fn next_due(&self) -> Option<Instant> {
        let now = Instant::now();
        (self.sites.values())
            .filter_map(|site| site.due.or(site.drafts_wanted().then_some(now)))
            .min()
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
                let item = [&item[..], &["completionItem"]].concat();
                self.snippets = message.truth(&[&item[..], &["snippetSupport"]].concat());
                let formats = message.texts(&[&item[..], &["documentationFormat"]].concat());
                self.described_in_markdown = formats.contains(&"markdown");
                let hover = ["params", "capabilities", "textDocument", "hover"];
                let formats = message.texts(&[&hover[..], &["contentFormat"]].concat());
                self.markdown = formats.contains(&"markdown");
                let symbols = ["params", "capabilities", "textDocument", "documentSymbol"];
                self.nested =
                    message.truth(&[&symbols[..], &["hierarchicalDocumentSymbolSupport"]].concat());
                let actions = ["params", "capabilities", "textDocument", "codeAction"];
                self.actions = message
                    .at(&[&actions[..], &["codeActionLiteralSupport"]].concat())
                    .is_some();
                let result = format!(
                    "{{\"capabilities\":{{\"positionEncoding\":\"{}\",\
                     \"textDocumentSync\":{{\"openClose\":true,\"change\":{WHOLE_TEXT},\
                     \"save\":{{\"includeText\":false}}}},\
                     \"completionProvider\":{{\"triggerCharacters\":{TRIGGERS}}},\
                     \"hoverProvider\":true,\"definitionProvider\":true,\
                     \"referencesProvider\":true,\"documentSymbolProvider\":true,\
                     \"workspaceSymbolProvider\":true,\
                     \"codeActionProvider\":{{\"codeActionKinds\":[\"quickfix\"]}},\
                     \"workspace\":{{\"fileOperations\":{{\"willRename\":{{\"filters\":[\
                     {{\"scheme\":\"file\",\"pattern\":{{\"glob\":\"**/content/**\"}}}}]}}}}}}}},\
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
            "textDocument/hover" => {
                let result = self.hover(message)?;
                self.respond(id.unwrap_or("null"), &result)
            }
            "textDocument/definition" => {
                let result = self.definition(message)?;
                self.respond(id.unwrap_or("null"), &result)
            }
            "textDocument/references" => {
                let result = self.references(message)?;
                self.respond(id.unwrap_or("null"), &result)
            }
            "textDocument/codeAction" => {
                let result = self.actions(message)?;
                self.respond(id.unwrap_or("null"), &result)
            }
            "workspace/willRenameFiles" => {
                let result = self.will_rename(message)?;
                self.respond(id.unwrap_or("null"), &result)
            }
            "textDocument/documentSymbol" => {
                let result = self.outline(message)?;
                self.respond(id.unwrap_or("null"), &result)
            }
            "workspace/symbol" => {
                let result = self.symbols(message)?;
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
            drafts: Checking::new(),
            left_out: HashSet::new(),
            drafts_stale: false,
            open_left_out: HashSet::new(),
            published: BTreeMap::new(),
            notes: BTreeMap::new(),
            drafts_told: BTreeMap::new(),
            nowhere,
            told: BTreeMap::new(),
            due: Some(Instant::now()),
            _watching: watching,
        };
        self.sites.insert(folder, site);
    }

    /// The file a request about a place in a file names, the folder of its
    /// site, its text as the editor holds it, and the place as an offset
    /// in that text, the site read first where it is yet to be; none where
    /// any can't be found.
    fn placed(&mut self, message: &Message) -> std::io::Result<Option<Placed>> {
        let at = |name| message.text(&["params", "position", name])?.parse().ok();
        let (Some((folder, file, text)), Some(line), Some(character)) =
            (self.named(message)?, at("line"), at("character"))
        else {
            return Ok(None);
        };
        let Some(offset) = diagnostics::offset_in(&text, line, character, self.encoding) else {
            return Ok(None);
        };
        Ok(Some(Placed {
            folder,
            file,
            text,
            offset,
        }))
    }

    /// The folder of the site of the file a request names, the file, and
    /// its text as the editor holds it, the site read first where it is
    /// yet to be; none where any can't be found.
    fn named(
        &mut self,
        message: &Message,
    ) -> std::io::Result<Option<(PathBuf, PathBuf, Arc<str>)>> {
        let uri = message.text(&["params", "textDocument", "uri"]);
        let Some(file) = uri.and_then(uri::path) else {
            return Ok(None);
        };
        let text = match self.open.get(&real(&file)) {
            Some(text) => Arc::clone(text),
            None => match std::fs::read_to_string(&file) {
                Ok(text) => Arc::from(text),
                Err(_) => return Ok(None),
            },
        };
        let Some(folder) = site_of(&file) else {
            return Ok(None);
        };
        self.read_if_unread(&folder)?;
        let left_out = |site: &Site| site.left_out.contains(&in_site(&folder, &file));
        if self.sites.get(&folder).is_some_and(left_out) {
            self.drafts_if_due(&folder)?;
        }
        Ok(Some((folder, file, text)))
    }

    /// Reads the site at `folder` where it is kept but yet to be read, so
    /// that what it holds is known.
    fn read_if_unread(&mut self, folder: &Path) -> std::io::Result<()> {
        let unread = (self.sites.get(folder))
            .is_some_and(|site| site.checking.renderer().is_none() && site.due.is_some());
        match unread {
            true => self.check(folder),
            false => Ok(()),
        }
    }

    /// What may be written where the cursor stands, as the protocol's
    /// list of completions, or `null` where nothing is offered there.
    fn complete(&mut self, message: &Message) -> std::io::Result<String> {
        let Some(Placed {
            folder,
            file,
            text,
            offset,
        }) = self.placed(message)?
        else {
            return Ok("null".to_owned());
        };
        let known = (self.sites.get(&folder)).and_then(|site| site.known_for(&folder, &file));
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
                let about = match item.about {
                    Some(about) => {
                        let (kind, said) = said(
                            vec![about.text.clone()],
                            about.more.as_deref(),
                            self.described_in_markdown,
                        );
                        format!(
                            ",\"documentation\":{{\"kind\":\"{kind}\",\"value\":{}}}",
                            json::string(&said)
                        )
                    }
                    None => String::new(),
                };
                format!(
                    "{{\"label\":{},\"kind\":{}{detail}{about}{filter}{format},\"sortText\":{},\
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

    /// What the word under the pointer is, as the protocol's hover, or
    /// `null` where nothing is said of it.
    fn hover(&mut self, message: &Message) -> std::io::Result<String> {
        let Some(Placed {
            folder,
            file,
            text,
            offset,
        }) = self.placed(message)?
        else {
            return Ok("null".to_owned());
        };
        let known = (self.sites.get(&folder)).and_then(|site| site.known_for(&folder, &file));
        let Some(hover) = hover::hover(&folder, &file, &text, offset, known) else {
            return Ok("null".to_owned());
        };
        let position = |offset| {
            let (line, character) = diagnostics::position_in(&text, offset, self.encoding);
            format!("{{\"line\":{line},\"character\":{character}}}")
        };
        let mut paragraphs = vec![hover.head];
        paragraphs.extend(hover.body);
        let (kind, said) = said(paragraphs, hover.more.as_deref(), self.markdown);
        Ok(format!(
            "{{\"contents\":{{\"kind\":\"{kind}\",\"value\":{}}},\
             \"range\":{{\"start\":{},\"end\":{}}}}}",
            json::string(&said),
            position(hover.range.start),
            position(hover.range.end),
        ))
    }

    /// Where what the word under the pointer names is written, as the
    /// protocol's location, or `null` where it names nothing written in
    /// the site.
    fn definition(&mut self, message: &Message) -> std::io::Result<String> {
        let Some(Placed {
            folder,
            file,
            text,
            offset,
        }) = self.placed(message)?
        else {
            return Ok("null".to_owned());
        };
        let known = (self.sites.get(&folder)).and_then(|site| site.known_for(&folder, &file));
        let to = hover::hover(&folder, &file, &text, offset, known).and_then(|hover| hover.to);
        let (path, range) = match (to, known) {
            (Some(Goes::File(path)), _) => (folder.join(path), diagnostics::START.to_owned()),
            (Some(Goes::At(place)), Some(known)) => {
                let sources = known.sources();
                let range = diagnostics::range(sources, place, self.encoding);
                (folder.join(sources.path(place.source)), range)
            }
            _ => return Ok("null".to_owned()),
        };
        Ok(format!(
            "{{\"uri\":{},\"range\":{range}}}",
            json::string(&uri::of(&path))
        ))
    }

    /// Every link in the site to the page or heading the word under the
    /// pointer names, or the cursor is in, as the protocol's locations;
    /// `null` where it is neither.
    fn references(&mut self, message: &Message) -> std::io::Result<String> {
        let Some(Placed {
            folder,
            file,
            text,
            offset,
        }) = self.placed(message)?
        else {
            return Ok("null".to_owned());
        };
        self.drafts_if_due(&folder)?;
        let Some(known) = (self.sites.get(&folder)).and_then(Site::known_whole) else {
            return Ok("null".to_owned());
        };
        let to = hover::hover(&folder, &file, &text, offset, Some(known)).and_then(|h| h.to);
        let Some(rel) = file.strip_prefix(&folder).ok() else {
            return Ok("null".to_owned());
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        let declaration = message.truth(&["params", "context", "includeDeclaration"]);
        let Some(found) = references::found(known, &rel, offset, to, declaration) else {
            return Ok("null".to_owned());
        };
        let sources = known.sources();
        let told: Vec<String> = (found.into_iter())
            .map(|written| {
                let (path, range) = match written {
                    Written::At(place) => (
                        folder.join(sources.path(place.source)),
                        diagnostics::range(sources, place, self.encoding),
                    ),
                    Written::File(path) => (folder.join(path), diagnostics::START.to_owned()),
                };
                format!(
                    "{{\"uri\":{},\"range\":{range}}}",
                    json::string(&uri::of(&path))
                )
            })
            .collect();
        Ok(format!("[{}]", told.join(",")))
    }

    /// The fixes for the problems the editor was last told of in the range
    /// a request names, as the protocol's code actions, each an edit to
    /// the file. A fix is offered only while the text it replaces is still
    /// where it was.
    fn actions(&mut self, message: &Message) -> std::io::Result<String> {
        let only = message.texts(&["params", "context", "only"]);
        let wanted =
            only.is_empty() || (only.iter()).any(|kind| *kind == "quickfix" || kind.is_empty());
        let Some((folder, file, text)) = self.named(message)?.filter(|_| self.actions && wanted)
        else {
            return Ok("[]".to_owned());
        };
        let at = |end: &str| -> Option<diagnostics::Position> {
            let read = |name| message.text(&["params", "range", end, name])?.parse().ok();
            Some((read("line")?, read("character")?))
        };
        let (Some(start), Some(end)) = (at("start"), at("end")) else {
            return Ok("[]".to_owned());
        };
        let told = (self.sites.get(&folder)).and_then(|site| site.told.get(&file));
        let uri = json::string(&uri::of(&file));
        let mut offered = Vec::new();
        for told in told.into_iter().flatten() {
            if told.from > end || start > told.to {
                continue;
            }
            let standing = (told.fixes.iter()).filter(|fix| {
                let from = diagnostics::offset_in(&text, fix.from.0, fix.from.1, self.encoding);
                let to = diagnostics::offset_in(&text, fix.to.0, fix.to.1, self.encoding);
                let was = from.zip(to).and_then(|(from, to)| text.get(from..to));
                was == Some(fix.was.as_str())
            });
            let standing: Vec<_> = standing.collect();
            for fix in &standing {
                offered.push(format!(
                    "{{\"title\":{},\"kind\":\"quickfix\",\"diagnostics\":[{}],\
                     \"isPreferred\":{},\"edit\":{{\"changes\":{{{uri}:[\
                     {{\"range\":{},\"newText\":{}}}]}}}}}}",
                    json::string(&fix.title),
                    told.diagnostic,
                    standing.len() == 1,
                    diagnostics::ranged(fix.from, fix.to),
                    json::string(&fix.text),
                ));
            }
        }
        Ok(format!("[{}]", offered.join(",")))
    }

    /// The edits that keep every link in a site leading to the pages that
    /// renaming the request's files and folders moves, and send readers on
    /// from the addresses they leave, as the protocol's edit to the files
    /// as they are before the renaming; `null` where there are none.
    fn will_rename(&mut self, message: &Message) -> std::io::Result<String> {
        let Some(Value::Sequence(files)) = message.at(&["params", "files"]).map(|n| &n.value)
        else {
            return Ok("null".to_owned());
        };
        let mut moves: BTreeMap<PathBuf, Vec<(String, String)>> = BTreeMap::new();
        for file in files {
            let path = |key| match file.get(key).map(|node| &node.value) {
                Some(Value::Scalar(at)) => uri::path(at),
                _ => None,
            };
            let (Some(from), Some(to)) = (path("oldUri"), path("newUri")) else {
                continue;
            };
            let Some(folder) = site_of(&from) else {
                continue;
            };
            let (Ok(old), Ok(new)) = (from.strip_prefix(&folder), to.strip_prefix(&folder)) else {
                continue;
            };
            let slashed = |path: &Path| path.to_string_lossy().replace('\\', "/");
            moves
                .entry(folder)
                .or_default()
                .push((slashed(old), slashed(new)));
        }
        let mut changes: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (folder, moves) in moves {
            let Some(site) = self.sites.get(&folder) else {
                continue;
            };
            // Renamed as the site stands, with all that is typed.
            if site.due.is_some() {
                self.check(&folder)?;
            }
            self.drafts_if_due(&folder)?;
            let Some(site) = self.sites.get(&folder) else {
                continue;
            };
            let Some(renderer) = site.known_whole() else {
                continue;
            };
            let unsaved = Unsaved::new(self.open.clone());
            let sources = renderer.sources();
            let left_out = site.left_out();
            for (place, text) in rename::renamed(renderer, &folder, unsaved, &moves, left_out) {
                let file = folder.join(sources.path(place.source));
                changes.entry(uri::of(&file)).or_default().push(format!(
                    "{{\"range\":{},\"newText\":{}}}",
                    diagnostics::range(sources, place, self.encoding),
                    json::string(&text)
                ));
            }
        }
        if changes.is_empty() {
            return Ok("null".to_owned());
        }
        let changes: Vec<String> = (changes.iter())
            .map(|(uri, edits)| format!("{}:[{}]", json::string(uri), edits.join(",")))
            .collect();
        Ok(format!("{{\"changes\":{{{}}}}}", changes.join(",")))
    }

    /// The headings of the page a request names, as the protocol's
    /// symbols: each beneath the one above it where the editor shows them
    /// so, and in a list otherwise; `null` for a file that is no page's.
    fn outline(&mut self, message: &Message) -> std::io::Result<String> {
        let Some((folder, file, text)) = self.named(message)? else {
            return Ok("null".to_owned());
        };
        let known = (self.sites.get(&folder)).and_then(|site| site.known_for(&folder, &file));
        let Some(headings) = symbols::headings(&folder, &file, &text, known) else {
            return Ok("null".to_owned());
        };
        let range = |at: &Range<usize>| {
            let position = |offset| {
                let (line, character) = diagnostics::position_in(&text, offset, self.encoding);
                format!("{{\"line\":{line},\"character\":{character}}}")
            };
            format!(
                "{{\"start\":{},\"end\":{}}}",
                position(at.start),
                position(at.end)
            )
        };
        let told = match self.nested {
            true => nested_symbols(&headings, &range),
            false => {
                let at = json::string(&uri::of(&file));
                let mut told = Vec::new();
                let mut left: Vec<&symbols::Heading> = headings.iter().rev().collect();
                while let Some(heading) = left.pop() {
                    told.push(format!(
                        "{{\"name\":{},\"kind\":{HEADING},\
                         \"location\":{{\"uri\":{at},\"range\":{}}}}}",
                        json::string(&heading.words),
                        range(&heading.at),
                    ));
                    left.extend(heading.within.iter().rev());
                }
                told
            }
        };
        Ok(format!("[{}]", told.join(",")))
    }

    /// The pages, snippets and templates of every site kept whose names
    /// match the request's query, as the protocol's symbols.
    fn symbols(&mut self, message: &Message) -> std::io::Result<String> {
        let query = message.text(&["params", "query"]).unwrap_or_default();
        let mut folders: Vec<PathBuf> = self.sites.keys().cloned().collect();
        folders.sort();
        let mut told = Vec::new();
        for folder in folders {
            self.read_if_unread(&folder)?;
            self.drafts_if_due(&folder)?;
            let Some(renderer) = (self.sites.get(&folder)).and_then(Site::known_whole) else {
                continue;
            };
            for found in symbols::found(renderer, query) {
                told.push(format!(
                    "{{\"name\":{},\"kind\":{},\
                     \"location\":{{\"uri\":{},\"range\":{}}}}}",
                    json::string(&found.name),
                    found.sort.number(),
                    json::string(&uri::of(&folder.join(&found.path))),
                    diagnostics::START,
                ));
            }
        }
        Ok(format!("[{}]", told.join(",")))
    }

    /// Puts off checking the site at `folder` until typing has paused.
    fn put_off(&mut self, folder: &Path) {
        if let Some(site) = self.sites.get_mut(folder) {
            site.due = Some(Instant::now() + self.pause);
        }
    }

    /// Checks the site at `folder` and tells the editor of each file whose
    /// problems differ from what it was last told, the files of pages left
    /// out as the check with them last told them, where one has.
    fn check(&mut self, folder: &Path) -> std::io::Result<()> {
        let encoding = self.encoding;
        let Some(site) = self.sites.get_mut(folder) else {
            return Ok(());
        };
        site.due = None;
        site.checking.set_unsaved(Unsaved::new(self.open.clone()));
        let options = options(folder, false);
        let checking = &mut site.checking;
        let checked =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| checking.check(&options)));
        let Ok(mut outcome) = checked else {
            // What a check that stopped part way kept may be anything.
            site.checking = Checking::new();
            site.drafts = Checking::new();
            site.drafts_stale = false;
            let params = format!(
                "{{\"type\":1,\"message\":{}}}",
                json::string(crate::wording::CHECK_STOPPED)
            );
            return self.notify("window/showMessage", &params).map(|_| ());
        };
        let sources = outcome.sources();
        site.left_out = (outcome.pages_left_out.iter())
            .map(|&source| sources.path(source).to_string_lossy().replace('\\', "/"))
            .collect();
        // That a page is left out is said only until the check with it
        // says what it would be once published.
        let files = &outcome.pages_left_out;
        let (notes, rest): (Vec<_>, Vec<_>) = std::mem::take(&mut outcome.problems)
            .into_iter()
            .partition(|problem| {
                matches!(
                    problem.kind,
                    Kind::DraftLeftOut { .. } | Kind::ScheduledPost { .. }
                ) && (problem.place).is_some_and(|at| files.contains(&at.source))
            });
        outcome.problems = rest;
        site.published = diagnostics::by_file(&outcome, folder, &site.nowhere, encoding);
        outcome.problems = notes;
        site.notes = diagnostics::by_file(&outcome, folder, &site.nowhere, encoding);
        site.drafts_stale = !site.left_out.is_empty();
        if !site.drafts_stale {
            site.drafts = Checking::new();
        }
        let open = &self.open;
        site.open_left_out = (site.left_out.iter())
            .map(|path| folder.join(path))
            .filter(|file| open.contains_key(&real(file)))
            .collect();
        let shown = &site.open_left_out;
        site.drafts_told.retain(|file, _| shown.contains(file));
        let found = site.told_now(folder);
        self.tell(folder, found)
    }

    /// Checks the site at `folder` with its drafts and posts waiting for
    /// their day as if published, and tells the editor of each of their
    /// files it holds open whose problems differ from what it was last
    /// told.
    fn check_drafts(&mut self, folder: &Path) -> std::io::Result<()> {
        let encoding = self.encoding;
        let Some(site) = self.sites.get_mut(folder) else {
            return Ok(());
        };
        site.drafts_stale = false;
        site.drafts.set_unsaved(Unsaved::new(self.open.clone()));
        let options = options(folder, true);
        let checking = &mut site.drafts;
        let checked =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| checking.check(&options)));
        site.drafts_told.clear();
        match checked {
            Ok(outcome) => {
                let mut told = diagnostics::by_file(&outcome, folder, &site.nowhere, encoding);
                for file in &site.open_left_out {
                    let found = told.remove(file).unwrap_or_default();
                    site.drafts_told.insert(file.clone(), found);
                }
            }
            // Their files are told as the site's own check told them.
            Err(_) => site.drafts = Checking::new(),
        }
        let found = site.told_now(folder);
        self.tell(folder, found)
    }

    /// Runs the check with the pages left out of the site at `folder`,
    /// where it is due, so that what is asked of them is answered as the
    /// site stands.
    fn drafts_if_due(&mut self, folder: &Path) -> std::io::Result<()> {
        match (self.sites.get(folder)).is_some_and(|site| site.drafts_stale) {
            true => self.check_drafts(folder),
            false => Ok(()),
        }
    }

    /// Tells the editor of each file whose problems in `found` differ from
    /// what it was last told of the site at `folder`.
    fn tell(&mut self, folder: &Path, found: BTreeMap<PathBuf, Vec<Told>>) -> std::io::Result<()> {
        let Some(site) = self.sites.get_mut(folder) else {
            return Ok(());
        };
        let before = std::mem::replace(&mut site.told, found);
        let found = &self.sites[folder].told;
        let gone: Vec<PathBuf> = (before.keys())
            .filter(|file| !found.contains_key(*file))
            .cloned()
            .collect();
        let same = |was: &[Told], told: &[Told]| {
            (was.iter().map(|was| &was.diagnostic)).eq(told.iter().map(|told| &told.diagnostic))
        };
        let changed: Vec<(PathBuf, Vec<String>)> = (found.iter())
            .filter(|(file, told)| (before.get(*file)).is_none_or(|was| !same(was, told)))
            .map(|(file, told)| {
                let told = told.iter().map(|told| told.diagnostic.clone());
                (file.clone(), told.collect())
            })
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
/// `paragraphs`, then where the docs say more, as Markdown where `markdown`
/// and as plain text otherwise: the protocol's word for which, and the text.
fn said(mut paragraphs: Vec<String>, more: Option<&str>, markdown: bool) -> (&'static str, String) {
    let (kind, more) = match markdown {
        true => (
            "markdown",
            more.map(|more| format!("[{}]({more})", crate::wording::HOVER_MORE)),
        ),
        false => ("plaintext", more.map(crate::wording::more)),
    };
    paragraphs.extend(more);
    (kind, paragraphs.join("\n\n"))
}

fn ended(answered: Answered) -> Option<u8> {
    match answered {
        Ok(code) => code,
        Err(_) => Some(1),
    }
}

/// `headings` as the protocol's symbols, each holding those beneath it,
/// placed in the page's text by `range`.
fn nested_symbols(
    headings: &[symbols::Heading],
    range: &dyn Fn(&Range<usize>) -> String,
) -> Vec<String> {
    (headings.iter())
        .map(|heading| {
            format!(
                "{{\"name\":{},\"kind\":{HEADING},\"range\":{},\
                 \"selectionRange\":{},\"children\":[{}]}}",
                json::string(&heading.words),
                range(&heading.section),
                range(&heading.at),
                nested_symbols(&heading.within, range).join(","),
            )
        })
        .collect()
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
