use super::*;

/// Four posts, one asking to be in the menu, each built with `template`,
/// and a loop noted once the pages running it reach `many` pages between
/// them.
fn four_posts(many: u64, template: &str) -> Built {
    let temp = TempSite::new(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        NO_HOME,
        ("content/post-1.md", "---\ntitle: Post 1\nmenu: true\n---\n"),
        ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
        ("content/post-3.md", "---\ntitle: Post 3\n---\n"),
        ("content/post-4.md", "---\ntitle: Post 4\n---\n"),
        ("templates/page.html", template),
    ]);
    LOWERED_WALKED
        .lock()
        .unwrap()
        .push((temp.root.clone(), many));
    let today = crate::date::Date::day_only(2026, 10, 4);
    let (renderer, problems) = crate::build::renderer(&temp.root, Some(today));
    assert!(problems.is_empty(), "{problems:?}");
    Built { renderer, problems }
}

/// What a build of the four posts would note of its loops.
fn noted(built: &Built) -> Vec<Problem> {
    let rendered: Vec<Rendered> = (1..=4)
        .map(|n| built.render(&format!("post-{n}.md")))
        .collect();
    for page in &rendered {
        assert!(page.problems.is_empty(), "{:?}", page.problems);
    }
    let outputs = rendered.iter().map(|page| (page.walked, &page.reads));
    built.renderer.loops_over_every_page(outputs)
}

#[test]
fn a_loop_over_every_page_on_every_page_is_noted_at_the_list() {
    // Each as written, the list the note names, and the list it offers.
    let cases = [
        (
            "{% for e in site.pages if e.menu %}{{ e.url }}{% endfor %}",
            "site.pages",
            Some("site.pages | selectattr(\"menu\")"),
        ),
        (
            "{% for e in site.pages | reverse %}{% if e.url == page.url %}{{ loop.index }}{% endif %}{% endfor %}",
            "site.pages | reverse",
            None,
        ),
        (
            "{% for e in site.pages if e.title is endingwith('1') %}{{ e.url }}{% endfor %}",
            "site.pages",
            None,
        ),
        (
            "{% for e in site.pages if page.menu %}{{ e.url }}{% endfor %}{% for e in site.pages | selectattr('menu') %}{% endfor %}{% if page.posts is defined %}{% for e in page.posts %}{% endfor %}{% endif %}{% for e in site.posts[:2] %}{% endfor %}",
            "site.pages",
            None,
        ),
        (
            "{% block main %}{% if page.title %}{% for e in page.folder.pages %}{{ e.url }}{% endfor %}{% endif %}{% endblock %}",
            "page.folder.pages",
            None,
        ),
    ];
    for (template, written, instead) in cases {
        let built = four_posts(16, template);
        let noted = noted(&built);
        let kinds: Vec<&Kind> = noted.iter().map(|problem| &problem.kind).collect();
        let wanted = Kind::LoopOverEveryPage {
            written: Some(written.to_owned()),
            pages: 4,
            instead: instead.map(str::to_owned),
        };
        assert_eq!(kinds, [&wanted], "{template}");
        assert_eq!(
            built.told(&noted),
            [(
                "loop-over-every-page",
                "templates/page.html".into(),
                written.into()
            )],
            "{template}"
        );
    }
}

#[test]
fn a_loop_not_known_from_the_templates_is_noted_of_the_build() {
    for template in [
        // No loop names the list.
        "{% set all = site.pages | list %}{% for e in all %}{{ e.url }}{% endfor %}",
        "{{ site.pages | selectattr('title') | list | length }}{% for e in site.pages | selectattr('menu') %}{% endfor %}",
        // Two do, and which ran is not known.
        "{% for e in site.pages if page.menu %}{{ e.url }}{% endfor %}{% for e in site.posts | reverse %}{% endfor %}",
    ] {
        let noted = noted(&four_posts(16, template));
        assert_eq!(noted.len(), 1, "{template}: {noted:?}");
        assert_eq!(noted[0].place, None, "{template}");
        assert_eq!(
            noted[0].kind,
            Kind::LoopOverEveryPage {
                written: None,
                pages: 4,
                instead: None,
            },
            "{template}"
        );
    }
}

#[test]
fn what_reaches_few_pages_is_not_noted() {
    let loop_with_if = "{% for e in site.pages if e.menu %}{{ e.url }}{% endfor %}";
    for (many, template) in [
        // One page short of what is noted.
        (17, loop_with_if),
        // Chosen once for the site: each page reaches the one chosen.
        (
            5,
            "{% for e in site.pages | selectattr('menu') %}{{ e.url }}{% endfor %}",
        ),
        (
            5,
            "{% for e in site.pages | where('menu', 'true') | sort(attribute='title') %}{{ e.url }}{% endfor %}",
        ),
        // The first of them taken, and their number.
        (
            5,
            "{% for e in site.pages[:1] %}{{ e.url }}{% endfor %}{{ site.pages | length }}",
        ),
        (
            5,
            "{{ (site.pages | sort(attribute='title') | first).url }}",
        ),
    ] {
        let noted = noted(&four_posts(many, template));
        assert!(noted.is_empty(), "{template}: {noted:?}");
    }
}
