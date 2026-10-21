//! Why Sass stopped, read from grass's words and the text it points at.
//! grass's version is pinned, so its words are matched as it writes them.

use std::fs;
use std::ops::Range;
use std::path::Path;

use crate::problem::{SassFault, SassType};
use crate::suggest;

const BUILT_IN: [&str; 7] = ["math", "color", "list", "map", "meta", "selector", "string"];

/// The extensions of the files Sass loads.
const STYLESHEETS: [&str; 3] = ["scss", "sass", "css"];

/// Why Sass stopped, from grass's `message` about `span` of `text`, and
/// the span to place it at. `file` is the stylesheet `text` is, which
/// grass reads only within `assets`.
pub(super) fn read(
    message: &str,
    text: &str,
    mut span: Range<usize>,
    file: &Path,
    assets: &Path,
) -> (SassFault, Range<usize>) {
    let first = message.lines().next().unwrap_or_default();
    let fault = match recognised(message, first, text, &mut span, file, assets) {
        Some(fault) => fault,
        None => match raised_by(text, &span) {
            Some(rule) => {
                span.start = rule;
                SassFault::Raised(super::unquoted(message).to_owned())
            }
            None => SassFault::Other(first.to_owned()),
        },
    };
    if !matches!(
        fault,
        SassFault::MixedIndent { .. } | SassFault::FirstLineIndented
    ) {
        let placed = &text[span.clone()];
        span.start += placed.len() - placed.trim_start().len();
        span.end = span.start + placed.trim().len();
    }
    (fault, span)
}

/// The fault grass's words name, with `span` moved to where it is.
fn recognised(
    message: &str,
    first: &str,
    text: &str,
    span: &mut Range<usize>,
    file: &Path,
    assets: &Path,
) -> Option<SassFault> {
    let written = text[span.clone()].trim();
    let at_rule = || {
        let rule = &text[span.start..];
        rule[..ident_end(rule, usize::from(rule.starts_with('@')))].to_owned()
    };
    Some(match first {
        "Undefined variable." => {
            span.start = namespace_start(text, span.start);
            SassFault::NoVariable(text[span.clone()].to_owned())
        }
        "Undefined mixin." => SassFault::NoMixin(written.to_owned()),
        "Undefined function." => {
            *span = callee(text, span.clone());
            SassFault::NoFunction(text[span.clone()].to_owned())
        }
        "unmatched \"}\"." => SassFault::ClosesNothing,
        "Expected \"important\"." => return unknown_flag(text, span),
        "expected more input." => match open_at(text, text.len()).within {
            Some((at, "/*")) => {
                *span = at..at + 2;
                SassFault::Unclosed("/*")
            }
            _ => SassFault::EndsEarly,
        },
        "\"+\" and \"-\" must be surrounded by whitespace in calculations." => {
            SassFault::Unspaced(written.to_owned())
        }
        "Top-level selectors may not contain the parent selector \"&\"." => {
            SassFault::TopLevelParent
        }
        "This at-rule is not allowed here." => match at_rule() {
            name if name == "@return" => SassFault::ReturnOutsideFunction,
            name => SassFault::NotHere(name),
        },
        "@use rules must be written before any other rules."
        | "@forward rules must be written before any other rules." => {
            SassFault::LoadAfterRules(at_rule())
        }
        "@content is only allowed within mixin declarations." => SassFault::ContentOutsideMixin,
        "@extend may only be used within style rules." => SassFault::ExtendOutsideRule,
        "Mixin doesn't accept a content block." => SassFault::NoContentBlock(written.to_owned()),
        "Function finished without @return." => {
            SassFault::NoReturn(text[callee(text, span.clone())].to_owned())
        }
        "Can't find stylesheet to import." => {
            let inside = super::quoted(written);
            let name = &written[inside];
            match name.strip_prefix("sass:") {
                Some(module) if !BUILT_IN.contains(&module) => {
                    SassFault::NoBuiltInModule(name.to_owned())
                }
                _ => SassFault::NoStylesheet {
                    written: name.to_owned(),
                    nearest: nearest_stylesheet(name, file, assets),
                },
            }
        }
        "Private members can't be accessed from outside their modules." => {
            SassFault::Private(written.rsplit('.').next().unwrap_or(written).to_owned())
        }
        "This file is already being loaded."
        | "Module loop: this module is already being loaded." => SassFault::LoadsItself,
        "Duplicate key." => SassFault::RepeatedKey(written.to_owned()),
        "to and from values have incompatible units" => {
            let (from, to) = written
                .split_once(" through ")
                .or_else(|| written.split_once(" to "))?;
            SassFault::ForUnits {
                from: from.trim().to_owned(),
                to: to.trim().to_owned(),
            }
        }
        "semicolons aren't allowed in the indented syntax." => SassFault::IndentedSemicolon,
        "Expected spaces, was tabs." | "Expected tabs, was spaces." => {
            let line = text[..span.end].rfind('\n').map_or(0, |at| at + 1);
            let rest = &text[line..];
            *span = line..line + rest.len() - rest.trim_start_matches([' ', '\t']).len();
            SassFault::MixedIndent {
                tab: first == "Expected spaces, was tabs.",
            }
        }
        "Indenting at the beginning of the document is illegal." => {
            *span = 0..text.len() - text.trim_start_matches([' ', '\t']).len();
            SassFault::FirstLineIndented
        }
        _ if first.starts_with("compound selectors may no longer be extended.") => {
            let consider = message.lines().nth(1).unwrap_or_default();
            SassFault::CompoundExtend(consider.split('`').nth(1).map(str::to_owned))
        }
        _ => return worded(first, text, span),
    })
}

/// The faults grass words around a name or a value.
fn worded(first: &str, text: &str, span: &mut Range<usize>) -> Option<SassFault> {
    let written = text[span.clone()].trim();
    let callable = || text[callee(text, span.clone())].to_owned();
    if let Some(rest) = first
        .strip_prefix("Expected ")
        .or_else(|| first.strip_prefix("expected "))
    {
        return expected(rest.strip_suffix('.')?, text, span);
    }
    if let Some(argument) = between(first, "Missing argument ", ".") {
        return Some(SassFault::MissingArgument {
            callable: callable(),
            argument: argument.to_owned(),
        });
    }
    if let Some(argument) = between(first, "No argument named ", ".") {
        return Some(SassFault::NoArgument {
            callable: callable(),
            argument: argument.to_owned(),
        });
    }
    if let Some(rest) = first.strip_prefix("Only ") {
        let (takes, rest) = rest
            .split_once(" arguments allowed, but ")
            .or_else(|| rest.split_once(" argument allowed, but "))?;
        let given = rest
            .strip_suffix(" were passed.")
            .or_else(|| rest.strip_suffix(" was passed."))?;
        return Some(SassFault::TooManyArguments {
            callable: callable(),
            takes: takes.parse().ok()?,
            given: given.parse().ok()?,
        });
    }
    if let Some(name) = first.strip_prefix("Function not found: ") {
        let argument = written
            .split_once('(')
            .and_then(|(_, inside)| inside.rsplit_once(')'))
            .and_then(|(inside, _)| inside.split(',').next())
            .map(|first| super::unquoted(first.trim()))
            .filter(|first| !first.is_empty() && !first.starts_with('$') && !first.contains('#'));
        return Some(SassFault::NoFunction(argument.unwrap_or(name).to_owned()));
    }
    if let Some(name) = between(first, "There is no module with the namespace \"", "\".") {
        let end = ident_end(text, span.start);
        let name = match &text[span.start..end] {
            "" => name,
            written => {
                span.end = end;
                written
            }
        };
        return Some(SassFault::NoNamespace {
            name: name.to_owned(),
            built_in: BUILT_IN.contains(&name),
        });
    }
    if let Some(name) = between(first, "There's already a module with namespace \"", "\".") {
        return Some(SassFault::TwoModules(
            namespace(written).unwrap_or_else(|| name.to_owned()),
        ));
    }
    if first == "This variable was not declared with !default in the @used module."
        || (first.starts_with('$')
            && first.ends_with(" was not declared with !default in the @used module."))
    {
        let name = match written.starts_with('$') {
            true => &text[span.start..ident_end(text, span.start + 1)],
            false => first.split(' ').next().unwrap_or_default(),
        };
        return Some(SassFault::NotDefault(name.to_owned()));
    }
    if let Some((one, other)) =
        between(first, "Incompatible units ", ".").and_then(|units| units.split_once(" and "))
    {
        let units = match (unit_in(written, one), unit_in(written, other)) {
            (Some(a), Some(b)) if b < a => (other, one),
            _ => (one, other),
        };
        return Some(SassFault::Units(units.0.to_owned(), units.1.to_owned()));
    }
    if let Some(operation) = between(first, "Undefined operation \"", "\".") {
        let operation = match written.contains('\n') || written.is_empty() {
            true => operation,
            false => written,
        };
        return Some(SassFault::Operation(operation.to_owned()));
    }
    if let Some(what) = between(first, "@function rules may not contain ", ".") {
        return Some(SassFault::InFunction(what.to_owned()));
    }
    if let Some(value) = first.strip_suffix(" isn't a valid CSS value.") {
        return Some(SassFault::NotCss {
            value: value.to_owned(),
            placed: span.start < span.end,
        });
    }
    argument_fault(first)
}

/// The faults grass words as an argument's: `$map: 1 is not a map.`
fn argument_fault(first: &str) -> Option<SassFault> {
    let (argument, rest) = first.strip_prefix('$')?.split_once(": ")?;
    let argument = format!("${argument}");
    if let Some(value) = between(rest, "Expected ", " to have no units.") {
        return Some(SassFault::HasUnits {
            argument,
            value: value.to_owned(),
        });
    }
    if let Some(rest) = between(rest, "Invalid index ", " elements.") {
        let (index, items) = rest.split_once(" for a list with ")?;
        return Some(SassFault::OutOfRange {
            argument,
            index: index.to_owned(),
            items: items.parse().ok()?,
        });
    }
    let rest = rest.strip_suffix('.')?;
    let (value, kind) = rest
        .rsplit_once(" is not an ")
        .or_else(|| rest.rsplit_once(" is not a "))?;
    let wanted = match kind {
        "number" => SassType::Number,
        "color" => SassType::Colour,
        "map" => SassType::Map,
        "string" => SassType::String,
        "function reference" => SassType::Function,
        "argument list" => SassType::Arguments,
        "calculation" => SassType::Calculation,
        "int" => SassType::WholeNumber,
        _ => return None,
    };
    Some(SassFault::WrongType {
        argument,
        wanted,
        value: value.to_owned(),
    })
}

/// The faults grass words as what it expected, `rest` being what follows
/// "expected", less its full stop.
fn expected(rest: &str, text: &str, span: &mut Range<usize>) -> Option<SassFault> {
    match rest {
        "\"" | "'" => {
            if let Some((at, quote)) = open_at(text, span.start).within
                && quote == rest
            {
                *span = at..at + 1;
            }
            return Some(SassFault::Unclosed(if rest == "'" { "'" } else { "\"" }));
        }
        "expression" => return Some(SassFault::NoValue),
        "identifier" => return Some(SassFault::NoName),
        "string" => return Some(SassFault::NoQuotedText),
        _ => {}
    }
    let mut chars = rest.strip_prefix('"')?.strip_suffix('"')?.chars();
    let (Some(wanted), None) = (chars.next(), chars.next()) else {
        return None;
    };
    let open = open_at(text, span.start);
    let innermost = open.brackets.last().copied();
    Some(match (wanted, innermost) {
        (';', _) => {
            let at = semicolon_place(text, span.start);
            *span = at..at;
            SassFault::Missing(';')
        }
        ('}', Some((at, "#{"))) => {
            *span = at..at + 2;
            SassFault::Unclosed("#{")
        }
        ('}', Some((at, "{"))) if text[span.start..].trim().is_empty() => {
            *span = at..at + 1;
            SassFault::Unclosed("{")
        }
        (')', Some((at, "("))) => {
            *span = at..at + 1;
            SassFault::Unclosed("(")
        }
        (']', Some((at, "["))) => {
            *span = at..at + 1;
            SassFault::Unclosed("[")
        }
        ('{', Some((_, "{"))) => match colonless(text, span.start) {
            Some(declaration) => SassFault::ReadsAsDeclaration(declaration),
            None => SassFault::Missing('{'),
        },
        (wanted, _) => SassFault::Missing(wanted),
    })
}

/// Where the `;` belongs that grass found missing at `next`, the text
/// after it: at the end of the line before `next`'s where the statement
/// began on an earlier line, as a declaration's next line reads on as
/// more of its value.
fn semicolon_place(text: &str, next: usize) -> usize {
    let before = &text[..next];
    let statement = before.rfind(['{', '}', ';']).map_or(0, |at| at + 1);
    let mut at = match before.rfind('\n') {
        Some(line) if line > statement && !text[statement..line].trim().is_empty() => {
            text[..line].trim_end().len()
        }
        _ => before.trim_end().len(),
    };
    if let Some((comment, "//")) = open_at(text, at).within {
        at = text[..comment].trim_end().len();
    }
    at
}

/// `!importnt` as written, with `span` moved to it from the word after
/// the `!`, where grass places it.
fn unknown_flag(text: &str, span: &mut Range<usize>) -> Option<SassFault> {
    let bang = text[..span.start].trim_end().strip_suffix('!')?.len();
    *span = bang..ident_end(text, span.start);
    Some(SassFault::UnknownFlag(text[span.clone()].to_owned()))
}

/// The declaration the statement ending at `end` reads as, written with
/// its colon: `color red` as `color: red;`.
fn colonless(text: &str, end: usize) -> Option<String> {
    if !text[end..].starts_with([';', '}']) {
        return None;
    }
    let start = text[..end].rfind(['{', '}', ';']).map_or(0, |at| at + 1);
    let statement = text[start..end].trim();
    let (property, value) = statement.split_once([' ', '\t'])?;
    let reads = property.starts_with(|c: char| c.is_ascii_alphabetic() || c == '-')
        && property
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        && !value.contains(['\n', ':']);
    reads.then(|| format!("{property}: {};", value.trim()))
}

/// Where the `@error` rule starts whose whole value `span` is.
fn raised_by(text: &str, span: &Range<usize>) -> Option<usize> {
    let rule = text[..span.start].trim_end().strip_suffix("@error")?.len();
    let before = text[..rule].chars().next_back();
    let after = text[span.end..].trim_start_matches([' ', '\t']);
    let alone = before.is_none_or(|c| !is_ident(c))
        && (after.is_empty() || after.starts_with([';', '}', '\n', '\r']));
    alone.then_some(rule)
}

/// The start of `module.` before `start`, if a module names what starts
/// there: grass places an unknown `module.$x` at `$x`.
fn namespace_start(text: &str, start: usize) -> usize {
    let Some(dot) = text[..start].strip_suffix('.') else {
        return start;
    };
    let name = dot.len() - dot.trim_end_matches(is_ident).len();
    match name {
        0 => start,
        _ => dot.len() - name,
    }
}

/// The name of the function or mixin called at `span`, without its
/// arguments.
fn callee(text: &str, span: Range<usize>) -> Range<usize> {
    let call = &text[span.clone()];
    let name = call.split('(').next().unwrap_or(call).trim_end();
    span.start..span.start + name.len()
}

/// The namespace a `@use` rule gives.
fn namespace(rule: &str) -> Option<String> {
    if let Some((_, after)) = rule.split_once(" as ") {
        let after = after.trim_start();
        let name = &after[..ident_end(after, 0)];
        return (!name.is_empty()).then(|| name.to_owned());
    }
    let url = &rule[super::quoted(rule)];
    let name = loaded_as(url.rsplit(['/', ':']).next()?);
    (!name.is_empty()).then(|| name.to_owned())
}

/// A stylesheet's name as it is loaded, without a partial's `_` or its
/// extension.
fn loaded_as(name: &str) -> &str {
    let name = name.strip_prefix('_').unwrap_or(name);
    STYLESHEETS
        .into_iter()
        .find_map(|extension| name.strip_suffix(extension)?.strip_suffix('.'))
        .unwrap_or(name)
}

/// Where in `written` a number with `unit` first stands.
fn unit_in(written: &str, unit: &str) -> Option<usize> {
    written.match_indices(unit).map(|(at, _)| at).find(|&at| {
        let before = written[..at].chars().next_back();
        let after = written[at + unit.len()..].chars().next();
        before.is_some_and(|c| c.is_ascii_digit() || c == '.') && after.is_none_or(|c| !is_ident(c))
    })
}

/// The name of the stylesheet nearest `written` in the folder it would be
/// found in, written as it would be loaded.
fn nearest_stylesheet(written: &str, file: &Path, assets: &Path) -> Option<String> {
    let (folder, name) = match written.rsplit_once('/') {
        Some((folder, name)) => (Some(folder), name),
        None => (None, written),
    };
    let mut directory = file.parent()?.to_path_buf();
    if let Some(folder) = folder {
        directory.push(folder);
    }
    let real = fs::canonicalize(&directory).ok()?;
    if !real.starts_with(fs::canonicalize(assets).ok()?) {
        return None;
    }
    let own = fs::canonicalize(file).ok();
    let mut names: Vec<String> = fs::read_dir(&real)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file() && fs::canonicalize(entry.path()).ok() != own)
        .filter_map(|entry| {
            let entry = entry.file_name().into_string().ok()?;
            let (_, extension) = entry.rsplit_once('.')?;
            STYLESHEETS
                .contains(&extension)
                .then(|| loaded_as(&entry).to_owned())
        })
        .collect();
    names.sort();
    names.dedup();
    let nearest = suggest::nearest(loaded_as(name), names.iter().map(String::as_str))?;
    Some(match folder {
        Some(folder) => format!("{folder}/{nearest}"),
        None => nearest.to_owned(),
    })
}

/// The part of `text` between `before` and `after`, where it starts with
/// one and ends with the other.
fn between<'a>(text: &'a str, before: &str, after: &str) -> Option<&'a str> {
    text.strip_prefix(before)?.strip_suffix(after)
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_' || !c.is_ascii()
}

/// Where the name starting at `start` ends.
fn ident_end(text: &str, start: usize) -> usize {
    let rest = &text[start.min(text.len())..];
    text.len() - rest.trim_start_matches(is_ident).len()
}

/// What is open at `end` of a stylesheet: the brackets not yet closed,
/// innermost last, and the string or comment `end` lies in, each with
/// where it opens.
struct Open {
    brackets: Vec<(usize, &'static str)>,
    within: Option<(usize, &'static str)>,
}

fn open_at(text: &str, end: usize) -> Open {
    let bytes = &text.as_bytes()[..end];
    let mut brackets = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let rest = &bytes[i..];
        match bytes[i] {
            b'\\' => i += 1,
            quote @ (b'"' | b'\'') => {
                let start = i;
                i += 1;
                while i < bytes.len() && bytes[i] != quote && bytes[i] != b'\n' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
                if i >= bytes.len() {
                    let quote = if quote == b'"' { "\"" } else { "'" };
                    return Open {
                        brackets,
                        within: Some((start, quote)),
                    };
                }
            }
            b'/' if rest.starts_with(b"//") => match rest.iter().position(|&b| b == b'\n') {
                Some(line) => i += line,
                None => {
                    return Open {
                        brackets,
                        within: Some((i, "//")),
                    };
                }
            },
            b'/' if rest.starts_with(b"/*") => {
                match rest[2..].windows(2).position(|pair| pair == b"*/") {
                    Some(close) => i += close + 3,
                    None => {
                        return Open {
                            brackets,
                            within: Some((i, "/*")),
                        };
                    }
                }
            }
            b'#' if rest.starts_with(b"#{") => {
                brackets.push((i, "#{"));
                i += 1;
            }
            b'{' => brackets.push((i, "{")),
            b'[' => brackets.push((i, "[")),
            b'(' => {
                brackets.push((i, "("));
                if plain_url(text, i) {
                    match rest.iter().position(|&b| b == b')') {
                        Some(close) => {
                            brackets.pop();
                            i += close;
                        }
                        None => break,
                    }
                }
            }
            b'}' if matches!(brackets.last(), Some((_, "{" | "#{"))) => {
                brackets.pop();
            }
            b')' if matches!(brackets.last(), Some((_, "("))) => {
                brackets.pop();
            }
            b']' if matches!(brackets.last(), Some((_, "["))) => {
                brackets.pop();
            }
            _ => {}
        }
        i += 1;
    }
    Open {
        brackets,
        within: None,
    }
}

/// Whether the `(` at `at` opens a `url(` whose address isn't quoted,
/// inside which `//` is part of the address.
fn plain_url(text: &str, at: usize) -> bool {
    let before = &text[..at];
    let Some(name) = before.get(before.len().saturating_sub(3)..) else {
        return false;
    };
    let starts = before[..before.len() - name.len()]
        .chars()
        .next_back()
        .is_none_or(|c| !is_ident(c));
    name.eq_ignore_ascii_case("url")
        && starts
        && !text[at + 1..].trim_start().starts_with(['"', '\''])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_open_skips_strings_comments_and_plain_addresses() {
        let open = |text: &str| {
            let open = open_at(text, text.len());
            (open.brackets, open.within)
        };
        assert_eq!(open("a { b: (1"), (vec![(2, "{"), (7, "(")], None));
        assert_eq!(open("a { b: \"}\" "), (vec![(2, "{")], None));
        assert_eq!(open("a { b: #{1 "), (vec![(2, "{"), (7, "#{")], None));
        assert_eq!(open("a { b: url(//x/{) "), (vec![(2, "{")], None));
        assert_eq!(open("a { // }\n"), (vec![(2, "{")], None));
        assert_eq!(open("a { /* } */ }"), (vec![], None));
        assert_eq!(open("a { b: 'c"), (vec![(2, "{")], Some((7, "'"))));
        assert_eq!(open("a {}\n/* b"), (vec![], Some((5, "/*"))));
    }
}
