//! How a built page's source keeps the indentation its templates were
//! written with. What a template puts on a line of its own follows that
//! line's indent on each of its lines, a line that came to nothing is not
//! written, and each line is written where `landing` says.
//!
//! The engine can't say what column a value is printed at, so each render
//! writes to a [`Sink`] that keeps account of the line being written, and
//! the formatter asks it.

use std::cell::RefCell;
use std::io;

use minijinja::{Captured, Error, Output, Template, Value};

use super::landing::{Deeper, Landing, Placing};

/// The line a template is writing.
struct Line {
    /// Where it starts in the output.
    start: usize,
    /// The spaces and tabs it opens with.
    indent: String,
    /// Whether nothing but those has been written on it.
    open: bool,
    /// Whether a value that came to nothing was put on it.
    emptied: bool,
    /// Whether the value put before it closed with its own line ending.
    ended: bool,
    /// Whether what is being written is a value, not the template's own
    /// text.
    value: bool,
}

thread_local! {
    /// One for each template being rendered, a snippet's above its
    /// caller's. Output the engine holds back, such as a `{% set %}`
    /// block's, never reaches a sink, so inside one the line is stale: it
    /// is nearly always an empty one, which changes nothing.
    static LINES: RefCell<Vec<Line>> = const { RefCell::new(Vec::new()) };
}

/// A run of what a template wrote that wasn't a value's: where it lies in
/// the output, and where in memory it was written from. The engine writes
/// a template's own text from the text it holds, so that says where in the
/// template the run is written.
#[derive(Copy, Clone, Debug)]
struct Run {
    at: usize,
    from: usize,
    len: usize,
}

/// Where what a template wrote is its own text, in the order written.
#[derive(Default, Debug)]
pub struct Own(Vec<Run>);

impl Own {
    /// Where in memory the byte at `at` of the output was written from;
    /// none for a byte of a value's.
    pub(super) fn written_from(&self, at: usize) -> Option<usize> {
        let after = self.0.partition_point(|run| run.at <= at);
        let run = self.0[..after].last()?;
        (at - run.at < run.len).then(|| run.from + (at - run.at))
    }

    /// Forgets what was written from `len` on, which the output no longer
    /// holds.
    fn truncate(&mut self, len: usize) {
        while let Some(run) = self.0.last_mut() {
            if run.at + run.len <= len {
                break;
            }
            if run.at < len {
                run.len = len - run.at;
                break;
            }
            self.0.pop();
        }
    }
}

/// What one template writes, and where its lines are written.
struct Sink<'p> {
    out: Vec<u8>,
    own: Own,
    placing: Placing<'p>,
    /// Whether what it writes starts a line where it is put.
    begins: bool,
    /// Where in memory the template's own text last written ends, until
    /// anything else is written.
    last: Option<usize>,
    /// The recursive loops' calls being run, the deepest last.
    deeper: Vec<&'p Deeper>,
    /// How long the output was as the last line whose line ending is kept
    /// began to write what it holds, until a kept line ending is reached.
    /// Where that line's own is left out, as in a branch not taken, the
    /// next kept one ends what it wrote.
    began: Option<usize>,
}

impl<'p> Sink<'p> {
    fn new(placing: Placing<'p>, begins: bool) -> Self {
        LINES.with_borrow_mut(|lines| {
            lines.push(Line {
                start: 0,
                indent: String::new(),
                open: true,
                emptied: false,
                ended: false,
                value: false,
            });
        });
        Self {
            out: Vec::new(),
            own: Own::default(),
            placing,
            begins,
            last: None,
            deeper: Vec::new(),
            began: None,
        }
    }

    /// Writes the template's own text, each line that starts a line of the
    /// output at the indent [`Placing`] gives it. A line that came to
    /// nothing, spaces before its line ending and no more, isn't written.
    fn own(&mut self, mut text: &[u8]) {
        let mut from = text.as_ptr() as usize;
        // A kept line ending is not written where nothing has been written
        // for it to end.
        if let Some(line) = self.placing.holding(from)
            && line.ending == Some(from)
        {
            let wrote = self.began.take().is_some_and(|len| self.out.len() > len);
            if !wrote {
                let ending = if text.starts_with(b"\r\n") { 2 } else { 1 };
                text = &text[ending.min(text.len())..];
                from += ending;
            }
        }
        let mut done = 0;
        for line in self.placing.starting(from..from + text.len()) {
            let at = line.at - from;
            let starts = if at > 0 {
                self.ending(&text[done..at]);
                done = at;
                true
            } else {
                self.begins || !self.out.is_empty()
            };
            let end = at + line.indent;
            if end > text.len() || blank(&text[at..end]) < line.indent {
                continue;
            }
            if !starts || !self.open() {
                // An indent kept for a line of the output is no use inside one.
                if line.kept {
                    self.put(&text[done..at]);
                    done = end;
                }
                continue;
            }
            if line.movable && self.land(line, Some(&text[at..end])) {
                done = end;
            }
        }
        if done < text.len() {
            self.ending(&text[done..]);
        }
    }

    /// Writes the indent of `line`, which starts the line being written, as
    /// [`Placing`] and the recursive loops being run say, in place of any
    /// spaces already on it. `typed` is the indent as the template has it,
    /// where it is still to write. Whether anything was written.
    fn land(&mut self, line: &Landing, typed: Option<&[u8]>) -> bool {
        let written = self.placing.written(line);
        let deeper: String = self.deeper.iter().map(|call| &*call.by).collect();
        let dangling =
            LINES.with_borrow(|lines| lines.last().is_some_and(|line| line.start < self.out.len()));
        if written.is_none() && deeper.is_empty() && !dangling {
            return false;
        }
        self.clear();
        self.made(&deeper);
        match (written, typed) {
            (Some(written), _) => self.made(written),
            (None, Some(typed)) => self.put(typed),
            (None, None) => {}
        }
        true
    }

    /// Whether nothing but spaces is written on the line being written.
    fn open(&self) -> bool {
        LINES.with_borrow(|lines| lines.last().is_some_and(|line| line.open))
    }

    /// Writes `text`, which ends a line of the template if it ends with a
    /// line ending, unless the line came to nothing.
    fn ending(&mut self, text: &[u8]) {
        let nothing = text.ends_with(b"\n")
            && text.iter().all(u8::is_ascii_whitespace)
            && self.open()
            && self
                .placing
                .holding(text.as_ptr() as usize + text.len() - 1)
                .is_some_and(|line| line.holds);
        if nothing {
            self.clear();
        } else {
            self.put(text);
        }
    }

    /// Takes back the spaces written on the line being written.
    fn clear(&mut self) {
        LINES.with_borrow_mut(|lines| {
            let line = lines.last_mut().expect("a sink has its line");
            self.out.truncate(line.start);
            self.own.truncate(line.start);
            line.indent.clear();
            line.open = true;
            line.emptied = false;
            line.ended = false;
        });
    }

    /// Writes an indent that is none of the template's text.
    fn made(&mut self, indent: &str) {
        self.out.extend_from_slice(indent.as_bytes());
        LINES.with_borrow_mut(|lines| {
            let line = lines.last_mut().expect("a sink has its line");
            line.indent.push_str(indent);
        });
    }

    /// Notes a recursive loop's call starting to run, from what was written
    /// before it and what is written now, or the call ending, from the
    /// template's own text written after it. `own` is where in memory the
    /// text now written is, where it is the template's, and `last` where its
    /// own text written before ends.
    fn track(&mut self, own: Option<usize>, last: Option<usize>) {
        if let Some(own) = own
            && self.deeper.last().is_some_and(|call| call.end == own)
        {
            self.deeper.pop();
            return;
        }
        if let Some(call) = last.and_then(|last| self.placing.call(last))
            && own != Some(call.end)
        {
            self.deeper.push(call);
        }
    }
}

impl Drop for Sink<'_> {
    fn drop(&mut self) {
        LINES.with_borrow_mut(Vec::pop);
    }
}

impl io::Write for Sink<'_> {
    fn write(&mut self, text: &[u8]) -> io::Result<usize> {
        if text.is_empty() {
            return Ok(0);
        }
        let value = LINES.with_borrow(|lines| lines.last().is_some_and(|line| line.value));
        let from = text.as_ptr() as usize;
        let last = self.last.take();
        self.track((!value).then_some(from), last);
        if value || self.placing.is_empty() {
            self.put(text);
        } else {
            let len = self.out.len();
            self.own(text);
            let end = from + text.len();
            // Text reaching past the indent of a line whose line ending is
            // kept is what the line holds being written.
            if let Some(line) = self.placing.holding(end - 1)
                && line.ending.is_some_and(|ending| end <= ending)
                && end > line.at + line.indent
            {
                self.began = Some(len);
            }
            // A value opening a line of the template with no indent before
            // it is placed by that line, before it is written, so that each
            // of its lines takes the indent.
            if let Some(line) = self.placing.valued(end)
                && line.indent == 0
                && line.movable
                && (self.begins || !self.out.is_empty())
                && self.open()
            {
                self.land(line, None);
            }
            if let Some(line) = self.placing.holding(end)
                && line.ending.is_some()
                && line.at + line.indent == end
            {
                self.began = Some(self.out.len());
            }
        }
        if !value {
            self.last = Some(from + text.len());
        }
        Ok(text.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Sink<'_> {
    fn put(&mut self, mut text: &[u8]) {
        LINES.with_borrow_mut(|lines| {
            let line = lines.last_mut().expect("a sink has its line");
            // A line ending a value writes is the value's to keep: only
            // the template's own takes the place of one already written,
            // or goes with a line left empty.
            if line.value {
                line.ended = false;
                line.emptied &= text.is_empty();
            }
            if std::mem::take(&mut line.ended) {
                text = match text {
                    [b'\r', b'\n', rest @ ..] | [b'\n', rest @ ..] => rest,
                    text => text,
                };
            }
            if line.emptied {
                let rest = &text[blank(text)..];
                let ending = match rest {
                    [b'\r', b'\n', ..] => 2,
                    [b'\n', ..] => 1,
                    _ => 0,
                };
                if ending > 0 {
                    self.out.truncate(line.start);
                    self.own.truncate(line.start);
                    line.indent.clear();
                    line.open = true;
                    line.emptied = false;
                    text = &rest[ending..];
                } else if !rest.is_empty() {
                    line.emptied = false;
                }
            }
            if !line.value && !text.is_empty() {
                self.own.0.push(Run {
                    at: self.out.len(),
                    from: text.as_ptr() as usize,
                    len: text.len(),
                });
            }
            self.out.extend_from_slice(text);
            let tail = match text.iter().rposition(|&byte| byte == b'\n') {
                Some(at) => {
                    let tail = &text[at + 1..];
                    line.start = self.out.len() - tail.len();
                    line.indent.clear();
                    line.open = true;
                    line.emptied = false;
                    tail
                }
                None => text,
            };
            if line.open {
                let blank = blank(tail);
                line.indent
                    .extend(tail[..blank].iter().map(|&byte| char::from(byte)));
                line.open = blank == tail.len();
            }
        });
    }
}

/// How many spaces and tabs `text` opens with.
fn blank(text: &[u8]) -> usize {
    text.iter()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count()
}

/// Renders `template`, giving what it wrote beside the engine's own
/// account. `begins` says whether what it writes starts a line where it is
/// put, so that its first line may be written as [`Placing`] says.
pub(super) fn render<'source>(
    template: &Template<'_, 'source>,
    context: impl Into<Value>,
    placing: Placing<'_>,
    begins: bool,
) -> Result<(Captured<'source>, String), Error> {
    render_placed(template, context, placing, begins).map(|(captured, text, _)| (captured, text))
}

/// Renders `template` as [`render`] does, giving as well where what it
/// wrote is its own text.
pub(super) fn render_placed<'source>(
    template: &Template<'_, 'source>,
    context: impl Into<Value>,
    placing: Placing<'_>,
    begins: bool,
) -> Result<(Captured<'source>, String, Own), Error> {
    let mut sink = Sink::new(placing, begins);
    let captured = template.render_captured_to(context, &mut sink)?;
    let text = String::from_utf8(std::mem::take(&mut sink.out)).expect("templates write text");
    Ok((captured, text, std::mem::take(&mut sink.own)))
}

/// Whether nothing but spaces has been written on the line the template
/// being rendered is writing.
pub(super) fn at_line_start() -> bool {
    LINES.with_borrow(|lines| lines.last().is_some_and(|line| line.open))
}

/// Runs `write`, which writes a value, so that the line endings it writes
/// are known for the value's.
pub(super) fn value<T>(write: impl FnOnce() -> T) -> T {
    let set = |value| {
        LINES.with_borrow_mut(|lines| {
            if let Some(line) = lines.last_mut() {
                line.value = value;
            }
        });
    };
    set(true);
    let written = write();
    set(false);
    written
}

/// Notes that a value which came to nothing was put on the line, so the
/// line is not written if nothing else is put on it.
pub(super) fn nothing() {
    LINES.with_borrow_mut(|lines| {
        if let Some(line) = lines.last_mut()
            && line.open
        {
            line.emptied = true;
        }
    });
}

/// Notes that the value just written closed with a line ending of its own,
/// so the one the template follows it with is not written as well: the two
/// would be a blank line neither file has.
pub(super) fn ended() {
    LINES.with_borrow_mut(|lines| {
        if let Some(line) = lines.last_mut()
            && line.open
            && line.indent.is_empty()
        {
            line.ended = true;
        }
    });
}

/// Writes markup of several lines with each line after its first indented
/// as the line it is put on, where it is the first thing on that line.
/// `None` where there is nothing to add and the markup is still to write.
pub(super) fn write(out: &mut Output, markup: &str) -> Option<Result<(), Error>> {
    let indent = LINES.with_borrow(|lines| {
        lines
            .last()
            .filter(|line| line.open && !line.indent.is_empty())
            .map(|line| line.indent.clone())
    })?;
    if !markup.contains('\n') {
        return None;
    }
    Some(
        out.write_str(&indented(markup, &indent))
            .map_err(|_| Error::from(minijinja::ErrorKind::WriteFailure)),
    )
}

/// `markup` with `indent` before each line after its first. An empty line
/// stays empty, and so does a line the spaces would change: one inside
/// `<pre>` or `<textarea>`, where they would show, or inside an attribute's
/// value.
fn indented(markup: &str, indent: &str) -> String {
    let mut out = String::with_capacity(markup.len() + markup.len() / 8);
    let mut reader = Reader {
        escaped: !markup.contains('<'),
        ..Reader::default()
    };
    for (number, line) in markup.split_inclusive('\n').enumerate() {
        if number > 0 && reader.takes_indent() && !matches!(line, "\n" | "\r\n") {
            out.push_str(indent);
        }
        out.push_str(line);
        reader.read(line);
    }
    out
}

/// Where in the markup a line starts, as far as it decides whether spaces
/// before the line change anything. It reads markup as an author writes it,
/// or markup escaped whole to be carried as text, as a feed carries a post;
/// what it can't follow it takes for text.
#[derive(Default)]
struct Reader {
    /// Whether the markup is escaped whole. In markup as written, an
    /// escaped tag is words about one, such as `<pre>` named in a sentence.
    escaped: bool,
    within: Within,
    /// Inside `<pre>` or `<textarea>`.
    shown: bool,
    /// The `<script>` or `<style>` whose opening tag is being read.
    opening: Option<&'static str>,
}

#[derive(Default, Clone, Copy)]
enum Within {
    #[default]
    Text,
    Tag,
    /// An attribute's value, with the quote that closes it.
    Value(u8),
    Comment,
    /// A script or a stylesheet, which ends only at its own closing tag.
    Raw(&'static str),
}

impl Reader {
    fn takes_indent(&self) -> bool {
        !self.shown && !matches!(self.within, Within::Value(_))
    }

    fn read(&mut self, line: &str) {
        let bytes = line.as_bytes();
        let mut at = 0;
        while at < bytes.len() {
            let rest = &line[at..];
            match self.within {
                Within::Text => {
                    let Some(found) = rest.find(['<', '&']) else {
                        return;
                    };
                    let rest = &rest[found..];
                    at += found + 1;
                    if let Some(escaped) = rest.strip_prefix("&lt;") {
                        if self.escaped {
                            self.element(escaped);
                        }
                    } else if rest.starts_with("<!--") {
                        self.within = Within::Comment;
                    } else if let Some(tag) = rest.strip_prefix('<')
                        && tag.starts_with(|c: char| c == '/' || c.is_ascii_alphabetic())
                    {
                        self.opening = self.element(tag);
                        self.within = Within::Tag;
                    }
                }
                Within::Tag => {
                    let Some(found) = rest.find(['"', '\'', '>']) else {
                        return;
                    };
                    at += found + 1;
                    self.within = match rest.as_bytes()[found] {
                        b'>' => self.opening.take().map_or(Within::Text, Within::Raw),
                        quote => Within::Value(quote),
                    };
                }
                Within::Value(quote) => {
                    let Some(found) = rest.find(char::from(quote)) else {
                        return;
                    };
                    at += found + 1;
                    self.within = Within::Tag;
                }
                Within::Comment => {
                    let Some(found) = rest.find("-->") else {
                        return;
                    };
                    at += found + 3;
                    self.within = Within::Text;
                }
                Within::Raw(name) => {
                    let closes = rest
                        .match_indices("</")
                        .find(|(found, _)| names(&rest[found + 2..], name));
                    let Some((found, _)) = closes else {
                        return;
                    };
                    at += found + 2;
                    self.within = Within::Tag;
                }
            }
        }
    }

    /// Reads the name a tag opens with, `tag` being what follows its `<`:
    /// notes entering or leaving an element that shows its spaces, and gives
    /// the name of a script or stylesheet being opened.
    fn element(&mut self, tag: &str) -> Option<&'static str> {
        let (closes, name) = match tag.strip_prefix('/') {
            Some(name) => (true, name),
            None => (false, tag),
        };
        if names(name, "pre") || names(name, "textarea") {
            self.shown = !closes;
        }
        ["script", "style"]
            .into_iter()
            .find(|raw| !closes && names(name, raw))
    }
}

/// Whether `text` opens with the element name `name`, in either case.
fn names(text: &str, name: &str) -> bool {
    let text = text.as_bytes();
    text.len() >= name.len()
        && text[..name.len()].eq_ignore_ascii_case(name.as_bytes())
        && !text
            .get(name.len())
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'-')
}

#[cfg(test)]
mod tests {
    use super::{Own, Run, indented};

    #[test]
    fn a_templates_own_text_is_found_where_it_was_written_from() {
        let run = |at, from, len| Run { at, from, len };
        let mut own = Own(vec![run(0, 100, 4), run(6, 110, 3), run(9, 200, 5)]);
        assert_eq!(own.written_from(0), Some(100));
        assert_eq!(own.written_from(3), Some(103));
        // Between two runs a value was written.
        assert_eq!(own.written_from(4), None);
        assert_eq!(own.written_from(8), Some(112));
        assert_eq!(own.written_from(9), Some(200));
        assert_eq!(own.written_from(14), None);
        // A line taken back takes what was written on it.
        own.truncate(11);
        assert_eq!(own.written_from(10), Some(201));
        assert_eq!(own.written_from(11), None);
        own.truncate(5);
        assert_eq!(own.written_from(8), None);
        assert_eq!(own.written_from(3), Some(103));
    }

    #[test]
    fn each_further_line_takes_the_indent() {
        assert_eq!(
            indented("<ul>\n  <li>one</li>\n</ul>\n", "    "),
            "<ul>\n      <li>one</li>\n    </ul>\n"
        );
    }

    #[test]
    fn an_empty_line_stays_empty() {
        assert_eq!(
            indented("<p>a</p>\n\n<p>b</p>", "\t"),
            "<p>a</p>\n\n\t<p>b</p>"
        );
        assert_eq!(indented("a\r\n\r\nb", "  "), "a\r\n\r\n  b");
    }

    #[test]
    fn lines_shown_as_written_are_left() {
        assert_eq!(
            indented(
                "<p>a</p>\n<pre><code>one\n  two</code></pre>\n<p>b</p>",
                "  "
            ),
            "<p>a</p>\n  <pre><code>one\n  two</code></pre>\n  <p>b</p>"
        );
        assert_eq!(
            indented("<TextArea>\none\n</TEXTAREA>\n<p>b</p>", "  "),
            "<TextArea>\none\n</TEXTAREA>\n  <p>b</p>"
        );
        assert_eq!(indented("<pretty>\none", "  "), "<pretty>\n  one");
    }

    #[test]
    fn an_attributes_value_is_left_and_its_tags_other_lines_are_not() {
        assert_eq!(
            indented("<img alt=\"one\ntwo\"\n  src=\"a.png\">\n<p>b</p>", "  "),
            "<img alt=\"one\ntwo\"\n    src=\"a.png\">\n  <p>b</p>"
        );
    }

    #[test]
    fn scripts_styles_and_comments_are_read_through() {
        assert_eq!(
            indented(
                "<script>\nif (a < b) {\n  c = \"\n}\n</script>\n<p>b</p>",
                "  "
            ),
            "<script>\n  if (a < b) {\n    c = \"\n  }\n  </script>\n  <p>b</p>"
        );
        assert_eq!(
            indented("<!-- it's\n<pre>\n-->\n<p>b</p>", "  "),
            "<!-- it's\n  <pre>\n  -->\n  <p>b</p>"
        );
        assert_eq!(indented("a < b and \"c\nd", "  "), "a < b and \"c\n  d");
    }

    #[test]
    fn a_tag_named_in_the_words_opens_nothing() {
        assert_eq!(
            indented(
                "<p>Inside <code>&lt;pre&gt;</code>, nothing is.</p>\n<p>b</p>\n<p>c</p>",
                "  "
            ),
            "<p>Inside <code>&lt;pre&gt;</code>, nothing is.</p>\n  <p>b</p>\n  <p>c</p>"
        );
    }

    #[test]
    fn escaped_markup_keeps_what_it_shows() {
        assert_eq!(
            indented(
                "&lt;p&gt;a&lt;/p&gt;\n&lt;pre&gt;one\n  two&lt;/pre&gt;\n&lt;p&gt;b",
                "  "
            ),
            "&lt;p&gt;a&lt;/p&gt;\n  &lt;pre&gt;one\n  two&lt;/pre&gt;\n  &lt;p&gt;b"
        );
    }
}
