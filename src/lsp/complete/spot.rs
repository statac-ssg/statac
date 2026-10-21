//! Where in YAML the editor's cursor stands, read as the editor holds the
//! text while the author types: in a field's name or in its value, and
//! under which fields.

use std::ops::Range;

use crate::content::words::key_name;
use crate::source::{Place, Sources};
use crate::yaml::{Node, Value};

/// Where the cursor stands.
#[derive(Clone, PartialEq, Eq, Debug)]
pub(super) enum Spot {
    /// In a field's name, in the mapping under the fields `path` names.
    Name {
        path: Vec<String>,
        /// The name typed so far, up to the cursor.
        typed: Range<usize>,
        /// The names the mapping holds besides.
        written: Vec<String>,
        /// Whether the name has its colon after it already.
        colon: bool,
    },
    /// In the value of the field `path` names, or in an item of its list.
    Value {
        path: Vec<String>,
        /// The value typed so far, up to the cursor, inside any quote.
        typed: Range<usize>,
        /// Whether it is an item of a list in brackets.
        flow: bool,
        /// The quote the value opens with.
        quote: Option<char>,
        /// The other items of its list.
        written: Vec<String>,
        /// Whether the cursor is straight after the field's colon, so a
        /// space must come before the value.
        spaced: bool,
    },
}

/// Where `offset` stands in the YAML at `at`, or none where it is outside
/// it or in a comment.
pub(super) fn find(sources: &Sources, at: Place, offset: usize) -> Option<Spot> {
    let region = at.span.range();
    if !holds(&region, offset) {
        return None;
    }
    let text = sources.text(at.source);
    let line = line_start(text, offset).max(region.start);
    if in_comment(&text[line..offset]) {
        return None;
    }
    let root = crate::yaml::read_partial(sources, at).node;
    if let Some(spot) = in_tree(text, &root, offset, &mut Vec::new()) {
        return Some(spot);
    }
    by_lines(text, region.start, &root, offset)
}

/// Where `offset` stands among the nodes the tree holds.
fn in_tree(text: &str, node: &Node, offset: usize, path: &mut Vec<String>) -> Option<Spot> {
    match &node.value {
        Value::Mapping(entries) => {
            for entry in entries {
                let key = entry.key.place.span.range();
                if holds(&key, offset) {
                    let written = (entries.iter())
                        .filter(|other| !std::ptr::eq(*other, entry))
                        .map(|other| key_name(&other.key))
                        .collect();
                    let typed = key.start..offset;
                    let path = path.clone();
                    return Some(Spot::Name {
                        path,
                        typed,
                        written,
                        colon: true,
                    });
                }
                let value = entry.value.place.span.range();
                // An empty value is placed at its colon, and is read by
                // its line.
                if value.is_empty() || !holds(&value, offset) {
                    continue;
                }
                path.push(key_name(&entry.key));
                if !matches!(entry.value.value, Value::Scalar(_)) {
                    return in_tree(text, &entry.value, offset, path);
                }
                // A word alone on a line below its field's name is the
                // name of a field inside it, its colon not yet typed.
                let below = text[key.end..value.start].contains('\n');
                let word = &text[value.start..offset];
                if below && !word.contains(char::is_whitespace) && !word.contains(':') {
                    return Some(Spot::Name {
                        path: path.clone(),
                        typed: value.start..offset,
                        written: Vec::new(),
                        colon: false,
                    });
                }
                return Some(scalar(
                    text,
                    &entry.value,
                    offset,
                    path.clone(),
                    false,
                    Vec::new(),
                ));
            }
            None
        }
        Value::Sequence(items) => {
            let span = node.place.span.range();
            let flow = text[span.clone()].starts_with('[');
            for item in items {
                if !holds(&item.place.span.range(), offset) {
                    continue;
                }
                return match item.value {
                    Value::Scalar(_) => {
                        let written = scalars(items, offset);
                        Some(scalar(text, item, offset, path.clone(), flow, written))
                    }
                    _ => in_tree(text, item, offset, path),
                };
            }
            // Between the brackets, after one opens or a comma.
            if flow && offset > span.start && offset < span.end {
                return Some(Spot::Value {
                    path: path.clone(),
                    typed: offset..offset,
                    flow,
                    quote: None,
                    written: scalars(items, offset),
                    spaced: false,
                });
            }
            None
        }
        Value::Scalar(_) => None,
    }
}

/// The cursor in the scalar `node`.
fn scalar(
    text: &str,
    node: &Node,
    offset: usize,
    path: Vec<String>,
    flow: bool,
    written: Vec<String>,
) -> Spot {
    let span = node.place.span.range();
    let (mut start, quote) = match text[span.start..].chars().next() {
        Some(quote @ ('"' | '\'')) if offset > span.start => (span.start + 1, Some(quote)),
        _ => (span.start.min(offset), None),
    };
    // Only the line the cursor is on is typed over.
    if let Some(newline) = text[start..offset].rfind('\n') {
        let line = start + newline + 1;
        start = line + (text[line..offset].len() - text[line..offset].trim_start().len());
    }
    Spot::Value {
        path,
        typed: start..offset,
        flow,
        quote,
        written,
        spaced: false,
    }
}

/// Where `offset` stands, read from the lines up to it: on a line the
/// tree leaves out, being part typed, or at a value not yet begun.
fn by_lines(text: &str, region: usize, root: &Node, offset: usize) -> Option<Spot> {
    let line = line_start(text, offset).max(region);
    let before = &text[line..offset];
    let indent = before.len() - before.trim_start_matches(' ').len();
    let mut at = line + indent;
    let mut dash = None;
    while let Some(after) = text[at..offset].strip_prefix('-')
        && (after.is_empty() || after.starts_with(' '))
    {
        dash = Some(at - line);
        at = offset - after.trim_start_matches(' ').len();
    }
    let rest = &text[at..offset];
    if let Some(colon) = colon_of(rest) {
        let mut path = parents(text, region, line, at - line)?;
        path.push(unquoted(rest[..colon].trim()));
        return Some(value_after(text, at + colon + 1, offset, path, root));
    }
    if let Some(dash) = dash {
        let path = item_parents(text, region, line, dash)?;
        let (typed, quote) = opened(text, at, offset);
        let written = match at_path(root, &path).map(|node| &node.value) {
            Some(Value::Sequence(items)) => scalars(items, offset),
            _ => Vec::new(),
        };
        return Some(Spot::Value {
            path,
            typed,
            flow: false,
            quote,
            written,
            spaced: false,
        });
    }
    if rest.contains(|c: char| c.is_whitespace() || "#\"'[]{},&*!|>%@`".contains(c)) {
        return None;
    }
    let path = parents(text, region, line, indent)?;
    let written = match at_path(root, &path).map(|node| &node.value) {
        Some(Value::Mapping(entries)) => entries.iter().map(|e| key_name(&e.key)).collect(),
        _ => Vec::new(),
    };
    Some(Spot::Name {
        path,
        typed: at..offset,
        written,
        colon: false,
    })
}

/// The cursor in a value that begins at `after`, straight after its
/// field's colon.
fn value_after(text: &str, after: usize, offset: usize, path: Vec<String>, root: &Node) -> Spot {
    let value = &text[after..offset];
    let begun = after + (value.len() - value.trim_start().len());
    if text[begun..offset].starts_with('[') {
        let item = text[begun..offset]
            .rfind(['[', ','])
            .map_or(begun, |at| begun + at + 1);
        let item = offset - text[item..offset].trim_start().len();
        let (typed, quote) = opened(text, item, offset);
        let written = match at_path(root, &path).map(|node| &node.value) {
            Some(Value::Sequence(items)) => scalars(items, offset),
            _ => Vec::new(),
        };
        return Spot::Value {
            path,
            typed,
            flow: true,
            quote,
            written,
            spaced: false,
        };
    }
    let (typed, quote) = opened(text, begun, offset);
    Spot::Value {
        path,
        typed,
        flow: false,
        quote,
        written: Vec::new(),
        spaced: after == offset,
    }
}

/// What is typed from `start` to `offset`, inside the quote it opens with.
fn opened(text: &str, start: usize, offset: usize) -> (Range<usize>, Option<char>) {
    match text[start..offset].chars().next() {
        Some(quote @ ('"' | '\'')) => (start + 1..offset, Some(quote)),
        _ => (start..offset, None),
    }
}

/// The names of the fields a line at `column` stands under, read from the
/// lines above `line`; none where it stands in an item of a list.
fn parents(text: &str, region: usize, line: usize, column: usize) -> Option<Vec<String>> {
    let mut path = Vec::new();
    let mut column = column;
    let mut at = line;
    while column > 0 && at > region {
        let (start, above) = line_above(text, region, at);
        at = start;
        let Some((indent, written)) = written_line(above) else {
            continue;
        };
        if indent >= column {
            continue;
        }
        path.insert(0, holder(written)?);
        column = indent;
    }
    Some(path)
}

/// The names of the fields an item whose dash is at `dash` belongs to,
/// read from the lines above `line`.
fn item_parents(text: &str, region: usize, line: usize, dash: usize) -> Option<Vec<String>> {
    let mut at = line;
    while at > region {
        let (start, above) = line_above(text, region, at);
        at = start;
        let Some((indent, written)) = written_line(above) else {
            continue;
        };
        let item = written == "-" || written.starts_with("- ");
        if indent > dash || (indent == dash && item) {
            continue;
        }
        let mut path = parents(text, region, start, indent)?;
        path.push(holder(written)?);
        return Some(path);
    }
    None
}

/// The name of the field a line opens, where it opens one that holds what
/// is below it: `name:` with nothing after but a comment.
fn holder(written: &str) -> Option<String> {
    if written == "-" || written.starts_with("- ") {
        return None;
    }
    let colon = colon_of(written)?;
    let after = written[colon + 1..].trim();
    (after.is_empty() || after.starts_with('#')).then(|| unquoted(written[..colon].trim()))
}

/// The line above the one starting at `at`, and where it starts.
fn line_above(text: &str, region: usize, at: usize) -> (usize, &str) {
    let end = at - 1;
    let start = line_start(text, end).max(region);
    (start, text[start..end].trim_end_matches('\r'))
}

/// A line's indent and what is written after it, where it holds more
/// than a comment.
fn written_line(line: &str) -> Option<(usize, &str)> {
    let written = line.trim_start_matches(' ');
    let blank = written.trim().is_empty() || written.starts_with('#');
    (!blank).then(|| (line.len() - written.len(), written.trim_end()))
}

/// Where the colon after a field's name is: the first followed by a space
/// or ending the text, past any quoted name.
fn colon_of(written: &str) -> Option<usize> {
    let from = match written.chars().next() {
        Some(quote @ ('"' | '\'')) => written[1..].find(quote).map_or(written.len(), |at| at + 2),
        _ => 0,
    };
    let rest = &written[from..];
    let colon = rest
        .match_indices(':')
        .find(|(at, _)| rest[at + 1..].is_empty() || rest[at + 1..].starts_with([' ', '\t']))?;
    Some(from + colon.0)
}

fn unquoted(name: &str) -> String {
    let quoted = name.len() >= 2
        && ((name.starts_with('"') && name.ends_with('"'))
            || (name.starts_with('\'') && name.ends_with('\'')));
    match quoted {
        true => name[1..name.len() - 1].to_owned(),
        false => name.to_owned(),
    }
}

/// The node the fields `path` names lead to from `root`.
fn at_path<'a>(root: &'a Node, path: &[String]) -> Option<&'a Node> {
    path.iter().try_fold(root, |node, name| match &node.value {
        Value::Mapping(entries) => (entries.iter())
            .find(|entry| key_name(&entry.key) == *name)
            .map(|entry| &entry.value),
        _ => None,
    })
}

/// The texts of `items`, but for one holding `offset`.
fn scalars(items: &[Node], offset: usize) -> Vec<String> {
    (items.iter())
        .filter(|item| !holds(&item.place.span.range(), offset))
        .filter_map(|item| match &item.value {
            Value::Scalar(text) if !text.is_empty() => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// Whether a comment has begun on a line before the cursor.
fn in_comment(before: &str) -> bool {
    before.trim_start().starts_with('#') || before.contains(" #")
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..offset].rfind('\n').map_or(0, |at| at + 1)
}

/// Whether the cursor at `offset` stands in `range`, at either end
/// included.
fn holds(range: &Range<usize>, offset: usize) -> bool {
    range.start <= offset && offset <= range.end
}
