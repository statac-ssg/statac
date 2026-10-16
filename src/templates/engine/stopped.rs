//! A template that stopped while a page was built, for a reason the engine
//! gives in its own words.

use std::ops::Range;

use minijinja::machinery::ast::{BinOpKind, CallArg, CallType, CompareOpKind, Expr, Stmt};
use minijinja::value::{Value, ValueKind};
use minijinja::{Error, ErrorKind};

use super::super::check::{full_span, parse_template, visit_exprs};
use super::super::guard::{self, Failed};
use super::super::own;
use super::super::values::DateValue;
use crate::problem::{Kind, Shape, Stopped, Wanted, Word};
use crate::suggest::nearest;

/// The most numbers the engine's `range` makes.
const RANGE_MOST: u64 = 100_000;

/// The named values each of the engine's filters that takes any takes.
const NAMED: &[(&str, &[&str])] = &[
    ("indent", &["width", "first", "blank"]),
    ("dictsort", &["by", "case_sensitive", "reverse"]),
    ("groupby", &["attribute", "default", "case_sensitive"]),
    ("unique", &["attribute", "case_sensitive"]),
    ("map", &["attribute", "default"]),
];

/// Where in which template the engine stopped: its name, its text, the
/// engine's range in it and that range widened to the whole expression.
pub(in crate::templates) struct Site<'a> {
    pub name: &'a str,
    pub text: &'a str,
    pub range: Range<usize>,
    pub at: Range<usize>,
}

/// The side of an arithmetic operator a missing value was handed to,
/// where that is what the engine stopped for: the span to find it in.
pub(in crate::templates) fn missing_operand(
    error: &Error,
    site: &Site<'_>,
) -> Option<Range<usize>> {
    let (_, left, right) = unsupported(error.detail()?)?;
    let operator = Reader::new(site).operator()?;
    match (left, right) {
        ("undefined", _) => Some(operator.left),
        (_, "undefined") => Some(operator.right),
        _ => None,
    }
}

/// The problem the engine's `error` comes to, and where it is told: at
/// `site` where the engine says, with `failed` the engine's own callable
/// that stopped and `file_of` naming a template by its file.
pub(in crate::templates) fn read_stopped(
    error: &Error,
    site: Option<&Site<'_>>,
    failed: Option<&Failed>,
    file_of: &dyn Fn(&str) -> Option<String>,
) -> (Kind, Option<Range<usize>>) {
    let reader = site.map(Reader::new);
    let failed =
        failed.filter(|f| site.is_some_and(|site| site.text[site.at.clone()].contains(f.callee)));
    let read = Read {
        error,
        reader: reader.as_ref(),
        failed,
        file_of,
    };
    let at = site.map(|site| site.at.clone());
    match read.unknown_word() {
        Some((kind, place)) => (kind, Some(place)),
        None => match read.reason() {
            Some((reason, place)) => (Kind::TemplateStopped { reason }, place.or(at)),
            None => {
                let detail = error
                    .detail()
                    .map_or_else(|| error.kind().to_string(), str::to_owned);
                (
                    Kind::TemplateStopped {
                        reason: Stopped::Engine(detail),
                    },
                    at,
                )
            }
        },
    }
}

/// `tried to use + operator on unsupported types string and number`, read
/// as the operator and the kinds of its two sides.
fn unsupported(detail: &str) -> Option<(&str, &str, &str)> {
    let rest = detail.strip_prefix("tried to use ")?;
    let (op, kinds) = rest.split_once(" operator on unsupported types ")?;
    let (left, right) = kinds.split_once(" and ")?;
    Some((op, left, right))
}

/// An operator as the template writes it.
struct Operator {
    written: String,
    left: Range<usize>,
    right: Range<usize>,
    whole: Range<usize>,
    /// Whether the left side is a bare name of a function.
    left_function: bool,
    right_function: bool,
}

/// A call of a function, filter or test as the template writes it.
struct Called {
    name: String,
    /// A filter's or test's input, as written.
    input: Option<String>,
    /// The names of the values given by name.
    named: Vec<String>,
}

struct Reader<'a> {
    text: &'a str,
    name: &'a str,
    range: Range<usize>,
    at: Range<usize>,
    ast: Option<Stmt<'a>>,
}

fn span_range(expr: &Expr<'_>) -> Range<usize> {
    let span = full_span(expr);
    span.start_offset as usize..span.end_offset as usize
}

fn own_range(expr: &Expr<'_>) -> Range<usize> {
    let span = expr.span();
    span.start_offset as usize..span.end_offset as usize
}

/// Whether `expr` is a bare name of one of the functions a template has.
fn names_function(expr: &Expr<'_>) -> bool {
    matches!(expr, Expr::Var(v)
        if guard::FUNCTIONS.contains(&v.id) || own::FUNCTIONS.contains(&v.id))
}

impl<'a> Reader<'a> {
    fn new(site: &Site<'a>) -> Reader<'a> {
        Reader {
            text: site.text,
            name: site.name,
            range: site.range.clone(),
            at: site.at.clone(),
            ast: parse_template(site.name, site.text).ok(),
        }
    }

    /// The first expression the engine's range is, that `f` answers for.
    fn find<R>(&self, mut f: impl FnMut(&Expr<'_>) -> Option<R>) -> Option<R> {
        let ast = self.ast.as_ref()?;
        let mut found = None;
        visit_exprs(ast, &mut |expr| {
            if found.is_some() {
                return;
            }
            let own = own_range(expr);
            let whole = span_range(expr);
            if [&self.range, &self.at]
                .iter()
                .any(|r| **r == own || **r == whole)
            {
                found = f(expr);
            }
        });
        found
    }

    /// The first expression inside the engine's range that `f` answers for.
    fn find_within<R>(&self, mut f: impl FnMut(&Expr<'_>) -> Option<R>) -> Option<R> {
        let ast = self.ast.as_ref()?;
        let mut found = None;
        visit_exprs(ast, &mut |expr| {
            let whole = span_range(expr);
            if found.is_none() && whole.start >= self.at.start && whole.end <= self.at.end {
                found = f(expr);
            }
        });
        found
    }

    fn written(&self, range: &Range<usize>) -> String {
        self.text[range.clone()].to_owned()
    }

    fn operator(&self) -> Option<Operator> {
        self.find(|expr| {
            let Expr::BinOp(op) = expr else { return None };
            let left = span_range(&op.left);
            let right = span_range(&op.right);
            Some(Operator {
                written: self.text.get(left.end..right.start)?.trim().to_owned(),
                whole: span_range(expr),
                left_function: names_function(&op.left),
                right_function: names_function(&op.right),
                left,
                right,
            })
        })
    }

    fn called(&self) -> Option<Called> {
        let text = self.text;
        self.find(|expr| {
            let (name, input, args) = match expr {
                Expr::Filter(f) => (
                    f.name,
                    f.expr.as_ref().map(|e| text[span_range(e)].to_owned()),
                    &f.args,
                ),
                Expr::Test(t) => (t.name, Some(text[span_range(&t.expr)].to_owned()), &t.args),
                Expr::Call(c) => match c.identify_call() {
                    CallType::Function(name) => (name, None, &c.args),
                    _ => return None,
                },
                _ => return None,
            };
            let named = args
                .iter()
                .filter_map(|arg| match arg {
                    CallArg::Kwarg(name, _) => Some((*name).to_owned()),
                    _ => None,
                })
                .collect();
            Some(Called {
                name: name.to_owned(),
                input,
                named,
            })
        })
    }

    /// The parameters of the macro `name` defined in the template.
    fn macro_params(&self, name: &str) -> Vec<String> {
        fn find(stmts: &[Stmt<'_>], name: &str) -> Option<Vec<String>> {
            stmts.iter().find_map(|stmt| match stmt {
                Stmt::Macro(m) if m.name == name => Some(
                    m.args
                        .iter()
                        .filter_map(|a| match a {
                            Expr::Var(v) => Some(v.id.to_owned()),
                            _ => None,
                        })
                        .collect(),
                ),
                Stmt::Template(t) => find(&t.children, name),
                Stmt::ForLoop(l) => find(&l.body, name).or_else(|| find(&l.else_body, name)),
                Stmt::IfCond(i) => find(&i.true_body, name).or_else(|| find(&i.false_body, name)),
                Stmt::WithBlock(w) => find(&w.body, name),
                Stmt::SetBlock(s) => find(&s.body, name),
                Stmt::AutoEscape(a) => find(&a.body, name),
                Stmt::FilterBlock(f) => find(&f.body, name),
                Stmt::Block(b) => find(&b.body, name),
                Stmt::Macro(m) => find(&m.body, name),
                Stmt::CallBlock(c) => find(&c.macro_decl.body, name),
                _ => None,
            })
        }
        self.ast
            .as_ref()
            .and_then(|ast| find(std::slice::from_ref(ast), name))
            .unwrap_or_default()
    }
}

struct Read<'r, 'a> {
    error: &'r Error,
    reader: Option<&'r Reader<'a>>,
    failed: Option<&'r Failed>,
    file_of: &'r dyn Fn(&str) -> Option<String>,
}

/// A kind of value, by the engine's word for it.
fn shape_of(word: &str, function: bool) -> Option<Shape> {
    Some(match word {
        "string" => Shape::Text,
        "number" | "integer" | "float" => Shape::Number,
        "bool" => Shape::TrueFalse,
        "none" => Shape::None,
        "sequence" | "iterator" => Shape::List,
        "map" => Shape::Map,
        "bytes" => Shape::Bytes,
        // A date is the plain object a template meets but for functions
        // named bare and bytes that aren't text.
        "plain object" if function => Shape::Other,
        "plain object" => Shape::Date,
        _ => return None,
    })
}

/// A value's kind, as it was handed on.
pub(in crate::templates) fn shape_of_value(value: &Value) -> Shape {
    match value.kind() {
        ValueKind::String => Shape::Text,
        ValueKind::Number => Shape::Number,
        ValueKind::Bool => Shape::TrueFalse,
        ValueKind::None | ValueKind::Undefined => Shape::None,
        ValueKind::Seq | ValueKind::Iterable => Shape::List,
        ValueKind::Map => Shape::Map,
        ValueKind::Bytes => Shape::Bytes,
        _ if value.downcast_object_ref::<DateValue>().is_some() => Shape::Date,
        _ => Shape::Other,
    }
}

impl Read<'_, '_> {
    fn detail(&self) -> &str {
        self.error.detail().unwrap_or_default()
    }

    /// The name of what was called: the engine's own callable that
    /// stopped, else the call the engine stopped at.
    fn callee(&self) -> Option<String> {
        match self.failed {
            Some(failed) => Some(failed.callee.to_owned()),
            None => self.reader?.called().map(|c| c.name),
        }
    }

    fn needs(&self, wanted: Wanted, given: Option<Shape>) -> Option<Stopped> {
        Some(Stopped::Needs {
            callee: self.callee()?,
            wanted,
            given,
        })
    }

    /// What the stopped callable was handed first that isn't text.
    fn not_text(&self) -> Option<Shape> {
        let failed = self.failed?;
        failed
            .args
            .iter()
            .find(|a| !a.is_kwargs() && a.kind() != ValueKind::String)
            .map(shape_of_value)
    }

    /// A test or filter, named in quotes for `select` or `map`, that the
    /// template language doesn't have.
    fn unknown_word(&self) -> Option<(Kind, Range<usize>)> {
        let (word, known): (Word, Vec<&str>) = match self.error.kind() {
            ErrorKind::UnknownTest => (Word::Test, guard::TESTS.to_vec()),
            ErrorKind::UnknownFilter => (
                Word::Filter,
                guard::FILTERS.iter().chain(own::FILTERS).copied().collect(),
            ),
            _ => return None,
        };
        let reader = self.reader?;
        reader.find(|expr| {
            let Expr::Filter(f) = expr else { return None };
            let index = match (word, f.name) {
                (Word::Test, "select" | "reject") | (Word::Filter, "map") => 0,
                (Word::Test, "selectattr" | "rejectattr") => 1,
                _ => return None,
            };
            let named = f
                .args
                .iter()
                .filter_map(|a| match a {
                    CallArg::Pos(e) => Some(e),
                    _ => None,
                })
                .nth(index)?;
            let Expr::Const(c) = named else { return None };
            let written = c.value.as_str()?.to_owned();
            let nearest = nearest(&written, known.iter().copied()).map(str::to_owned);
            Some((
                Kind::UnknownWord {
                    word,
                    written,
                    nearest,
                },
                span_range(named),
            ))
        })
    }

    /// The reason the engine stopped, where it is one told in Statac's own
    /// words, and where to tell it if not at the engine's range.
    fn reason(&self) -> Option<(Stopped, Option<Range<usize>>)> {
        let detail = self.detail();
        let reader = self.reader;
        let reason = match (self.error.kind(), detail) {
            (_, "strings can only be multiplied with integers") => Stopped::RepeatNotWhole,
            (_, "repeated string is too large") => Stopped::RepeatedTooLong,
            (_, "cannot perform a containment check on this value") => {
                // A comparison is spanned from the token before it, so it
                // is placed from its first value to its last.
                let place = reader?.find_within(|expr| match expr {
                    Expr::BinOp(op) if matches!(op.op, BinOpKind::In) => {
                        Some(span_range(&op.left).start..span_range(&op.right).end)
                    }
                    Expr::Compare(c)
                        if c.ops.iter().any(|op| {
                            matches!(op.op, CompareOpKind::In | CompareOpKind::NotIn)
                        }) =>
                    {
                        let last = c.ops.last()?;
                        Some(span_range(&c.expr).start..span_range(&last.expr).end)
                    }
                    _ => None,
                });
                let reason = Stopped::Needs {
                    callee: "in".into(),
                    wanted: Wanted::LookIn,
                    given: None,
                };
                return Some((reason, place));
            }
            (_, "cannot slice by step size of 0") => Stopped::SliceStep,
            (_, "object is not callable") => self.not_callable()?,
            (ErrorKind::MissingArgument, "") => {
                let called = reader?.called();
                let input = called
                    .filter(|c| self.failed.is_none_or(|f| f.callee == c.name))
                    .and_then(|c| c.input);
                Stopped::MissingArgument {
                    callee: self.callee()?,
                    input,
                }
            }
            (ErrorKind::TooManyArguments, "") => Stopped::TooManyArguments(self.callee()?),
            (ErrorKind::InvalidOperation, "") => match reader?.called() {
                Some(c) if c.name == "dict" => self.needs(Wanted::NamedValues, None)?,
                _ => Stopped::CantWorkOut(reader?.written(&reader?.at)),
            },
            (_, "cannot convert value into pair list" | "cannot convert value into pairs") => {
                self.needs(Wanted::Map, None)?
            }
            (_, "cannot get absolute value") => self.needs(Wanted::Number, None)?,
            (_, "invalid float literal") => Stopped::NotANumber(self.callee()?),
            (_, "cannot parse float from empty string") => Stopped::EmptyText(self.callee()?),
            (_, "cannot get first item from value" | "cannot get last item from value") => {
                self.needs(Wanted::ListOrText, None)?
            }
            (_, "cannot convert value to list") => Stopped::CantList {
                callee: self.callee()?,
                of: self
                    .failed
                    .and_then(|f| f.args.first())
                    .map_or(Shape::Other, shape_of_value),
            },
            (_, "count cannot be 0") => self.needs(Wanted::CountAboveZero, None)?,
            (_, "filter name is required" | "filter name must be a string") => {
                self.needs(Wanted::FilterName, None)?
            }
            (_, "value is not a string") => self.needs(Wanted::Text, self.not_text())?,
            (_, "format argument must be a mapping") => {
                Stopped::FormatNamed(self.first_mark(|mark| mark.starts_with("%("))?)
            }
            (_, "range has too many elements") => Stopped::RangeTooLong(RANGE_MOST),
            (_, "cannot create range with step of 0") => Stopped::RangeStep,
            (_, "cannot recurse outside of recursive loop") => Stopped::LoopNotRecursive,
            (_, "loop() takes one argument") => Stopped::LoopTakesOne,
            (ErrorKind::CannotUnpack, "value is not iterable") => {
                Stopped::Unpack(reader?.written(&reader?.at))
            }
            (_, "tried to extend a second time in a template") => Stopped::ExtendsTwice,
            (_, "template name was not a string") => Stopped::ExtendsName,
            (_, "cannot super outside of block") => Stopped::SuperOutside,
            (_, "recursion limit exceeded") => {
                Stopped::CallsTooDeep(reader.and_then(|r| r.called()).map(|c| c.name))
            }
            _ => return self.reason_with_words(detail).map(|r| (r, None)),
        };
        Some((reason, None))
    }

    /// The reasons whose words carry what the engine knows of them.
    fn reason_with_words(&self, detail: &str) -> Option<Stopped> {
        let reader = self.reader;
        if let Some((_, left, right)) = unsupported(detail) {
            let operator = reader.and_then(Reader::operator);
            let op = operator.as_ref().map(|o| o.written.clone()).or_else(|| {
                detail
                    .strip_prefix("tried to use ")
                    .and_then(|r| r.split(' ').next())
                    .map(str::to_owned)
            })?;
            let (lf, rf) = operator
                .as_ref()
                .map_or((false, false), |o| (o.left_function, o.right_function));
            return Some(Stopped::Operator {
                op,
                left: shape_of(left, lf)?,
                right: shape_of(right, rf)?,
            });
        }
        if let Some(sum) = detail.strip_prefix("unable to calculate ") {
            let mut words = sum.rsplitn(3, ' ');
            let right = words.next()?;
            let op = words.next()?;
            let zero = right.parse::<f64>().is_ok_and(|n| n == 0.0);
            if zero && matches!(op, "/" | "//" | "%") {
                return Some(Stopped::DividedByZero);
            }
            let written = match reader.and_then(Reader::operator) {
                Some(operator) => reader?.written(&operator.whole),
                None => reader?.written(&reader?.at),
            };
            return Some(Stopped::TooLarge(written));
        }
        if let Some(rest) = detail.strip_prefix("value of type ") {
            if let Some(kind) = rest.strip_suffix(" cannot be sliced") {
                return Some(Stopped::CantSlice(shape_of(kind, false)?));
            }
            if rest.ends_with(" is not callable") {
                return self.not_callable();
            }
        }
        if let Some((kind, name)) = detail.split_once(" has no method named ") {
            return self.no_method(shape_of(kind, false)?, name);
        }
        if detail.starts_with("expected object or keyword arguments") {
            return self.needs(Wanted::NamedValues, None);
        }
        if let Some(name) = detail
            .strip_prefix("unknown keyword argument ")
            .map(|n| n.trim_matches(['\'', '`']))
        {
            return self.no_such_argument(name);
        }
        if let Some(name) = detail.strip_prefix("duplicate argument ") {
            return Some(Stopped::GivenTwice {
                name: name.trim_matches('`').to_owned(),
                callee: self.callee()?,
            });
        }
        if let Some(rest) = detail.strip_prefix("cannot convert ") {
            let (kind, target) = rest.split_once(" to ")?;
            return self.cant_convert(kind, target);
        }
        if let Some(kind) = detail.strip_prefix("cannot calculate length of value of type ") {
            return Some(Stopped::NoLength(shape_of(kind, false)?));
        }
        if let Some(kind) = detail.strip_suffix(" is not iterable") {
            let of = shape_of(kind, false)?;
            return match self.failed {
                Some(_) => self.needs(Wanted::List, Some(of)),
                None => Some(Stopped::CantLoop(of)),
            };
        }
        if detail.starts_with("cannot join value of type ") {
            return self.needs(Wanted::List, None);
        }
        if detail.starts_with("cannot round value") {
            return self.needs(Wanted::Number, None);
        }
        if detail.starts_with("can only sum numbers") {
            return self.needs(Wanted::Numbers, None);
        }
        if detail.starts_with("zip filter argument must be iterable") {
            return self.needs(Wanted::Lists, None);
        }
        if detail.starts_with("cannot reverse values of type ") {
            return self.needs(Wanted::ListOrText, None);
        }
        if let Some(rest) = detail.strip_prefix("invalid format spec at offset ") {
            let (offset, rest) = rest.split_once("; '")?;
            let (given, rest) = rest.split_once("' cannot be formatted in ")?;
            let letter = rest.strip_suffix("')")?.chars().last()?;
            let mark = self.mark_at(offset.parse().ok()?, letter);
            return Some(match letter {
                'c' => Stopped::FormatCharacter(mark),
                _ => Stopped::FormatNeeds {
                    mark,
                    given: shape_of(given, false)?,
                },
            });
        }
        if let Some(rest) = detail.strip_prefix("invalid conversion type '") {
            let letter = rest.chars().next()?;
            return Some(Stopped::FormatHasNo(format!("%{letter}")));
        }
        if detail.starts_with("missing an argument for format spec") {
            return Some(Stopped::FormatTooFew);
        }
        if detail.ends_with("requires integer or char") {
            let mark = self.first_mark(|mark| mark.ends_with('c'));
            return Some(Stopped::FormatCharacter(
                mark.unwrap_or_else(|| "%c".into()),
            ));
        }
        if let Some(rest) = detail.strip_prefix("sequence of wrong length (expected ") {
            let (wants, rest) = rest.split_once(", got ")?;
            let given = rest.strip_suffix(')')?;
            return Some(Stopped::UnpackCount {
                names: reader?.written(&reader?.at),
                wants: wants.parse().ok()?,
                given: given.parse().ok()?,
            });
        }
        if detail.starts_with("can only assign to namespaces") {
            let written = reader?.written(&reader?.at);
            let (name, _) = written.rsplit_once('.')?;
            return Some(Stopped::NotANamespace(name.trim().to_owned()));
        }
        if let Some(rest) = detail.strip_prefix("cycle in template inheritance. ") {
            let named = rest.strip_suffix(" was referenced more than once")?;
            let named = named.trim_matches('"');
            let file = (self.file_of)(named)?;
            return Some(match reader.is_some_and(|r| r.name == named) {
                true => Stopped::ExtendsItself(file),
                false => Stopped::ExtendsAgain(file),
            });
        }
        None
    }

    fn not_callable(&self) -> Option<Stopped> {
        let reader = self.reader?;
        reader.find(|expr| match expr {
            Expr::Call(c) => Some(Stopped::NotCallable(reader.written(&span_range(&c.expr)))),
            _ => None,
        })
    }

    fn no_method(&self, of: Shape, name: &str) -> Option<Stopped> {
        let reader = self.reader?;
        reader.find(|expr| {
            let Expr::Call(c) = expr else { return None };
            let CallType::Method(receiver, method) = c.identify_call() else {
                return None;
            };
            if method != name {
                return None;
            }
            let receiver = reader.written(&span_range(receiver));
            let args = reader.text[span_range(&c.expr).end..span_range(expr).end].trim();
            let args = match args.trim_start_matches('(').trim_end_matches(')').trim() {
                "" => "",
                _ => args,
            };
            let filter = (of != Shape::Map
                && (guard::FILTERS.contains(&method) || own::FILTERS.contains(&method)))
            .then(|| format!("{receiver} | {method}{args}"));
            Some(Stopped::NoMethod {
                of,
                name: method.to_owned(),
                filter,
            })
        })
    }

    fn no_such_argument(&self, name: &str) -> Option<Stopped> {
        let callee = self.callee()?;
        let known: Vec<String> = match NAMED.iter().find(|(filter, _)| *filter == callee) {
            Some((_, names)) => names.iter().map(|&n| n.to_owned()).collect(),
            None => self.reader?.macro_params(&callee),
        };
        let nearest = nearest(name, known.iter().map(String::as_str)).map(str::to_owned);
        Some(Stopped::NoSuchArgument {
            callee,
            name: name.to_owned(),
            nearest,
        })
    }

    /// `cannot convert string to usize`: a value the callee can't take, or
    /// a value given by a name it doesn't take.
    fn cant_convert(&self, kind: &str, target: &str) -> Option<Stopped> {
        let called = self.reader.and_then(Reader::called);
        if kind == "kwargs"
            || (kind == "map" && called.as_ref().is_some_and(|c| !c.named.is_empty()))
        {
            let name = called?.named.into_iter().next()?;
            return self.no_such_argument(&name);
        }
        let of = shape_of(kind, false)?;
        if matches!(target, "integer" | "float") {
            return Some(Stopped::CantNumber {
                callee: self.callee()?,
                of,
            });
        }
        let negative = self.failed.is_some_and(|f| {
            f.args
                .iter()
                .any(|a| a.kind() == ValueKind::Number && a.as_i64().is_some_and(|n| n < 0))
        });
        let integer = matches!(
            target,
            "usize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "isize"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
        );
        let (wanted, given) = match target {
            "usize" | "u8" | "u16" | "u32" | "u64" | "u128" if negative => {
                (Wanted::WholeFromZero, None)
            }
            // A number that isn't whole: "not a number" would contradict itself.
            _ if integer && of == Shape::Number => (Wanted::Whole, None),
            _ if integer => (Wanted::Whole, Some(of)),
            "f32" | "f64" => (Wanted::Number, Some(of)),
            "bool" => (Wanted::TrueFalse, Some(of)),
            "char" => (Wanted::Character, Some(of)),
            "string" | "str" | "String" => (Wanted::Text, Some(of)),
            _ => return None,
        };
        self.needs(wanted, given)
    }

    /// The format the stopped `format` was handed.
    fn format(&self) -> Option<&str> {
        self.failed
            .filter(|f| f.callee == "format")
            .and_then(|f| f.args.first())
            .and_then(Value::as_str)
    }

    /// The `%` mark of the format whose body starts `offset` bytes in, as
    /// written; as `%` and its letter where the format isn't known.
    fn mark_at(&self, offset: usize, letter: char) -> String {
        self.format()
            .and_then(|format| {
                let body = format.get(offset..)?;
                Some(format!("%{}", &body[..mark_end(body)?]))
            })
            .unwrap_or_else(|| format!("%{letter}"))
    }

    /// The first `%` mark of the format that `is` picks, as written.
    fn first_mark(&self, is: impl Fn(&str) -> bool) -> Option<String> {
        let format = self.format()?;
        format
            .match_indices('%')
            .filter_map(|(at, _)| {
                let body = &format[at + 1..];
                Some(format!("%{}", &body[..mark_end(body)?]))
            })
            .find(|mark| is(mark))
    }
}

/// Where a `%` mark's body ends: after its letter, a name in brackets
/// first where it has one.
fn mark_end(body: &str) -> Option<usize> {
    let from = match body.starts_with('(') {
        true => body.find(')')? + 1,
        false => 0,
    };
    body[from..]
        .char_indices()
        .find(|(_, c)| c.is_ascii_alphabetic() || *c == '%')
        .map(|(i, c)| from + i + c.len_utf8())
}
