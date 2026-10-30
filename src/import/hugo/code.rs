//! What Hugo's options on a code block ask of its lines: the braces ending
//! a fence's first line, `{linenos=true,hl_lines=[2,"4-6"]}`, read as
//! Hugo's Markdown reader reads attributes, and the options handed to
//! `highlight`, `linenos=table,hl_lines=2 4-6`. Hugo counts the lines to
//! mark from the block's first line as 1, whatever number `linenostart`
//! shows there.

/// What a code block's options ask of its lines, as Hugo reads them.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub(super) struct Asked {
    pub(super) numbered: bool,
    pub(super) marked: Vec<(i64, i64)>,
    /// Whether some option has no equal in Statac.
    pub(super) left: bool,
}

/// A value in braces, as Hugo's Markdown reader reads it.
#[derive(Clone, PartialEq, Debug)]
enum Value {
    Text(String),
    Number(f64),
    Truth(bool),
    Null,
    List(Vec<Value>),
    Fields,
}

/// What the braces opening `braces` ask, where Hugo's reader reads them:
/// none where it doesn't, which stops Hugo's build.
pub(super) fn from_braces(braces: &str) -> Option<Asked> {
    let mut reader = Reader {
        text: braces.as_bytes(),
        at: 0,
    };
    let mut named: Vec<(String, Value)> = Vec::new();
    for (name, value) in reader.attributes()? {
        let name = name.to_ascii_lowercase();
        named.retain(|(known, _)| *known != name);
        named.push((name, value));
    }
    let start = named.iter().find(|(name, _)| name == "linenostart");
    let start = match start.map(|(_, value)| value) {
        None => Some(1),
        Some(Value::Number(n)) => Some(n.trunc() as i64),
        Some(Value::Text(text)) => text.trim().parse().ok(),
        Some(_) => None,
    };
    let mut asked = Asked::default();
    for (name, value) in &named {
        match (name.as_str(), value) {
            ("linenos", Value::Truth(on)) => asked.numbered = *on,
            ("linenos", Value::Text(text)) => asked.numbered = text != "false",
            ("hl_lines", Value::List(items)) => {
                asked.marked = items.iter().filter_map(item).collect()
            }
            ("hl_lines", Value::Text(text)) => asked.marked = runs(text),
            ("hl_lines", Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => {
                asked.marked = vec![(*n as i64, *n as i64)];
            }
            ("hl_lines", Value::Number(_)) | ("linenumbersintable" | "linenostart", _) => {}
            _ => asked.left = true,
        }
    }
    asked.left |= asked.numbered && start != Some(1);
    Some(asked)
}

/// What the options handed to `highlight` ask, `linenos=table,hl_lines=1`,
/// read as Hugo reads them.
pub(super) fn from_options(options: &str) -> Asked {
    let options = options.trim_matches(' ');
    let mut named: Vec<(String, &str)> = Vec::new();
    if !options.is_empty() {
        for option in options.split(',') {
            let Some((name, value)) = option.split_once('=').filter(|(_, v)| !v.contains('='))
            else {
                return Asked {
                    left: true,
                    ..Asked::default()
                };
            };
            let name = name.trim_matches(' ').to_ascii_lowercase();
            named.retain(|(known, _)| *known != name);
            named.push((name, value));
        }
    }
    let mut asked = Asked::default();
    let mut start = Some(1);
    for (name, value) in named {
        match name.as_str() {
            "linenos" => asked.numbered = value != "false",
            "hl_lines" => asked.marked = runs(value),
            "linenostart" => start = value.trim().parse().ok(),
            "linenumbersintable" => {}
            _ => asked.left = true,
        }
    }
    asked.left |= asked.numbered && start != Some(1);
    asked
}

/// The lines one item of `hl_lines=[2,"4-6"]` marks: a number, or a
/// number or run of them in quotes. None where Hugo reads none.
fn item(value: &Value) -> Option<(i64, i64)> {
    match value {
        Value::Number(n) => Some((n.trunc() as i64, n.trunc() as i64)),
        Value::Text(text) => {
            let mut numbers = text.split('-');
            let first: i64 = numbers.next()?.parse().ok()?;
            let last = match numbers.next() {
                Some(last) => last.parse().ok()?,
                None => first,
            };
            Some((first, last))
        }
        _ => None,
    }
}

/// The lines `hl_lines="2 4-6"` marks, runs apart by spaces: none at all
/// where one can't be read.
fn runs(text: &str) -> Vec<(i64, i64)> {
    let mut runs = Vec::new();
    for run in text.trim().split(' ').filter(|run| !run.is_empty()) {
        let mut numbers = run.split('-');
        let first: Option<i64> = numbers.next().and_then(|n| n.parse().ok());
        let last = match numbers.next() {
            Some(last) => last.parse().ok(),
            None => first,
        };
        match (first, last) {
            (Some(first), Some(last)) => runs.push((first, last)),
            _ => return Vec::new(),
        }
    }
    runs
}

/// Braces read as Hugo's Markdown reader reads attributes.
struct Reader<'a> {
    text: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    /// Spaces, but not line endings.
    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\x0b' | b'\x0c')) {
            self.at += 1;
        }
    }

    fn name_byte(c: u8, first: bool) -> bool {
        c.is_ascii_alphabetic()
            || matches!(c, b'_' | b':')
            || !first && (c.is_ascii_digit() || matches!(c, b'.' | b'-'))
    }

    /// A run of bytes a name holds, at least one.
    fn name(&mut self) -> Option<String> {
        if !Self::name_byte(self.peek()?, true) {
            return None;
        }
        let start = self.at;
        while self.peek().is_some_and(|c| Self::name_byte(c, false)) {
            self.at += 1;
        }
        Some(String::from_utf8_lossy(&self.text[start..self.at]).into_owned())
    }

    fn attributes(&mut self) -> Option<Vec<(String, Value)>> {
        self.skip_spaces();
        if self.peek()? != b'{' {
            return None;
        }
        self.at += 1;
        let mut attributes = Vec::new();
        loop {
            if self.peek()? == b'}' {
                self.at += 1;
                return Some(attributes);
            }
            attributes.push(self.attribute()?);
            self.skip_spaces();
            if self.peek() == Some(b',') {
                self.at += 1;
                self.skip_spaces();
            }
        }
    }

    fn attribute(&mut self) -> Option<(String, Value)> {
        self.skip_spaces();
        let c = self.peek()?;
        if matches!(c, b'#' | b'.') {
            self.at += 1;
            let start = self.at;
            while self.peek().is_some_and(|c| {
                !c.is_ascii_whitespace()
                    && (!c.is_ascii_punctuation() || matches!(c, b'_' | b'-' | b':' | b'.'))
            }) {
                self.at += 1;
            }
            let value = String::from_utf8_lossy(&self.text[start..self.at]).into_owned();
            let name = if c == b'#' { "id" } else { "class" };
            return Some((name.to_owned(), Value::Text(value)));
        }
        let name = self.name()?;
        self.skip_spaces();
        if self.peek()? != b'=' {
            return None;
        }
        self.at += 1;
        let value = self.value()?;
        if name == "class" && !matches!(value, Value::Text(_)) {
            return None;
        }
        Some((name, value))
    }

    fn value(&mut self) -> Option<Value> {
        self.skip_spaces();
        match self.peek()? {
            b'{' => self.attributes().map(|_| Value::Fields),
            b'[' => self.list(),
            b'"' => self.quoted().map(Value::Text),
            c if c == b'-' || c == b'+' || c.is_ascii_digit() => self.number(),
            _ => {
                let word = self.name()?;
                Some(match word.as_str() {
                    "true" => Value::Truth(true),
                    "false" => Value::Truth(false),
                    "null" => Value::Null,
                    _ => Value::Text(word),
                })
            }
        }
    }

    fn list(&mut self) -> Option<Value> {
        self.at += 1;
        let mut items = Vec::new();
        for i in 0.. {
            let c = self.peek()?;
            let comma = i != 0 && c == b',';
            if comma {
                self.at += 1;
            }
            if c == b']' {
                self.at += 1;
                return Some(Value::List(items));
            }
            self.skip_spaces();
            items.push(self.value()?);
            self.skip_spaces();
        }
        None
    }

    /// A value in double quotes, with JSON's backslashes, on one line.
    fn quoted(&mut self) -> Option<String> {
        self.at += 1;
        let mut value = Vec::new();
        while let Some(c) = self.peek() {
            match c {
                b'\n' | b'\r' => return None,
                b'"' => {
                    self.at += 1;
                    return Some(String::from_utf8_lossy(&value).into_owned());
                }
                b'\\'
                    if self
                        .text
                        .get(self.at + 1)
                        .is_some_and(|n| !matches!(n, b'\n' | b'\r')) =>
                {
                    let next = self.text[self.at + 1];
                    match next {
                        b'"' | b'/' | b'\\' => value.push(next),
                        b'b' => value.push(b'\x08'),
                        b'f' => value.push(b'\x0c'),
                        b'n' => value.push(b'\n'),
                        b'r' => value.push(b'\r'),
                        b't' => value.push(b'\t'),
                        _ => {
                            value.push(b'\\');
                            self.at += 1;
                            continue;
                        }
                    }
                    self.at += 2;
                }
                _ => {
                    value.push(c);
                    self.at += 1;
                }
            }
        }
        None
    }

    fn number(&mut self) -> Option<Value> {
        let mut written = String::new();
        if let Some(sign @ (b'-' | b'+')) = self.peek() {
            if sign == b'-' {
                written.push('-');
            }
            self.at += 1;
        }
        let digits = |reader: &mut Self, written: &mut String| {
            while let Some(c) = reader.peek().filter(u8::is_ascii_digit) {
                written.push(c as char);
                reader.at += 1;
            }
        };
        if !self.peek()?.is_ascii_digit() {
            return None;
        }
        digits(self, &mut written);
        if self.peek() == Some(b'.') {
            written.push('.');
            self.at += 1;
            digits(self, &mut written);
        }
        if let Some(e @ (b'e' | b'E')) = self.peek() {
            written.push(e as char);
            self.at += 1;
            if let Some(sign @ (b'-' | b'+')) = self.peek() {
                written.push(sign as char);
                self.at += 1;
            }
            digits(self, &mut written);
        }
        written.parse().ok().map(Value::Number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asked(numbered: bool, marked: &[(i64, i64)], left: bool) -> Asked {
        Asked {
            numbered,
            marked: marked.to_vec(),
            left,
        }
    }

    #[test]
    fn braces_are_read_as_hugo_reads_them() {
        for (braces, expected) in [
            ("{linenos=true}", Some(asked(true, &[], false))),
            ("{linenos=table}", Some(asked(true, &[], false))),
            ("{linenos=inline}", Some(asked(true, &[], false))),
            ("{linenos=false}", Some(asked(false, &[], false))),
            ("{linenos=\"false\"}", Some(asked(false, &[], false))),
            ("{lineNos=true}", Some(asked(true, &[], false))),
            ("{linenos=1}", Some(asked(false, &[], true))),
            (
                "{hl_lines=[2,\"4-6\"]}",
                Some(asked(false, &[(2, 2), (4, 6)], false)),
            ),
            (
                "{hl_lines=[ 2 , 3 ]}",
                Some(asked(false, &[(2, 2), (3, 3)], false)),
            ),
            (
                "{hl_lines=[\"x\", 3]}",
                Some(asked(false, &[(3, 3)], false)),
            ),
            (
                "{hl_lines=\"2 4-6\"}",
                Some(asked(false, &[(2, 2), (4, 6)], false)),
            ),
            ("{hl_lines=\"2 x\"}", Some(asked(false, &[], false))),
            ("{hl_lines=3}", Some(asked(false, &[(3, 3)], false))),
            (
                "{linenos=table,hl_lines=[8,\"15-17\"],linenostart=199}",
                Some(asked(true, &[(8, 8), (15, 17)], true)),
            ),
            (
                "{linenos=true linenostart=1}",
                Some(asked(true, &[], false)),
            ),
            ("{linenostart=5}", Some(asked(false, &[], false))),
            (
                "{linenos=true,linenos=false}",
                Some(asked(false, &[], false)),
            ),
            ("{lineNumbersInTable=false}", Some(asked(false, &[], false))),
            ("{hl_inline=true}", Some(asked(false, &[], true))),
            ("{.wide}", Some(asked(false, &[], true))),
            ("{#example linenos=true}", Some(asked(true, &[], true))),
            ("{style=monokai}", Some(asked(false, &[], true))),
            ("{}", Some(asked(false, &[], false))),
            ("{linenos=}", None),
            ("{linenos true}", None),
            ("{hl_lines=[2,]}", None),
            ("{linenos=true", None),
        ] {
            assert_eq!(from_braces(braces), expected, "{braces}");
        }
    }

    #[test]
    fn options_handed_to_highlight_are_read_as_hugo_reads_them() {
        for (options, expected) in [
            ("", asked(false, &[], false)),
            ("linenos=table,hl_lines=1", asked(true, &[(1, 1)], false)),
            (
                "linenos=inline, hl_lines=3 6-8",
                asked(true, &[(3, 3), (6, 8)], false),
            ),
            ("linenos=false", asked(false, &[], false)),
            ("linenos=no", asked(true, &[], false)),
            ("lineNos=true,lineNoStart=1", asked(true, &[], false)),
            ("linenos=true,linenostart=199", asked(true, &[], true)),
            ("linenostart=199", asked(false, &[], false)),
            ("hl_lines=2 x", asked(false, &[], false)),
            ("style=emacs", asked(false, &[], true)),
            ("linenos", asked(false, &[], true)),
            ("hl_lines=1=2", asked(false, &[], true)),
        ] {
            assert_eq!(from_options(options), expected, "{options}");
        }
    }
}
