//! Templates read before any page is built: every name, field, filter,
//! test and snippet call is checked against what the site has, so a
//! mistake is told once, at its file, line and text, rather than on
//! whichever pages happen to reach it.
//!
//! The walk is over the engine's own parse of the template, which only its
//! unstable interface hands out; the engine is pinned to one release so a
//! change to that interface is a compile error here, never a silent gap.

use std::collections::{BTreeMap, BTreeSet};

use minijinja::machinery::ast::{self, BinOpKind, CallArg, CallType, Expr, Stmt};
use minijinja::machinery::{Span, WhitespaceConfig, parse};
use minijinja::syntax::SyntaxConfig;
use minijinja::value::Value;

use super::files::{Role, Snippet};
use super::{guard, own};
use crate::markdown::Piece;
use crate::problem::{
    ArgumentFault, Instead, Kind, Lack, Problem, TemplateFault, Unsupported, Word,
};
use crate::source::{Place, SourceId, Sources};
use crate::suggest::nearest;

/// Reads a template the way the environment does.
// `SyntaxConfig` is a unit struct unless the engine's `custom_syntax`
// feature is on.
#[allow(clippy::default_constructed_unit_structs)]
pub(super) fn parse_template<'s>(
    name: &'s str,
    source: &'s str,
) -> Result<Stmt<'s>, minijinja::Error> {
    parse(source, name, SyntaxConfig::default(), WHITESPACE)
}

/// How a tag's surrounding whitespace is read: a block tag takes its line's
/// leading spaces and its own line ending with it. Rendering uses the same,
/// so a template read back for checking lines up with what is built.
pub(super) const WHITESPACE: WhitespaceConfig = WhitespaceConfig {
    keep_trailing_newline: true,
    lstrip_blocks: true,
    trim_blocks: true,
};

/// What a value is, as far as reading the template can tell.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Ty {
    Text,
    Html,
    Number,
    Bool,
    Date,
    List(Box<Ty>),
    Record(Rec),
    /// A snippet, a macro or a function.
    Callable,
    /// Not known when the template is read, so nothing about it is checked.
    Any,
}

/// The kinds of thing with fields a template can be handed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Rec {
    Page,
    Site,
    Loop,
    /// A folder, a tag, a series or a declared list.
    List,
    Date,
    /// A date written as a year, a year and month, or a whole date.
    When,
    /// A page's mentions, as `page.webmentions` gives them.
    Webmentions,
    Mention,
}

impl Ty {
    fn list(of: Ty) -> Ty {
        Ty::List(Box::new(of))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Field {
    pub ty: Ty,
    /// Whether every value of the record has it.
    pub always: bool,
}

impl Field {
    fn of(ty: Ty, always: bool) -> Field {
        Field { ty, always }
    }
}

/// Every name a template may read, and what each holds.
#[derive(Debug, Default)]
pub(super) struct Schema {
    pub page: BTreeMap<String, Field>,
    pub site: BTreeMap<String, Field>,
    list: BTreeMap<String, Field>,
    date: BTreeMap<String, Field>,
    when: BTreeMap<String, Field>,
    webmentions: BTreeMap<String, Field>,
    mention: BTreeMap<String, Field>,
    looping: BTreeMap<String, Field>,
    /// Each snippet by its name.
    pub snippets: BTreeMap<String, Snippet>,
    /// The templates, by the names `extends` and `layout:` use.
    pub templates: BTreeSet<String>,
}

impl Schema {
    pub fn new() -> Schema {
        let fields = |rows: &[(&str, Ty, bool)]| -> BTreeMap<String, Field> {
            rows.iter()
                .map(|(name, ty, always)| ((*name).to_owned(), Field::of(ty.clone(), *always)))
                .collect()
        };
        let pages = Ty::list(Ty::Record(Rec::Page));
        Schema {
            list: fields(&[
                ("name", Ty::Text, true),
                ("title", Ty::Text, true),
                ("url", Ty::Text, true),
                ("page", Ty::Record(Rec::Page), false),
                ("posts", pages.clone(), true),
                ("pages", pages.clone(), true),
                ("word", Ty::Text, true),
                ("order", Ty::Text, true),
                ("feed", Ty::Text, true),
                ("pager", pages, true),
            ]),
            date: fields(&[
                ("year", Ty::Number, true),
                ("month", Ty::Number, true),
                ("day", Ty::Number, true),
                ("weekday", Ty::Text, false),
            ]),
            when: fields(&[
                ("year", Ty::Number, true),
                ("month", Ty::Number, false),
                ("day", Ty::Number, false),
            ]),
            webmentions: fields(&[
                ("said", Ty::list(Ty::Record(Rec::Mention)), true),
                ("faces", Ty::list(Ty::Record(Rec::Mention)), true),
            ]),
            mention: fields(&[
                ("kind", Ty::Text, true),
                ("author", Ty::Text, false),
                ("author_url", Ty::Text, false),
                ("url", Ty::Text, true),
                ("published", Ty::Record(Rec::Date), false),
                ("words", Ty::Text, false),
                ("face", Ty::Any, false),
            ]),
            looping: fields(&[
                ("index", Ty::Number, true),
                ("index0", Ty::Number, true),
                ("revindex", Ty::Number, true),
                ("revindex0", Ty::Number, true),
                ("first", Ty::Bool, true),
                ("last", Ty::Bool, true),
                ("length", Ty::Number, true),
                ("depth", Ty::Number, true),
                ("depth0", Ty::Number, true),
                ("previtem", Ty::Any, false),
                ("nextitem", Ty::Any, false),
                ("changed", Ty::Callable, true),
                ("cycle", Ty::Callable, true),
            ]),
            ..Schema::default()
        }
    }

    pub fn record(&self, rec: Rec) -> &BTreeMap<String, Field> {
        match rec {
            Rec::Page => &self.page,
            Rec::Site => &self.site,
            Rec::Loop => &self.looping,
            Rec::List => &self.list,
            Rec::Date => &self.date,
            Rec::When => &self.when,
            Rec::Webmentions => &self.webmentions,
            Rec::Mention => &self.mention,
        }
    }
}

/// The type of a page's own field, from its declaration.
pub(super) fn declared_ty(kind: Option<&crate::config::declarations::FieldKind>) -> Ty {
    use crate::config::declarations::FieldKind;
    match kind {
        Some(FieldKind::Text | FieldKind::Url | FieldKind::OneOf(_) | FieldKind::Named(_)) => {
            Ty::Text
        }
        Some(FieldKind::Number) => Ty::Number,
        Some(FieldKind::TrueFalse) => Ty::Bool,
        Some(FieldKind::List) => Ty::list(Ty::Any),
        Some(FieldKind::Date) => Ty::Record(Rec::Date),
        Some(FieldKind::When) => Ty::Record(Rec::When),
        None => Ty::Any,
    }
}

/// One file to check.
pub(super) struct Checked<'a> {
    pub name: &'a str,
    pub source: SourceId,
    pub role: &'a Role,
}

/// Checks every template and snippet of a site, and gives the problems
/// with the index of the file each is in.
///
/// Each template named in `with_url` is checked against the schema beside
/// it, in place of `schema`.
pub(super) fn check_site(
    schema: &Schema,
    with_url: Option<(&Schema, &BTreeSet<String>)>,
    files: &[Checked],
    sources: &Sources,
) -> Vec<(usize, Problem)> {
    let mut problems = Vec::new();
    let mut handed: BTreeMap<String, Vec<Vec<Ty>>> = BTreeMap::new();

    for (at, file) in files.iter().enumerate() {
        if matches!(file.role, Role::Snippet(_)) {
            continue;
        }
        let schema = match with_url {
            Some((with_url, names)) if names.contains(file.name) => with_url,
            _ => schema,
        };
        let mut checker = Checker::new(schema, file, sources, None);
        checker.run();
        problems.extend(checker.problems.into_iter().map(|p| (at, p)));
        for (name, args) in checker.calls {
            handed.entry(name).or_default().push(args);
        }
    }

    // A snippet's values take the types its calls hand it. Snippets call
    // snippets, so the types settle over a few rounds.
    let snippet_files: Vec<(usize, &Checked)> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| matches!(f.role, Role::Snippet(_)))
        .collect();
    let mut param_types: BTreeMap<String, Vec<Ty>> = BTreeMap::new();
    let mut rounds = 0;
    loop {
        rounds += 1;
        let mut next: BTreeMap<String, Vec<Ty>> = BTreeMap::new();
        for (_, file) in &snippet_files {
            let Role::Snippet(name) = file.role else {
                continue;
            };
            let params = schema
                .snippets
                .get(name)
                .map(|s| s.params.clone())
                .unwrap_or_default();
            next.insert(name.clone(), settle(&params, handed.get(name)));
        }
        let settled = next == param_types;
        param_types = next;
        let last = settled || rounds == 4;
        let mut more: BTreeMap<String, Vec<Vec<Ty>>> = BTreeMap::new();
        for (at, file) in &snippet_files {
            let Role::Snippet(name) = file.role else {
                continue;
            };
            let params = schema
                .snippets
                .get(name)
                .map(|s| s.params.clone())
                .unwrap_or_default();
            let types = &param_types[name];
            let typed = params.iter().cloned().zip(types.iter().cloned()).collect();
            let mut checker = Checker::new(schema, file, sources, Some((name.clone(), typed)));
            checker.run();
            if last {
                problems.extend(checker.problems.drain(..).map(|p| (*at, p)));
            }
            for (name, args) in checker.calls {
                more.entry(name).or_default().push(args);
            }
        }
        if last {
            break;
        }
        for (name, calls) in more {
            let entry = handed.entry(name).or_default();
            for call in calls {
                if !entry.contains(&call) {
                    entry.push(call);
                }
            }
        }
    }
    problems
}

/// What a piece's file is handed besides `page` and `site`, typed. A
/// picture's size is any value, since it is empty words where unknown.
fn handed_to(piece: Piece) -> Vec<(&'static str, Ty)> {
    piece
        .handed()
        .iter()
        .map(|&name| {
            let ty = match name {
                "body" => Ty::Html,
                "level" | "number" => Ty::Number,
                "outside" => Ty::Bool,
                "notes" => Ty::list(Ty::Any),
                "width" | "height" | "photo" => Ty::Any,
                _ => Ty::Text,
            };
            (name, ty)
        })
        .collect()
}

/// The type each value of a snippet takes: the one its calls agree on.
fn settle(params: &[String], calls: Option<&Vec<Vec<Ty>>>) -> Vec<Ty> {
    (0..params.len())
        .map(|i| {
            let mut seen: Option<Ty> = None;
            for call in calls.into_iter().flatten() {
                let ty = call.get(i).cloned().unwrap_or(Ty::Any);
                match &seen {
                    None => seen = Some(ty),
                    Some(t) if *t == ty => {}
                    Some(_) => return Ty::Any,
                }
            }
            seen.unwrap_or(Ty::Any)
        })
        .collect()
}

#[derive(Debug, Clone)]
struct Info {
    ty: Ty,
    /// The value may be missing when the template runs: a field some pages
    /// lack, an item that may not be there, or a value whose type can't be
    /// seen.
    maybe_missing: bool,
    /// For a list, whether an item of it may be missing, as from
    /// `map(attribute=...)` of a field some pages lack.
    items_maybe_missing: bool,
}

impl Info {
    fn of(ty: Ty) -> Info {
        Info::maybe(ty, false)
    }

    fn maybe(ty: Ty, maybe_missing: bool) -> Info {
        Info {
            ty,
            maybe_missing,
            items_maybe_missing: false,
        }
    }

    fn unknown() -> Info {
        Info::maybe(Ty::Any, true)
    }

    /// Already told; nothing more is said about what follows from it.
    fn told() -> Info {
        Info::of(Ty::Any)
    }
}

/// Whether the guards see a value in this position: printed, or handed to
/// a filter, test or function. Anywhere else, a missing value inside a
/// list or map passes unseen.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Seen {
    ByGuard,
    /// Kept by `set` or `with`: the guards see it wherever it is printed
    /// or handed on later.
    Stored,
    Unseen,
}

struct Checker<'a> {
    schema: &'a Schema,
    name: &'a str,
    source: SourceId,
    text: &'a str,
    /// The snippet being checked, with the types of what it is handed.
    snippet: Option<(String, Vec<(String, Ty)>)>,
    /// Names Statac hands this template besides `page` and `site`.
    extra: Vec<(&'static str, Ty)>,
    scopes: Vec<BTreeMap<String, Info>>,
    /// Paths known present here, from the `if` around this point.
    present: Vec<BTreeSet<String>>,
    /// Inside a test of whether something is there, where a field nothing
    /// has is the answer, not a mistake.
    probing: bool,
    /// Names bound by `import`, already refused.
    imported: BTreeSet<String>,
    problems: Vec<Problem>,
    /// Snippet calls, with the types of what each hands.
    calls: Vec<(String, Vec<Ty>)>,
    /// `page.<field>` lookups whose value is printed, with their spans.
    printed_page_fields: Vec<(String, Span)>,
}

impl<'a> Checker<'a> {
    fn new(
        schema: &'a Schema,
        file: &'a Checked<'a>,
        sources: &'a Sources,
        snippet: Option<(String, Vec<(String, Ty)>)>,
    ) -> Checker<'a> {
        let extra = match file.role {
            Role::Template if file.name == super::render::REDIRECT => {
                vec![("to", Ty::Text), ("title", Ty::Text)]
            }
            Role::Template if file.name.starts_with("feed.") => vec![
                ("list", Ty::Record(Rec::List)),
                ("posts", Ty::list(Ty::Record(Rec::Page))),
            ],
            Role::Piece(piece) => handed_to(*piece),
            _ => Vec::new(),
        };
        Checker {
            schema,
            name: file.name,
            source: file.source,
            text: sources.text(file.source),
            snippet,
            extra,
            scopes: vec![BTreeMap::new()],
            present: vec![BTreeSet::new()],
            probing: false,
            imported: BTreeSet::new(),
            problems: Vec::new(),
            calls: Vec::new(),
            printed_page_fields: Vec::new(),
        }
    }

    fn run(&mut self) {
        let ast = match parse_template(self.name, self.text) {
            Ok(ast) => ast,
            Err(error) => {
                self.problems.push(syntax(self.source, self.text, &error));
                return;
            }
        };
        self.stmt(&ast);
        self.snippet_prints_page();
    }

    fn at(&self, span: Span) -> Place {
        place(self.source, span)
    }

    fn spelling(&self, span: Span) -> &'a str {
        &self.text[span.start_offset as usize..span.end_offset as usize]
    }

    fn tell(&mut self, kind: Kind, span: Span) {
        self.problems.push(Problem::at(kind, self.at(span)));
    }

    // Names

    fn bind(&mut self, name: &str, info: Info) {
        self.scopes
            .last_mut()
            .expect("a scope")
            .insert(name.to_owned(), info);
    }

    fn bind_target(&mut self, target: &Expr<'_>, info: Info) {
        match target {
            Expr::Var(var) => self.bind(var.id, info),
            Expr::List(list) => {
                for item in &list.items {
                    self.bind_target(item, Info::of(Ty::Any));
                }
            }
            // `ns.value` assigns into a namespace already bound.
            other => {
                self.expr(other, Seen::Unseen);
            }
        }
    }

    fn visible(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for scope in self.scopes.iter().rev() {
            names.extend(scope.keys().cloned());
        }
        if let Some((_, params)) = &self.snippet {
            names.extend(params.iter().map(|(p, _)| p.clone()));
        }
        names.extend(self.extra.iter().map(|(n, _)| (*n).to_owned()));
        names.push("page".into());
        names.push("site".into());
        names
    }

    fn lookup_var(&self, name: &str) -> Option<Info> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info.clone());
            }
        }
        self.lookup_global(name)
    }

    fn lookup_global(&self, name: &str) -> Option<Info> {
        if let Some((_, params)) = &self.snippet {
            if let Some((_, ty)) = params.iter().find(|(p, _)| p == name) {
                return Some(Info::of(ty.clone()));
            }
            if super::files::SUPPLIED.contains(&name) || super::files::WRAPPED.contains(&name) {
                return Some(Info::unknown());
            }
        }
        if let Some((_, ty)) = self.extra.iter().find(|(n, _)| *n == name) {
            return Some(Info::of(ty.clone()));
        }
        match name {
            "page" => Some(Info::of(Ty::Record(Rec::Page))),
            "site" => Some(Info::of(Ty::Record(Rec::Site))),
            "self" | "super" | "varargs" | "kwargs" | "caller" => Some(Info::of(Ty::Any)),
            _ if self.schema.snippets.contains_key(name) => Some(Info::of(Ty::Callable)),
            _ if guard::FUNCTIONS.contains(&name) || own::FUNCTIONS.contains(&name) => {
                Some(Info::of(Ty::Callable))
            }
            _ => None,
        }
    }

    fn unknown_var(&mut self, name: &str, span: Span) {
        let visible = self.visible();
        let nearest = nearest(name, visible.iter().map(String::as_str)).map(str::to_owned);
        self.tell(
            Kind::UnknownWord {
                word: Word::Name,
                written: name.to_owned(),
                nearest,
            },
            span,
        );
    }

    // Narrowing by `if`

    fn path_of(expr: &Expr<'_>) -> Option<String> {
        match expr {
            Expr::Var(var) => Some(var.id.to_owned()),
            Expr::GetAttr(attr) => Some(format!("{}.{}", Self::path_of(&attr.expr)?, attr.name)),
            Expr::GetItem(item) => match &item.subscript_expr {
                Expr::Const(c) => Some(format!("{}.{}", Self::path_of(&item.expr)?, c.value)),
                _ => None,
            },
            _ => None,
        }
    }

    /// The paths a test proves present when it is true, and when it is
    /// false.
    fn proves(test: &Expr<'_>) -> (Vec<String>, Vec<String>) {
        match test {
            Expr::Var(_) | Expr::GetAttr(_) | Expr::GetItem(_) => {
                (Self::path_of(test).into_iter().collect(), Vec::new())
            }
            Expr::Test(t) if t.name == "defined" => {
                (Self::path_of(&t.expr).into_iter().collect(), Vec::new())
            }
            Expr::Test(t) if t.name == "undefined" => {
                (Vec::new(), Self::path_of(&t.expr).into_iter().collect())
            }
            Expr::UnaryOp(op) if matches!(op.op, ast::UnaryOpKind::Not) => {
                let (yes, no) = Self::proves(&op.expr);
                (no, yes)
            }
            Expr::BinOp(op) if matches!(op.op, ast::BinOpKind::ScAnd) => {
                let (mut a, _) = Self::proves(&op.left);
                let (b, _) = Self::proves(&op.right);
                a.extend(b);
                (a, Vec::new())
            }
            _ => (Vec::new(), Vec::new()),
        }
    }

    fn is_present(&self, path: &str) -> bool {
        self.present.iter().any(|set| set.contains(path))
    }

    fn with_present<R>(&mut self, paths: Vec<String>, f: impl FnOnce(&mut Self) -> R) -> R {
        self.present.push(paths.into_iter().collect());
        let r = f(self);
        self.present.pop();
        r
    }

    /// Checks a test of whether something is there.
    fn probe(&mut self, expr: &Expr<'_>, seen: Seen) -> Info {
        let was = std::mem::replace(&mut self.probing, true);
        let info = self.expr(expr, seen);
        self.probing = was;
        info
    }

    // Statements

    fn body(&mut self, stmts: &[Stmt<'_>]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    fn stmt(&mut self, stmt: &Stmt<'_>) {
        match stmt {
            Stmt::Template(t) => self.body(&t.children),
            Stmt::EmitExpr(e) => {
                self.note_printed_page_field(&e.expr);
                self.expr(&e.expr, Seen::ByGuard);
            }
            Stmt::EmitRaw(_) => {}
            Stmt::ForLoop(f) => {
                let iter = self.expr(&f.iter, Seen::Unseen);
                let item = match iter.ty {
                    Ty::List(of) => *of,
                    _ => Ty::Any,
                };
                self.scopes.push(BTreeMap::new());
                self.bind_target(&f.target, Info::maybe(item, iter.items_maybe_missing));
                self.bind("loop", Info::of(Ty::Record(Rec::Loop)));
                let proven = match &f.filter_expr {
                    Some(filter) => {
                        self.probe(filter, Seen::Unseen);
                        Self::proves(filter).0
                    }
                    None => Vec::new(),
                };
                self.with_present(proven, |c| c.body(&f.body));
                self.scopes.pop();
                self.body(&f.else_body);
            }
            Stmt::IfCond(i) => {
                self.probe(&i.expr, Seen::Unseen);
                let (yes, no) = Self::proves(&i.expr);
                self.with_present(yes, |c| c.body(&i.true_body));
                self.with_present(no, |c| c.body(&i.false_body));
            }
            Stmt::WithBlock(w) => {
                self.scopes.push(BTreeMap::new());
                for (target, value) in &w.assignments {
                    let info = self.expr(value, Seen::Stored);
                    self.bind_target(target, info);
                }
                self.body(&w.body);
                self.scopes.pop();
            }
            Stmt::Set(s) => {
                let info = self.expr(&s.expr, Seen::Stored);
                self.bind_target(&s.target, info);
            }
            Stmt::SetBlock(s) => {
                if let Some(filter) = &s.filter {
                    self.expr(filter, Seen::ByGuard);
                }
                self.body(&s.body);
                self.bind_target(&s.target, Info::of(Ty::Html));
            }
            Stmt::AutoEscape(a) => {
                self.expr(&a.enabled, Seen::Unseen);
                self.body(&a.body);
            }
            Stmt::FilterBlock(f) => {
                self.expr(&f.filter, Seen::ByGuard);
                self.body(&f.body);
            }
            Stmt::Block(b) => self.body(&b.body),
            Stmt::Extends(e) => self.extends(&e.name),
            Stmt::Include(i) => self.refuse_include(stmt_span(stmt), &i.name),
            Stmt::Import(i) => {
                if let Expr::Var(var) = &i.name {
                    self.imported.insert(var.id.to_owned());
                }
                let calls = self.calls_through_module(stmt_span(stmt));
                self.refuse("import", calls, stmt_span(stmt));
            }
            Stmt::FromImport(i) => {
                let mut calls = Vec::new();
                for (name, alias) in &i.names {
                    let bound = match alias.as_ref().unwrap_or(name) {
                        Expr::Var(v) => v.id,
                        _ => continue,
                    };
                    self.imported.insert(bound.to_owned());
                    if let Expr::Var(v) = name {
                        calls.extend(self.replacement(v.id));
                    }
                }
                self.refuse("from", calls, stmt_span(stmt));
            }
            Stmt::Macro(m) => {
                self.bind(m.name, Info::of(Ty::Callable));
                self.macro_body(m);
            }
            Stmt::CallBlock(c) => {
                self.call(&c.call, c.call.span(), Seen::Unseen);
                self.macro_body(&c.macro_decl);
            }
            Stmt::Continue(_) | Stmt::Break(_) => {}
            Stmt::Do(d) => {
                self.call(&d.call, d.call.span(), Seen::Unseen);
            }
        }
    }

    fn macro_body(&mut self, m: &ast::Macro<'_>) {
        for default in &m.defaults {
            self.expr(default, Seen::Unseen);
        }
        self.scopes.push(BTreeMap::new());
        for arg in &m.args {
            // A macro is handed whatever its call holds, missing or not.
            self.bind_target(arg, Info::unknown());
        }
        self.body(&m.body);
        self.scopes.pop();
    }

    fn extends(&mut self, name: &Expr<'_>) {
        let Expr::Const(c) = name else {
            self.expr(name, Seen::Unseen);
            return;
        };
        let Some(wanted) = c.value.as_str().map(super::files::named) else {
            return;
        };
        if self.schema.templates.contains(wanted) {
            return;
        }
        let nearest =
            nearest(wanted, self.schema.templates.iter().map(String::as_str)).map(str::to_owned);
        self.tell(
            Kind::MissingTemplate {
                path: super::files::template_path(wanted),
                chosen: crate::problem::Chooser::Written,
                nearest,
            },
            name.span(),
        );
    }

    // `include` and `import`

    fn refuse(&mut self, statement: &str, calls: Vec<String>, span: Span) {
        let kind = Kind::NotSupported {
            what: Unsupported::Statement {
                statement: statement.to_owned(),
                calls,
            },
        };
        let (start, end) = (span.start_offset as usize, span.end_offset as usize);
        // The engine places a statement inside its `{%` and `%}`; the tag
        // is the text the author would take out.
        let open = self.text[..start].rfind("{%").filter(|&at| {
            self.text[at + 2..start]
                .trim_start_matches(['-', '+'])
                .trim()
                .is_empty()
        });
        let close = self.text[end..].find("%}").filter(|&at| {
            self.text[end..end + at]
                .trim_end_matches(['-', '+'])
                .trim()
                .is_empty()
        });
        let span = match (open, close) {
            (Some(open), Some(close)) => crate::source::Span::new(open, end + close + 2),
            _ => crate::source::Span::new(start, end),
        };
        self.problems.push(Problem::at(
            kind,
            Place {
                source: self.source,
                span,
            },
        ));
    }

    fn refuse_include(&mut self, span: Span, name: &Expr<'_>) {
        let calls = match name {
            Expr::Const(c) => c
                .value
                .as_str()
                .and_then(|file| {
                    let stem = file
                        .trim_start_matches("snippets/")
                        .trim_end_matches(".html");
                    self.replacement(stem)
                })
                .into_iter()
                .collect(),
            _ => Vec::new(),
        };
        self.refuse("include", calls, span);
    }

    /// The call that does what including or importing `name` did, handed
    /// the names the snippet reads; none where the snippet can't be read.
    fn replacement(&self, name: &str) -> Option<String> {
        let params = match self.schema.snippets.get(name) {
            Some(snippet) if !snippet.readable => return None,
            Some(snippet) => snippet.call_arguments(),
            None => String::new(),
        };
        Some(format!("{{{{ {name}({params}) }}}}"))
    }

    /// Every `module.name(...)` call after an `import ... as module`,
    /// written as the plain call it becomes.
    fn calls_through_module(&self, import_span: Span) -> Vec<String> {
        let Ok(ast) = parse_template(self.name, self.text) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        visit_exprs(&ast, &mut |expr| {
            if let Expr::Call(call) = expr
                && let CallType::Method(Expr::Var(module), _) = call.identify_call()
                && self.imported.contains(module.id)
                && call.span().start_offset > import_span.end_offset
            {
                let text = self.spelling(full_span(expr, self.text));
                let rewritten = text.replacen(&format!("{}.", module.id), "", 1);
                found.push(format!("{{{{ {rewritten} }}}}"));
            }
        });
        found
    }

    // Expressions

    fn expr(&mut self, expr: &Expr<'_>, seen: Seen) -> Info {
        match expr {
            Expr::Var(var) => {
                if self.imported.contains(var.id) {
                    return Info::told();
                }
                match self.lookup_var(var.id) {
                    Some(info) => Info {
                        maybe_missing: info.maybe_missing && !self.is_present(var.id),
                        ..info
                    },
                    None => {
                        self.unknown_var(var.id, var.span());
                        Info::told()
                    }
                }
            }
            Expr::Const(c) => Info::of(const_ty(&c.value)),
            Expr::Slice(s) => {
                let info = self.expr(&s.expr, Seen::Unseen);
                for part in [&s.start, &s.stop, &s.step].into_iter().flatten() {
                    self.expr(part, Seen::Unseen);
                }
                Info::of(info.ty)
            }
            Expr::UnaryOp(op) => {
                self.expr(&op.expr, Seen::Unseen);
                Info::of(match op.op {
                    ast::UnaryOpKind::Not => Ty::Bool,
                    ast::UnaryOpKind::Neg => Ty::Number,
                })
            }
            Expr::BinOp(op) => {
                use ast::BinOpKind::*;
                // What `+`, `~`, `and` and `or` make is what is stored or
                // handed on; a comparison or `in` is consumed on the spot.
                let operands = match op.op {
                    Eq | Ne | Lt | Lte | Gt | Gte | In => Seen::Unseen,
                    _ => seen,
                };
                let left = match op.op {
                    ScOr | ScAnd => self.probe(&op.left, operands),
                    _ => self.expr(&op.left, operands),
                };
                let right = self.expr(&op.right, operands);
                match op.op {
                    Concat => Info::of(Ty::Text),
                    Add | Sub | Mul | Div | FloorDiv | Rem | Pow => {
                        Info::of(if left.ty == right.ty {
                            left.ty
                        } else {
                            Ty::Any
                        })
                    }
                    ScOr | ScAnd => Info::maybe(
                        if left.ty == right.ty {
                            left.ty
                        } else {
                            Ty::Any
                        },
                        right.maybe_missing,
                    ),
                    _ => Info::of(Ty::Bool),
                }
            }
            Expr::Compare(c) => {
                self.expr(&c.expr, Seen::Unseen);
                for op in &c.ops {
                    self.expr(&op.expr, Seen::Unseen);
                }
                Info::of(Ty::Bool)
            }
            Expr::IfExpr(i) => {
                self.probe(&i.test_expr, Seen::Unseen);
                let (yes, no) = Self::proves(&i.test_expr);
                let a = self.with_present(yes, |c| c.expr(&i.true_expr, seen));
                let b = match &i.false_expr {
                    Some(f) => self.with_present(no, |c| c.expr(f, seen)),
                    None => Info::unknown(),
                };
                Info {
                    ty: if a.ty == b.ty { a.ty } else { Ty::Any },
                    maybe_missing: a.maybe_missing || b.maybe_missing,
                    items_maybe_missing: a.items_maybe_missing || b.items_maybe_missing,
                }
            }
            Expr::Filter(f) => self.filter(f, expr.span()),
            Expr::Test(t) => {
                if matches!(t.name, "defined" | "undefined") {
                    self.misspelt_guard(&t.expr);
                }
                match t.name {
                    "defined" | "undefined" | "none" => self.probe(&t.expr, Seen::ByGuard),
                    _ => self.expr(&t.expr, Seen::ByGuard),
                };
                self.args(&t.args, Seen::ByGuard);
                if !guard::TESTS.contains(&t.name) {
                    let nearest = nearest(t.name, guard::TESTS.iter().copied()).map(str::to_owned);
                    self.tell(
                        Kind::UnknownWord {
                            word: Word::Test,
                            written: t.name.to_owned(),
                            nearest,
                        },
                        expr.span(),
                    );
                }
                Info::of(Ty::Bool)
            }
            Expr::GetAttr(attr) => {
                let base = self.expr(&attr.expr, Seen::Unseen);
                self.field(expr, &attr.expr, base, attr.name)
            }
            Expr::GetItem(item) => {
                let base = self.expr(&item.expr, Seen::Unseen);
                self.expr(&item.subscript_expr, Seen::Unseen);
                match (&item.subscript_expr, &base.ty) {
                    (Expr::Const(c), Ty::Record(_)) if c.value.as_str().is_some() => {
                        let name = c.value.as_str().unwrap_or_default().to_owned();
                        self.field(expr, &item.expr, base, &name)
                    }
                    (_, Ty::List(of)) => Info::maybe((**of).clone(), true),
                    _ => Info::unknown(),
                }
            }
            Expr::Call(call) => self.call(call, full_span(expr, self.text), seen),
            Expr::List(list) => {
                let mut tys = Vec::new();
                let mut items_maybe_missing = false;
                for item in &list.items {
                    let info = self.expr(item, seen);
                    if seen == Seen::Unseen && info.maybe_missing {
                        self.hidden_in_literal(item, false);
                    }
                    items_maybe_missing |= info.maybe_missing;
                    tys.push(info.ty);
                }
                Info {
                    items_maybe_missing,
                    ..Info::of(Ty::list(common(tys)))
                }
            }
            Expr::Map(map) => {
                for (key, value) in map.keys.iter().zip(&map.values) {
                    for part in [key, value] {
                        let info = self.expr(part, seen);
                        if seen == Seen::Unseen && info.maybe_missing {
                            self.hidden_in_literal(part, true);
                        }
                    }
                }
                Info::of(Ty::Any)
            }
        }
    }

    fn hidden_in_literal(&mut self, item: &Expr<'_>, map: bool) {
        let span = full_span(item, self.text);
        let written = self.spelling(span).to_owned();
        self.tell(
            Kind::NoValue {
                instead: Instead::Default(defaulted(&written)),
                written,
                lack: Lack::InLiteral { map },
            },
            span,
        );
    }

    /// `base.name`, where `whole` is the lookup and `base_expr` its left.
    fn field(&mut self, whole: &Expr<'_>, base_expr: &Expr<'_>, base: Info, name: &str) -> Info {
        let span = full_span(whole, self.text);
        let present = Self::path_of(whole).is_some_and(|p| self.is_present(&p));
        match &base.ty {
            Ty::Record(rec) => match self.schema.record(*rec).get(name) {
                Some(field) => Info::maybe(field.ty.clone(), !field.always && !present),
                None if self.probing || present => Info::unknown(),
                None => {
                    self.unknown_field(*rec, base_expr, name, span);
                    Info::told()
                }
            },
            Ty::List(of) if name.parse::<usize>().is_ok() => Info::maybe((**of).clone(), true),
            Ty::Any | Ty::Callable => Info::maybe(Ty::Any, !present),
            _ if self.probing || present => Info::unknown(),
            _ => {
                let written = self.spelling(span).to_owned();
                self.tell(
                    Kind::UnknownWord {
                        word: Word::Field,
                        written,
                        nearest: None,
                    },
                    span,
                );
                Info::told()
            }
        }
    }

    /// Tells a test of whether a page has a field that no page has and
    /// that is one slip from a field a page can have, which is false on
    /// every page.
    fn misspelt_guard(&mut self, tested: &Expr<'_>) {
        let Expr::GetAttr(attr) = tested else {
            return;
        };
        let Expr::Var(var) = &attr.expr else {
            return;
        };
        let on_a_page = self
            .lookup_var(var.id)
            .is_some_and(|info| info.ty == Ty::Record(Rec::Page));
        let path = format!("{}.{}", var.id, attr.name);
        let fields = self.schema.record(Rec::Page);
        if !on_a_page
            || fields.contains_key(attr.name)
            || attr.name.chars().count() < 4
            || self.is_present(&path)
        {
            return;
        }
        let Some(meant) = fields
            .keys()
            .find(|field| crate::suggest::is_one_slip(attr.name, field))
        else {
            return;
        };
        let kind = Kind::AlwaysFalse {
            name: attr.name.to_owned(),
            meant: format!("{}.{meant}", var.id),
        };
        self.tell(kind, full_span(tested, self.text));
    }

    fn unknown_field(&mut self, rec: Rec, base_expr: &Expr<'_>, name: &str, span: Span) {
        let base_text = self.spelling(full_span(base_expr, self.text)).to_owned();
        let written = format!("{base_text}.{name}");
        let fields = self.schema.record(rec);
        let near = nearest(name, fields.keys().map(String::as_str));
        let kind = match (near, rec) {
            (Some(near), _) => Kind::UnknownWord {
                word: Word::Field,
                written,
                nearest: Some(format!("{base_text}.{near}")),
            },
            (None, Rec::Page | Rec::Site) => Kind::NoValue {
                instead: Instead::Default(defaulted(&written)),
                written,
                lack: Lack::Unset,
            },
            (None, _) => Kind::UnknownWord {
                word: Word::Field,
                written,
                nearest: None,
            },
        };
        self.tell(kind, span);
    }

    fn args(&mut self, args: &[CallArg<'_>], seen: Seen) -> Vec<Info> {
        args.iter()
            .map(|arg| match arg {
                CallArg::Pos(e)
                | CallArg::Kwarg(_, e)
                | CallArg::PosSplat(e)
                | CallArg::KwargSplat(e) => self.expr(e, seen),
            })
            .collect()
    }

    fn call(&mut self, call: &ast::Call<'_>, span: Span, seen: Seen) -> Info {
        match call.identify_call() {
            CallType::Function(name) => {
                if let Some(snippet) = self.schema.snippets.get(name) {
                    let infos = self.args(&call.args, Seen::ByGuard);
                    self.check_snippet_call(call, name, snippet, &infos);
                    Info::of(Ty::Html)
                } else if guard::FUNCTIONS.contains(&name) || own::FUNCTIONS.contains(&name) {
                    self.args(&call.args, Seen::ByGuard);
                    match name {
                        "related" => Info::of(Ty::list(Ty::Record(Rec::Page))),
                        _ => Info::of(Ty::Any),
                    }
                } else if self.lookup_var(name).is_some() {
                    // A macro, `caller` or `loop`: its values reach it as
                    // they are.
                    self.args(&call.args, Seen::Unseen);
                    Info::of(Ty::Html)
                } else {
                    self.args(&call.args, Seen::ByGuard);
                    let callables = self
                        .schema
                        .snippets
                        .keys()
                        .map(String::as_str)
                        .chain(guard::FUNCTIONS.iter().copied())
                        .chain(own::FUNCTIONS.iter().copied());
                    let nearest = nearest(name, callables).map(str::to_owned);
                    self.tell(
                        Kind::UnknownWord {
                            word: Word::Snippet,
                            written: name.to_owned(),
                            nearest,
                        },
                        call.expr.span(),
                    );
                    let _ = span;
                    Info::told()
                }
            }
            CallType::Method(receiver, _) => {
                if let Expr::Var(module) = receiver
                    && self.imported.contains(module.id)
                {
                    self.args(&call.args, Seen::ByGuard);
                    return Info::told();
                }
                self.expr(receiver, Seen::Unseen);
                self.args(&call.args, Seen::Unseen);
                Info::of(Ty::Any)
            }
            CallType::Block(_) => Info::of(Ty::Html),
            CallType::Object(callee) => {
                self.expr(callee, Seen::Unseen);
                self.args(&call.args, seen);
                Info::of(Ty::Any)
            }
        }
    }

    fn check_snippet_call(
        &mut self,
        call: &ast::Call<'_>,
        name: &str,
        snippet: &Snippet,
        infos: &[Info],
    ) {
        if !snippet.readable {
            return;
        }
        let params = &snippet.params;
        let mut handed: Vec<Option<Ty>> = vec![None; params.len()];
        let mut unnamed: Vec<(&Expr<'_>, &Info)> = Vec::new();
        let mut named: Vec<&str> = Vec::new();
        let wrong = |fault| Kind::WrongArgument {
            snippet: name.to_owned(),
            fault,
        };
        for (arg, info) in call.args.iter().zip(infos) {
            match arg {
                CallArg::Pos(e) => unnamed.push((e, info)),
                CallArg::Kwarg(key, e) => {
                    named.push(key);
                    match params.iter().position(|p| p == key) {
                        Some(i) => handed[i] = Some(info.ty.clone()),
                        None if snippet.wrapped.iter().any(|w| w == key) => {}
                        None if super::files::SUPPLIED.contains(key) => {
                            self.tell(wrong(ArgumentFault::Supplied((*key).to_owned())), e.span());
                        }
                        None => {
                            let nearest =
                                nearest(key, params.iter().map(String::as_str)).map(str::to_owned);
                            self.tell(
                                wrong(ArgumentFault::NoSuchName {
                                    given: (*key).to_owned(),
                                    nearest,
                                }),
                                e.span(),
                            );
                        }
                    }
                }
                CallArg::PosSplat(_) | CallArg::KwargSplat(_) => {
                    handed.iter_mut().for_each(|h| *h = Some(Ty::Any));
                }
            }
        }
        if let Some((first, _)) = unnamed.first() {
            let span = full_span(first, self.text);
            match super::files::unnamed_fills(snippet.takes.as_ref(), params, unnamed.len(), |p| {
                named.contains(&p)
            }) {
                super::files::Unnamed::Fills(fills) => {
                    for (param, (_, info)) in fills.iter().zip(&unnamed) {
                        if let Some(i) = params.iter().position(|p| p == param) {
                            handed[i] = Some(info.ty.clone());
                        }
                    }
                }
                super::files::Unnamed::Gathers(param) => {
                    if let Some(i) = params.iter().position(|p| *p == param) {
                        handed[i] = Some(Ty::list(Ty::Any));
                    }
                }
                super::files::Unnamed::NeedsNames => self.tell(
                    wrong(ArgumentFault::NeedsNames {
                        wanted: params.clone(),
                    }),
                    span,
                ),
                super::files::Unnamed::TooMany { takes, past } => {
                    let at = unnamed
                        .get(past)
                        .map_or(span, |(e, _)| full_span(e, self.text));
                    let others = params
                        .iter()
                        .filter(|p| !takes.contains(p))
                        .cloned()
                        .collect();
                    self.tell(wrong(ArgumentFault::TooMany { takes, others }), at);
                }
                super::files::Unnamed::Twice { name: twice, at } => {
                    let at = unnamed
                        .get(at)
                        .map_or(span, |(e, _)| full_span(e, self.text));
                    self.tell(wrong(ArgumentFault::Twice(twice)), at);
                }
                super::files::Unnamed::Nothing => {}
            }
        }
        self.calls.push((
            name.to_owned(),
            handed.into_iter().map(|h| h.unwrap_or(Ty::Any)).collect(),
        ));
    }

    fn filter(&mut self, f: &ast::Filter<'_>, span: Span) -> Info {
        let input = match &f.expr {
            Some(e) if matches!(f.name, "default" | "d") => self.probe(e, Seen::ByGuard),
            Some(e) => self.expr(e, Seen::ByGuard),
            None => Info::of(Ty::Any),
        };
        let args = self.args(&f.args, Seen::ByGuard);
        let ours = own::FILTERS.contains(&f.name);
        if !guard::FILTERS.contains(&f.name) && !ours {
            let known = guard::FILTERS.iter().chain(own::FILTERS).copied();
            let nearest = nearest(f.name, known).map(str::to_owned);
            self.tell(
                Kind::UnknownWord {
                    word: Word::Filter,
                    written: f.name.to_owned(),
                    nearest,
                },
                span,
            );
            return Info::told();
        }
        let item = match &input.ty {
            Ty::List(of) => Some((**of).clone()),
            _ => None,
        };
        // The field a filter reads from each item, by the value naming it.
        let attribute_arg = match f.name {
            "map" | "sort" | "unique" => kwarg(&f.args, "attribute"),
            "groupby" => kwarg(&f.args, "attribute").or_else(|| positional(&f.args, 0)),
            "selectattr" | "rejectattr" => positional(&f.args, 0),
            _ => None,
        };
        let mut picked: Option<Ty> = None;
        if let (Some(arg), Some(item)) = (attribute_arg, &item)
            && let Expr::Const(c) = arg
            && let Some(names) = c.value.as_str()
        {
            let paths: Vec<&str> = match f.name {
                "sort" => names.split(',').map(str::trim).collect(),
                _ => vec![names],
            };
            let probing = matches!(f.name, "selectattr" | "rejectattr");
            for path in paths {
                picked = self.attribute_path(item, path, arg.span(), probing);
            }
        }
        let list_of_item = || Ty::list(item.clone().unwrap_or(Ty::Any));
        let same_items = |ty: Ty| Info {
            items_maybe_missing: input.items_maybe_missing,
            ..Info::of(ty)
        };
        match f.name {
            "default" | "d" => Info::maybe(
                if input.ty == Ty::Any {
                    args.first().map_or(Ty::Any, |a| a.ty.clone())
                } else {
                    input.ty.clone()
                },
                input.maybe_missing && args.first().is_some_and(|a| a.maybe_missing),
            ),
            "sort" | "reverse" | "list" | "unique" | "reject" | "rejectattr" | "newest_first"
            | "in" | "tagged" | "where" => same_items(list_of_item()),
            // These keep only what tests true, so no missing item is kept.
            "select" | "selectattr" => Info::of(list_of_item()),
            "batch" | "slice" => Info::of(Ty::list(list_of_item())),
            "first" | "last" | "min" | "max" => Info::maybe(item.unwrap_or(Ty::Any), true),
            // An item lacking the field stops `map`, so none is missing.
            "map" => match (&picked, attribute_arg) {
                (Some(ty), _) => Info::of(Ty::list(ty.clone())),
                (None, Some(_)) => Info::of(Ty::list(Ty::Any)),
                (None, None) => same_items(Ty::list(Ty::Any)),
            },
            "attr" => Info::unknown(),
            "newest_date" => Info::maybe(Ty::Date, true),
            "length" | "count" | "int" | "float" | "abs" | "round" | "sum" => Info::of(Ty::Number),
            "bool" => Info::of(Ty::Bool),
            "safe" | "markdown" | "json" | "highlight" => Info::of(Ty::Html),
            "groupby" | "items" | "dictsort" | "lines" | "split" | "zip" | "chain" => {
                Info::of(Ty::list(Ty::Any))
            }
            _ => Info::of(Ty::Text),
        }
    }

    /// Checks a dotted field path against an item type: the type it
    /// reaches.
    fn attribute_path(&mut self, item: &Ty, path: &str, span: Span, probing: bool) -> Option<Ty> {
        let mut ty = item.clone();
        for piece in path.split('.') {
            match &ty {
                Ty::Record(rec) => {
                    let fields = self.schema.record(*rec);
                    match fields.get(piece) {
                        Some(field) => {
                            ty = field.ty.clone();
                        }
                        None if probing => return None,
                        None => {
                            let near = nearest(piece, fields.keys().map(String::as_str));
                            let written = self.spelling(span).to_owned();
                            let kind = match near {
                                Some(near) => Kind::UnknownWord {
                                    word: Word::Field,
                                    nearest: Some(written.replacen(piece, near, 1)),
                                    written,
                                },
                                None => Kind::NoValue {
                                    written: piece.to_owned(),
                                    lack: Lack::Unset,
                                    instead: Instead::Nothing,
                                },
                            };
                            self.tell(kind, span);
                            return None;
                        }
                    }
                }
                Ty::List(of) if piece.parse::<usize>().is_ok() => {
                    ty = (**of).clone();
                }
                _ => return None,
            }
        }
        Some(ty)
    }

    /// Notes `page.<field>` when it is what an output tag prints, directly
    /// or through filters.
    fn note_printed_page_field(&mut self, expr: &Expr<'_>) {
        let mut expr = expr;
        while let Expr::Filter(f) = expr {
            match &f.expr {
                Some(inner) => expr = inner,
                None => return,
            }
        }
        if let Expr::GetAttr(attr) = expr
            && let Expr::Var(var) = &attr.expr
            && var.id == "page"
        {
            self.printed_page_fields
                .push((attr.name.to_owned(), full_span(expr, self.text)));
        }
    }

    /// A snippet handed a page that prints a field of `page`, the page
    /// being built, which on a list isn't the page it was handed.
    fn snippet_prints_page(&mut self) {
        let Some((snippet, params)) = self.snippet.clone() else {
            return;
        };
        for (param, ty) in &params {
            if *ty != Ty::Record(Rec::Page) {
                continue;
            }
            for (field, span) in self.printed_page_fields.clone() {
                if !self.schema.page.contains_key(&field) {
                    continue;
                }
                self.tell(
                    Kind::PageInSnippet {
                        snippet: snippet.clone(),
                        written: format!("page.{field}"),
                        handed: format!("{param}.{field}"),
                    },
                    span,
                );
            }
        }
    }
}

/// A template the engine can't read, at the place it stopped.
pub(super) fn syntax(source: SourceId, text: &str, error: &minijinja::Error) -> Problem {
    let (fault, range) = super::engine::read_syntax(text, error);
    Problem::at(
        Kind::MalformedTemplate {
            found: TemplateFault::Syntax(fault),
        },
        Place {
            source,
            span: crate::source::Span::new(range.start, range.end),
        },
    )
}

/// A value written with a `default` of nothing, in brackets where it is
/// more than one name.
pub(super) fn defaulted(written: &str) -> String {
    match written.contains(char::is_whitespace) {
        true => format!("({written}) | default(\"\")"),
        false => format!("{written} | default(\"\")"),
    }
}

pub(super) fn place(source: SourceId, span: Span) -> Place {
    Place {
        source,
        span: crate::source::Span::new(span.start_offset as usize, span.end_offset as usize),
    }
}

fn kwarg<'e, 'a>(args: &'e [CallArg<'a>], name: &str) -> Option<&'e Expr<'a>> {
    args.iter().find_map(|a| match a {
        CallArg::Kwarg(k, e) if *k == name => Some(e),
        _ => None,
    })
}

fn positional<'e, 'a>(args: &'e [CallArg<'a>], n: usize) -> Option<&'e Expr<'a>> {
    args.iter()
        .filter_map(|a| match a {
            CallArg::Pos(e) => Some(e),
            _ => None,
        })
        .nth(n)
}

fn const_ty(value: &Value) -> Ty {
    use minijinja::value::ValueKind;
    match value.kind() {
        ValueKind::String => Ty::Text,
        ValueKind::Number => Ty::Number,
        ValueKind::Bool => Ty::Bool,
        _ => Ty::Any,
    }
}

fn common(tys: Vec<Ty>) -> Ty {
    let mut it = tys.into_iter();
    let Some(first) = it.next() else {
        return Ty::Any;
    };
    if it.all(|t| t == first) {
        first
    } else {
        Ty::Any
    }
}

/// The names a template reads that it doesn't bind itself, in the order
/// first read, leaving out the callables in `callables` where it only
/// calls them.
pub(super) fn free_names(stmt: &Stmt<'_>, callables: &BTreeSet<String>) -> Vec<String> {
    let mut walk = Free {
        scopes: vec![BTreeSet::new()],
        callables,
        found: Vec::new(),
    };
    walk.stmt(stmt);
    walk.found
}

struct Free<'c> {
    scopes: Vec<BTreeSet<String>>,
    callables: &'c BTreeSet<String>,
    found: Vec<String>,
}

impl Free<'_> {
    fn bound(&self, name: &str) -> bool {
        self.scopes.iter().any(|s| s.contains(name))
    }

    fn read(&mut self, name: &str) {
        if !self.bound(name) && !self.found.iter().any(|f| f == name) {
            self.found.push(name.to_owned());
        }
    }

    fn bind(&mut self, target: &Expr<'_>) {
        match target {
            Expr::Var(var) => {
                self.scopes
                    .last_mut()
                    .expect("a scope")
                    .insert(var.id.to_owned());
            }
            Expr::List(list) => {
                for item in &list.items {
                    self.bind(item);
                }
            }
            other => self.expr(other),
        }
    }

    fn body(&mut self, stmts: &[Stmt<'_>]) {
        for stmt in stmts {
            self.stmt(stmt);
        }
    }

    fn scoped(&mut self, f: impl FnOnce(&mut Self)) {
        self.scopes.push(BTreeSet::new());
        f(self);
        self.scopes.pop();
    }

    fn stmt(&mut self, stmt: &Stmt<'_>) {
        match stmt {
            Stmt::Template(t) => self.body(&t.children),
            Stmt::EmitExpr(e) => self.expr(&e.expr),
            Stmt::EmitRaw(_) | Stmt::Continue(_) | Stmt::Break(_) => {}
            Stmt::ForLoop(f) => {
                self.expr(&f.iter);
                self.scoped(|walk| {
                    walk.bind(&f.target);
                    walk.scopes
                        .last_mut()
                        .expect("a scope")
                        .insert("loop".into());
                    if let Some(filter) = &f.filter_expr {
                        walk.expr(filter);
                    }
                    walk.body(&f.body);
                });
                self.body(&f.else_body);
            }
            Stmt::IfCond(i) => {
                self.expr(&i.expr);
                self.body(&i.true_body);
                self.body(&i.false_body);
            }
            Stmt::WithBlock(w) => self.scoped(|walk| {
                for (target, value) in &w.assignments {
                    walk.expr(value);
                    walk.bind(target);
                }
                walk.body(&w.body);
            }),
            Stmt::Set(s) => {
                self.expr(&s.expr);
                self.bind(&s.target);
            }
            Stmt::SetBlock(s) => {
                if let Some(filter) = &s.filter {
                    self.expr(filter);
                }
                self.body(&s.body);
                self.bind(&s.target);
            }
            Stmt::AutoEscape(a) => {
                self.expr(&a.enabled);
                self.body(&a.body);
            }
            Stmt::FilterBlock(f) => {
                self.expr(&f.filter);
                self.body(&f.body);
            }
            Stmt::Block(b) => self.body(&b.body),
            Stmt::Extends(e) => self.expr(&e.name),
            Stmt::Include(i) => self.expr(&i.name),
            Stmt::Import(i) => {
                self.expr(&i.expr);
                self.bind(&i.name);
            }
            Stmt::FromImport(i) => {
                self.expr(&i.expr);
                for (name, alias) in &i.names {
                    self.bind(alias.as_ref().unwrap_or(name));
                }
            }
            Stmt::Macro(m) => {
                self.scopes
                    .last_mut()
                    .expect("a scope")
                    .insert(m.name.to_owned());
                self.macro_body(m);
            }
            Stmt::CallBlock(c) => {
                self.call(&c.call);
                self.macro_body(&c.macro_decl);
            }
            Stmt::Do(d) => self.call(&d.call),
        }
    }

    fn macro_body(&mut self, m: &ast::Macro<'_>) {
        for default in &m.defaults {
            self.expr(default);
        }
        self.scoped(|walk| {
            for arg in &m.args {
                walk.bind(arg);
            }
            for name in ["caller", "varargs", "kwargs"] {
                walk.scopes.last_mut().expect("a scope").insert(name.into());
            }
            walk.body(&m.body);
        });
    }

    fn call(&mut self, call: &ast::Call<'_>) {
        match &call.expr {
            Expr::Var(var) if self.callables.contains(var.id) => {}
            other => self.expr(other),
        }
        self.args(&call.args);
    }

    fn args(&mut self, args: &[CallArg<'_>]) {
        for arg in args {
            match arg {
                CallArg::Pos(e)
                | CallArg::Kwarg(_, e)
                | CallArg::PosSplat(e)
                | CallArg::KwargSplat(e) => self.expr(e),
            }
        }
    }

    fn expr(&mut self, expr: &Expr<'_>) {
        match expr {
            Expr::Var(var) => self.read(var.id),
            Expr::Const(_) => {}
            Expr::Slice(s) => {
                self.expr(&s.expr);
                for part in [&s.start, &s.stop, &s.step].into_iter().flatten() {
                    self.expr(part);
                }
            }
            Expr::UnaryOp(op) => self.expr(&op.expr),
            Expr::BinOp(op) => {
                self.expr(&op.left);
                self.expr(&op.right);
            }
            Expr::Compare(c) => {
                self.expr(&c.expr);
                for op in &c.ops {
                    self.expr(&op.expr);
                }
            }
            Expr::IfExpr(i) => {
                self.expr(&i.test_expr);
                self.expr(&i.true_expr);
                if let Some(f) = &i.false_expr {
                    self.expr(f);
                }
            }
            Expr::Filter(f) => {
                if let Some(e) = &f.expr {
                    self.expr(e);
                }
                self.args(&f.args);
            }
            Expr::Test(t) => {
                self.expr(&t.expr);
                self.args(&t.args);
            }
            Expr::GetAttr(a) => self.expr(&a.expr),
            Expr::GetItem(i) => {
                self.expr(&i.expr);
                self.expr(&i.subscript_expr);
            }
            Expr::Call(c) => self.call(c),
            Expr::List(l) => {
                for item in &l.items {
                    self.expr(item);
                }
            }
            Expr::Map(m) => {
                for (k, v) in m.keys.iter().zip(&m.values) {
                    self.expr(k);
                    self.expr(v);
                }
            }
        }
    }
}

/// The span of an expression in `text` from its first character. The
/// parser starts the span of each lookup or call after the first in a chain
/// at its own `.`, `[` or `(`, so `page.book.author` is spanned
/// `.book.author`, and a filter or test is spanned from its name. It starts
/// a comparison, an `if ... else` value and an `a not in b` at the token
/// before them, and a value in brackets inside them.
pub(super) fn full_span(expr: &Expr<'_>, text: &str) -> Span {
    let end = expr.span();
    if let Some(first) = spanned_from_before(expr, text) {
        let start = full_span(first, text);
        return opened(
            text,
            Span {
                end_line: end.end_line,
                end_col: end.end_col,
                end_offset: end.end_offset,
                ..start
            },
        );
    }
    let inner = match expr {
        Expr::GetAttr(a) => Some(&a.expr),
        Expr::GetItem(i) => Some(&i.expr),
        Expr::Slice(s) => Some(&s.expr),
        Expr::Call(c) => Some(&c.expr),
        Expr::Filter(f) => f.expr.as_ref(),
        Expr::Test(t) => Some(&t.expr),
        _ => None,
    };
    match inner {
        Some(inner) => {
            let start = full_span(inner, text);
            if start.start_offset < end.start_offset {
                opened(
                    text,
                    Span {
                        start_line: start.start_line,
                        start_col: start.start_col,
                        start_offset: start.start_offset,
                        ..end
                    },
                )
            } else {
                end
            }
        }
        None => end,
    }
}

/// The first value of an expression the parser spans from the token
/// before it.
fn spanned_from_before<'e, 's>(expr: &'e Expr<'s>, text: &str) -> Option<&'e Expr<'s>> {
    match expr {
        Expr::IfExpr(i) => Some(&i.true_expr),
        Expr::Compare(c) => Some(&c.expr),
        Expr::BinOp(op) if compares(op.op) => Some(&op.left),
        // `a not in b` is read as `not` around `a in b`, both spanned alike.
        Expr::UnaryOp(op) => match &op.expr {
            Expr::BinOp(within) if matches!(within.op, BinOpKind::In) => {
                let left = full_span(&within.left, text).end_offset as usize;
                let right = full_span(&within.right, text).start_offset as usize;
                text.get(left..right)?
                    .contains("not")
                    .then_some(&within.left)
            }
            _ => None,
        },
        _ => None,
    }
}

fn compares(op: BinOpKind) -> bool {
    matches!(
        op,
        BinOpKind::Eq
            | BinOpKind::Ne
            | BinOpKind::Lt
            | BinOpKind::Lte
            | BinOpKind::Gt
            | BinOpKind::Gte
            | BinOpKind::In
    )
}

/// `span`, taken back over the brackets opened before it that it closes:
/// the parser starts the span of a value in brackets inside them.
fn opened(text: &str, span: Span) -> Span {
    let from = span.start_offset as usize;
    let Some(written) = text.get(from..span.end_offset as usize) else {
        return span;
    };
    let mut unopened = 0usize;
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    for c in written.chars() {
        match (quote, c) {
            (Some(_), _) if escaped => escaped = false,
            (Some(_), '\\') => escaped = true,
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(') => depth += 1,
            (None, ')') if depth > 0 => depth -= 1,
            (None, ')') => unopened += 1,
            _ => {}
        }
    }
    let mut start = from;
    while unopened > 0 {
        let open = text[..start].trim_end().len();
        if !text[..open].ends_with('(') {
            break;
        }
        start = open - 1;
        unopened -= 1;
    }
    let walked = &text[start..from];
    let lines = walked.matches('\n').count();
    let start_col = match lines {
        0 => span.start_col.saturating_sub(walked.len() as u16),
        _ => {
            let line_start = text[..start].rfind('\n').map_or(0, |at| at + 1);
            u16::try_from(text[line_start..start].chars().count()).unwrap_or(u16::MAX)
        }
    };
    Span {
        start_line: span
            .start_line
            .saturating_sub(u16::try_from(lines).unwrap_or(u16::MAX)),
        start_col,
        start_offset: start as u32,
        ..span
    }
}

pub(super) fn stmt_span(stmt: &Stmt<'_>) -> Span {
    match stmt {
        Stmt::Template(s) => s.span(),
        Stmt::EmitExpr(s) => s.span(),
        Stmt::EmitRaw(s) => s.span(),
        Stmt::ForLoop(s) => s.span(),
        Stmt::IfCond(s) => s.span(),
        Stmt::WithBlock(s) => s.span(),
        Stmt::Set(s) => s.span(),
        Stmt::SetBlock(s) => s.span(),
        Stmt::AutoEscape(s) => s.span(),
        Stmt::FilterBlock(s) => s.span(),
        Stmt::Block(s) => s.span(),
        Stmt::Import(s) => s.span(),
        Stmt::FromImport(s) => s.span(),
        Stmt::Extends(s) => s.span(),
        Stmt::Include(s) => s.span(),
        Stmt::Macro(s) => s.span(),
        Stmt::CallBlock(s) => s.span(),
        Stmt::Continue(s) => s.span(),
        Stmt::Break(s) => s.span(),
        Stmt::Do(s) => s.span(),
    }
}

/// Calls `f` on every expression in a template, outermost first.
pub(super) fn visit_exprs<'s>(stmt: &Stmt<'s>, f: &mut dyn FnMut(&Expr<'s>)) {
    fn args<'s>(args: &[CallArg<'s>], f: &mut dyn FnMut(&Expr<'s>)) {
        for arg in args {
            match arg {
                CallArg::Pos(e)
                | CallArg::Kwarg(_, e)
                | CallArg::PosSplat(e)
                | CallArg::KwargSplat(e) => expr(e, f),
            }
        }
    }
    fn expr<'s>(e: &Expr<'s>, f: &mut dyn FnMut(&Expr<'s>)) {
        f(e);
        match e {
            Expr::Var(_) | Expr::Const(_) => {}
            Expr::Slice(s) => {
                expr(&s.expr, f);
                for p in [&s.start, &s.stop, &s.step].into_iter().flatten() {
                    expr(p, f);
                }
            }
            Expr::UnaryOp(o) => expr(&o.expr, f),
            Expr::BinOp(o) => {
                expr(&o.left, f);
                expr(&o.right, f);
            }
            Expr::Compare(c) => {
                expr(&c.expr, f);
                for op in &c.ops {
                    expr(&op.expr, f);
                }
            }
            Expr::IfExpr(i) => {
                expr(&i.test_expr, f);
                expr(&i.true_expr, f);
                if let Some(e) = &i.false_expr {
                    expr(e, f);
                }
            }
            Expr::Filter(x) => {
                if let Some(e) = &x.expr {
                    expr(e, f);
                }
                args(&x.args, f);
            }
            Expr::Test(t) => {
                expr(&t.expr, f);
                args(&t.args, f);
            }
            Expr::GetAttr(a) => expr(&a.expr, f),
            Expr::GetItem(i) => {
                expr(&i.expr, f);
                expr(&i.subscript_expr, f);
            }
            Expr::Call(c) => {
                expr(&c.expr, f);
                args(&c.args, f);
            }
            Expr::List(l) => {
                for i in &l.items {
                    expr(i, f);
                }
            }
            Expr::Map(m) => {
                for (k, v) in m.keys.iter().zip(&m.values) {
                    expr(k, f);
                    expr(v, f);
                }
            }
        }
    }
    fn body<'s>(stmts: &[Stmt<'s>], f: &mut dyn FnMut(&Expr<'s>)) {
        for s in stmts {
            visit_exprs(s, f);
        }
    }
    match stmt {
        Stmt::Template(t) => body(&t.children, f),
        Stmt::EmitExpr(e) => expr(&e.expr, f),
        Stmt::EmitRaw(_) | Stmt::Continue(_) | Stmt::Break(_) => {}
        Stmt::ForLoop(l) => {
            expr(&l.target, f);
            expr(&l.iter, f);
            if let Some(e) = &l.filter_expr {
                expr(e, f);
            }
            body(&l.body, f);
            body(&l.else_body, f);
        }
        Stmt::IfCond(i) => {
            expr(&i.expr, f);
            body(&i.true_body, f);
            body(&i.false_body, f);
        }
        Stmt::WithBlock(w) => {
            for (t, v) in &w.assignments {
                expr(t, f);
                expr(v, f);
            }
            body(&w.body, f);
        }
        Stmt::Set(s) => {
            expr(&s.target, f);
            expr(&s.expr, f);
        }
        Stmt::SetBlock(s) => {
            expr(&s.target, f);
            if let Some(e) = &s.filter {
                expr(e, f);
            }
            body(&s.body, f);
        }
        Stmt::AutoEscape(a) => {
            expr(&a.enabled, f);
            body(&a.body, f);
        }
        Stmt::FilterBlock(b) => {
            expr(&b.filter, f);
            body(&b.body, f);
        }
        Stmt::Block(b) => body(&b.body, f),
        Stmt::Import(i) => {
            expr(&i.expr, f);
            expr(&i.name, f);
        }
        Stmt::FromImport(i) => expr(&i.expr, f),
        Stmt::Extends(e) => expr(&e.name, f),
        Stmt::Include(i) => expr(&i.name, f),
        Stmt::Macro(m) => {
            for d in &m.defaults {
                expr(d, f);
            }
            body(&m.body, f);
        }
        Stmt::CallBlock(c) => {
            expr(&c.call.expr, f);
            args(&c.call.args, f);
            body(&c.macro_decl.body, f);
        }
        Stmt::Do(d) => {
            expr(&d.call.expr, f);
            args(&d.call.args, f);
        }
    }
}
