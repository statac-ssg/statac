//! Statac's own filters and functions. Each goes through the same guard as
//! the engine's, so a missing value handed to one is refused as surely.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::sync::Arc;

use minijinja::machinery::ast::CallType;
use minijinja::value::{Kwargs, Rest, Value, ValueOrKwargs};
use minijinja::{Environment, Error, State};

use super::frame;
use super::guard::{self, Callee, Input};
use super::render::Shared;
use super::values::{DateValue, Group, ListValue, Lists, Over, PageValue, Pages, Which};
use crate::content::words::PageWord;
use crate::date::Date;
use crate::dates::ShowDates;
use crate::highlight::{self, Language};
use crate::links::Attribute;
use crate::lists::Filter;
use crate::markdown::{Piece, PieceValue, closed_up, words_of};
use crate::problem::{Called, Kind, Problem, Relation, Stopped, Word};
use crate::reads::Key;
use crate::site::{Id, ListId, PageId};
use crate::source::{Place, Span};

/// Statac's own filters, by the names templates use. `sort` is the
/// engine's own, shadowed, so it is listed with the engine's.
pub(super) const FILTERS: &[&str] = &[
    "absolute",
    "date",
    "digits",
    "highlight",
    "in",
    "json",
    "markdown",
    "moment",
    "newest_date",
    "newest_first",
    "plain",
    "tagged",
    "truncate",
    "urlencode",
    "where",
];

/// Statac's own functions.
pub(super) const FUNCTIONS: &[&str] = &[
    "asset",
    "css",
    "feed_id",
    "headline",
    "js",
    "related",
    "url",
    "webmention",
];

/// `text | highlight("rust")`: the text as the inside of a `<code>`,
/// coloured as the language `word` names, as a fence's first word would. A
/// word that names none is told where the filter is written, and the text
/// is as it is.
fn highlighted(state: &State, value: &Value, word: &str) -> Value {
    let text = value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned);
    let Some(shared) = frame::shared() else {
        return Value::from_safe_string(highlight::plain(&text));
    };
    let at = || {
        super::explain::filter(&shared, state, "highlight", &text, Some(word))
            .or_else(|| frame::with(|frame| frame.calling).flatten())
    };
    if super::render::colouring_off(&shared) {
        return Value::from_safe_string(highlight::plain(&text));
    }
    let language = match highlight::first_word(word).language() {
        Language::Unknown { nearest } => {
            let kind = Kind::UnknownCodeLanguage {
                written: word.to_owned(),
                nearest: nearest.map(str::to_owned),
            };
            super::media::told_at(at(), kind);
            Language::Plain
        }
        named => named,
    };
    Value::from_safe_string(super::render::code_html(&shared, &text, language, at))
}

fn shared() -> Result<Arc<Shared>, Error> {
    frame::shared().ok_or_else(|| Error::new(minijinja::ErrorKind::InvalidOperation, "no site"))
}

fn checked(name: &str, args: &[Value]) -> Result<(), Error> {
    guard::check_args(name, Callee::Filter, args, Input::Whole)
}

/// Registers each filter and function with the guard in front of it.
pub(super) fn install(env: &mut Environment<'_>) {
    env.add_filter("absolute", |value: Value| {
        checked("absolute", std::slice::from_ref(&value))?;
        absolute(&value)
    });
    env.add_filter("markdown", |state: &State, value: Value| {
        checked("markdown", std::slice::from_ref(&value))?;
        Ok::<_, Error>(markdown(state, &value))
    });
    env.add_filter(
        "highlight",
        |state: &State, value: Value, language: String| {
            checked(
                "highlight",
                &[value.clone(), Value::from(language.as_str())],
            )?;
            Ok::<_, Error>(highlighted(state, &value, &language))
        },
    );
    env.add_filter("plain", |value: Value| {
        checked("plain", std::slice::from_ref(&value))?;
        Ok::<_, Error>(Value::from(plain(&value)))
    });
    env.add_filter(
        "truncate",
        |value: Value, longest: usize, ending: Option<String>| {
            checked("truncate", std::slice::from_ref(&value))?;
            Ok::<_, Error>(Value::from(truncate(
                &plain(&value),
                longest,
                ending.as_deref().unwrap_or_default(),
            )))
        },
    );
    env.add_filter("digits", |value: Value| {
        checked("digits", std::slice::from_ref(&value))?;
        let shared = shared()?;
        let words = super::render::words_of(&shared, frame::language());
        Ok::<_, Error>(Value::from(crate::dates::format::in_digits(
            &plain(&value),
            words.digits.as_ref(),
        )))
    });
    env.add_filter("json", |state: &State, value: Value| {
        checked("json", std::slice::from_ref(&value))?;
        // A template writing JSON writes every value so already.
        if super::guard::writes_json(state) {
            return Ok(value);
        }
        let mut out = String::new();
        json(&value, &mut out);
        Ok::<_, Error>(Value::from_safe_string(out))
    });
    env.add_filter("urlencode", |value: Value| {
        checked("urlencode", std::slice::from_ref(&value))?;
        Ok::<_, Error>(Value::from(urlencode(&value)?))
    });
    env.add_filter("newest_first", |posts: Value| {
        checked("newest_first", std::slice::from_ref(&posts))?;
        let shared = shared()?;
        let site = &shared.site;
        let newest_first = |pages: Vec<(PageId, usize)>| {
            let mut dated: Vec<_> = pages
                .into_iter()
                .map(|(id, number)| {
                    date_read(&shared, id);
                    let page = site.pages.get(id);
                    (
                        (id, number),
                        page,
                        crate::lists::moment(&shared.dates, page),
                    )
                })
                .collect();
            dated.sort_by(|a, b| {
                crate::lists::by_moment(crate::lists::Order::NewestFirst, (a.1, a.2), (b.1, b.2))
            });
            dated.into_iter().map(|(page, _, _)| page).collect()
        };
        if let Some(pages) = posts.downcast_object_ref::<Pages>()
            && let Some((mut asked, handed)) = pages.known_by()
        {
            let over = pages.over();
            over.read_field(shared.name_id("date"));
            asked.push_str(" | newest_first");
            return pages.once((asked, handed), over, || {
                Ok(newest_first(pages.each().collect()))
            });
        }
        let pages = numbered_pages_of(&posts, "newest_first")?;
        let sorted: Vec<(PageId, usize)> = newest_first(pages);
        Ok::<_, Error>(Value::from(
            sorted
                .into_iter()
                .map(|(id, number)| PageValue::numbered(&shared, id, number))
                .collect::<Vec<_>>(),
        ))
    });
    env.add_filter("newest_date", |posts: Value| {
        checked("newest_date", std::slice::from_ref(&posts))?;
        let pages = pages_of(&posts, "newest_date")?;
        let shared = shared()?;
        let dates: Vec<Date> = pages
            .iter()
            .filter_map(|(_, id)| date_read(&shared, *id))
            .collect();
        let dates: Vec<&Date> = dates.iter().collect();
        Ok::<_, Error>(match shared.dates.newest(&dates) {
            Some(date) => DateValue::value(&shared, *date),
            None => Value::UNDEFINED,
        })
    });
    env.add_filter("tagged", |posts: Value, name: String| {
        checked("tagged", &[posts.clone(), Value::from(name.as_str())])?;
        let shared = shared()?;
        let language = frame::language();
        frame::read(Key::ListsIn(language));
        let filter = Filter::tagged(&shared.site, language, &name).map_err(frame::raise)?;
        filtered(&shared, &posts, "tagged", &filter, &[&name])
    });
    env.add_filter("in", |posts: Value, name: String| {
        checked("in", &[posts.clone(), Value::from(name.as_str())])?;
        let shared = shared()?;
        let language = frame::language();
        frame::read(Key::ListsIn(language));
        let filter = Filter::in_folder(&shared.site, language, &name).map_err(frame::raise)?;
        filtered(&shared, &posts, "in", &filter, &[&name])
    });
    env.add_filter("where", |posts: Value, field: String, wanted: Value| {
        checked("where", &[posts.clone(), wanted.clone()])?;
        let shared = shared()?;
        frame::read(Key::Lists);
        let wanted = wanted
            .as_str()
            .map_or_else(|| wanted.to_string(), str::to_owned);
        let filter = Filter::where_field(&shared.site, &field, &wanted).map_err(frame::raise)?;
        filtered(&shared, &posts, "where", &filter, &[&field, &wanted])
    });
    env.add_filter("date", |value: Value, format: Option<String>| {
        checked("date", std::slice::from_ref(&value))?;
        let date = date_of(&value, "date")?;
        let shared = shared()?;
        super::values::written_date(&shared, &date, format.as_deref())
            .map(Value::from)
            .map_err(|refusal| frame::raise(refusal.into_kind()))
    });
    env.add_filter("moment", |value: Value, form: Option<String>| {
        checked("moment", std::slice::from_ref(&value))?;
        let date = date_of(&value, "moment")?;
        let shared = shared()?;
        frame::read(Key::Setting(shared.name_id("timezone")));
        match form.as_deref() {
            None => Ok(Value::from(shared.dates.moment(&date))),
            Some("rss") => Ok(Value::from(shared.dates.moment_rss(&date))),
            Some(other) => Err(frame::raise(Kind::UnknownWord {
                word: Word::Name,
                written: other.to_owned(),
                nearest: crate::suggest::nearest(other, ["rss"]).map(str::to_owned),
            })),
        }
    });
    env.add_filter("sort", sorted);

    env.add_function("css", |name: Value| {
        check_function("css", &name)?;
        let shared = shared()?;
        let name = name.to_string();
        // With no code coloured there is no stylesheet of colours to ask for.
        if name == highlight::STYLESHEET
            && shared.assets.id(&name).is_none()
            && super::render::colouring_off(&shared)
        {
            return Ok(Value::from_safe_string(String::new()));
        }
        let tag = frame::with(|f| shared.assets.css(&name, &mut f.reads))
            .unwrap_or_else(|| shared.assets.css(&name, &mut Default::default()));
        tag.map(Value::from_safe_string).map_err(frame::raise)
    });
    env.add_function("js", |name: Value| {
        check_function("js", &name)?;
        let shared = shared()?;
        let name = name.to_string();
        let tag = frame::with(|f| shared.assets.js(&name, &mut f.scripts, &mut f.reads))
            .unwrap_or_else(|| {
                shared
                    .assets
                    .js(&name, &mut Default::default(), &mut Default::default())
            });
        tag.map(Value::from_safe_string).map_err(frame::raise)
    });
    env.add_function("asset", |name: Value| {
        check_function("asset", &name)?;
        let shared = shared()?;
        let name = name.to_string();
        let address = frame::with(|f| shared.assets.asset(&name, &mut f.reads))
            .unwrap_or_else(|| shared.assets.asset(&name, &mut Default::default()));
        address.map(Value::from).map_err(frame::raise)
    });
    env.add_function("url", |address: Value| {
        check_function("url", &address)?;
        let shared = shared()?;
        frame::read(Key::Setting(shared.name_id("url")));
        match address.as_str() {
            Some(address) if crate::markdown::from_the_top(address) => {
                Ok(Value::from(format!("{}{address}", shared.base)))
            }
            _ => Err(frame::raise(Kind::TemplateStopped {
                reason: Stopped::NotAnAddress,
            })),
        }
    });
    env.add_function("headline", |page: Value| {
        check_function("headline", &page)?;
        let Some(page) = page.downcast_object_ref::<PageValue>() else {
            return Err(frame::raise(Kind::TemplateStopped {
                reason: Stopped::NotAPage("headline".into()),
            }));
        };
        headline(&page.shared, page.id).map(Value::from)
    });
    env.add_function("related", |page: Value| {
        check_function("related", &page)?;
        let Some(page) = page.downcast_object_ref::<PageValue>() else {
            return Err(frame::raise(Kind::TemplateStopped {
                reason: Stopped::NotAPage("related".into()),
            }));
        };
        Ok(related(&page.shared, page.id))
    });
    env.add_function("feed_id", |page: Value| {
        check_function("feed_id", &page)?;
        let Some(page) = page.downcast_object_ref::<PageValue>() else {
            return Err(frame::raise(Kind::TemplateStopped {
                reason: Stopped::NotAPage("feed_id".into()),
            }));
        };
        Ok(Value::from(feed_id(&page.shared, page.id)?))
    });
    env.add_function(
        "webmention",
        |state: &State, given: Rest<ValueOrKwargs>| -> Result<Value, Error> {
            if !given.is_empty() {
                return Err(frame::raise(Kind::WrongArgument {
                    snippet: "webmention".into(),
                    fault: crate::problem::ArgumentFault::NeedsNames { wanted: Vec::new() },
                }));
            }
            webmention(&shared()?, state)
        },
    );
}

/// The line naming the relay that receives the site's mentions, from the
/// host of its `url`. Nothing on a site without webmentions, which is told
/// at the call; nothing on one without `url`, which the site is told of.
fn webmention(shared: &Arc<Shared>, state: &State) -> Result<Value, Error> {
    for name in ["webmentions", "url"] {
        frame::read(Key::Setting(shared.name_id(name)));
    }
    if shared.settings.truth("webmentions") != Some(true) {
        let wanted = |call: &CallType| matches!(call, CallType::Function("webmention"));
        let running = (state.calling_span())
            .zip(shared.templates.by_name.get(state.name()))
            .and_then(|(span, &at)| super::media::call_at(shared, at, span, &wanted));
        let at = running.or_else(|| super::media::only_call(shared, state.name(), &wanted));
        super::media::told_at(at, Kind::WebmentionsOff);
        return Ok(Value::from_safe_string(String::new()));
    }
    // The relay knows the site by its own address, whatever address the
    // run was given.
    let host = (shared.settings.home())
        .and_then(|url| url::Url::parse(url).ok())
        .and_then(|url| url.host_str().map(str::to_owned));
    Ok(Value::from_safe_string(match host {
        Some(host) => format!(
            "<link rel=\"webmention\" href=\"https://webmention.io/{}/webmention\">",
            escape(&host)
        ),
        None => String::new(),
    }))
}

fn check_function(name: &str, value: &Value) -> Result<(), Error> {
    guard::check_args(
        name,
        Callee::Function,
        std::slice::from_ref(value),
        Input::Whole,
    )
}

/// The engine's `sort`, with one rule of Statac's: items that lack a field
/// sorted by go after the rest, in the order they came, whichever way the
/// rest are sorted, and a warning says so.
fn sorted(state: &State, value: Value, kwargs: Kwargs) -> Result<Value, Error> {
    checked("sort", std::slice::from_ref(&value))?;
    let Some(attribute) = kwargs.get::<Option<&str>>("attribute")?.map(str::to_owned) else {
        return minijinja::filters::sort(state, value, kwargs);
    };
    let keys: Vec<&str> = attribute
        .split(',')
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .collect();
    let lacked = |item: &Value| {
        keys.iter()
            .copied()
            .find(|k| guard::field_of(item, k).is_undefined())
    };
    // The items carrying every field in order, then those without, with
    // where each of those stood and the field it lacks.
    let in_order = |kwargs: Kwargs| {
        let mut carrying = Vec::new();
        let mut without: Vec<(usize, Value, &str)> = Vec::new();
        for (i, item) in value.try_iter()?.enumerate() {
            match lacked(&item) {
                Some(key) => without.push((i, item, key)),
                None => carrying.push(item),
            }
        }
        let run = minijinja::filters::sort(state, Value::from(carrying), kwargs)?;
        Ok::<_, Error>((run, without))
    };
    let field_of = |without: &[(usize, Value, &str)]| match without {
        [(_, _, key), rest @ ..] if rest.iter().all(|(_, _, k)| k == key) => (*key).to_owned(),
        _ => attribute.clone(),
    };
    let tell = |field: &str, count: usize, first: &Value, index: usize| {
        frame::with(|frame| {
            let problem = super::explain::sorted_last(
                frame,
                state.name(),
                state.calling_span(),
                &attribute,
                field,
                count,
                (first, index),
            );
            if !frame.problems.contains(&problem) {
                frame.problems.push(problem);
            }
        });
    };
    // A list that is the same whichever page asks, put in order by what
    // each page says of itself: worked out once, with what there is to
    // tell of the pages put last, which every page asking is told.
    if let Some(pages) = value.downcast_object_ref::<Pages>()
        && let Some((mut asked, handed)) = pages.known_by()
        && (keys.iter())
            .all(|key| super::values::of_the_front(key.split('.').next().unwrap_or(key)))
    {
        let shared = &pages.shared;
        let reverse = kwargs.get::<Option<bool>>("reverse")?;
        let case_sensitive = kwargs.get::<Option<bool>>("case_sensitive")?;
        asked.push_str(&format!(
            " | sort {attribute} {reverse:?} {case_sensitive:?}"
        ));
        let asked = (asked, handed);
        let over = pages.over();
        for key in &keys {
            let field = key.split('.').next().unwrap_or(key);
            over.read_field(shared.name_id(field));
        }
        let numbered = |item: &Value| {
            let page = item.downcast_object_ref::<PageValue>()?;
            Some((page.id, page.number))
        };
        let sorted = pages.once(asked.clone(), over, || {
            let (run, without) = in_order(kwargs)?;
            if let Some((index, first, _)) = without.first()
                && let Some(first) = numbered(first)
            {
                let last = super::render::PutLast {
                    field: field_of(&without),
                    count: without.len(),
                    first,
                    index: *index,
                };
                let mut all = shared.put_last.lock().expect("never poisoned");
                all.insert(asked.clone(), last);
            }
            let without = without.iter().map(|(_, item, _)| item.clone());
            Ok(run
                .try_iter()?
                .chain(without)
                .filter_map(|item| numbered(&item))
                .collect())
        })?;
        let last = shared
            .put_last
            .lock()
            .expect("never poisoned")
            .get(&asked)
            .cloned();
        if let Some(last) = last {
            let (id, number) = last.first;
            let first = PageValue::numbered(shared, id, number);
            tell(&last.field, last.count, &first, last.index);
        }
        return Ok(sorted);
    }
    let (run, without) = in_order(kwargs)?;
    let Some((index, first, _)) = without.first() else {
        return Ok(run);
    };
    tell(&field_of(&without), without.len(), first, *index);
    Ok(Value::from_iter(
        run.try_iter()?
            .chain(without.into_iter().map(|(_, item, _)| item)),
    ))
}

/// The pages in a list handed to a filter that works on pages.
fn pages_of(posts: &Value, filter: &str) -> Result<Vec<(Value, PageId)>, Error> {
    let not_a_list = || {
        frame::raise(Kind::TemplateStopped {
            reason: Stopped::NotAList(filter.to_owned()),
        })
    };
    let items = posts.try_iter().map_err(|_| not_a_list())?;
    items
        .map(|item| {
            let id = item
                .downcast_object_ref::<PageValue>()
                .map(|page| page.id)
                .ok_or_else(not_a_list)?;
            Ok((item, id))
        })
        .collect()
}

/// The pages in a list handed to a filter that works on pages, each with
/// which of its list's pages it is.
fn numbered_pages_of(posts: &Value, filter: &str) -> Result<Vec<(PageId, usize)>, Error> {
    let not_a_list = || {
        frame::raise(Kind::TemplateStopped {
            reason: Stopped::NotAList(filter.to_owned()),
        })
    };
    let items = posts.try_iter().map_err(|_| not_a_list())?;
    items
        .map(|item| {
            item.downcast_object_ref::<PageValue>()
                .map(|page| (page.id, page.number))
                .ok_or_else(not_a_list)
        })
        .collect()
}

/// The pages of `posts` that `filter` keeps, each recorded as read for what
/// the filter asks of it. Of a list that is the same whichever page asks,
/// they are worked out once for the site, in the language of the page
/// asking, and recorded as what the filter asks of every page.
fn filtered(
    shared: &Arc<Shared>,
    posts: &Value,
    name: &str,
    filter: &Filter,
    wanted: &[&str],
) -> Result<Value, Error> {
    if let Filter::In(list, _) = filter {
        super::values::read_whole(*list);
    }
    let field = filter.reads().map(|field| shared.name_id(field));
    if let Some(pages) = posts.downcast_object_ref::<Pages>()
        && let Some((mut asked, mut handed)) = pages.known_by()
    {
        if let Some(field) = field {
            frame::read(Key::FieldOfEvery(field));
        }
        let language = frame::language().index();
        asked.push_str(&format!(" | {name} in {language} {}", wanted.len()));
        handed.extend(wanted.iter().copied().map(Value::from));
        let over = match pages.over() {
            Over::List => Over::List,
            Over::Every | Over::Writing(..) => Over::Every,
        };
        return pages.once((asked, handed), over, || {
            let kept = pages
                .each()
                .filter(|(id, _)| filter.keeps(&shared.site, *id));
            Ok(kept.collect())
        });
    }
    let pages = pages_of(posts, name)?;
    Ok(Value::from(
        pages
            .into_iter()
            .filter(|(_, id)| {
                if let Some(field) = field {
                    frame::read(Key::Field(*id, field));
                }
                filter.keeps(&shared.site, *id)
            })
            .map(|(v, _)| v)
            .collect::<Vec<_>>(),
    ))
}

/// `related(page)`: the posts sharing a tag with the page, recorded as
/// reading its tags and every post each of them holds.
fn related(shared: &Arc<Shared>, page: PageId) -> Value {
    frame::read(Key::Field(page, shared.name_id("tags")));
    for tag in crate::lists::tags_of(&shared.site, page) {
        super::values::read_whole(tag);
    }
    let groups = crate::lists::related(&shared.site, page);
    Pages::value(
        shared,
        Which::Groups(groups.into_iter().map(Group::of).collect()),
    )
}

/// A page's date, recorded as read.
fn date_read(shared: &Shared, page: PageId) -> Option<Date> {
    frame::read(Key::Field(page, shared.name_id("date")));
    shared.site.pages.get(page).date.as_ref().map(|d| d.value)
}

fn date_of(value: &Value, filter: &str) -> Result<Date, Error> {
    let part = value
        .downcast_object_ref::<DateValue>()
        .map(|date| match date.part_printed() {
            Some(printed) => Err(printed),
            None => Ok(date.date),
        });
    // A part of a date reaches a filter as the text it is printed as.
    let printed = match part {
        Some(Ok(date)) => return Ok(date),
        Some(Err(printed)) => Some(printed),
        None => None,
    };
    printed
        .as_deref()
        .or_else(|| value.as_str())
        .and_then(|text| crate::date::read(text).ok())
        .ok_or_else(|| {
            frame::raise(Kind::TemplateStopped {
                reason: Stopped::NotADate(filter.to_owned()),
            })
        })
}

/// A page's id in a feed, recorded as read from what makes it.
fn feed_id(shared: &Arc<Shared>, id: PageId) -> Result<String, Error> {
    for name in [
        "feed_key",
        "url",
        "feed_ids_before",
        "feed_ids_from",
        "timezone",
    ] {
        frame::read(Key::Setting(shared.name_id(name)));
    }
    for name in ["id", "date", "url"] {
        frame::read(Key::Field(id, shared.name_id(name)));
    }
    // The feed a list's page speaks for is known by its address.
    frame::read(Key::Lists);
    crate::feeds::entry_id(&shared.site, &shared.settings, &shared.dates, id).ok_or_else(|| {
        frame::raise(Kind::TemplateStopped {
            reason: Stopped::NoFeedKey(crate::feeds::mint_key()),
        })
    })
}

/// What a feed or a tab calls a page, as text with no markup, the first
/// there is of: the `title` its front matter gives; for the site's home
/// page, the site's `title`; the heading its writing opens with, as a
/// reader reads it; the name of the list it speaks for; its `summary`, or
/// else its opening words, cut short; its date; its address. A list's later
/// pages are its page continued, and called the same. None of the last
/// four is a name the author wrote, so a page called by one is noted. A
/// page with `untitled: true` is called by its date before its `summary`
/// or opening words, and is never noted.
pub(super) fn headline(shared: &Arc<Shared>, id: PageId) -> Result<String, Error> {
    let (name, by) = called(shared, id)?;
    if let Some(by) = by
        && !meant_untitled(shared, id)
    {
        let kind = Kind::UntitledPage {
            page: shared.site.named(id, &shared.sources),
            by,
            name: name.clone(),
        };
        let problem = Problem::placed(kind, shared.site.head(id, &shared.sources));
        frame::with(|frame| {
            if !frame.problems.contains(&problem) {
                frame.problems.push(problem);
            }
        });
    }
    Ok(name)
}

/// A page's name, and what it is called by where that is no name the
/// author wrote.
fn called(shared: &Arc<Shared>, id: PageId) -> Result<(String, Option<Called>), Error> {
    let page = shared.site.pages.get(id);
    let field = |name: &str| frame::read(Key::Field(id, shared.name_id(name)));
    field("title");
    if page.words.get(PageWord::Title).is_some()
        && let Some(title) = &page.title
    {
        return Ok((title.value.clone(), None));
    }
    field("list");
    let fronts = page.speaks_for;
    let site_list = |list: ListId| shared.site.is_home(list);
    if fronts.is_some_and(site_list)
        && let Some(title) = super::values::setting_in(shared, page.language, "title")
    {
        return Ok((title.to_owned(), None));
    }
    // Without a `title` of its own, a page's title is the heading its
    // writing opens with, as written.
    if let Some(title) = &page.title {
        let heading = super::render::outline(shared, id)
            .first()
            .map(|heading| heading.text.clone())
            .filter(|text| !text.is_empty());
        return Ok((heading.unwrap_or_else(|| title.value.clone()), None));
    }
    if let Some(list) = fronts.filter(|&list| !site_list(list)) {
        return Ok((super::values::list_name(shared, list), None));
    }
    let dated = || {
        field("date");
        let date = page.date.as_ref()?;
        super::values::written_date(shared, &date.value, None).ok()
    };
    let date_first = meant_untitled(shared, id);
    if date_first && let Some(written) = dated() {
        return Ok((written, Some(Called::Date)));
    }
    field("summary");
    let opening = match page.words.text(PageWord::Summary) {
        Some(summary) => Some((closed_up(summary.value), Called::Summary)),
        None => super::render::opening(shared, id)?.map(|words| (words, Called::OpeningWords)),
    };
    if let Some((words, by)) = opening.filter(|(words, _)| !words.is_empty()) {
        return Ok((shortened(&words, HEADLINE), Some(by)));
    }
    if !date_first && let Some(written) = dated() {
        return Ok((written, Some(Called::Date)));
    }
    field("url");
    Ok((shared.address_of(id), Some(Called::Address)))
}

/// Whether a page says, itself or by its folder, that it goes without a
/// title, recorded as read.
fn meant_untitled(shared: &Shared, id: PageId) -> bool {
    frame::read(Key::Field(id, shared.name_id("untitled")));
    shared.site.pages.get(id).words.truth(PageWord::Untitled) == Some(true)
}

/// How many characters opening words may run to before they are cut.
const HEADLINE: usize = 70;

/// Words cut to at most `longest` characters at the last space that fits,
/// any punctuation and space the cut leaves at their end taken off, and an
/// ellipsis after; as they are where they fit. One word longer than that
/// is cut inside itself.
fn shortened(words: &str, longest: usize) -> String {
    if words.chars().count() <= longest {
        return words.to_owned();
    }
    let end = words
        .char_indices()
        .nth(longest)
        .map_or(words.len(), |(at, _)| at);
    let cut = &words[..end];
    let kept = match cut.rfind(char::is_whitespace) {
        Some(at) if !cut[..at].trim_end().is_empty() => &cut[..at],
        _ => cut,
    };
    let kept = kept.trim_end_matches(|c: char| c.is_whitespace() || c.is_ascii_punctuation());
    format!("{kept}\u{2026}")
}

/// An address in the site made whole with the site's own, or every one in
/// a piece of HTML. Other addresses are left as they are.
fn absolute(value: &Value) -> Result<Value, Error> {
    let shared = shared()?;
    frame::read(Key::Setting(shared.name_id("url")));
    let Some(url) = shared.settings.text("url") else {
        return Err(frame::raise(Kind::TemplateStopped {
            reason: Stopped::NoSiteAddress,
        }));
    };
    let origin = origin_of(url.value);
    let text = value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned);
    Ok(Value::from(match value.is_safe() {
        true => absolute_in_html(&text, origin),
        false => absolute_one(&text, origin),
    }))
}

/// `https://example.org` of `https://example.org/blog/`.
pub(super) fn origin_of(url: &str) -> &str {
    let after = url.find("//").map_or(0, |at| at + 2);
    match url[after..].find('/') {
        Some(at) => &url[..after + at],
        None => url,
    }
}

/// An address in the site made whole, its path spelled as it travels.
fn absolute_one(address: &str, origin: &str) -> String {
    match address.starts_with('/') && !address.starts_with("//") {
        true => crate::content::address::whole(address, origin),
        false => address.to_owned(),
    }
}

/// Every address in a piece of HTML that starts at the top of the site
/// made whole: each in an attribute the link check reads, a `srcset`'s
/// one by one.
fn absolute_in_html(html: &str, origin: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some((at, srcset)) = next_attribute(rest) {
        let (before, after) = rest.split_at(at);
        out.push_str(before);
        let quote = after.as_bytes()[0];
        out.push(char::from(quote));
        let value = &after[1..];
        let from_top = |address: &str| address.starts_with('/') && !address.starts_with("//");
        if srcset {
            let end = value.find(char::from(quote)).unwrap_or(value.len());
            let mut written = 0;
            for (address, start) in crate::html::addresses(&value[..end], true, 0) {
                out.push_str(&value[written..start]);
                if from_top(address) {
                    out.push_str(origin);
                }
                written = start;
            }
            out.push_str(&value[written..end]);
            rest = &value[end..];
            continue;
        }
        if from_top(value) {
            out.push_str(origin);
        }
        rest = value;
    }
    out.push_str(rest);
    out
}

/// Where the quote opening the value of the next attribute the link check
/// reads is, and whether it is a `srcset`'s.
fn next_attribute(html: &str) -> Option<(usize, bool)> {
    let lower = html.to_ascii_lowercase();
    Attribute::NAMED
        .iter()
        .filter_map(|(name, attribute)| {
            let mut from = 0;
            while let Some(at) = lower[from..].find(name) {
                let start = from + at;
                let equals = start + name.len();
                let quote = equals + 1;
                let before = lower[..start].chars().next_back();
                if before.is_some_and(char::is_whitespace)
                    && html.as_bytes().get(equals) == Some(&b'=')
                    && matches!(html.as_bytes().get(quote), Some(b'"' | b'\''))
                {
                    return Some((quote, *attribute == Attribute::Srcset));
                }
                from = equals;
            }
            None
        })
        .min()
}

/// Words read as Markdown: a caption, above all. Writing that is one
/// paragraph comes back as its words, so it sits inside a caption or a
/// heading. A snippet call in it is shown as written, and the pieces in
/// [`PIECES_OF_A_VALUE`] are written as the site writes them.
///
/// What the writing would be told is told, at the value's place in its
/// file where it has one, the filter's in the template where it hasn't.
/// Text standing in its file as it is, as most front matter does, is read
/// there; any other, such as a block of YAML or a quoted value with
/// escapes, is read alone and its problems placed by lining it up with
/// what is written.
fn markdown(state: &State, value: &Value) -> Value {
    if value.is_safe() {
        return value.clone();
    }
    let text = value
        .as_str()
        .map_or_else(|| value.to_string(), str::to_owned);
    let shared = frame::shared();
    let origin = shared.as_ref().zip(frame::origin(value));
    let in_place = origin.and_then(|(shared, place)| {
        let written = shared.sources.spelling(place);
        let at = written.find(text.as_str()).filter(|_| !text.is_empty())?;
        let start = place.span.start as usize + at;
        Some(Place {
            source: place.source,
            span: Span::new(start, start + text.len()),
        })
    });
    let mut alone = crate::source::Sources::default();
    let (sources, source, body) = match (&shared, in_place) {
        (Some(shared), Some(place)) => (&shared.sources, place.source, place.span),
        _ => {
            let id = alone.add("", text.clone());
            (&alone, id, Span::new(0, text.len()))
        }
    };
    let base = shared.as_ref().map_or("", |shared| shared.base.as_str());
    let mut document = crate::markdown::read(sources, source, body, &|_| Some(false), base);
    if let Some(shared) = shared.as_ref().filter(|_| !document.in_site.is_empty()) {
        frame::read(Key::Setting(shared.name_id("url")));
    }
    // Set in the language of the page being built.
    let page = frame::with(|frame| frame.page.map(|page| (Arc::clone(&frame.shared), page)));
    let page = page.flatten();
    if let Some((shared, page)) = &page
        && let Some(words) = super::render::typography_of(shared, *page)
    {
        let typography = crate::markdown::Typography::of(words);
        crate::markdown::set_typography(&mut document, sources, &typography);
    }
    let of_value = OfValue {
        sources,
        page: page.as_ref().map(|(shared, page)| (shared, *page)),
        whole: match in_place {
            Some(_) => None,
            None => Some(origin.map(|(_, place)| place)),
        },
        failed: RefCell::new(BTreeSet::new()),
    };
    let words = shared
        .as_ref()
        .map(|shared| super::render::words_of(shared, frame::language()));
    let html = crate::markdown::to_html(
        &document,
        &crate::markdown::Options {
            snippets: &of_value,
            back_to_the_text: None,
            digits: words.and_then(|words| words.digits.as_ref()),
            right_to_left: words
                .is_some_and(|words| words.direction == crate::language::Direction::RightToLeft),
            highlighter: None,
        },
    );
    let trimmed = html.trim_end();
    let bare = trimmed
        .strip_prefix("<p>")
        .and_then(|rest| rest.strip_suffix("</p>"))
        .filter(|inside| !inside.contains("<p>"));
    if let Some(shared) = &shared {
        // Without its `<p>`, the paragraph closes nothing for the value.
        let held = bare.map(|_| document.held_in_paragraph).unwrap_or_default();
        let told: Vec<Problem> = (document.problems.into_iter())
            .chain(held)
            .filter(|problem| crate::markdown::told_of_a_value(&problem.kind))
            .collect();
        if !told.is_empty() {
            let filter = super::explain::filter(shared, state, "markdown", &text, None);
            let origin = origin.map(|(_, place)| place);
            let lined_up = match in_place {
                Some(_) => None,
                None => origin.and_then(|place| lined_up(&text, shared.sources.spelling(place))),
            };
            let placed = |place: Option<Place>| match (in_place, origin) {
                (Some(_), _) => place,
                (None, Some(origin)) => Some(match (place.map(|p| p.span), &lined_up) {
                    (Some(span), Some(map)) => Place {
                        source: origin.source,
                        span: Span::new(
                            origin.span.start as usize + map[span.start as usize],
                            origin.span.start as usize + map[span.end as usize],
                        ),
                    },
                    _ => origin,
                }),
                (None, None) => filter,
            };
            frame::with(|frame| {
                for mut problem in told {
                    problem.place = placed(problem.place);
                    // A value in no file has no place to point at for the note shown.
                    if let crate::problem::Kind::DefinitionWrittenTwice { shown, .. } =
                        &mut problem.kind
                    {
                        *shown = origin.and(placed(*shown));
                    }
                    if origin.is_some() {
                        problem = problem.met(Relation::ReadAsMarkdown, filter);
                    }
                    let problem = super::render::met_building(problem, frame);
                    frame.problems.push(problem);
                }
            });
        }
    }
    Value::from_safe_string(bare.map_or_else(|| html.clone(), str::to_owned))
}

/// Where each byte of `text`, and its end, stands in `written`, the same
/// text as written in a file: quoted, its escapes written out or its lines
/// indented. Each character is found at the next written one alike, any
/// space matching any space, as folded lines are joined by one. None where
/// a character is never found, written as an escape such as `é`.
fn lined_up(text: &str, written: &str) -> Option<Vec<usize>> {
    let mut map = Vec::with_capacity(text.len() + 1);
    let mut at = 0;
    for (i, c) in text.char_indices() {
        let found = written[at..].char_indices().find_map(|(skip, w)| {
            if w == '\\'
                && let Some((escaped, length)) = unescaped(&written[at + skip..])
                && escaped == c
            {
                return Some((skip, length));
            }
            (w == c || (w.is_whitespace() && c.is_whitespace())).then_some((skip, w.len_utf8()))
        });
        let Some((skip, length)) = found else {
            if text[i..].trim().is_empty() {
                map.resize(text.len() + 1, written.len());
                return Some(map);
            }
            return None;
        };
        at += skip;
        map.extend((0..c.len_utf8()).map(|k| at + k.min(length - 1)));
        at += length;
    }
    map.push(at);
    Some(map)
}

/// The character a YAML or JSON escape at the start of `written` stands
/// for, and how long the escape is.
fn unescaped(written: &str) -> Option<(char, usize)> {
    let mut chars = written.strip_prefix('\\')?.chars();
    let hex = |digits: usize| {
        let code = written.get(2..2 + digits)?;
        u32::from_str_radix(code, 16).ok()
    };
    let letter = chars.next()?;
    let c = match letter {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '0' => '\0',
        'a' => '\x07',
        'b' => '\x08',
        'e' => '\x1b',
        'f' => '\x0c',
        'v' => '\x0b',
        'N' => '\u{85}',
        '_' => '\u{a0}',
        'L' => '\u{2028}',
        'P' => '\u{2029}',
        'x' => return Some((char::from_u32(hex(2)?)?, 4)),
        'U' => return Some((char::from_u32(hex(8)?)?, 10)),
        'u' => {
            let high = hex(4)?;
            if let Some(c) = char::from_u32(high) {
                return Some((c, 6));
            }
            // A character beyond the first plane, written as JSON writes
            // one: two escapes, the high half and the low.
            let low = written.get(6..8).filter(|s| *s == "\\u").and(
                written
                    .get(8..12)
                    .and_then(|code| u32::from_str_radix(code, 16).ok()),
            )?;
            let c = char::from_u32(0x10000 + ((high - 0xd800) << 10) + (low.checked_sub(0xdc00)?))?;
            return Some((c, 12));
        }
        other => other,
    };
    Some((c, 1 + letter.len_utf8()))
}

/// The pieces a value's writing is written through as the site writes
/// them. The others need what only a page's own writing has: a heading
/// and a footnote their ids, which a page showing many values would
/// repeat, and a picture the folder of the file it is written in.
const PIECES_OF_A_VALUE: [Piece; 5] = [
    Piece::Link,
    Piece::Code,
    Piece::InlineCode,
    Piece::Callout,
    Piece::Table,
];

/// Writes the calls and pieces of a value read as Markdown: a call as the
/// text it was written as.
struct OfValue<'s> {
    sources: &'s crate::source::Sources,
    /// The site and the page being built, where one is.
    page: Option<(&'s Arc<Shared>, PageId)>,
    /// Where the whole value is written, where it has a place, for a
    /// value read alone: a place in it is then no place in the site's
    /// files. None for a value read where it stands.
    whole: Option<Option<Place>>,
    /// Pieces whose file stopped once in this value, written as Statac
    /// writes them for the rest of it.
    failed: RefCell<BTreeSet<Piece>>,
}

impl crate::markdown::Snippets for OfValue<'_> {
    fn html(
        &self,
        _: &crate::markdown::Call,
        place: crate::source::Place,
        _: crate::markdown::Placement,
        _: Option<&str>,
    ) -> String {
        escape(self.sources.spelling(place))
    }

    fn writes(&self, piece: Piece) -> bool {
        PIECES_OF_A_VALUE.contains(&piece)
            && !self.failed.borrow().contains(&piece)
            && self
                .page
                .is_some_and(|(shared, _)| shared.templates.pieces.contains_key(&piece))
    }

    fn reads(&self, piece: Piece, name: &str) -> bool {
        self.page
            .is_none_or(|(shared, _)| super::pieces::reads(shared, piece, name))
    }

    fn piece(
        &self,
        piece: Piece,
        values: Vec<(&'static str, PieceValue)>,
        place: Place,
    ) -> Option<String> {
        let (shared, page) = self.page?;
        let calling = self.whole.unwrap_or(Some(place));
        let before = frame::with(|frame| {
            let before = frame.calling;
            frame.calling = calling.or(before);
            before
        })
        .flatten();
        let built = super::pieces::build(shared, page, piece, values);
        frame::with(|frame| frame.calling = before);
        match built? {
            Ok(html) => Some(html),
            Err(error) => {
                self.failed.borrow_mut().insert(piece);
                frame::with(|frame| {
                    let callers = calling.as_slice();
                    for problem in super::explain::explain(&error, frame, callers) {
                        if !frame.problems.contains(&problem) {
                            frame.problems.push(problem);
                        }
                    }
                });
                None
            }
        }
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

/// A value's words: markup's are what a reader reads, its tags taken off
/// and each run of space closed up to one; anything else is words already.
pub(super) fn plain(value: &Value) -> String {
    match (value.is_safe(), value.as_str()) {
        (true, Some(markup)) => closed_up(&words_of(markup)),
        (_, Some(words)) => words.to_owned(),
        _ => value.to_string(),
    }
}

/// Words cut at a word to fit in `longest` characters, the ending counted
/// in.
pub(super) fn truncate(words: &str, longest: usize, ending: &str) -> String {
    if words.chars().count() <= longest {
        return words.to_owned();
    }
    let room = longest.saturating_sub(ending.chars().count());
    let cut = words
        .char_indices()
        .nth(room)
        .map_or(words.len(), |(at, _)| at);
    let kept = &words[..cut];
    let kept = match words[cut..].starts_with(char::is_whitespace) {
        true => kept,
        false => kept
            .rfind(char::is_whitespace)
            .map_or(kept, |at| &kept[..at]),
    };
    format!("{}{ending}", kept.trim_end())
}

/// A value as JSON, safe inside a `<script>` element.
pub(super) fn json(value: &Value, out: &mut String) {
    use minijinja::value::ValueKind;
    if let Some(date) = value.downcast_object_ref::<DateValue>() {
        let printed = date.part_printed();
        return crate::json::write_string(&printed.unwrap_or_else(|| date.date.to_string()), out);
    }
    if let Some(page) = value.downcast_object_ref::<PageValue>() {
        return crate::json::write_string(&page.shared.address_of(page.id), out);
    }
    if let Some(list) = value.downcast_object_ref::<ListValue>() {
        return crate::json::write_string(&super::values::list_name(&list.shared, list.id), out);
    }
    if let Some(lists) = value.downcast_object_ref::<Lists>() {
        out.push('[');
        for (i, &id) in lists.lists.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            crate::json::write_string(&super::values::list_name(&lists.shared, id), out);
        }
        out.push(']');
        return;
    }
    match value.kind() {
        ValueKind::Undefined | ValueKind::None => out.push_str("null"),
        ValueKind::Bool => out.push_str(if value.is_true() { "true" } else { "false" }),
        ValueKind::Number => out.push_str(&value.to_string()),
        ValueKind::Seq | ValueKind::Iterable => {
            out.push('[');
            for (i, item) in value.try_iter().into_iter().flatten().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                json(&item, out);
            }
            out.push(']');
        }
        ValueKind::Map => {
            out.push('{');
            for (i, key) in value.try_iter().into_iter().flatten().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                crate::json::write_string(&key.to_string(), out);
                out.push(':');
                json(&value.get_item(&key).unwrap_or_default(), out);
            }
            out.push('}');
        }
        _ => crate::json::write_string(value.as_str().unwrap_or(&value.to_string()), out),
    }
}

/// Text written as part of an address: everything but letters, digits and
/// `/._-` written as `%` and its bytes; a map as `name=value` pairs.
fn urlencode(value: &Value) -> Result<String, Error> {
    fn encoded(text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        for byte in text.bytes() {
            match byte {
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'/' | b'.' | b'-' | b'_' => {
                    out.push(char::from(byte))
                }
                _ => write!(out, "%{byte:02X}").expect("writing to a string can't fail"),
            }
        }
        out
    }
    if value.kind() == minijinja::value::ValueKind::Map {
        let mut pairs = Vec::new();
        for key in value.try_iter()? {
            let item = value.get_item(&key)?;
            pairs.push(format!(
                "{}={}",
                encoded(&key.to_string()),
                encoded(&item.to_string())
            ));
        }
        return Ok(pairs.join("&"));
    }
    Ok(encoded(
        &value
            .as_str()
            .map_or_else(|| value.to_string(), str::to_owned),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each character of a value is found where it is written, through
    /// quotes, escapes and the indentation of a block; none where one
    /// can't be found.
    #[test]
    fn a_value_is_lined_up_with_how_it_is_written() {
        let at = |text: &str, written: &str, of: &str| {
            let map = lined_up(text, written)?;
            let start = text.find(of)?;
            Some(written[map[start]..map[start + of.len()]].to_owned())
        };
        let cases = [
            ("a \"b\" $x$", r#""a \"b\" $x$""#, "$x$", "$x$"),
            ("one\ntwo $y$\n", "|\n  one\n  two $y$\n", "$y$", "$y$"),
            ("é \u{1}", r#""é \x01""#, "\u{1}", r"\x01"),
            ("a 😀 b", r#""a 😀 b""#, "😀", r"😀"),
            ("a\\b", r#""a\\b""#, "\\b", r"\\b"),
        ];
        for (text, written, of, found) in cases {
            assert_eq!(at(text, written, of).as_deref(), Some(found), "{written}");
        }
        assert_eq!(lined_up("é", "x"), None);
    }

    #[test]
    fn markup_comes_back_as_its_words() {
        assert_eq!(
            words_of("<p>Post 1 &amp; <em>Post&#32;2</em> &#x2014; a &lt;b&gt;</p>"),
            "Post 1 & Post 2 \u{2014} a <b>"
        );
        assert_eq!(words_of("a & b"), "a & b");
    }

    #[test]
    fn words_are_cut_at_a_word_with_the_ending_counted() {
        assert_eq!(truncate("one two three", 20, "..."), "one two three");
        assert_eq!(truncate("one two three", 10, "..."), "one two...");
        assert_eq!(truncate("one two three", 7, ""), "one two");
        assert_eq!(truncate("onetwothree", 5, ""), "onetw");
    }

    #[test]
    fn addresses_at_the_top_of_the_site_are_made_whole() {
        let origin = origin_of("https://example.org/blog/");
        assert_eq!(origin, "https://example.org");
        assert_eq!(
            absolute_one("/blog/post-1/", origin),
            "https://example.org/blog/post-1/"
        );
        assert_eq!(
            absolute_one("https://example.com/", origin),
            "https://example.com/"
        );
        assert_eq!(
            absolute_one("//cdn.example.com/a", origin),
            "//cdn.example.com/a"
        );
        assert_eq!(
            absolute_one("/blog/café 1/?q=a b#c d", origin),
            "https://example.org/blog/caf%C3%A9%201/?q=a b#c d"
        );
        assert_eq!(
            absolute_in_html(
                r##"<a href="/post-1/">1</a> <img src='/a.png' alt="/b"> <a href="#x">"##,
                origin
            ),
            r##"<a href="https://example.org/post-1/">1</a> <img src='https://example.org/a.png' alt="/b"> <a href="#x">"##
        );
        assert_eq!(
            absolute_in_html(
                r#"<source srcset="/a-400.avif 400w, b-800.avif 800w,/c,d.avif 2x" sizes="/"><img src="/a.jpg" srcset='//x.example/a.jpg'>"#,
                origin
            ),
            r#"<source srcset="https://example.org/a-400.avif 400w, b-800.avif 800w,https://example.org/c,d.avif 2x" sizes="/"><img src="https://example.org/a.jpg" srcset='//x.example/a.jpg'>"#
        );
    }

    #[test]
    fn every_attribute_the_link_check_reads_is_made_whole() {
        let origin = "https://example.org";
        for (name, _) in Attribute::NAMED {
            assert_eq!(
                absolute_in_html(&format!(r#"<x {name}="/a.png" data-{name}="/b">"#), origin),
                format!(r#"<x {name}="https://example.org/a.png" data-{name}="/b">"#),
            );
        }
        assert_eq!(
            absolute_in_html(
                r#"<video src="/a.mp4" poster="/a.png" controls></video>"#,
                origin
            ),
            r#"<video src="https://example.org/a.mp4" poster="https://example.org/a.png" controls></video>"#
        );
    }

    #[test]
    fn json_is_safe_inside_a_script() {
        let mut out = String::new();
        json(
            &Value::from_iter([Value::from("</script>"), Value::from(1), Value::from(true)]),
            &mut out,
        );
        assert_eq!(out, "[\"\\u003c/script\\u003e\",1,true]");
    }

    #[test]
    fn json_is_safe_inside_an_attribute_in_single_quotes() {
        let mut out = String::new();
        json(&Value::from("a'b"), &mut out);
        assert_eq!(out, "\"a\\u0027b\"");
    }

    #[test]
    fn an_address_piece_is_percent_written() {
        assert_eq!(
            urlencode(&Value::from("Re: post 1/2")).unwrap(),
            "Re%3A%20post%201/2"
        );
    }
}
