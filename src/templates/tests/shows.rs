//! What is said of one page to whoever is writing its template: what
//! builds it, the lists holding it, and each name a template can read.

use super::*;
use crate::origin::Arrival;
use crate::templates::{Held, Holds, Row};

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
    assert_eq!(row(page, "page.words").held, Held::Shown("2".into()));
    assert_eq!(row(page, "page.maths").held, Held::Shown("false".into()));
    assert_eq!(row(page, "page.photo").held, Held::Call);

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
fn asking_what_a_page_shows_leaves_no_frame_behind() {
    let built = explained_site();
    let page = built.at("/blog/post-2/");
    built.renderer.explained(page, 1);
    let rendered = built.renderer.render(page);
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
}
