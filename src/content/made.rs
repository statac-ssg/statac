//! Pages made by a page's `each:`. With `each: data.<table>` it makes one
//! page per item of the table: the item's fields are its page's front
//! matter, the page's `permalink` is the pattern their addresses are filled
//! in from, and its `layout` builds them. Its own writing is the page above
//! them, as a folder's `index.md` is.
//!
//! A table is a list of items, or items by name; an item is `name: value`
//! lines. An item by name is `page.key` on its page and `{key}` in the
//! pattern.
//!
//! With `each: site.posts`, or a line choosing among them as `posts:` does,
//! it makes one page per post once lists are filled, the post's pieces
//! filling the pattern. The page made is `page.post`'s, not a post itself:
//! it has no date, and is listed only by the page that made it.

use std::collections::HashMap;

use super::{Reader, slug, writes_other_than_html};
use crate::address::{self, Address, Pieces, Top};
use crate::config::words::{self, Holder, PageWord, Words};
use crate::data::{Data, Reached};
use crate::ids::{DataId, PageId};
use crate::origin::{Arrival, Sourced};
use crate::problem::{AddressFault, Expected, Found, Kind, Problem, Requirer, Word, WrongPlace};
use crate::site::{Item, List, ListKey, Made, Page, PagePlace, Site};
use crate::source::{Place, Sources, Span};
use crate::value::{self, Value};
use crate::yaml::{Node, Resolved, Value as Yaml};

/// Fields' names and where each is written, flat or nested, for finding
/// one written both ways.
type Shapes = Vec<(String, Place)>;

/// One item of a table, before its page is made.
struct Entry<'a> {
    item: Item,
    /// Its name, where the table holds items by name.
    key: Option<Sourced<String>>,
    node: &'a Node,
}

/// What a page making pages asks for, read.
enum Asking<'a> {
    Table(Asked<'a>),
    Posts {
        chain: Sourced<String>,
        pattern: Sourced<String>,
    },
}

/// A page making a page per post, waiting for lists to be filled.
pub(super) struct ForPosts {
    by: PageId,
    /// The `each:` line.
    chain: Sourced<String>,
    pattern: Sourced<String>,
    /// What each page made is handed: the page's own `layout`, then what
    /// it and the folders around it hand down.
    words: Words,
}

/// What a table's page making pages asks for, read.
struct Asked<'a> {
    by: PageId,
    doc: usize,
    file: DataId,
    /// The table's name, as `each:` writes it.
    table: String,
    node: &'a Node,
    pattern: Sourced<String>,
}

/// Why a piece of a pattern couldn't be filled.
enum Unfilled {
    /// The item or post has no `field` for `{token}`.
    Lacks { field: String, token: String },
    /// The field is there and the problem with it is told.
    Told,
}

impl Reader<'_> {
    /// Makes the pages each page with `each:` asks for, listing them under
    /// the page that asked.
    pub(super) fn make_pages(
        &mut self,
        sources: &Sources,
        site: &mut Site,
        pages_of: &HashMap<usize, PageId>,
        shapes: (&mut Shapes, &mut Shapes),
        problems: &mut Vec<Problem>,
    ) {
        let mut declaring: Vec<(usize, PageId)> = pages_of
            .iter()
            .filter(|(_, id)| site.pages.get(**id).makes_pages())
            .map(|(doc, id)| (*doc, *id))
            .collect();
        declaring.sort_by_key(|(_, id)| *id);
        // The tables are read while pages are added to the site.
        let data = std::mem::take(&mut site.data);
        let (flat, nested) = shapes;
        for (doc, by) in declaring {
            match asked(sources, site, &data, doc, by, problems) {
                Some(Asking::Table(asked)) => {
                    self.make(sources, site, &data, &asked, (flat, nested), problems);
                }
                Some(Asking::Posts { chain, pattern }) => {
                    let run = site.lists.push(List {
                        page: Some(by),
                        ..List::new(ListKey::Declared(by), site.pages.get(by).language)
                    });
                    site.pages.get_mut(by).speaks_for = Some(run);
                    let mut words = Words::default();
                    words.hand_down_from(&self.handed(doc));
                    self.for_posts.push(ForPosts {
                        by,
                        chain,
                        pattern,
                        words,
                    });
                }
                None => {}
            }
        }
        site.data = data;
        problems.extend(same_ids(site));
    }

    fn make(
        &mut self,
        sources: &Sources,
        site: &mut Site,
        data: &Data,
        asked: &Asked,
        (flat, nested): (&mut Shapes, &mut Shapes),
        problems: &mut Vec<Problem>,
    ) {
        let file = data.file(asked.file);
        let Some(entries) = entries(sources, asked, problems) else {
            return;
        };
        let by = site.pages.get(asked.by);
        let (language, by_path, folder) = (by.language, by.path.clone(), by.folder);
        // The pages made are filed in the page's folder, and listed, once
        // lists are filled, in the list the page speaks for.
        let run = site.lists.push(List {
            page: Some(asked.by),
            ..List::new(ListKey::Declared(asked.by), language)
        });
        site.pages.get_mut(asked.by).speaks_for = Some(run);
        let folder_at = self.docs[asked.doc].folder;
        let handed = self.handed(asked.doc);
        let top = self.top_of(folder_at);
        let folder_name = slug::tidied_path(&self.folders[folder_at].rel[top.len()..]);
        let top = Top::new(top);
        let folder_name = folder_name
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned();
        let declarations = &self.settings.declarations;
        let (mut drafts, mut waiting) = (Vec::new(), Vec::new());
        let mut included = super::Included::default();
        for entry in entries {
            let read = words::read_front_matter(
                sources,
                Some(entry.node),
                Holder::Page,
                declarations,
                self.several_languages(),
            );
            problems.extend(read.problems);
            words::shapes(&read.own.fields, flat, nested);
            let mut words = read.own;
            if let Some(each) = words.get(PageWord::Each)
                && let Some(place) = each.origin.place
            {
                problems.push(Problem::at(
                    Kind::WrongPlace {
                        key: "each".into(),
                        why: WrongPlace::Item,
                    },
                    place,
                ));
            }
            words.words.retain(|(word, _)| *word != PageWord::Each);
            words.hand_down_from(&handed);
            let head = match &entry.key {
                Some(key) => key.origin.place.expect("a key is written"),
                None => first_line(sources, entry.node.place),
            };
            let named = entry.item.named(&file.path);
            // Held back as a written page is: a draft, and a post dated
            // after today, unless the run includes them.
            if let Some(draft) = words.get(PageWord::Draft)
                && draft.value.as_bool() == Some(true)
            {
                if self.run.drafts {
                    included.drafts += 1;
                } else {
                    drafts.push(file.source);
                    if let Some(key) = words.text(PageWord::TranslationKey) {
                        let key = (language, key.value.to_owned());
                        self.keys_left_out.push(key);
                    }
                    let kind = Kind::DraftLeftOut {
                        path: named.clone(),
                    };
                    problems.push(Problem::at(kind, draft.origin.place.unwrap_or(head)));
                    continue;
                }
            }
            for (field, of_type) in
                declarations.required(words.text(PageWord::Type).map(|t| t.value))
            {
                if !words.has(field) {
                    problems.push(Problem::at(
                        Kind::MissingField {
                            field: field.to_owned(),
                            required_by: of_type
                                .map_or(Requirer::EveryPage, |t| Requirer::Type(t.to_owned())),
                        },
                        head,
                    ));
                }
            }
            let title = words.get(PageWord::Title).and_then(|title| {
                title
                    .value
                    .as_text()
                    .map(|t| Sourced::new(t.to_owned(), title.origin.clone()))
            });
            let date = words.get(PageWord::Date).and_then(|d| {
                d.value
                    .as_date()
                    .map(|date| Sourced::new(*date, d.origin.clone()))
            });
            if let Some(date) = &date
                && self.dates.after_today(&date.value)
            {
                if self.run.drafts {
                    included.waiting += 1;
                } else {
                    waiting.push(file.source);
                    if let Some(key) = words.text(PageWord::TranslationKey) {
                        let key = (language, key.value.to_owned());
                        self.keys_left_out.push(key);
                    }
                    let kind = Kind::ScheduledPost {
                        path: named.clone(),
                        date: date.value.iso_day(),
                    };
                    problems.push(Problem::at(kind, date.origin.place.unwrap_or(head)));
                    continue;
                }
            }
            let key = entry.key.as_ref().map(|key| key.value.as_str());
            let slug = key
                .or(title.as_ref().map(|t| t.value.as_str()))
                .map(|name| piece(name, &named));
            let item = Filling {
                sources,
                words: &words,
                key,
                slug: slug.as_deref(),
                name: key,
                date: date.as_ref().map(|d| &d.value),
                folder: &folder_name,
                named: &named,
                post: false,
            };
            let Some(expanded) = item.address(&top, &asked.pattern, head, problems) else {
                continue;
            };
            let Some(address) = read_address(&expanded, &words, head, problems) else {
                continue;
            };
            let path = match &entry.item {
                Item::Key(key) => format!("{by_path}/{key}"),
                Item::At(at) => format!("{by_path}/{at}"),
                Item::Post { .. } => unreachable!("a table's entries are items"),
            };
            site.pages.push(Page {
                source: Some(file.source),
                language,
                path,
                place: PagePlace::Ordinary,
                folder,
                words,
                title,
                date,
                address,
                body: Span::new(0, 0),
                speaks_for: None,
                made: Some(Made {
                    by: asked.by,
                    item: entry.item,
                    head,
                    fields: Some(entry.node.place),
                }),
            });
        }
        self.drafts.extend(drafts);
        self.waiting.extend(waiting);
        self.included.drafts += included.drafts;
        self.included.waiting += included.waiting;
    }

    /// What a page making pages hands each page it makes: its own `layout`
    /// first, then what it and the folders around it hand down.
    fn handed(&self, doc: usize) -> Words {
        let d = &self.docs[doc];
        let mut handed = Words::default();
        if let Some(layout) = d.words.own.get(PageWord::Layout) {
            handed.words.push((PageWord::Layout, layout.clone()));
        }
        if self.place(doc) != PagePlace::Folder {
            handed.hand_down_from(&d.words.layer);
        }
        for layer in self.layers(Some(d.folder)) {
            handed.hand_down_from(layer);
        }
        handed
    }
}

/// Makes a page for each post each page in `for_posts` chooses, once lists
/// are filled, listing them under the page that asked.
pub(super) fn make_for_posts(
    sources: &Sources,
    site: &mut Site,
    for_posts: Vec<ForPosts>,
    problems: &mut Vec<Problem>,
) {
    for asked in for_posts {
        let by = site.pages.get(asked.by);
        let chain = Sourced::new(asked.chain.value.as_str(), asked.chain.origin.clone());
        let posts = match crate::lists::chosen(site, sources, by, &chain) {
            Ok(posts) => posts,
            Err(problem) => {
                problems.push(problem);
                continue;
            }
        };
        let Some(head) = chain.origin.place.or_else(|| by.head(sources)) else {
            continue;
        };
        let (source, language, folder) = (by.source, by.language, by.folder);
        let by_path = by.path.clone();
        let top = match &site.lists.get(folder).key {
            ListKey::Folder(path) => Top::new(site.top_above(path)),
            _ => Top::new(""),
        };
        let mut made = Vec::with_capacity(posts.len());
        for post in posts {
            let p = site.pages.get(post);
            let pieces = PostPieces::of(sources, site, post);
            let named = p.file_named(sources).expect("a post is read from a file");
            let key = match &p.made {
                Some(Made {
                    item: Item::Key(key),
                    ..
                }) => Some(key.as_str()),
                _ => None,
            };
            let filling = Filling {
                sources,
                words: &p.words,
                key,
                slug: Some(&pieces.slug),
                name: Some(&pieces.name),
                date: p.date.as_ref().map(|d| &d.value),
                folder: &pieces.folder,
                named: &named,
                post: true,
            };
            let Some(expanded) = filling.address(&top, &asked.pattern, head, problems) else {
                continue;
            };
            let Some(address) = read_address(&expanded, &asked.words, head, problems) else {
                continue;
            };
            made.push(Page {
                source,
                language,
                path: format!("{by_path}/{}", p.path),
                place: PagePlace::Ordinary,
                folder,
                words: asked.words.clone(),
                title: None,
                date: None,
                address,
                body: Span::new(0, 0),
                speaks_for: None,
                made: Some(Made {
                    by: asked.by,
                    item: Item::Post { page: post, named },
                    head,
                    fields: None,
                }),
            });
        }
        let run = site.pages.get(asked.by).speaks_for;
        for page in made {
            let id = site.pages.push(page);
            if let Some(run) = run {
                site.lists.get_mut(run).undated.push(id);
            }
        }
    }
}

/// What a post fills a pattern with, as its own `permalink` would be
/// filled: its name in an address, its name as written, and the last
/// piece of its folder's address.
struct PostPieces {
    slug: String,
    name: String,
    folder: String,
}

impl PostPieces {
    fn of(sources: &Sources, site: &Site, post: PageId) -> PostPieces {
        let page = site.pages.get(post);
        let folder = match &site.lists.get(page.folder).key {
            ListKey::Folder(path) if path != site.top_above(path) => {
                let tidied = slug::tidied_path(&format!("{path}/"));
                let tidied = tidied.trim_end_matches('/');
                tidied.rsplit('/').next().unwrap_or(tidied).to_owned()
            }
            _ => String::new(),
        };
        // A post made from an item of a table is named as its item is.
        if let Some(made) = &page.made {
            let named = page
                .file_named(sources)
                .expect("a post is read from a file");
            let key = match &made.item {
                Item::Key(key) => Some(key.as_str()),
                _ => None,
            };
            let slug = key
                .or(page.title.as_ref().map(|t| t.value.as_str()))
                .map(|name| piece(name, &named))
                .unwrap_or_default();
            let name = key.map_or_else(|| slug.clone(), str::to_owned);
            return PostPieces { slug, name, folder };
        }
        let file = crate::lists::file_name(page);
        let name = match page.place {
            PagePlace::Ordinary => file.rsplit_once('.').map_or(file, |(stem, _)| stem),
            _ => file,
        };
        let numbered = site.lists.get(page.folder).numbered;
        let path = page
            .source
            .map(|source| sources.path(source).to_string_lossy())
            .expect("a post is read from a file");
        PostPieces {
            slug: super::name_in_address(name, numbered, &path),
            name: name.to_owned(),
            folder,
        }
    }
}

/// The address `expanded` names, read as a file where the layout writes
/// something other than HTML; none where it can't be, with why said.
fn read_address(
    expanded: &str,
    words: &Words,
    head: Place,
    problems: &mut Vec<Problem>,
) -> Option<Address> {
    let last = expanded.rsplit('/').next().unwrap_or_default();
    let read = if writes_other_than_html(words) && !expanded.ends_with('/') && !last.contains('.') {
        Address::read_file(expanded)
    } else {
        Address::read(expanded)
    };
    match read {
        Ok(address) => Some(address),
        Err((fault, _)) => {
            problems.push(Problem::at(Kind::UnusableUrl { fault }, head));
            None
        }
    }
}

/// What the page `by` asks to make pages of, or none where it can't, with
/// what is wrong said.
fn asked<'a>(
    sources: &Sources,
    site: &Site,
    data: &'a Data,
    doc: usize,
    by: PageId,
    problems: &mut Vec<Problem>,
) -> Option<Asking<'a>> {
    let page = site.pages.get(by);
    let each = page.words.text(PageWord::Each)?;
    let place = each.origin.place.or_else(|| page.head(sources))?;
    let written = each.value.trim();
    if let Some(posts) = page.words.text(PageWord::Posts)
        && posts.origin.arrival == Arrival::Written
        && let Some(posts_at) = posts.origin.place
    {
        let ((first, first_at), (second, at)) = match posts_at.span.start < place.span.start {
            true => (("posts", posts_at), ("each", place)),
            false => (("each", place), ("posts", posts_at)),
        };
        let kind = Kind::TwoLists {
            first: first.into(),
            second: second.into(),
            first_at,
        };
        problems.push(Problem::at(kind, at));
        return None;
    }
    // A chain is told what is wrong with it once lists are filled.
    if !written.starts_with("data.") && (written.starts_with("site.") || written.contains('|')) {
        let pattern = pattern(page, place, Requirer::EachPost, problems)?;
        let chain = Sourced::new(each.value.to_owned(), each.origin.clone());
        return Some(Asking::Posts { chain, pattern });
    }
    let not_a_table = || {
        Problem::at(
            Kind::WrongKind {
                field: "each".into(),
                expected: Expected::TableName,
                found: Found::Written(written.to_owned()),
                respelt: None,
            },
            place,
        )
    };
    let Some(rest) = written.strip_prefix("data.") else {
        problems.push(not_a_table());
        return None;
    };
    let names: Vec<&str> = rest.split('.').collect();
    let (file, node) = match data.reach(&names) {
        Some(Reached::Value { file, node }) => (file, node),
        // Told already, where the file was read.
        Some(Reached::Unread) => return None,
        Some(Reached::Folder(_)) => {
            problems.push(not_a_table());
            return None;
        }
        None => {
            let known = data.names();
            let nearest = crate::suggest::nearest(rest, known.iter().map(String::as_str));
            problems.push(Problem::at(
                Kind::UnknownWord {
                    word: Word::Table,
                    written: written.to_owned(),
                    nearest: nearest.map(|name| format!("data.{name}")),
                },
                place,
            ));
            return None;
        }
    };
    let pattern = pattern(page, place, Requirer::Each, problems)?;
    Some(Asking::Table(Asked {
        by,
        doc,
        file,
        table: written.to_owned(),
        node,
        pattern,
    }))
}

/// The `permalink` pattern a page making pages writes, or none where it
/// writes none it can use, with what is wrong said at `place`.
fn pattern(
    page: &Page,
    place: Place,
    required_by: Requirer,
    problems: &mut Vec<Problem>,
) -> Option<Sourced<String>> {
    match page.words.get(PageWord::Permalink) {
        Some(Sourced {
            value: Value::Text(pattern),
            origin,
        }) if origin.arrival == Arrival::Written => {
            Some(Sourced::new(pattern.clone(), origin.clone()))
        }
        // A pattern that can't be read is told where it is written.
        Some(Sourced {
            value: Value::Refused,
            ..
        }) => None,
        _ => {
            problems.push(Problem::at(
                Kind::MissingField {
                    field: "permalink".into(),
                    required_by,
                },
                place,
            ));
            None
        }
    }
}

/// The items of the table `asked` names, or none where it holds no items,
/// with what is wrong said.
fn entries<'a>(
    sources: &Sources,
    asked: &Asked<'a>,
    problems: &mut Vec<Problem>,
) -> Option<Vec<Entry<'a>>> {
    let reader = value::Reader::new(sources, &asked.table);
    let mut entries = Vec::new();
    match &asked.node.value {
        Yaml::Sequence(items) => {
            for (at, node) in items.iter().enumerate() {
                entries.push(Entry {
                    item: Item::At(at + 1),
                    key: None,
                    node,
                });
            }
        }
        Yaml::Mapping(items) => {
            for item in items {
                match reader.text(&item.key) {
                    Ok(Some(key)) => entries.push(Entry {
                        item: Item::Key(key.value.clone()),
                        key: Some(key),
                        node: &item.value,
                    }),
                    Ok(None) => problems.push(reader.wrong(&item.key, Expected::Text, None)),
                    Err(problem) => problems.push(problem),
                }
            }
        }
        Yaml::Scalar(_) if asked.node.resolved() == Some(Resolved::Null) => {}
        Yaml::Scalar(_) => {
            problems.push(reader.wrong(asked.node, Expected::Table, None));
            return None;
        }
    }
    // An item written empty is an item with no fields.
    entries.retain(|entry| {
        let fields = matches!(entry.node.value, Yaml::Mapping(_))
            || entry.node.resolved() == Some(Resolved::Null);
        if !fields {
            problems.push(reader.wrong(entry.node, Expected::Table, None));
        }
        fields
    });
    Some(entries)
}

/// What one item fills its pattern with.
struct Filling<'a> {
    sources: &'a Sources,
    words: &'a Words,
    key: Option<&'a str>,
    /// Its name in an address: an item's key's or title's, or a post's own.
    slug: Option<&'a str>,
    /// Its name as written: an item's key, or a post's file's name.
    name: Option<&'a str>,
    date: Option<&'a crate::date::Date>,
    /// The last piece of the address of the folder its page is in.
    folder: &'a str,
    /// The item as a problem names it.
    named: &'a str,
    /// Whether it is a post, whose own `permalink` is its own address.
    post: bool,
}

impl Filling<'_> {
    /// The address of the page made for it, filled in from an item's own
    /// `permalink`, or else the pattern it was handed, read from `top`;
    /// none where a piece couldn't be filled, with what is wrong said.
    fn address(
        &self,
        top: &Top,
        pattern: &Sourced<String>,
        head: Place,
        problems: &mut Vec<Problem>,
    ) -> Option<String> {
        let own = self
            .words
            .get(PageWord::Permalink)
            .filter(|p| !self.post && p.origin.arrival == Arrival::Written);
        if let Some(own) = own {
            let Value::Text(written) = &own.value else {
                return None;
            };
            let pieces = Pieces {
                date: self.date,
                slug: self.slug.unwrap_or_default(),
                folder: self.folder,
                name: self.name.unwrap_or(self.slug.unwrap_or_default()),
            };
            let pattern = (written.as_str(), &own.origin);
            let written = top.pattern(written, own.origin.place, problems);
            let filling = (self.named, Whose::Item, head);
            let written = own_fields(
                self.sources,
                self.words,
                written,
                pattern,
                filling,
                problems,
            )?;
            return match address::expand(&written, &pieces) {
                Ok(expanded) => Some(top.address(&expanded)),
                Err(fault) => {
                    let place = own.origin.place.unwrap_or(head);
                    problems.push(Problem::at(Kind::UnusableUrl { fault }, place));
                    None
                }
            };
        }
        let written = top.pattern(&pattern.value, pattern.origin.place, problems);
        let filled = address::filled(written, |token| self.piece(token, problems));
        match filled {
            Ok(expanded) => Some(top.address(&expanded)),
            Err(unfilled) => {
                let whose = match self.post {
                    true => Whose::Post,
                    false => Whose::Item,
                };
                let pattern = (pattern.value.as_str(), &pattern.origin);
                told(
                    self.sources,
                    pattern,
                    unfilled,
                    (self.named, whose, head),
                    problems,
                );
                None
            }
        }
    }

    /// What the item fills `{token}` with: its own field of that name, or
    /// else a piece every page has.
    fn piece(&self, token: &str, problems: &mut Vec<Problem>) -> Result<String, Unfilled> {
        if let Some(own) = own_piece(self.sources, self.words, token, self.named, problems) {
            return own;
        }
        let lacks = |field: &str| Unfilled::Lacks {
            field: field.to_owned(),
            token: token.to_owned(),
        };
        match token {
            "key" => self
                .key
                .map(|key| piece(key, self.named))
                .ok_or(lacks(token)),
            "slug" => self.slug.map(str::to_owned).ok_or(lacks("title")),
            "name" => self.name.map(slug::nfc).ok_or(lacks("key")),
            "folder" => Ok(self.folder.to_owned()),
            "year" | "month" | "day" => {
                let date = self.date.ok_or(lacks("date"))?;
                Ok(match token {
                    "year" => format!("{:04}", date.year),
                    "month" => format!("{:02}", date.month),
                    _ => format!("{:02}", date.day),
                })
            }
            _ => Err(lacks(token)),
        }
    }
}

/// What a pattern is filled in for, as its problems name it.
#[derive(Copy, Clone)]
pub(super) enum Whose {
    Item,
    Post,
    /// A page of the author's, by its `permalink`.
    Page,
}

/// What a page's own `words` fill `{token}` with: one of Statac's words
/// or a field of the author's, by that name or nested at that path, as
/// `book.author`. None where they hold neither.
fn own_piece(
    sources: &Sources,
    words: &Words,
    token: &str,
    named: &str,
    problems: &mut Vec<Problem>,
) -> Option<Result<String, Unfilled>> {
    let lacks = || Unfilled::Lacks {
        field: token.to_owned(),
        token: token.to_owned(),
    };
    if let Some(word) = PageWord::named(token)
        && let Some(written) = words.get(word)
    {
        return Some(match &written.value {
            Value::Text(text) => Ok(piece(text, named)),
            Value::Whole(n) => Ok(n.to_string()),
            Value::Date(date) => Ok(date.iso_day()),
            Value::Bool(b) => Ok(b.to_string()),
            _ => Err(lacks()),
        });
    }
    let field = words.at(token)?;
    Some(match value::Reader::new(sources, token).text(field) {
        Ok(Some(text)) => Ok(piece(&text.value, named)),
        Ok(None) => Err(lacks()),
        Err(problem) => {
            // A declared field's value is already told where it is read.
            if !problems.contains(&problem) {
                problems.push(problem);
            }
            Err(Unfilled::Told)
        }
    })
}

/// `written` with each piece naming a field rather than one of
/// [`address::PIECES`] filled in from the page's own `words`, as an item's
/// fields fill its pattern, the rest left for [`address::expand`]. None
/// where a field can't fill its piece, with why said at the piece in
/// `pattern`, the text and origin `written` was read from.
pub(super) fn own_fields(
    sources: &Sources,
    words: &Words,
    written: &str,
    pattern: (&str, &crate::origin::Origin),
    filling: (&str, Whose, Place),
    problems: &mut Vec<Problem>,
) -> Option<String> {
    let (named, ..) = filling;
    let filled = address::filled(written, |token| {
        if address::PIECES.contains(&token) {
            return Ok(format!("{{{token}}}"));
        }
        own_piece(sources, words, token, named, problems).unwrap_or(Err(Unfilled::Lacks {
            field: token.to_owned(),
            token: token.to_owned(),
        }))
    });
    match filled {
        Ok(filled) => Some(filled),
        Err(unfilled) => {
            told(sources, pattern, unfilled, filling, problems);
            None
        }
    }
}

/// Says why a pattern couldn't be filled in for what `named` names: at
/// the piece it lacks a field for, where the pattern is written, or else
/// at `head`.
fn told(
    sources: &Sources,
    (pattern, origin): (&str, &crate::origin::Origin),
    unfilled: Unfilled,
    (named, whose, head): (&str, Whose, Place),
    problems: &mut Vec<Problem>,
) {
    let Unfilled::Lacks { field, token } = unfilled else {
        return;
    };
    let braced = format!("{{{token}}}");
    let place = pattern
        .find(&braced)
        .and_then(|at| {
            value::place_within(sources, origin, pattern, Span::new(at, at + braced.len()))
        })
        .unwrap_or(head);
    let named = named.to_owned();
    let piece = braced;
    let fault = match whose {
        Whose::Item => AddressFault::ItemLacks {
            item: named,
            field,
            piece,
        },
        Whose::Post => AddressFault::PostLacks {
            post: named,
            field,
            piece,
        },
        Whose::Page => AddressFault::PageLacks {
            page: named,
            field,
            piece,
        },
    };
    problems.push(Problem::at(Kind::UnusableUrl { fault }, place));
}

/// `text` as a name in an address, made from where it comes from where it
/// holds no letter or figure, as a page's name is.
fn piece(text: &str, named: &str) -> String {
    let slug = slug::slugify(text);
    match slug.is_empty() {
        true => slug::made_slug(&format!("{named}/{text}")),
        false => slug,
    }
}

/// The first line of what is written at `place`.
fn first_line(sources: &Sources, place: Place) -> Place {
    let text = sources.spelling(place);
    let end = text.find(['\n', '\r']).unwrap_or(text.len());
    Place {
        source: place.source,
        span: Span::new(place.span.start as usize, place.span.start as usize + end),
    }
}

#[cfg(test)]
mod tests;

/// A problem for each page giving an `id` another page gave first.
fn same_ids(site: &Site) -> Vec<Problem> {
    let mut first: HashMap<&str, Place> = HashMap::new();
    let mut problems = Vec::new();
    for (_, page) in site.pages.iter() {
        let Some(id) = page.words.get(PageWord::Id) else {
            continue;
        };
        let (Some(text), Some(place)) = (id.value.as_text(), id.origin.place) else {
            continue;
        };
        match first.get(text) {
            Some(&earlier) => problems.push(Problem::at(
                Kind::SameId {
                    id: text.to_owned(),
                    first: earlier,
                },
                place,
            )),
            None => {
                first.insert(text, place);
            }
        }
    }
    problems
}
