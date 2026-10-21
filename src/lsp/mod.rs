//! `statac lsp`: a language server an editor starts, speaking the Language
//! Server Protocol on standard input and output. Each time a file of a
//! site is opened or saved, it checks the site as `statac check` does and
//! tells the editor every problem at the place it is written.
//!
//! Nothing but the protocol may be written to standard output, which the
//! editor reads as messages.

mod diagnostics;
#[cfg(test)]
mod tests;
mod wire;

use std::collections::{BTreeSet, HashMap};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use diagnostics::Encoding;

use crate::build::{self, Mode, Options};
use crate::json;
use crate::source::Sources;
use crate::yaml::{Node, Value};

/// The protocol's codes for a request it can't answer.
const PARSE_ERROR: i32 = -32700;
const METHOD_NOT_FOUND: i32 = -32601;
const NOT_INITIALISED: i32 = -32002;

/// Answers the editor's messages on `input` with its own on `output` until
/// the editor says to stop, and gives the exit code: 0 where it asked to
/// shut down first, as the protocol has it, and 1 otherwise.
pub(crate) fn run(input: &mut dyn BufRead, output: &mut dyn Write) -> u8 {
    let mut server = Server {
        output,
        encoding: Encoding::Utf16,
        initialised: false,
        shut_down: false,
        sites: HashMap::new(),
    };
    loop {
        let Ok(Some(text)) = wire::read(input) else {
            return 1;
        };
        let answered = match Message::read(text) {
            Some(message) => server.answer(&message),
            None => server.fail("null", PARSE_ERROR, "The message isn't JSON."),
        };
        match answered {
            Ok(None) => {}
            Ok(Some(code)) => return code,
            Err(_) => return 1,
        }
    }
}

struct Server<'a> {
    output: &'a mut dyn Write,
    encoding: Encoding,
    initialised: bool,
    shut_down: bool,
    /// Each site checked, by its folder, with the files its last check
    /// told the editor of a problem in.
    sites: HashMap<PathBuf, BTreeSet<PathBuf>>,
}

/// What answering one message came to: the exit code where the editor
/// said to stop, and an error where the editor can no longer be written to.
type Answered = std::io::Result<Option<u8>>;

impl Server<'_> {
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
                let result = format!(
                    "{{\"capabilities\":{{\"positionEncoding\":\"{}\",\
                     \"textDocumentSync\":{{\"openClose\":true,\"change\":0,\
                     \"save\":{{\"includeText\":false}}}}}},\
                     \"serverInfo\":{{\"name\":\"statac\",\"version\":{}}}}}",
                    self.encoding.name(),
                    json::string(env!("CARGO_PKG_VERSION")),
                );
                self.respond(id.unwrap_or("null"), &result)
            }
            "shutdown" => {
                self.shut_down = true;
                self.respond(id.unwrap_or("null"), "null")
            }
            "exit" => Ok(Some(u8::from(!self.shut_down))),
            "textDocument/didOpen" | "textDocument/didSave" => {
                let uri = message.text(&["params", "textDocument", "uri"]);
                let Some(file) = uri.and_then(uri::path) else {
                    return Ok(None);
                };
                let Some(site) = site_of(&file) else {
                    return Ok(None);
                };
                // A site already checked is checked again only when saved.
                if method == "textDocument/didOpen" && self.sites.contains_key(&site) {
                    return Ok(None);
                }
                self.check(&site, &file)
            }
            _ => match id {
                Some(id) => self.fail(id, METHOD_NOT_FOUND, "Statac doesn't answer this."),
                None => Ok(None),
            },
        }
    }

    /// Checks the site at `site` and tells the editor every problem found,
    /// and that a file told of a problem before now has none.
    fn check(&mut self, site: &Path, file: &Path) -> Answered {
        let options = Options {
            mode: Mode::Check,
            site: site.to_path_buf(),
            out: None,
            today: None,
            drafts: false,
            url: None,
        };
        let checked = std::panic::catch_unwind(|| build::run(&options));
        let Ok(outcome) = checked else {
            let params = format!(
                "{{\"type\":1,\"message\":{}}}",
                json::string(crate::wording::CHECK_STOPPED)
            );
            return self.notify("window/showMessage", &params);
        };
        let settings = site.join(crate::config::FILE_NAME);
        let nowhere = match settings.is_file() {
            true => settings,
            false => file.to_path_buf(),
        };
        let found = diagnostics::by_file(&outcome, site, &nowhere, self.encoding);
        let before = self
            .sites
            .insert(site.to_path_buf(), found.keys().cloned().collect());
        for gone in before
            .unwrap_or_default()
            .difference(&found.keys().cloned().collect())
        {
            self.publish(gone, &[])?;
        }
        for (file, told) in &found {
            self.publish(file, told)?;
        }
        Ok(None)
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

/// The folder of the site `file` is in: the nearest folder above it that
/// is a site, as `statac check` finds one.
fn site_of(file: &Path) -> Option<PathBuf> {
    file.ancestors()
        .skip(1)
        .find(|folder| matches!(crate::new::site_mark(folder), Ok(Some(_))))
        .map(Path::to_path_buf)
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
