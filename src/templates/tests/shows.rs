//! What is said of one page to whoever is writing its template: what
//! builds it, the lists holding it, and each name a template can read.

use super::*;
use crate::origin::Arrival;
use crate::problem::Holds;
use crate::templates::{Held, Reached, Row};

fn explained_site() -> Built {
    site(&[
        (
            "statac.yaml",
            "language: en\ntitle: Site\ntimezone: UTC\npages.feed: false\nmotto: A motto\n",
        ),
        (
            "content/blog/index.md",
            "---\ntitle: Blog\npages.layout: post\n---\n",
        ),
        (
            "content/blog/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\ntags: [Rust]\nbook:\n  author: Author 1\n---\n\
             Some writing.\n",
        ),
        (
            "content/blog/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\nsubtitle: Second\n---\n",
        ),
        ("data/people.yaml", "person-1:\n  role: Role 1\n"),
        ("templates/home.html", "{{ page.title }}"),
        ("templates/folder.html", "{{ page.title }}"),
        ("templates/page.html", "{{ page.title }}"),
        ("templates/post.html", "{{ page.title }}"),
    ])
}

fn row<'a>(rows: &'a [Row], name: &str) -> &'a Row {
    rows.iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("no row for {name}"))
}

#[test]
fn a_page_is_told_with_its_layout_and_the_lists_holding_it() {
    let built = explained_site();
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let explained = built.renderer.explained(built.at("/blog/post-1/"), 1);
    assert_eq!(explained.address, "/blog/post-1/");
    let source = explained.source.expect("a page of the author's");
    let path = built.renderer.sources().path(source);
    assert!(path.ends_with("content/blog/post-1.md"), "{path:?}");

    let layout = explained.layout.expect("a template builds it");
    assert_eq!(layout.name, "post");
    assert_eq!(layout.origin.arrival, Arrival::HandedDown);

    let lists: Vec<_> = (explained.lists.iter())
        .map(|member| (member.list.as_str(), member.at, member.of))
        .collect();
    assert_eq!(lists, [("", 2, 2), ("blog", 2, 2), ("Rust", 1, 1)]);
}

#[test]
fn every_name_a_template_can_write_is_told_with_what_it_holds() {
    let built = explained_site();
    let explained = built.renderer.explained(built.at("/blog/post-1/"), 1);
    let groups: Vec<&str> = explained.groups.iter().map(|group| group.of).collect();
    assert_eq!(groups, ["page", "site", "site.data"]);
    let sources = built.renderer.sources();
    let written = |row: &Row| row.from.map(|place| sources.spelling(place).to_owned());

    let page = &explained.groups[0].rows;
    let title = row(page, "page.title");
    assert_eq!(title.holds, Holds::Text);
    assert_eq!(title.held, Held::Shown("Post 1".into()));
    assert_eq!(written(title).as_deref(), Some("Post 1"));
    let date = row(page, "page.date");
    assert_eq!(date.holds, Holds::Date);
    assert_eq!(date.has, ["day", "month", "weekday", "year"]);
    assert_eq!(row(page, "page.tags").held, Held::Shown("Rust".into()));
    assert_eq!(row(page, "page.word_count").held, Held::Shown("2".into()));
    assert_eq!(row(page, "page.maths").held, Held::Shown("false".into()));
    assert_eq!(row(page, "page.image").held, Held::Call);

    // A field of the author's own is followed into what it holds.
    assert_eq!(row(page, "page.book").held, Held::Fields);
    let author = row(page, "page.book.author");
    assert_eq!(author.held, Held::Shown("Author 1".into()));
    assert_eq!(written(author).as_deref(), Some("Author 1"));

    // A name another page writes is told here too, holding nothing.
    let subtitle = row(page, "page.subtitle");
    assert_eq!(subtitle.held, Held::Nothing);
    assert_eq!(subtitle.from, None);
    assert_eq!(row(page, "page.summary").held, Held::Nothing);

    let site = &explained.groups[1].rows;
    assert_eq!(row(site, "site.title").held, Held::Shown("Site".into()));
    assert_eq!(row(site, "site.motto").held, Held::Shown("A motto".into()));
    assert_eq!(row(site, "site.posts").held, Held::Count(2));
    assert_eq!(row(site, "site.data").held, Held::Fields);

    let data = &explained.groups[2].rows;
    let role = row(data, "site.data.people[\"person-1\"].role");
    assert_eq!(role.held, Held::Shown("Role 1".into()));
    assert_eq!(written(role).as_deref(), Some("Role 1"));
}

#[test]
fn a_value_of_statacs_that_holds_only_its_fields_is_told_field_by_field() {
    use crate::described::Group;
    let built = site(&[
        (
            "statac.yaml",
            "url: https://example.org/\nsearch_without_javascript: duckduckgo\n",
        ),
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "{{ page.title }}"),
    ]);
    let explained = built.renderer.explained(built.at("/post-1/"), 1);
    let site = &explained.groups[1].rows;
    let form = row(site, "site.search_without_javascript");
    assert_eq!((form.holds, &form.held), (Holds::Fields, &Held::Fields));
    let action = row(site, "site.search_without_javascript.action");
    assert_eq!(action.holds, Holds::Text);
    assert_eq!(action.held, Held::Shown("https://duckduckgo.com/".into()));
    assert_eq!(action.about, Group::Search.of("action"));
    let fields = row(site, "site.search_without_javascript.fields");
    assert_eq!(
        (fields.holds, &fields.held),
        (Holds::Items, &Held::Count(1))
    );
    assert_eq!(fields.about, Group::Search.of("fields"));
}

#[test]
fn asking_what_a_page_shows_leaves_no_frame_behind() {
    let built = explained_site();
    let page = built.at("/blog/post-2/");
    built.renderer.explained(page, 1);
    let rendered = built.renderer.render(page);
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
}

#[test]
fn a_name_is_reached_by_its_top_name_or_under_data_by_its_file() {
    let built = explained_site();
    let renderer = &built.renderer;
    let explained = renderer.explained(built.at("/blog/post-1/"), 1);
    let page = &explained.groups[0].rows;
    let on_page = |name: &str| Reached::Page(renderer.name_id(name));
    assert_eq!(row(page, "page.title").reached, on_page("title"));
    assert_eq!(row(page, "page.book.author").reached, on_page("book"));
    assert_eq!(row(page, "page.image").reached, on_page("image"));
    let site = &explained.groups[1].rows;
    let on_site = Reached::Site(renderer.name_id("motto"));
    assert_eq!(row(site, "site.motto").reached, on_site);
    let data = &explained.groups[2].rows;
    let people = row(data, "site.data.people");
    assert!(
        matches!(&people.reached, Reached::Data(files) if files.len() == 1),
        "{:?}",
        people.reached
    );
    let role = row(data, "site.data.people[\"person-1\"].role");
    assert_eq!(role.reached, people.reached);
    assert_eq!(row(site, "site.data").reached, people.reached);
}

/// A site whose post's layout writes its title and its writing alone, and
/// its date through a snippet handed the post.
fn asking_site() -> Built {
    site(&[
        (
            "statac.yaml",
            "language: en\ntitle: Site\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\n---\nSome writing.\n",
        ),
        ("data/people.yaml", "person-1: Person 1\n"),
        ("snippets/dated.html", "{{ post.date }}"),
        (
            "templates/page.html",
            "{{ page.title }}{{ page.content }}{{ dated(page) }}{{ site.data.people[\"person-1\"] }}",
        ),
        (
            "templates/home.html",
            "{% for post in site.posts %}{{ post.title }}{% endfor %}",
        ),
    ])
}

#[test]
fn an_output_notes_what_it_asks_of_its_own_page_and_the_site_by_name() {
    let built = asking_site();
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let renderer = &built.renderer;
    let name = |name: &str| renderer.name_id(name);

    let post = built.render("post-1.md");
    assert!(post.problems.is_empty(), "{:?}", post.problems);
    let asked = &post.asked;
    for written in ["title", "content", "date"] {
        assert!(asked.asked_page(name(written)), "{written}");
    }
    // Answered from the same writing, or the same language, but never
    // asked for.
    for unwritten in ["excerpt", "maths", "direction", "word_count"] {
        assert!(!asked.asked_page(name(unwritten)), "{unwritten}");
    }
    assert!(asked.asked_site(name("data")));
    let people = (renderer.site().data.files.iter()).map(|(id, _)| id).next();
    assert!(asked.reached(people.unwrap()));

    // Another page's title, reached in a loop, is no name of this one's.
    let home = built.render("index.md");
    assert!(home.problems.is_empty(), "{:?}", home.problems);
    assert!(home.asked.asked_site(name("posts")));
    assert!(!home.asked.asked_page(name("title")));
}
