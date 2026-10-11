//! The author's raw HTML followed through the elements it opens and
//! closes, as a browser builds them, to find a tag that reaches past the
//! writing into the page around it: an element or a comment left open,
//! or an end tag closing nothing the writing opened.
//!
//! The pieces of raw HTML in one stretch of writing are read in order as
//! one, as a browser reads them on the page. Of what Markdown writes
//! round them, only its blocks count: each closes what is opened inside
//! it, as its end tag does in a browser. Where a browser forgives what is
//! written, nothing is told.

use std::collections::HashMap;
use std::ops::Range;

use crate::html::{self, RAW_TEXT, comment_close, next};
use crate::problem::Unmatched;

/// What is told, at its place in the masked text.
pub(super) type Told = (Unmatched, Range<usize>);

/// Elements that hold nothing, so are never left open.
const VOID: &[&str] = &[
    "area", "base", "basefont", "bgsound", "br", "col", "embed", "frame", "hr", "image", "img",
    "input", "keygen", "link", "meta", "param", "source", "track", "wbr",
];

/// Elements a browser closes by itself, never told open or closing nothing.
const CLOSED_BY_ITSELF: &[&str] = &[
    "p", "li", "dt", "dd", "option", "optgroup", "rb", "rt", "rtc", "rp", "tr", "td", "th",
    "thead", "tbody", "tfoot", "colgroup", "caption", "html", "head", "body",
];

/// Elements a browser opens again round what follows, until their own end
/// tag, after an element holding them closes.
const FORMATTING: &[&str] = &[
    "a", "b", "big", "code", "em", "font", "i", "nobr", "s", "small", "strike", "strong", "tt", "u",
];

/// Elements past whose end a browser opens no formatting element again.
const MARKERS: &[&str] = &[
    "applet", "caption", "marquee", "object", "template", "td", "th",
];

/// Elements an end tag for an element of another name, other than a
/// formatting element, never closes or reaches past.
const SPECIAL: &[&str] = &[
    "address",
    "applet",
    "area",
    "article",
    "aside",
    "base",
    "basefont",
    "bgsound",
    "blockquote",
    "body",
    "br",
    "button",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dir",
    "div",
    "dl",
    "dt",
    "embed",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hgroup",
    "hr",
    "html",
    "iframe",
    "img",
    "input",
    "keygen",
    "li",
    "link",
    "listing",
    "main",
    "marquee",
    "menu",
    "meta",
    "nav",
    "noembed",
    "noframes",
    "noscript",
    "object",
    "ol",
    "p",
    "param",
    "plaintext",
    "pre",
    "script",
    "search",
    "section",
    "select",
    "source",
    "style",
    "summary",
    "table",
    "tbody",
    "td",
    "template",
    "textarea",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
    "wbr",
    "xmp",
];

/// Elements an end tag doesn't reach past to close an element below them.
const SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template",
];
const BUTTON_SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "button",
];
const LIST_ITEM_SCOPE: &[&str] = &[
    "applet", "caption", "html", "table", "td", "th", "marquee", "object", "template", "ol", "ul",
];
const TABLE_SCOPE: &[&str] = &["html", "table", "template"];

/// Start tags that close a paragraph open round them.
const CLOSE_A_PARAGRAPH: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "li",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "p",
    "plaintext",
    "pre",
    "search",
    "section",
    "summary",
    "table",
    "ul",
    "xmp",
];

/// A block Markdown writes round the HTML inside it.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Block {
    /// With whether it stands at the top of the writing.
    Paragraph {
        top: bool,
    },
    Item,
    Cell,
    Other,
}

#[derive(Clone, Copy, PartialEq)]
enum Holds {
    Markup,
    /// Text up to the element's end tag, as `<script>` does.
    Text,
    Comment,
    /// One of Markdown's own, by how deep it is in the writing.
    Block(Block, usize),
}

struct Opened {
    /// In small letters; empty for a comment or a block.
    name: String,
    /// As written.
    written: String,
    holds: Holds,
    /// Where its opening tag is in the masked text, on its first line.
    at: Range<usize>,
}

impl Opened {
    fn is(&self, set: &[&str]) -> bool {
        matches!(self.holds, Holds::Markup | Holds::Text) && set.contains(&self.name.as_str())
    }

    fn is_marker(&self) -> bool {
        self.is(MARKERS) || matches!(self.holds, Holds::Block(Block::Cell, _))
    }

    fn is_special(&self) -> bool {
        self.is(SPECIAL) || matches!(self.holds, Holds::Block(..))
    }

    fn is_paragraph(&self) -> bool {
        self.is(&["p"]) || matches!(self.holds, Holds::Block(Block::Paragraph { .. }, _))
    }

    fn swallows(&self) -> bool {
        matches!(self.holds, Holds::Text | Holds::Comment)
    }
}

/// Whether an end tag reaches past every element of `above` to close one
/// below them, where `scope` are those it doesn't reach past.
fn in_scope(above: &[Opened], scope: &[&str]) -> bool {
    !above.iter().any(|opened| {
        opened.is(scope)
            || matches!(opened.holds, Holds::Block(Block::Cell, _)) && scope.contains(&"td")
    })
}

/// An element a paragraph or another of Markdown's blocks closed round
/// it, which the author may close later: a later end tag of its name is
/// taken as closing it, and it is never told left open.
struct Held {
    opened: Opened,
    /// Whether a paragraph at the top of the writing closed it.
    top: bool,
}

/// A formatting element closed by an element holding it, which a browser
/// opens again round what follows.
struct Reopened {
    opened: Opened,
    /// Where in the stack the innermost marker open round it is.
    within: Option<usize>,
}

/// The elements a stretch of writing's raw HTML holds open, and the tags
/// told so far.
#[derive(Default)]
pub(super) struct Elements {
    /// Innermost last.
    stack: Vec<Opened>,
    /// How many elements of each name are open.
    named: HashMap<String, usize>,
    reopened: Vec<Reopened>,
    held: Vec<Held>,
    /// A tag a piece of HTML ended inside: its text so far, and where its
    /// `<` is in the masked text.
    partial: Option<(String, usize)>,
    told: Vec<Told>,
}

impl Elements {
    /// Reads on through `html`, which starts at `at` in the masked text.
    pub fn read(&mut self, html: &str, at: usize) {
        let joined;
        let (text, carried, from) = match self.partial.take() {
            Some((partial, from)) => {
                joined = partial + html;
                (joined.as_str(), joined.len() - html.len(), from)
            }
            None => (html, 0, at),
        };
        let place = |i: usize| match i < carried {
            true => from + i,
            false => at + i - carried,
        };
        let b = text.as_bytes();
        let mut i = 0;
        loop {
            match self.stack.last() {
                Some(top) if top.holds == Holds::Comment => {
                    let Some(end) = comment_close(b, i) else {
                        return;
                    };
                    self.stack.pop();
                    i = end;
                    continue;
                }
                Some(top) if top.holds == Holds::Text => {
                    match html::end_tag(b, i, top.name.as_bytes()) {
                        Some(lt) => i = lt,
                        None => return,
                    }
                }
                _ => {}
            }
            let Some(lt) = next(b, b'<', i) else {
                return;
            };
            let rest = &b[lt..];
            if rest.starts_with(b"<!--") {
                // `<!-->` and `<!--->` close as they open.
                i = match &rest[4..] {
                    [b'>', ..] => lt + 5,
                    [b'-', b'>', ..] => lt + 6,
                    _ => {
                        self.stack.push(Opened {
                            name: String::new(),
                            written: "<!--".into(),
                            holds: Holds::Comment,
                            at: place(lt)..place(lt + 4),
                        });
                        lt + 4
                    }
                };
                continue;
            }
            if rest.starts_with(b"<!") || rest.starts_with(b"<?") {
                i = next(b, b'>', lt).map_or(b.len(), |gt| gt + 1);
                continue;
            }
            let closing = rest.get(1) == Some(&b'/');
            let name = lt + 1 + usize::from(closing);
            if !b.get(name).is_some_and(u8::is_ascii_alphabetic) {
                i = lt + 1;
                continue;
            }
            let tag = html::tag(text, name, usize::MAX);
            if b[tag.end - 1] != b'>' {
                self.partial = Some((text[lt..].to_owned(), place(lt)));
                return;
            }
            let line = first_line(text, lt, tag.end);
            let span = match lt < carried {
                true => from + lt..from + line.min(carried),
                false => place(lt)..place(line),
            };
            self.met(text, name, &tag, closing, span);
            i = tag.end;
        }
    }

    /// Reads a tag a block of HTML ended inside as ending there.
    pub fn settle(&mut self) {
        let Some((text, from)) = self.partial.take() else {
            return;
        };
        let closing = text.as_bytes()[1] == b'/';
        let name = 1 + usize::from(closing);
        let tag = html::tag(&text, name, usize::MAX);
        let line = first_line(&text, 0, text.len());
        self.met(&text, name, &tag, closing, from..from + line);
    }

    /// Opens `block`, `level` deep in the writing. Every block Markdown
    /// writes closes a paragraph open round it, as a `<div>` does.
    pub fn enclose(&mut self, block: Block, level: usize) {
        self.close_paragraph();
        self.stack.push(Opened {
            name: String::new(),
            written: String::new(),
            holds: Holds::Block(block, level),
            at: 0..0,
        });
    }

    /// Closes the block `level` deep in the writing, and what is opened
    /// inside it, unless an element open inside it keeps its end tag from
    /// reaching it.
    pub fn unenclose(&mut self, level: usize) {
        let found = (self.stack.iter()).rposition(|opened| match opened.holds {
            Holds::Block(_, at) => at == level,
            _ => false,
        });
        let Some(at) = found else {
            return;
        };
        let Holds::Block(block, _) = self.stack[at].holds else {
            unreachable!("a block");
        };
        let scope = match block {
            Block::Paragraph { .. } => BUTTON_SCOPE,
            Block::Item => LIST_ITEM_SCOPE,
            Block::Cell => TABLE_SCOPE,
            Block::Other => SCOPE,
        };
        let swallowed = self.stack.last().is_some_and(Opened::swallows);
        match !swallowed && in_scope(&self.stack[at + 1..], scope) {
            true => self.close(at, true),
            false => {
                self.stack.remove(at);
            }
        }
    }

    /// The innermost element the raw HTML holds open, by its name as
    /// written, other than one a browser closes by itself; none inside a
    /// comment or a text element, which takes in what follows.
    pub fn holding(&self) -> Option<&str> {
        if self.stack.last().is_some_and(Opened::swallows) {
            return None;
        }
        (self.stack.iter().rev())
            .find(|opened| opened.holds == Holds::Markup && !opened.is(CLOSED_BY_ITSELF))
            .map(|opened| opened.written.as_str())
    }

    /// Every tag told, at its place in the masked text; and what a
    /// paragraph at the top of the writing left open and closed for it,
    /// which reaches past a value shown as that paragraph's words alone.
    pub fn finish(mut self) -> (Vec<Told>, Vec<Told>) {
        self.settle();
        // A comment or a text element left open takes in everything after
        // it: the end tags of what is open below it too.
        if let Some(top) = self.stack.pop_if(|top| top.swallows()) {
            let unmatched = match top.holds {
                Holds::Comment => Unmatched::CommentNeverClosed,
                _ => Unmatched::NeverClosed(top.written),
            };
            self.told.push((unmatched, top.at));
            return (self.told, Vec::new());
        }
        let reopened = self.reopened.into_iter().map(|reopened| reopened.opened);
        for opened in self.stack.into_iter().chain(reopened) {
            if opened.holds == Holds::Markup && !opened.is(CLOSED_BY_ITSELF) {
                (self.told).push((Unmatched::NeverClosed(opened.written), opened.at));
            }
        }
        let contained = (self.held.into_iter())
            .filter(|held| held.top)
            .map(|held| (Unmatched::NeverClosed(held.opened.written), held.opened.at))
            .collect();
        (self.told, contained)
    }

    /// The tag `tag` of `text`, its name at `name`, placed at `at`.
    fn met(&mut self, text: &str, name: usize, tag: &html::Tag, closing: bool, at: Range<usize>) {
        let written = &text[name..name + tag.name.len()];
        match closing {
            true => self.end(&tag.name, written, at),
            false => {
                let b = &text.as_bytes()[..tag.end];
                self.start(&tag.name, written, at, b.ends_with(b"/>"));
            }
        }
    }

    fn start(&mut self, name: &str, written: &str, at: Range<usize>, closes_itself: bool) {
        if CLOSE_A_PARAGRAPH.contains(&name) {
            self.close_paragraph();
        }
        if VOID.contains(&name) {
            return;
        }
        // Inside SVG or MathML, and for either itself, `/>` closes what it
        // opens; elsewhere a browser reads past it.
        let foreign = ["svg", "math"].iter().any(|&n| self.is_open(n));
        if closes_itself && (foreign || name == "svg" || name == "math") {
            return;
        }
        // A link closes the link before it.
        if name == "a" {
            self.reopened.retain(|reopened| reopened.opened.name != "a");
            if self.is_open("a") {
                self.end_formatting("a");
            }
        }
        let holds = match !foreign && (name == "script" || RAW_TEXT.contains(&name)) {
            true => Holds::Text,
            false => Holds::Markup,
        };
        *self.named.entry(name.to_owned()).or_default() += 1;
        self.stack.push(Opened {
            name: name.to_owned(),
            written: written.to_owned(),
            holds,
            at,
        });
    }

    fn end(&mut self, name: &str, written: &str, at: Range<usize>) {
        // With no paragraph open, `</p>` is an empty one.
        if name == "p" {
            self.close_paragraph();
            return;
        }
        if !self.is_open(name) {
            let reopened = (self.reopened.iter()).rposition(|r| r.opened.name == name);
            let held = (self.held.iter()).rposition(|held| held.opened.name == name);
            if let Some(reopened) = reopened {
                self.reopened.remove(reopened);
            } else if let Some(held) = held {
                self.held.remove(held);
            } else if !CLOSED_BY_ITSELF.contains(&name) && !VOID.contains(&name) {
                (self.told).push((Unmatched::ClosesNothing(written.to_owned()), at));
            }
            return;
        }
        if FORMATTING.contains(&name) {
            self.end_formatting(name);
            return;
        }
        let open = self.innermost(name);
        let above = &self.stack[open + 1..];
        let closes = match name {
            "li" => in_scope(above, LIST_ITEM_SCOPE),
            "table" | "caption" | "td" | "th" | "tr" | "thead" | "tbody" | "tfoot" => {
                in_scope(above, TABLE_SCOPE)
            }
            _ if SPECIAL.contains(&name) => in_scope(above, SCOPE),
            _ => !above.iter().any(Opened::is_special),
        };
        if closes {
            self.close(open, false);
        }
    }

    /// A formatting element's end tag closes it. What is open inside it
    /// closes with it, unless an element there holds markup of its own,
    /// which a browser keeps open past it.
    fn end_formatting(&mut self, name: &str) {
        let open = self.innermost(name);
        let above = &self.stack[open + 1..];
        if !in_scope(above, SCOPE) {
            return;
        }
        match above.iter().any(Opened::is_special) {
            true => {
                let opened = self.stack.remove(open);
                self.forget(&opened);
            }
            false => self.close(open, false),
        }
    }

    /// Closes the paragraph open round what comes, where there is one.
    fn close_paragraph(&mut self) {
        let Some(open) = self.stack.iter().rposition(Opened::is_paragraph) else {
            return;
        };
        if in_scope(&self.stack[open + 1..], BUTTON_SCOPE) {
            self.close(open, true);
        }
    }

    /// Closes the element at `at` in the stack, and every element above
    /// it, `implicitly` where the author wrote no end tag for it. A
    /// formatting element closed so is opened again round what follows,
    /// unless a marker closed with it stops that.
    fn close(&mut self, at: usize, implicitly: bool) {
        self.reopened
            .retain(|reopened| reopened.within.is_none_or(|marker| marker < at));
        let within = self.stack[..at].iter().rposition(Opened::is_marker);
        let top = matches!(
            self.stack[at].holds,
            Holds::Block(Block::Paragraph { top: true }, _)
        );
        let mut marked = false;
        let closed: Vec<Opened> = self.stack.drain(at..).collect();
        for (n, opened) in closed.into_iter().enumerate() {
            self.forget(&opened);
            marked |= opened.is_marker();
            if n == 0 || opened.holds != Holds::Markup {
                continue;
            }
            if opened.is(FORMATTING) {
                if !marked {
                    self.reopened.push(Reopened { opened, within });
                }
            } else if implicitly && !opened.is(CLOSED_BY_ITSELF) {
                self.held.push(Held { opened, top });
            }
        }
    }

    fn forget(&mut self, opened: &Opened) {
        if let Some(open) = self.named.get_mut(&opened.name) {
            *open -= 1;
        }
    }

    fn is_open(&self, name: &str) -> bool {
        self.named.get(name).is_some_and(|&open| open > 0)
    }

    /// Where in the stack the innermost element named `name` is.
    fn innermost(&self, name: &str) -> usize {
        (self.stack.iter())
            .rposition(|opened| opened.is(&[name]))
            .expect("an element open by that name")
    }
}

/// Where the tag from `lt` to `end` in `text` ends on its first line,
/// without the line's ending.
fn first_line(text: &str, lt: usize, end: usize) -> usize {
    let line = text[lt..end].find('\n').map_or(end, |n| lt + n);
    lt + text[lt..line].trim_end().len()
}
