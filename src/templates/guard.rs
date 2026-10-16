//! Nothing missing reaches the page through a filter, a test, a function
//! or a printed list.
//!
//! Under semi-strict undefined the engine already refuses to print a
//! missing value, look inside one or loop over one. What it lets through
//! is a missing value handed to a callable, which the callable is free to
//! turn into `null`, an empty string or `False`, and a missing value inside
//! a list or map, which prints as `undefined`. Every filter, test and
//! global function is registered again here, under its own name, as a
//! wrapper that refuses a missing value anywhere in what it is handed, and
//! the formatter refuses one anywhere in what it prints. `default` alone
//! may be handed one, as its input or as what it gives instead; in the
//! second case what it gives is still missing, and is refused wherever it
//! goes next. An empty value, which the engine prints as `none`, is
//! refused wherever it would be turned into words.
//!
//! A filter over items meets a missing value as a field an item lacks.
//! Where the filter only tests it, as `selectattr` does, the item fails
//! the test, as it would in an `if`. Where the filter uses it to group,
//! tell apart or gather, where the item goes would be a guess, so it is
//! refused, naming the item and the field. `sort`, which puts such items
//! last and says so, is Statac's own.

use minijinja::value::{Rest, Value, ValueKind};
use minijinja::{Environment, Error, ErrorKind, State, UndefinedBehavior};

use super::frame;
use crate::problem::{Kind, Stopped, Wanted};

/// Every filter the engine ships, by the name templates use. A filter not
/// in this list is refused when templates are read, so one added by a
/// later release can never run unguarded.
pub(super) const FILTERS: &[&str] = &[
    "abs",
    "attr",
    "batch",
    "bool",
    "capitalize",
    "chain",
    "count",
    "d",
    "default",
    "dictsort",
    "e",
    "escape",
    "first",
    "float",
    "format",
    "groupby",
    "indent",
    "int",
    "items",
    "join",
    "last",
    "length",
    "lines",
    "list",
    "lower",
    "map",
    "max",
    "min",
    "pprint",
    "reject",
    "rejectattr",
    "replace",
    "reverse",
    "round",
    "safe",
    "select",
    "selectattr",
    "slice",
    "sort",
    "split",
    "string",
    "sum",
    "title",
    "trim",
    "unique",
    "upper",
    "zip",
];

/// Every test the engine ships.
pub(super) const TESTS: &[&str] = &[
    "!=",
    "<",
    "<=",
    "==",
    ">",
    ">=",
    "boolean",
    "defined",
    "divisibleby",
    "endingwith",
    "eq",
    "equalto",
    "escaped",
    "even",
    "false",
    "filter",
    "float",
    "ge",
    "greaterthan",
    "gt",
    "in",
    "int",
    "integer",
    "iterable",
    "le",
    "lessthan",
    "lower",
    "lt",
    "mapping",
    "ne",
    "none",
    "number",
    "odd",
    "safe",
    "sameas",
    "sequence",
    "startingwith",
    "string",
    "test",
    "true",
    "undefined",
    "upper",
];

/// Every global function the engine ships.
pub(super) const FUNCTIONS: &[&str] = &["debug", "dict", "namespace", "range"];

/// Where in what a callable was handed the missing value sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Step {
    Index(usize),
    Key(String),
}

/// Which value handed to a callable held the missing value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Role {
    /// What a filter or test is applied to, or what is printed.
    Input,
    /// A value in brackets, counted from 0 after the input.
    Arg(usize),
    Kwarg(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Callee {
    Filter,
    Test,
    Function,
    Snippet,
    Print,
}

/// What the last guard to refuse a value found, read once the error comes
/// back from the engine, to point at the value in the template.
#[derive(Debug, Clone)]
pub(super) struct Refusal {
    pub callee: String,
    pub kind: Callee,
    pub role: Role,
    pub path: Vec<Step>,
    /// Whether the value is there and empty, rather than missing.
    pub empty: bool,
    /// For a filter that groups or tells items apart by a field: the field
    /// an item lacks, and the item.
    pub by_field: Option<(String, Value)>,
}

/// What a missing value was found as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Found {
    pub path: Vec<Step>,
    pub empty: bool,
}

/// The first missing value inside `value`, an empty path for `value`
/// itself. Statac's own objects never hold one and may be large, so the
/// search doesn't walk into them. Empty values count as missing where
/// `empty` says.
pub(super) fn find_missing(value: &Value, empty: bool) -> Option<Found> {
    if value.is_undefined() {
        return Some(Found {
            path: Vec::new(),
            empty: false,
        });
    }
    if empty && value.is_none() {
        return Some(Found {
            path: Vec::new(),
            empty: true,
        });
    }
    if super::values::opaque(value) {
        return None;
    }
    let inside = |step: Step, item: &Value| {
        find_missing(item, empty).map(|mut found| {
            found.path.insert(0, step);
            found
        })
    };
    if let Some(own) = super::media::own_fields(value) {
        return own
            .iter()
            .find_map(|(name, item)| inside(Step::Key(name.clone()), item));
    }
    match value.kind() {
        ValueKind::Seq | ValueKind::Iterable => value
            .try_iter()
            .ok()?
            .enumerate()
            .find_map(|(i, item)| inside(Step::Index(i), &item)),
        ValueKind::Map => value.try_iter().ok()?.find_map(|key| {
            let item = value.get_item(&key).ok()?;
            let name = key.as_str().map_or_else(|| key.to_string(), str::to_owned);
            inside(Step::Key(name), &item)
        }),
        _ => None,
    }
}

pub(super) fn refuse(callee: &str, kind: Callee, role: Role, found: Found) -> Error {
    frame::with(|frame| {
        frame.refusal = Some(Refusal {
            callee: callee.to_owned(),
            kind,
            role,
            path: found.path,
            empty: found.empty,
            by_field: None,
        })
    });
    Error::new(ErrorKind::UndefinedError, REFUSED)
}

/// The words of an error a guard raised.
pub(super) const REFUSED: &str = "refused a missing value";

/// One of the engine's own filters, tests or functions that stopped, and
/// what it was handed, the input first.
#[derive(Debug, Clone)]
pub(super) struct Failed {
    pub callee: &'static str,
    pub args: Vec<Value>,
}

/// Notes what `callee` was handed where it stopped, unless a callable it
/// called stopped first.
fn failed(callee: &'static str, args: &[Value], error: Error) -> Error {
    frame::with(|frame| {
        if frame.failed.is_none() {
            frame.failed = Some(Failed {
                callee,
                args: args.to_vec(),
            });
        }
    });
    error
}

/// How much of a filter's or test's input is checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Input {
    /// Neither it nor anything inside it may be missing.
    Whole,
    /// It may not be missing, but the items it holds may: the filter tests
    /// each item, through a guarded test or for truth.
    Itself,
    /// It may be missing; if present, nothing inside it may be.
    MayBeMissing,
}

impl Input {
    pub fn of_filter(name: &str) -> Input {
        match name {
            "default" | "d" => Input::MayBeMissing,
            "select" | "reject" | "selectattr" | "rejectattr" => Input::Itself,
            _ => Input::Whole,
        }
    }

    pub fn of_test(name: &str) -> Input {
        match name {
            "defined" | "undefined" => Input::MayBeMissing,
            _ => Input::Whole,
        }
    }
}

/// Whether a filter turns what it is handed into words, so an empty value
/// handed to it would publish as `none`.
fn writes_words(filter: &str) -> bool {
    !matches!(
        filter,
        "default"
            | "d"
            | "bool"
            | "select"
            | "reject"
            | "selectattr"
            | "rejectattr"
            | "length"
            | "count"
            | "list"
            | "first"
            | "last"
            | "reverse"
            | "sort"
            | "unique"
            | "map"
            | "batch"
            | "slice"
            | "json"
    )
}

/// Refuses any missing value in `args`. The first value is the input of a
/// filter or test, checked as `input` says.
pub(super) fn check_args(
    callee: &str,
    kind: Callee,
    args: &[Value],
    input: Input,
) -> Result<(), Error> {
    let first_is_input = matches!(kind, Callee::Filter | Callee::Test);
    let empty = kind == Callee::Filter && writes_words(callee);
    if (empty || callee == "json") && args.iter().any(super::values::holds_bytes) {
        return Err(super::values::not_text());
    }
    for (i, arg) in args.iter().enumerate() {
        if i == 1 && first_is_input && matches!(callee, "default" | "d") && arg.is_undefined() {
            continue;
        }
        if i == 0 && first_is_input {
            match input {
                Input::MayBeMissing if arg.is_undefined() => continue,
                Input::Itself if arg.is_undefined() => {
                    return Err(refuse(
                        callee,
                        kind,
                        Role::Input,
                        Found {
                            path: Vec::new(),
                            empty: false,
                        },
                    ));
                }
                Input::Itself => continue,
                _ => {}
            }
        }
        if arg.is_kwargs() {
            for key in arg.try_iter()? {
                let value = arg.get_item(&key)?;
                if let Some(found) = find_missing(&value, false) {
                    let name = key.as_str().unwrap_or_default().to_owned();
                    return Err(refuse(callee, kind, Role::Kwarg(name), found));
                }
            }
            continue;
        }
        let words = empty && i == 0;
        if let Some(found) = find_missing(arg, words) {
            let role = match (first_is_input, i) {
                (true, 0) => Role::Input,
                (true, n) => Role::Arg(n - 1),
                (false, n) => Role::Arg(n),
            };
            return Err(refuse(callee, kind, role, found));
        }
    }
    Ok(())
}

/// The field a filter reads from each item and uses as it is, as named.
/// None where the filter is given a `default` for an item without.
fn field_used(callee: &str, args: &[Value]) -> Option<String> {
    let kwargs = args.iter().find(|a| a.is_kwargs());
    let kwarg = |name: &str| {
        kwargs
            .and_then(|k| k.get_item(&Value::from(name)).ok())
            .filter(|v| !v.is_undefined())
    };
    if kwarg("default").is_some() {
        return None;
    }
    let named = match callee {
        "groupby" => kwarg("attribute").or_else(|| args.get(1).filter(|a| !a.is_kwargs()).cloned()),
        "map" | "unique" => kwarg("attribute"),
        _ => None,
    };
    named?.as_str().map(str::to_owned)
}

/// Looks up a dotted field path, as the engine's filters do: a piece that
/// reads as a number is an index.
pub(super) fn field_of(item: &Value, path: &str) -> Value {
    let mut value = item.clone();
    for piece in path.split('.') {
        value = match piece.parse::<i64>() {
            Ok(n) => value.get_item(&Value::from(n)),
            Err(_) => value.get_attr(piece),
        }
        .unwrap_or(Value::UNDEFINED);
        if value.is_undefined() {
            break;
        }
    }
    value
}

/// Refuses to group, tell apart or gather items by a field some lack.
fn check_by_field(callee: &str, args: &[Value]) -> Result<(), Error> {
    let Some(field) = field_used(callee, args) else {
        return Ok(());
    };
    let Some(Ok(items)) = args.first().map(Value::try_iter) else {
        return Ok(());
    };
    for (i, item) in items.enumerate() {
        if !field_of(&item, &field).is_undefined() {
            continue;
        }
        let error = refuse(
            callee,
            Callee::Filter,
            Role::Input,
            Found {
                path: vec![Step::Index(i)],
                empty: false,
            },
        );
        frame::with(|frame| {
            if let Some(refusal) = &mut frame.refusal {
                refusal.by_field = Some((field.clone(), item.clone()));
            }
        });
        return Err(error);
    }
    Ok(())
}

/// `select`, `reject`, `selectattr` and `rejectattr`, as the engine runs
/// them, except that an item whose value is missing fails any test that
/// would refuse it. The value is only tested, never shown.
fn select_or_reject(state: &State, callee: &str, args: &[Value]) -> Result<Value, Error> {
    let name_in = |value: &Value, wanted| {
        value.as_str().map(str::to_owned).ok_or_else(|| {
            frame::raise(Kind::TemplateStopped {
                reason: Stopped::Needs {
                    callee: callee.to_owned(),
                    wanted,
                    given: Some(super::engine::shape_of_value(value)),
                },
            })
        })
    };
    let Some((input, mut rest)) = args.split_first() else {
        return Err(Error::from(ErrorKind::MissingArgument));
    };
    let attribute = match callee.ends_with("attr") {
        true => {
            let (named, after) = rest
                .split_first()
                .ok_or_else(|| Error::from(ErrorKind::MissingArgument))?;
            rest = after;
            Some(name_in(named, Wanted::FieldName)?)
        }
        false => None,
    };
    let test = match rest.split_first() {
        Some((named, after)) => {
            rest = after;
            let named = name_in(named, Wanted::TestName)?;
            let &name = TESTS
                .iter()
                .find(|&&t| t == named)
                .ok_or_else(|| Error::from(ErrorKind::UnknownTest))?;
            Some((name, builtin_test(name)))
        }
        None => None,
    };
    let invert = callee.starts_with("reject");
    let choose = || {
        let mut kept = Vec::new();
        for item in input.try_iter()? {
            let value = match &attribute {
                Some(path) => field_of(&item, path),
                None => item.clone(),
            };
            let passed = match &test {
                None => value.is_true(),
                Some((name, _)) if value.is_undefined() && Input::of_test(name) == Input::Whole => {
                    false
                }
                Some((name, test)) => {
                    let call: Vec<Value> =
                        std::iter::once(value).chain(rest.iter().cloned()).collect();
                    check_args(name, Callee::Test, &call, Input::of_test(name))?;
                    test.call(state, &call)?.is_true()
                }
            };
            if passed != invert {
                kept.push(item);
            }
        }
        Ok::<_, Error>(kept)
    };
    // Pages chosen from a list that is the same whichever page asks, by
    // what each says of itself, which a layout asks on every page it
    // builds: worked out once.
    let plain = |value: &Value| {
        matches!(
            value.kind(),
            ValueKind::None | ValueKind::Bool | ValueKind::Number | ValueKind::String
        )
    };
    if let Some(pages) = input.downcast_object_ref::<super::values::Pages>()
        && let Some((mut asked, mut handed)) = pages.known_by()
        && let Some(path) = &attribute
        && let Some(field) = path.split('.').next()
        && super::values::of_the_front(field)
        && rest.iter().all(plain)
    {
        let shared = &pages.shared;
        frame::read(crate::reads::Key::FieldOfEvery(shared.name_id(field)));
        let tested = test.as_ref().map_or("", |(name, _)| name);
        asked.push_str(&format!(" | {callee} {path} {tested} {}", rest.len()));
        handed.extend(rest.iter().cloned());
        return pages.once((asked, handed), || {
            Ok(choose()?
                .iter()
                .filter_map(|page| page.downcast_object_ref::<super::values::PageValue>())
                .map(|page| (page.id, page.number))
                .collect())
        });
    }
    Ok(Value::from(choose()?))
}

fn builtin_filter(name: &str) -> Value {
    use minijinja::filters::*;
    match name {
        "abs" => Value::from_function(abs),
        "attr" => Value::from_function(attr),
        "batch" => Value::from_function(batch),
        "bool" => Value::from_function(bool),
        "capitalize" => Value::from_function(capitalize),
        "chain" => Value::from_function(chain),
        "count" | "length" => Value::from_function(length),
        "d" | "default" => Value::from_function(default),
        "dictsort" => Value::from_function(dictsort),
        "e" | "escape" => Value::from_function(escape),
        "first" => Value::from_function(first),
        "float" => Value::from_function(float),
        "format" => Value::from_function(format),
        "groupby" => Value::from_function(groupby),
        "indent" => Value::from_function(indent),
        "int" => Value::from_function(int),
        "items" => Value::from_function(items),
        "join" => Value::from_function(join),
        "last" => Value::from_function(last),
        "lines" => Value::from_function(lines),
        "list" => Value::from_function(list),
        "lower" => Value::from_function(lower),
        "map" => Value::from_function(map),
        "max" => Value::from_function(max),
        "min" => Value::from_function(min),
        "pprint" => Value::from_function(pprint),
        "reject" => Value::from_function(reject),
        "rejectattr" => Value::from_function(rejectattr),
        "replace" => Value::from_function(replace),
        "reverse" => Value::from_function(reverse),
        "round" => Value::from_function(round),
        "safe" => Value::from_function(safe),
        "select" => Value::from_function(select),
        "selectattr" => Value::from_function(selectattr),
        "slice" => Value::from_function(slice),
        "sort" => Value::from_function(sort),
        "split" => Value::from_function(split),
        "string" => Value::from_function(string),
        "sum" => Value::from_function(sum),
        "title" => Value::from_function(title),
        "trim" => Value::from_function(trim),
        "unique" => Value::from_function(unique),
        "upper" => Value::from_function(upper),
        "zip" => Value::from_function(zip),
        other => unreachable!("no built-in filter {other}"),
    }
}

fn builtin_test(name: &str) -> Value {
    use minijinja::tests::*;
    match name {
        "boolean" => Value::from_function(is_boolean),
        "defined" => Value::from_function(is_defined),
        "divisibleby" => Value::from_function(is_divisibleby),
        "endingwith" => Value::from_function(is_endingwith),
        "eq" | "equalto" | "==" => Value::from_function(is_eq),
        "escaped" | "safe" => Value::from_function(is_safe),
        "even" => Value::from_function(is_even),
        "false" => Value::from_function(is_false),
        "filter" => Value::from_function(is_filter),
        "float" => Value::from_function(is_float),
        "ge" | ">=" => Value::from_function(is_ge),
        "gt" | "greaterthan" | ">" => Value::from_function(is_gt),
        "in" => Value::from_function(is_in),
        "int" | "integer" => Value::from_function(is_integer),
        "iterable" => Value::from_function(is_iterable),
        "le" | "<=" => Value::from_function(is_le),
        "lt" | "lessthan" | "<" => Value::from_function(is_lt),
        "lower" => Value::from_function(is_lower),
        "mapping" => Value::from_function(is_mapping),
        "ne" | "!=" => Value::from_function(is_ne),
        "none" => Value::from_function(is_none),
        "number" => Value::from_function(is_number),
        "odd" => Value::from_function(is_odd),
        "sameas" => Value::from_function(is_sameas),
        "sequence" => Value::from_function(is_sequence),
        "startingwith" => Value::from_function(is_startingwith),
        "string" => Value::from_function(is_string),
        "test" => Value::from_function(is_test),
        "true" => Value::from_function(is_true),
        "undefined" => Value::from_function(is_undefined),
        "upper" => Value::from_function(is_upper),
        other => unreachable!("no built-in test {other}"),
    }
}

fn builtin_function(name: &str) -> Value {
    use minijinja::functions::*;
    match name {
        "debug" => Value::from_function(debug),
        "dict" => Value::from_function(dict),
        "namespace" => Value::from_function(namespace),
        "range" => Value::from_function(range),
        other => unreachable!("no built-in function {other}"),
    }
}

/// Writes text into HTML with the five characters that would be read as
/// markup escaped, and nothing else: an address keeps its slashes.
fn write_html(out: &mut minijinja::Output, text: &str) -> Result<(), Error> {
    let mut rest = text;
    while let Some(at) = rest.find(['&', '<', '>', '"', '\'']) {
        out.write_str(&rest[..at])
            .and_then(|()| {
                out.write_str(match rest.as_bytes()[at] {
                    b'&' => "&amp;",
                    b'<' => "&lt;",
                    b'>' => "&gt;",
                    b'"' => "&quot;",
                    _ => "&#x27;",
                })
            })
            .map_err(|_| Error::from(ErrorKind::WriteFailure))?;
        rest = &rest[at + 1..];
    }
    out.write_str(rest)
        .map_err(|_| Error::from(ErrorKind::WriteFailure))
}

/// The escaping a template writing JSON prints with: each value written as
/// a JSON value, markup and all.
pub(super) const JSON: &str = "json";

/// Sets semi-strict undefined, and replaces the formatter and every
/// built-in filter, test and function with its guarded form.
pub(super) fn install(env: &mut Environment<'_>) {
    env.set_undefined_behavior(UndefinedBehavior::SemiStrict);
    env.set_formatter(|out, state, value| {
        super::indent::value(|| {
            if let Some(found) = find_missing(value, true) {
                return Err(refuse("", Callee::Print, Role::Input, found));
            }
            if super::values::holds_bytes(value) {
                return Err(super::values::not_text());
            }
            let json = matches!(state.auto_escape(), minijinja::AutoEscape::Custom(JSON));
            if !json && let Some(text) = value.as_str() {
                if text.is_empty() {
                    super::indent::nothing();
                    return Ok(());
                }
                if value.is_safe() {
                    let written = match super::indent::write(out, text) {
                        Some(written) => written,
                        None => out
                            .write_str(text)
                            .map_err(|_| Error::from(ErrorKind::WriteFailure)),
                    };
                    if text.ends_with('\n') {
                        super::indent::ended();
                    }
                    return written;
                }
            }
            match state.auto_escape() {
                minijinja::AutoEscape::Html if !value.is_safe() => {
                    let text = value
                        .as_str()
                        .map_or_else(|| value.to_string(), str::to_owned);
                    write_html(out, &text)
                }
                minijinja::AutoEscape::Custom(JSON) => {
                    let mut json = String::new();
                    super::own::json(value, &mut json);
                    out.write_str(&json)
                        .map_err(|_| Error::from(ErrorKind::WriteFailure))
                }
                _ => minijinja::escape_formatter(out, state, value),
            }
        })
    });
    for &name in FILTERS {
        let inner = builtin_filter(name);
        let input = Input::of_filter(name);
        let selects = matches!(name, "select" | "reject" | "selectattr" | "rejectattr");
        env.add_filter(name, move |state: &State, args: Rest<Value>| {
            check_args(name, Callee::Filter, &args, input)?;
            check_by_field(name, &args)?;
            match selects {
                true => select_or_reject(state, name, &args),
                false => inner.call(state, &args),
            }
            .map_err(|error| failed(name, &args, error))
        });
    }
    for &name in TESTS {
        let inner = builtin_test(name);
        let input = Input::of_test(name);
        env.add_test(name, move |state: &State, args: Rest<Value>| {
            check_args(name, Callee::Test, &args, input)?;
            let passed = inner
                .call(state, &args)
                .map_err(|error| failed(name, &args, error))?;
            Ok::<bool, Error>(passed.is_true())
        });
    }
    for &name in FUNCTIONS {
        let inner = builtin_function(name);
        env.add_function(name, move |state: &State, args: Rest<Value>| {
            check_args(name, Callee::Function, &args, Input::Whole)?;
            inner
                .call(state, &args)
                .map_err(|error| failed(name, &args, error))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guarded() -> Environment<'static> {
        let mut env = Environment::new();
        install(&mut env);
        env
    }

    fn rendered(source: &str) -> Result<String, Error> {
        let env = guarded();
        let context = minijinja::context! {
            pages => vec![
                minijinja::context! { title => "Post 1", date => "2026-10-04" },
                minijinja::context! { title => "Post 2" },
            ],
            blank => (),
        };
        env.render_str(source, context)
    }

    #[test]
    fn a_missing_value_reaches_no_page_through_any_callable() {
        for source in [
            "{{ [x] }}",
            "{{ {'a': x} }}",
            "{{ ['a', x] | join(', ') }}",
            "{{ x | join(', ') }}",
            "{{ x | escape }}",
            "{{ x | upper }}",
            "{{ x is odd }}",
            "{{ x is string }}",
            "{{ 1 is eq(x) }}",
            "{{ range(x) }}",
            "{{ dict(a=x) }}",
            "{{ pages | map(attribute='titel') | join(', ') }}",
            "{{ pages | groupby('date') | length }}",
            "{{ pages | unique(attribute='date') | length }}",
            "{{ x | default(y) }}",
        ] {
            let refused = rendered(source).expect_err(source);
            assert_eq!(refused.kind(), ErrorKind::UndefinedError, "{source}");
        }
    }

    #[test]
    fn an_empty_value_never_publishes_as_none() {
        for source in ["{{ blank }}", "{{ [blank] }}", "{{ blank | upper }}"] {
            assert!(rendered(source).is_err(), "{source}");
        }
        assert_eq!(
            rendered("{{ blank | default('a') is none }}").unwrap(),
            "True"
        );
        assert_eq!(
            rendered("{% if blank %}a{% else %}b{% endif %}").unwrap(),
            "b"
        );
    }

    #[test]
    fn what_a_template_may_ask_of_a_missing_value_still_works() {
        for (source, expected) in [
            ("{% if x %}a{% else %}b{% endif %}", "b"),
            ("{{ x is defined }}", "False"),
            ("{{ x | default('a') }}", "a"),
            ("{{ x or 'a' }}", "a"),
            ("{{ pages | selectattr('date') | length }}", "1"),
            (
                "{{ pages | map(attribute='title') | join(', ') }}",
                "Post 1, Post 2",
            ),
        ] {
            assert_eq!(rendered(source).unwrap(), expected, "{source}");
        }
    }

    /// What each filter over items makes of a field some items lack, all
    /// lack, or all have: the text it gives, or None where it is refused.
    #[test]
    fn a_filter_over_items_tests_a_missing_field_and_refuses_to_use_one() {
        let env = guarded();
        let item = |name: &str, rank: Option<i64>| match rank {
            Some(rank) => minijinja::context! { name, rank },
            None => minijinja::context! { name },
        };
        let context = minijinja::context! {
            some => vec![item("Item 1", Some(2)), item("Item 2", None), item("Item 3", Some(1))],
            lacking => vec![item("Item 1", None), item("Item 2", None)],
            all => vec![item("Item 1", Some(2)), item("Item 2", Some(1))],
        };
        let names = "| map(attribute='name') | join(', ')";
        for (source, expected) in [
            // Tested: an item lacking the field fails the test.
            (
                format!("some | selectattr('rank', 'eq', 2) {names}"),
                Some("Item 1"),
            ),
            (
                format!("some | selectattr('rank', 'ne', 2) {names}"),
                Some("Item 3"),
            ),
            (
                format!("some | selectattr('rank') {names}"),
                Some("Item 1, Item 3"),
            ),
            (
                format!("some | selectattr('rank', 'defined') {names}"),
                Some("Item 1, Item 3"),
            ),
            (
                format!("some | selectattr('rank', 'undefined') {names}"),
                Some("Item 2"),
            ),
            (
                format!("lacking | selectattr('rank', 'eq', 2) {names}"),
                Some(""),
            ),
            (
                format!("all | selectattr('rank', 'eq', 2) {names}"),
                Some("Item 1"),
            ),
            (
                format!("some | rejectattr('rank', 'eq', 2) {names}"),
                Some("Item 2, Item 3"),
            ),
            (format!("some | rejectattr('rank') {names}"), Some("Item 2")),
            (
                format!("lacking | rejectattr('rank', 'eq', 2) {names}"),
                Some("Item 1, Item 2"),
            ),
            (
                format!("all | rejectattr('rank', 'eq', 2) {names}"),
                Some("Item 2"),
            ),
            (
                "[1, x, 3] | select('odd') | join(', ')".into(),
                Some("1, 3"),
            ),
            ("[1, x, 3] | select | join(', ')".into(), Some("1, 3")),
            // `reject` keeps a missing item, so `join` is handed it.
            ("[2, x] | reject('odd') | join(', ')".into(), None),
            ("some | selectattr('rank', 'eq', x) | length".into(), None),
            // Used: an item lacking the field stops the filter. `sort` is
            // Statac's own, tested with the site's filters.
            ("some | groupby('rank') | length".into(), None),
            ("some | groupby(attribute='rank') | length".into(), None),
            ("lacking | groupby('rank') | length".into(), None),
            ("all | groupby('rank') | length".into(), Some("2")),
            (
                "some | groupby('rank', default=0) | length".into(),
                Some("3"),
            ),
            ("some | unique(attribute='rank') | length".into(), None),
            ("lacking | unique(attribute='rank') | length".into(), None),
            ("all | unique(attribute='rank') | length".into(), Some("2")),
            ("some | map(attribute='rank') | length".into(), None),
            ("lacking | map(attribute='rank') | length".into(), None),
            (
                "all | map(attribute='rank') | join(', ')".into(),
                Some("2, 1"),
            ),
            (
                "some | map(attribute='rank', default=0) | join(', ')".into(),
                Some("2, 0, 1"),
            ),
            ("all | map(attribute='rank') | sum".into(), Some("3")),
            ("some | map(attribute='rank') | sum".into(), None),
        ] {
            let source = format!("{{{{ {source} }}}}");
            let result = env.render_str(&source, &context);
            match expected {
                Some(expected) => assert_eq!(result.unwrap(), expected, "{source}"),
                None => assert_eq!(
                    result.map_err(|e| e.kind()),
                    Err(ErrorKind::UndefinedError),
                    "{source}"
                ),
            }
        }
        // The engine's `sum` takes no field to add up.
        let summed = env.render_str("{{ all | sum(attribute='rank') }}", &context);
        assert_eq!(
            summed.map_err(|e| e.kind()),
            Err(ErrorKind::TooManyArguments)
        );
    }

    #[test]
    fn what_default_gives_in_place_of_a_missing_value_may_be_missing_too() {
        for (source, expected) in [
            ("{{ x | default(y) | default('a') }}", Some("a")),
            ("{{ x | d(y) | d('a') }}", Some("a")),
            ("{{ 'b' | default(y) }}", Some("b")),
            ("{% if x | default(y) %}a{% else %}b{% endif %}", Some("b")),
            ("{{ x | default(y) is defined }}", Some("False")),
            ("{{ x | default(y) }}", None),
            ("{{ x | default(y) | upper }}", None),
            ("{{ x | default(y) is odd }}", None),
            ("{{ x | default([y]) | default('a') }}", None),
        ] {
            let result = rendered(source);
            match expected {
                Some(expected) => assert_eq!(result.unwrap(), expected, "{source}"),
                None => assert_eq!(
                    result.map_err(|e| e.kind()),
                    Err(ErrorKind::UndefinedError),
                    "{source}"
                ),
            }
        }
    }

    #[test]
    fn every_listed_callable_is_one_the_engine_ships() {
        for &name in FILTERS {
            builtin_filter(name);
        }
        for &name in TESTS {
            builtin_test(name);
        }
        for &name in FUNCTIONS {
            builtin_function(name);
        }
    }
}
