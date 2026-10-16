//! Small sites built end to end: their templates read and checked, and
//! their pages built.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::origin::Arrival;
use crate::problem::{
    ArgumentFault, Called, Chooser, Instead, Kind, Lack, Named, Needed, Relation, Stopped,
    TemplateFault, Unsupported, Word,
};
use crate::reads::Key;
use crate::site::{Id, PageId};

/// A site written to a fresh folder, removed when dropped.
struct TempSite {
    root: PathBuf,
}

impl TempSite {
    fn new(files: &[(&str, &str)]) -> TempSite {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let root = std::env::temp_dir().join(format!(
            "statac-templates-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("content")).unwrap();
        std::fs::write(root.join("statac.yaml"), "language: en\n").unwrap();
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        TempSite { root }
    }
}

impl Drop for TempSite {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

/// A site read and ready to build, with the problems reading it found.
struct Built {
    renderer: Renderer,
    problems: Vec<Problem>,
}

/// The site's own page set aside, so `site.pages` holds the posts alone.
const NO_HOME: (&str, &str) = ("content/index.md", "---\nindex_page: false\n---\n");

fn site(files: &[(&str, &str)]) -> Built {
    let temp = TempSite::new(files);
    let today = crate::date::Date::day_only(2026, 10, 4);
    let (renderer, problems) = crate::build::renderer(&temp.root, Some(today));
    Built { renderer, problems }
}

/// The sites whose outputs may take fewer of the engine's steps than
/// [`STEPS`], by their folders.
static LOWERED: Mutex<Vec<(PathBuf, u64)>> = Mutex::new(Vec::new());

pub(super) fn lowered(root: &Path) -> Option<u64> {
    let lowered = LOWERED.lock().unwrap();
    lowered
        .iter()
        .find(|(at, _)| at == root)
        .map(|&(_, steps)| steps)
}

/// The sites whose loops over every page are noted sooner than
/// [`super::walks::MANY`], by their folders.
static LOWERED_WALKED: Mutex<Vec<(PathBuf, u64)>> = Mutex::new(Vec::new());

pub(super) fn lowered_walked(root: &Path) -> Option<u64> {
    let lowered = LOWERED_WALKED.lock().unwrap();
    lowered
        .iter()
        .find(|(at, _)| at == root)
        .map(|&(_, many)| many)
}

/// A site written to a fresh folder, each of its outputs allowed `steps`.
fn site_of_steps(steps: u64, files: &[(&str, &str)]) -> TempSite {
    let temp = TempSite::new(files);
    LOWERED.lock().unwrap().push((temp.root.clone(), steps));
    temp
}

/// [`site`], each output allowed `steps`.
fn site_taking(steps: u64, files: &[(&str, &str)]) -> Built {
    let temp = site_of_steps(steps, files);
    let today = crate::date::Date::day_only(2026, 10, 4);
    let (renderer, problems) = crate::build::renderer(&temp.root, Some(today));
    Built { renderer, problems }
}

impl Built {
    fn id(&self, path: &str) -> PageId {
        self.renderer
            .site()
            .pages
            .iter()
            .find(|(_, page)| page.path == path)
            .map(|(id, _)| id)
            .unwrap_or_else(|| panic!("no page {path}"))
    }

    fn render(&self, path: &str) -> Rendered {
        self.renderer.render(self.id(path))
    }

    fn html(&self, path: &str) -> String {
        let rendered = self.render(path);
        assert!(
            rendered.problems.is_empty(),
            "{path}: {:?}",
            rendered.problems
        );
        rendered.html.expect("built")
    }

    /// Each problem's name, the file it is in and the text it is at.
    fn told(&self, problems: &[Problem]) -> Vec<(&'static str, String, String)> {
        let sources = self.renderer.sources();
        problems
            .iter()
            .map(|p| {
                let place = p.place.expect("placed");
                (
                    p.kind.name(),
                    sources.path(place.source).to_string_lossy().into_owned(),
                    sources.spelling(place).to_owned(),
                )
            })
            .collect()
    }

    /// Where a problem was met from, as each related place's relation,
    /// file and text; the file and text empty where it has no place.
    fn context(&self, problem: &Problem) -> Vec<(Relation, String, String)> {
        let sources = self.renderer.sources();
        problem
            .context
            .iter()
            .map(|(relation, place)| match place {
                Some(place) => (
                    relation.clone(),
                    sources.path(place.source).to_string_lossy().into_owned(),
                    sources.spelling(*place).to_owned(),
                ),
                None => (relation.clone(), String::new(), String::new()),
            })
            .collect()
    }

    /// The problems reading the templates found.
    fn template_problems(&self) -> Vec<(&'static str, String, String)> {
        let sources = self.renderer.sources();
        let in_templates: Vec<Problem> = self
            .problems
            .iter()
            .filter(|p| {
                p.place.is_some_and(|place| {
                    let path = sources.path(place.source);
                    path.starts_with("templates") || path.starts_with("snippets")
                })
            })
            .cloned()
            .collect();
        self.told(&in_templates)
    }
}

const BASE: (&str, &str) = (
    "templates/base.html",
    "<title>{{ page.title }}</title>{% block main %}{% endblock %}",
);
const PAGE: (&str, &str) = (
    "templates/page.html",
    "{% extends \"base\" %}{% block main %}{{ page.content }}{% endblock %}",
);
const HOME: (&str, &str) = (
    "templates/home.html",
    "{% extends \"base\" %}{% block main %}{% for post in page.posts %}{{ post.title }};{% endfor %}{% endblock %}",
);

#[test]
fn a_page_is_built_through_its_layout_and_the_template_it_extends() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\n---\nWords *here*.\n",
        ),
        BASE,
        PAGE,
        HOME,
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "<title>Post 1</title><p>Words <em>here</em>.</p>"
    );
    assert_eq!(built.html("index.md"), "<title>Home</title>Post 1;");

    let post = built.id("post-1.md");
    let reads = built.render("post-1.md").reads;
    let keys = reads.keys();
    assert!(keys.contains(&Key::Body(post)));
    let title = built.renderer.name_id("title");
    assert!(keys.contains(&Key::Field(post, title)));
    let templates: Vec<_> = keys
        .iter()
        .filter(|k| matches!(k, Key::Template(_)))
        .collect();
    assert_eq!(templates.len(), 2, "page and base: {keys:?}");
    let site_list = built
        .renderer
        .site()
        .own_list(crate::site::LanguageId::from_index(0))
        .unwrap();
    assert!(
        built
            .render("index.md")
            .reads
            .keys()
            .contains(&Key::ListPage(site_list, 1))
    );
}

#[test]
fn a_template_is_named_with_or_without_its_html() {
    let built = site(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nlayout: post.html\n---\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\nlayout: computed\n---\n",
        ),
        BASE,
        (
            "templates/post.html",
            "{% extends \"base.html\" %}{% block main %}{{ page.content }}{% endblock %}",
        ),
        (
            "templates/computed.html",
            "{% extends \"ba\" ~ \"se.html\" %}{% block main %}{{ page.content }}{% endblock %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "<title>Post 1</title>");
    assert_eq!(built.html("post-2.md"), "<title>Post 2</title>");
}

#[test]
fn the_layout_is_chosen_as_written_handed_down_typed_or_by_default() {
    let built = site(&[
        ("content/index.md", "---\ntitle: Home\n---\n"),
        ("content/post-1.md", "---\nlayout: post\n---\n"),
        ("content/notes/index.md", "---\npages.layout: post\n---\n"),
        ("content/notes/note-1.md", "---\ntitle: Note 1\n---\n"),
        ("content/notes/note-2.md", "---\ntype: note\n---\n"),
        ("content/about.md", "---\nlayout: psot\n---\n"),
        BASE,
        PAGE,
        HOME,
        ("templates/post.html", "{% extends \"page\" %}"),
        ("templates/note.html", "{% extends \"page\" %}"),
        ("templates/folder.html", "{% extends \"page\" %}"),
    ]);
    let layout = |path: &str| built.renderer.layout_of(built.id(path));
    let chosen = layout("post-1.md").unwrap();
    assert_eq!(chosen.name, "post");
    assert_eq!(chosen.origin.arrival, Arrival::Written);
    let sources = built.renderer.sources();
    assert_eq!(sources.spelling(chosen.origin.place.unwrap()), "post");

    let handed = layout("notes/note-1.md").unwrap();
    assert_eq!(handed.name, "post");
    assert_eq!(handed.origin.arrival, Arrival::HandedDown);
    assert_eq!(layout("notes/note-2.md").unwrap().name, "note");
    assert_eq!(layout("notes/index.md").unwrap().name, "folder");
    let home = layout("index.md").unwrap();
    assert_eq!(home.name, "home");
    assert_eq!(home.origin.arrival, Arrival::Defaulted);

    let missing = layout("about.md").unwrap_err();
    assert_eq!(
        missing.kind,
        Kind::MissingTemplate {
            path: "templates/psot.html".into(),
            chosen: Chooser::Written,
            nearest: Some("post".into()),
        }
    );
    assert_eq!(sources.spelling(missing.place.unwrap()), "psot");
}

#[test]
fn a_default_layout_the_site_lacks_is_told_at_the_page() {
    let built = site(&[("content/post-1.md", "---\ntitle: Post 1\n---\n"), BASE]);
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert_eq!(
        built.told(&rendered.problems),
        [("missing-template", "content/post-1.md".into(), "---".into())]
    );
    assert!(matches!(
        rendered.problems[0].kind,
        Kind::MissingTemplate {
            chosen: Chooser::Default,
            ..
        }
    ));
}

#[test]
fn a_snippet_takes_what_it_reads_and_is_called_by_its_name() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\n---\n",
        ),
        BASE,
        PAGE,
        ("snippets/card.html", "<b>{{ post.title }}</b>"),
        (
            "snippets/figure.html",
            "{# takes photo, alt #}\n<img src=\"{{ photo }}\" alt=\"{{ alt }}\"{% if caption is defined %} title=\"{{ caption }}\"{% endif %}>",
        ),
        (
            "templates/home.html",
            "{% extends \"base\" %}{% block main %}{% for post in page.posts %}{{ card(post) }}{{ card(post=post) }}{% endfor %}{{ figure('a.png', 'A') }}{{ figure('b.png', alt='B', caption='C') }}{% endblock %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<title>Home</title><b>Post 1</b><b>Post 1</b><img src=\"a.png\" alt=\"A\"><img src=\"b.png\" alt=\"B\" title=\"C\">"
    );
}

#[test]
fn what_is_put_on_a_line_of_its_own_follows_the_lines_indent() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/index.md",
            "---\ntitle: Home\n---\nOne.\n\n- a\n- b\n\n```\ncode\n  kept\n```\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\n---\n",
        ),
        (
            "snippets/card.html",
            "<li>\n  <b>{{ post.title }}</b>\n  {{ inner() }}\n</li>\n",
        ),
        ("snippets/inner.html", "<i>\n  i\n</i>\n"),
        (
            "snippets/blank.html",
            "{% if not post.title %}untitled{% endif %}\n",
        ),
        (
            "templates/home.html",
            "<main>\n    {{ page.content }}\n    <ul>\n        {% for post in page.posts %}\n        {{ card(post) }}\n        {{ blank(post) }}\n        {% endfor %}\n    </ul>\n    <p>{{ blank(page) }}</p>\n\n</main>\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<main>\n    <p>One.</p>\n    <ul>\n      <li>a</li>\n      <li>b</li>\n    </ul>\n    <pre class=\"code\"><code>code\n  kept</code></pre>\n    <ul>\n        <li>\n          <b>Post 1</b>\n          <i>\n            i\n          </i>\n        </li>\n    </ul>\n    <p></p>\n\n</main>\n"
    );
}

#[test]
fn a_line_ending_a_value_writes_is_kept_after_a_value_that_came_to_nothing() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "templates/home.html",
            "<pre>{% for line in [\"a\", \"\", \"\", \"b\"] %}{{ line }}{{ \"\\n\" }}{% endfor %}</pre>\n{{ \"\" }}\n<p>c</p>\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("index.md"), "<pre>a\n\n\nb\n</pre>\n<p>c</p>\n");
}

#[test]
fn a_call_handing_a_snippet_what_it_cant_take_is_told_at_the_call() {
    let built = site(&[
        ("content/index.md", "---\ntitle: Home\n---\n"),
        BASE,
        ("snippets/card.html", "{{ post.title }}{{ size }}"),
        ("snippets/one.html", "{{ post.title }}"),
        (
            "templates/home.html",
            "{{ one(pots=page) }}{{ card(page, 2) }}{{ one(standalone=true) }}",
        ),
    ]);
    assert_eq!(
        built.template_problems(),
        [
            (
                "wrong-argument",
                "templates/home.html".into(),
                "page".into()
            ),
            (
                "wrong-argument",
                "templates/home.html".into(),
                "page".into()
            ),
            (
                "wrong-argument",
                "templates/home.html".into(),
                "true".into()
            ),
        ]
    );
    let kinds: Vec<&Kind> = built.problems.iter().map(|p| &p.kind).collect();
    assert_eq!(
        kinds[0],
        &Kind::WrongArgument {
            snippet: "one".into(),
            fault: ArgumentFault::NoSuchName {
                given: "pots".into(),
                nearest: Some("post".into()),
            },
        }
    );
    assert_eq!(
        kinds[1],
        &Kind::WrongArgument {
            snippet: "card".into(),
            fault: ArgumentFault::NeedsNames {
                wanted: vec!["post".into(), "size".into()],
            },
        }
    );
    assert!(matches!(
        kinds[2],
        Kind::WrongArgument {
            fault: ArgumentFault::Supplied(_),
            ..
        }
    ));
    // A template with a problem told when it was read builds nothing more.
    let rendered = built.render("index.md");
    assert_eq!(rendered.html, None);
    assert!(rendered.problems.is_empty());
}

#[test]
fn include_and_import_are_refused_with_the_call_filled_in() {
    let built = site(&[
        ("content/index.md", "---\ntitle: Home\n---\n"),
        ("snippets/card.html", "{{ post.title }}"),
        (
            "templates/home.html",
            "{% include \"card.html\" %}\n{% import \"card.html\" as c %}{{ c.card(page) }}\n{% from \"card.html\" import card %}",
        ),
    ]);
    let refused: Vec<&Kind> = built.problems.iter().map(|p| &p.kind).collect();
    assert_eq!(
        refused,
        [
            &Kind::NotSupported {
                what: Unsupported::Statement {
                    statement: "include".into(),
                    calls: vec!["{{ card(post) }}".into()],
                },
            },
            &Kind::NotSupported {
                what: Unsupported::Statement {
                    statement: "import".into(),
                    calls: vec!["{{ card(page) }}".into()],
                },
            },
            &Kind::NotSupported {
                what: Unsupported::Statement {
                    statement: "from".into(),
                    calls: vec!["{{ card(post) }}".into()],
                },
            },
        ]
    );
    assert_eq!(
        built.template_problems()[0].2,
        "{% include \"card.html\" %}"
    );
}

#[test]
fn a_call_that_wraps_writing_hands_it_to_the_snippet_as_body() {
    let built = site(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n@box(\"wide\")\n<img src=\"/a.png\" alt=\"\">\n<img src=\"/b.png\" alt=\"\">\n@end\n\n> @box(\"inner\")\n> ## Inside\n>\n> A @note('call') in it.\n> @end\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\n---\nA @box(\"x\") mid-sentence.\n@end\n",
        ),
        (
            "content/post-3.md",
            "---\ntitle: Post 3\n---\n@box(\"x\")\nNever ended.\n",
        ),
        BASE,
        PAGE,
        (
            "snippets/box.html",
            "{# takes classes #}\n<div class=\"{{ classes }}\">\n{{ body }}\n</div>",
        ),
        ("snippets/note.html", "<em>{{ text }}</em>"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "<title>Post 1</title><div class=\"wide\">\n\
         <img src=\"/a.png\" alt=\"\">\n<img src=\"/b.png\" alt=\"\">\n</div>\n\
         <blockquote>\n  <div class=\"inner\">\n  <h2 id=\"inside\">Inside</h2>\n  \
         <p>A <em>call</em> in it.</p>\n  </div>\n</blockquote>"
    );
    let rendered = built.render("post-2.md");
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "malformed-call",
            "content/post-2.md".into(),
            "@box(\"x\")".into()
        )]
    );
    assert_eq!(
        rendered.problems[0].kind,
        Kind::MalformedCall {
            found: crate::markdown::Malformed::WrapsNothing("box".into())
        }
    );
    let rendered = built.render("post-3.md");
    assert_eq!(
        rendered.problems[0].kind,
        Kind::MalformedCall {
            found: crate::markdown::Malformed::NoEnd("box".into())
        }
    );
    assert_eq!(rendered.html, None);
}

#[test]
fn a_call_in_the_writing_builds_the_same_snippet() {
    let built = site(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\nSee @note('this').\n\n@note(text=\"alone\")\n\n<div>@note('in html') @other('x')</div>\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\n---\nSee @nope('x').\n",
        ),
        BASE,
        PAGE,
        (
            "snippets/note.html",
            "<em>{{ text }}</em>{% if standalone %}!{% endif %}",
        ),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        rendered.html.as_deref(),
        Some(
            "<title>Post 1</title><p>See <em>this</em>.</p>\n<em>alone</em>!\n<div><em>in html</em> @other('x')</div>"
        )
    );
    // The call built inside HTML is said, once.
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "call-in-html",
            "content/post-1.md".into(),
            "@note('in html')".into()
        )]
    );
    let rendered = built.render("post-2.md");
    assert_eq!(
        built.told(&rendered.problems),
        [("unknown-word", "content/post-2.md".into(), "nope".into())]
    );
    assert_eq!(rendered.html, None);
}

#[test]
fn a_missing_value_is_told_at_the_template_naming_the_page() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\nsummary: Two\n---\n",
        ),
        BASE,
        (
            "templates/page.html",
            "{% extends \"base\" %}{% block main %}<p>{{ page.summary }}</p>{% endblock %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-2.md"), "<title>Post 2</title><p>Two</p>");
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    let problem = &rendered.problems[0];
    assert_eq!(
        problem.kind,
        Kind::NoValue {
            written: "page.summary".into(),
            lack: Lack::OnPage("content/post-1.md".into()),
            instead: Instead::Default("page.summary | default(\"\")".into()),
        }
    );
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "no-value",
            "templates/page.html".into(),
            "page.summary".into()
        )]
    );
    assert_eq!(
        built.context(problem),
        [(
            Relation::Building(None),
            "content/post-1.md".into(),
            "---".into()
        )]
    );
}

#[test]
fn a_snippet_handed_a_post_is_offered_the_posts_field() {
    let built = site(&[
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummary: One\ndate: 2026-10-01\n---\n",
        ),
        BASE,
        PAGE,
        ("snippets/card.html", "{{ page.summary }} {{ post.title }}"),
        (
            "templates/home.html",
            "{% extends \"base\" %}{% block main %}{% for post in page.posts %}{{ card(post) }}{% endfor %}{% endblock %}",
        ),
    ]);
    assert_eq!(
        built.template_problems(),
        [(
            "page-in-snippet",
            "snippets/card.html".into(),
            "page.summary".into()
        )]
    );
    let rendered = built.render("index.md");
    let problem = &rendered.problems[0];
    assert_eq!(
        problem.kind,
        Kind::NoValue {
            written: "page.summary".into(),
            lack: Lack::OnPage("content/index.md".into()),
            instead: Instead::Handed("post.summary".into()),
        }
    );
    assert_eq!(
        built.context(problem),
        [
            (
                Relation::CalledFrom,
                "templates/home.html".into(),
                "card(post)".into()
            ),
            (
                Relation::Building(None),
                "content/index.md".into(),
                "---".into()
            ),
        ]
    );
}

#[test]
fn a_value_a_snippet_reads_and_its_call_left_out_is_told_at_the_call() {
    let built = site(&[
        ("content/index.md", "---\ntitle: Home\n---\n"),
        BASE,
        (
            "snippets/card.html",
            "{{ post.title }}{{ size | default(1) }}",
        ),
        ("templates/home.html", "{{ card(size=2) }}"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let rendered = built.render("index.md");
    assert_eq!(
        rendered.problems[0].kind,
        Kind::WrongArgument {
            snippet: "card".into(),
            fault: ArgumentFault::NotGiven("post".into()),
        }
    );
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "wrong-argument",
            "templates/home.html".into(),
            "card(size=2)".into()
        )]
    );
}

#[test]
fn a_value_that_may_be_missing_inside_a_list_the_template_writes_is_refused() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\nsummary: Two\n---\n",
        ),
        BASE,
        (
            "templates/page.html",
            "{% for x in [page.summary] %}{{ x }}{% endfor %}{% set y = {'a': page.summary | default('')} %}{% set z = [page.title] + [page.summary] %}",
        ),
    ]);
    assert_eq!(
        built.template_problems(),
        [(
            "no-value",
            "templates/page.html".into(),
            "page.summary".into()
        )]
    );
    assert_eq!(
        built.problems[0].kind,
        Kind::NoValue {
            written: "page.summary".into(),
            lack: Lack::InLiteral { map: false },
            instead: Instead::Default("page.summary | default(\"\")".into()),
        }
    );
}

#[test]
fn what_default_gives_in_place_of_a_missing_value_is_blamed_where_it_is_used() {
    let pages = [
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\nsummary: Two\n---\n",
        ),
        (
            "content/post-3.md",
            "---\ntitle: Post 3\ndescription: Three\nsummary: Two\n---\n",
        ),
    ];
    let chained = "{{ page.description | default(page.summary) | default(\"\") }}";
    let built = site(&[
        pages[0],
        pages[1],
        pages[2],
        ("templates/page.html", chained),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    for (path, html) in [
        ("post-1.md", ""),
        ("post-2.md", "Two"),
        ("post-3.md", "Three"),
    ] {
        assert_eq!(built.html(path), html, "{path}");
    }
    // Kept in a list nothing guards, it is told when the template is read.
    let kept = "{% for x in [page.description | default(page.summary)] %}{% endfor %}\
                {% for x in [page.description | default(page.summary) | default(\"\")] %}{% endfor %}";
    let built = site(&[pages[0], pages[1], pages[2], ("templates/page.html", kept)]);
    assert_eq!(
        built.template_problems(),
        [(
            "no-value",
            "templates/page.html".into(),
            "page.description | default(page.summary)".into()
        )]
    );
    for (template, two) in [
        ("{{ page.description | default(page.summary) }}", "Two"),
        ("{{ page.description | d(page.summary) | upper }}", "TWO"),
        (
            "{% if page.description | default(page.summary) is upper %}{% else %}two{% endif %}",
            "two",
        ),
        (
            "{{ [page.title, page.description | default(page.summary)] | join(\" \") }}",
            "Post 2 Two",
        ),
    ] {
        let built = site(&[
            pages[0],
            pages[1],
            pages[2],
            ("templates/page.html", template),
        ]);
        assert_eq!(built.html("post-2.md"), two, "{template}");
        let rendered = built.render("post-1.md");
        assert_eq!(
            rendered
                .problems
                .iter()
                .map(|p| &p.kind)
                .collect::<Vec<_>>(),
            [&Kind::NoValue {
                written: "page.summary".into(),
                lack: Lack::OnPage("content/post-1.md".into()),
                instead: Instead::Default("page.summary | default(\"\")".into()),
            }],
            "{template}"
        );
        assert_eq!(
            built.told(&rendered.problems),
            [(
                "no-value",
                "templates/page.html".into(),
                "page.summary".into()
            )],
            "{template}"
        );
    }
}

#[test]
fn a_filter_over_pages_tests_a_field_some_lack_without_stopping() {
    let pages = [
        ("content/post-1.md", "---\ntitle: Post 1\ngroup: a\n---\n"),
        ("content/post-2.md", "---\ntitle: Post 2\ngroup: a\n---\n"),
        ("content/post-3.md", "---\ntitle: Post 3\ngroup: b\n---\n"),
        ("content/post-4.md", "---\ntitle: Post 4\n---\n"),
    ];
    let names = "| map(attribute=\"title\") | join(\", \")";
    for (filter, html) in [
        (
            "selectattr(\"group\", \"eq\", page.group)",
            "Post 1, Post 2",
        ),
        ("selectattr(\"group\", \"ne\", page.group)", "Post 3"),
        ("selectattr(\"group\")", "Post 1, Post 2, Post 3"),
        ("selectattr(\"group\", \"undefined\")", "Post 4"),
        (
            "rejectattr(\"group\", \"eq\", page.group)",
            "Post 3, Post 4",
        ),
        ("rejectattr(\"group\", \"defined\")", "Post 4"),
    ] {
        let template = format!("{{{{ site.pages | {filter} {names} }}}}");
        let built = site(&[
            pages[0],
            pages[1],
            pages[2],
            pages[3],
            NO_HOME,
            ("templates/page.html", &template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        assert_eq!(built.html("post-1.md"), html, "{template}");
        // The page being built lacks the value the template hands the test
        // itself, so that is told, and not another page's.
        let rendered = built.render("post-4.md");
        if template.contains("page.group") {
            assert_eq!(
                rendered.problems[0].kind,
                Kind::NoValue {
                    written: "page.group".into(),
                    lack: Lack::OnPage("content/post-4.md".into()),
                    instead: Instead::Default("page.group | default(\"\")".into()),
                },
                "{template}"
            );
            assert_eq!(
                built.told(&rendered.problems),
                [(
                    "no-value",
                    "templates/page.html".into(),
                    "page.group".into()
                )],
                "{template}"
            );
        } else {
            assert!(rendered.problems.is_empty(), "{template}");
        }
    }
}

#[test]
fn a_filter_using_a_field_an_item_lacks_names_the_item_at_the_filter() {
    let pages = [
        ("content/post-1.md", "---\ntitle: Post 1\nrank: 2\n---\n"),
        ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
        ("content/post-3.md", "---\ntitle: Post 3\nrank: 1\n---\n"),
    ];
    for (template, filter, at, selected) in [
        (
            "{{ site.pages | groupby(\"rank\") | length }}",
            "groupby",
            "\"rank\"",
            "site.pages | selectattr(\"rank\", \"defined\") | groupby(\"rank\")",
        ),
        (
            "{{ site.pages | unique(attribute=\"title\") | groupby(attribute=\"rank\") | length }}",
            "groupby",
            "\"rank\"",
            "site.pages | unique(attribute=\"title\") | selectattr(\"rank\", \"defined\") \
             | groupby(attribute=\"rank\")",
        ),
        (
            "{{ site.pages | unique(attribute=\"rank\") | length }}",
            "unique",
            "\"rank\"",
            "site.pages | selectattr(\"rank\", \"defined\") | unique(attribute=\"rank\")",
        ),
        (
            "{{ site.pages | map(attribute=\"rank\") | join(\", \") }}",
            "map",
            "\"rank\"",
            "site.pages | selectattr(\"rank\", \"defined\") | map(attribute=\"rank\")",
        ),
    ] {
        let built = site(&[
            pages[0],
            pages[1],
            pages[2],
            NO_HOME,
            ("templates/page.html", template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        let rendered = built.render("post-1.md");
        assert_eq!(
            rendered
                .problems
                .iter()
                .map(|p| &p.kind)
                .collect::<Vec<_>>(),
            [&Kind::NoValue {
                written: "rank".into(),
                lack: Lack::ByField(Box::new(crate::problem::ByField {
                    item: "content/post-2.md".into(),
                    field: "rank".into(),
                    filter: filter.into(),
                })),
                instead: Instead::Selected(selected.into()),
            }],
            "{template}"
        );
        assert_eq!(
            built.told(&rendered.problems),
            [("no-value", "templates/page.html".into(), at.into())],
            "{template}"
        );
    }
    let given = "{{ site.pages | map(attribute=\"rank\", default=0) | join(\", \") }}";
    let built = site(&[
        pages[0],
        pages[1],
        pages[2],
        NO_HOME,
        ("templates/page.html", given),
    ]);
    assert_eq!(built.html("post-1.md"), "2, 0, 1");
}

#[test]
fn sort_puts_items_lacking_the_field_last_in_their_own_order_and_warns() {
    let pages = [
        ("content/post-1.md", "---\ntitle: Post 1\nrank: 2\n---\n"),
        ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
        ("content/post-3.md", "---\ntitle: Post 3\nrank: 1\n---\n"),
        ("content/post-4.md", "---\ntitle: Post 4\n---\n"),
    ];
    let titles = "| map(attribute=\"title\") | join(\", \") }}";
    let warned = |count, first: &str, pages, instead: &str| {
        Some(Kind::SortedLast {
            field: "rank".into(),
            count,
            first: match pages {
                true => Named::File(first.into()),
                false => Named::Item(first.into()),
            },
            instead: Some(instead.into()),
        })
    };
    for (template, html, warning) in [
        (
            format!("{{{{ site.pages | sort(attribute=\"rank\") {titles}"),
            "Post 3, Post 1, Post 2, Post 4",
            warned(
                2,
                "content/post-2.md",
                true,
                "site.pages | selectattr(\"rank\", \"defined\") | sort(attribute=\"rank\")",
            ),
        ),
        (
            format!("{{{{ site.pages | sort(attribute=\"rank\", reverse=true) {titles}"),
            "Post 1, Post 3, Post 2, Post 4",
            warned(
                2,
                "content/post-2.md",
                true,
                "site.pages | selectattr(\"rank\", \"defined\") \
                 | sort(attribute=\"rank\", reverse=true)",
            ),
        ),
        (
            format!("{{{{ site.pages | sort(attribute=\"title, rank\") {titles}"),
            "Post 1, Post 3, Post 2, Post 4",
            warned(
                2,
                "content/post-2.md",
                true,
                "site.pages | selectattr(\"rank\", \"defined\") | sort(attribute=\"title, rank\")",
            ),
        ),
        (
            format!(
                "{{{{ site.pages | rejectattr(\"rank\", \"defined\") \
                 | sort(attribute=\"rank\", reverse=true) {titles}"
            ),
            "Post 2, Post 4",
            warned(
                2,
                "content/post-2.md",
                true,
                "site.pages | rejectattr(\"rank\", \"defined\") | selectattr(\"rank\", \"defined\") \
                 | sort(attribute=\"rank\", reverse=true)",
            ),
        ),
        (
            "{{ [{\"n\": \"a\", \"rank\": 2}, {\"n\": \"b\"}, {\"n\": \"c\", \"rank\": 1}] \
             | sort(attribute=\"rank\") | map(attribute=\"n\") | join(\", \") }}"
                .into(),
            "c, a, b",
            warned(
                1,
                "item 2",
                false,
                "[{\"n\": \"a\", \"rank\": 2}, {\"n\": \"b\"}, {\"n\": \"c\", \"rank\": 1}] \
                 | selectattr(\"rank\", \"defined\") | sort(attribute=\"rank\")",
            ),
        ),
        (
            format!(
                "{{{{ site.pages | selectattr(\"rank\", \"defined\") \
                 | sort(attribute=\"rank\") {titles}"
            ),
            "Post 3, Post 1",
            None,
        ),
        (
            format!(
                "{{% set ranked = site.pages | selectattr(\"rank\", \"defined\") \
                 | sort(attribute=\"rank\") %}}\
                 {{{{ (ranked + (site.pages | rejectattr(\"rank\", \"defined\"))) {titles}"
            ),
            "Post 3, Post 1, Post 2, Post 4",
            None,
        ),
    ] {
        let built = site(&[
            pages[0],
            pages[1],
            pages[2],
            pages[3],
            NO_HOME,
            ("templates/page.html", &template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        let rendered = built.render("post-1.md");
        assert_eq!(rendered.html.as_deref(), Some(html), "{template}");
        assert_eq!(
            rendered
                .problems
                .iter()
                .map(|p| &p.kind)
                .collect::<Vec<_>>(),
            warning.iter().collect::<Vec<_>>(),
            "{template}"
        );
        if warning.is_some() {
            let at = match template.contains("title, rank") {
                true => "\"title, rank\"",
                false => "\"rank\"",
            };
            assert_eq!(
                built.told(&rendered.problems),
                [("sorted-last", "templates/page.html".into(), at.into())],
                "{template}"
            );
        }
    }
}

#[test]
fn a_sort_warning_is_told_once_for_every_page_built_through_its_template() {
    let temp = TempSite::new(&[
        ("content/post-1.md", "---\ntitle: Post 1\nrank: 2\n---\n"),
        ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
        ("content/post-3.md", "---\ntitle: Post 3\nrank: 1\n---\n"),
        NO_HOME,
        (
            "templates/page.html",
            "{% for p in site.pages | sort(attribute=\"rank\") %}{{ p.title }}{% endfor %}\
             {{ site.pages | sort(attribute=\"title, rank\") | length }}",
        ),
    ]);
    let outcome = crate::build::run(&crate::build::Options {
        mode: crate::build::Mode::Check,
        site: temp.root.clone(),
        out: None,
        today: Some(crate::date::Date::day_only(2026, 10, 4)),
        drafts: false,
        url: None,
    });
    let sorted_last: Vec<&Problem> = outcome
        .problems
        .iter()
        .filter(|p| p.kind.name() == "sorted-last")
        .collect();
    assert_eq!(sorted_last.len(), 2, "{:?}", outcome.problems);
    assert!(!outcome.stopped(), "{:?}", outcome.problems);
}

#[test]
fn a_name_nothing_has_is_told_when_the_template_is_read() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "templates/page.html",
            "{{ page.titel }}{{ page.title | uper }}{{ posts }}{{ crad(page) }}{% if page.foo is defined %}{{ page.foo }}{% endif %}{{ page.bar | default('') }}",
        ),
        ("snippets/card.html", "{{ post.title }}"),
    ]);
    let told: Vec<_> = built
        .problems
        .iter()
        .map(|p| match &p.kind {
            Kind::UnknownWord {
                word,
                written,
                nearest,
            } => (*word, written.clone(), nearest.clone()),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        told,
        [
            (Word::Field, "page.titel".into(), Some("page.title".into())),
            (Word::Filter, "uper".into(), Some("upper".into())),
            (Word::Name, "posts".into(), None),
            (Word::Snippet, "crad".into(), Some("card".into())),
        ]
    );
}

/// What reading `template` as a site's page layout told, where a post has
/// a `subtitle` and a `top`.
fn guarded(template: &str) -> Vec<(Kind, String)> {
    let built = site(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsubtitle: One\ntop: 1\n---\n",
        ),
        ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
        ("templates/page.html", template),
    ]);
    let told = built.template_problems();
    built
        .problems
        .iter()
        .zip(told)
        .map(|(problem, (_, _, at))| (problem.kind.clone(), at))
        .collect()
}

fn always_false(name: &str, meant: &str) -> Kind {
    Kind::AlwaysFalse {
        name: name.into(),
        meant: meant.into(),
    }
}

#[test]
fn a_guard_one_slip_from_a_field_is_told_always_false() {
    for (written, at) in [
        ("subtitel", "page.subtitel"),
        ("subtitlee", "page.subtitlee"),
        ("subtile", "page.subtile"),
        ("subtitke", "page.subtitke"),
        ("Subtitel", "page.Subtitel"),
    ] {
        let template = format!(
            "{{% if page.{written} is defined %}}<p>{{{{ page.{written} }}}}</p>{{% endif %}}"
        );
        assert_eq!(
            guarded(&template),
            [(always_false(written, "page.subtitle"), at.to_owned())],
            "{written}"
        );
    }
    assert_eq!(
        guarded("{% if page.subtitel is not defined %}<p>None</p>{% endif %}"),
        [(
            always_false("subtitel", "page.subtitle"),
            "page.subtitel".into()
        )]
    );
    assert_eq!(
        guarded(
            "{% for post in site.pages %}{% if post.subtitel is defined %}\
             {{ post.subtitel }}{% endif %}{% endfor %}"
        ),
        [(
            always_false("subtitel", "post.subtitle"),
            "post.subtitel".into()
        )]
    );
}

#[test]
fn a_guard_further_than_one_slip_or_asking_without_insisting_is_silent() {
    for template in [
        "{% if page.sbutitlle is defined %}{{ page.sbutitlle }}{% endif %}",
        "{% if page.tpo is defined %}{{ page.tpo }}{% endif %}",
        "{% if page.subtitel %}{{ page.subtitel }}{% endif %}",
        "{{ page.subtitel | default('') }}",
        "{% if page.subtitel and page.title %}{{ page.title }}{% endif %}",
        "{% if page.subtitle is defined %}{{ page.subtitle }}{% endif %}",
    ] {
        assert_eq!(guarded(template), [], "{template}");
    }
}

#[test]
fn a_declared_field_is_one_a_page_can_have() {
    let built = site(&[
        ("statac.yaml", "language: en\nfields.subtitel: text\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsubtitle: One\n---\n",
        ),
        (
            "templates/page.html",
            "{% if page.subtitel is defined %}{{ page.subtitel }}{% endif %}",
        ),
    ]);
    assert_eq!(built.template_problems(), []);
}

#[test]
fn a_dates_weekday_is_a_field_the_check_accepts_and_its_value_the_days_name() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-03-04\n---\n",
        ),
        (
            "templates/page.html",
            "{{ page.date.weekday }} {% if page.date.weekday is defined %}yes{% endif %}",
        ),
    ]);
    assert_eq!(built.template_problems(), []);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "Wednesday yes");
}

#[test]
fn a_template_that_cant_be_read_is_told_where_it_stops() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "<p>{% if page.title %}</p>"),
    ]);
    assert_eq!(built.template_problems().len(), 1);
    assert!(matches!(
        &built.problems[0].kind,
        Kind::MalformedTemplate {
            found: TemplateFault::Syntax(_)
        }
    ));
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert!(rendered.problems.is_empty());
}

#[test]
fn a_template_that_cant_be_read_is_told_once_and_not_at_what_uses_it() {
    let broken = "{% if page.title %}<p>";
    let page = "{% extends \"base\" %}{% block main %}{{ page.content }}{% endblock %}{% block aside %}{% endblock %}";
    let calls = "{% extends \"base\" %}{% block main %}{{ page.content }}{{ card(pots=1) }}{{ card(1, 2, 3) }}{% endblock %}";
    let plain = "---\ntitle: Post 1\n---\n";
    let calling = "---\ntitle: Post 1\n---\n@card(pots=1)\n\n@card()\nwrapped\n@end\n";
    let cases: [(&[(&str, &str)], &str); 4] = [
        (
            &[
                ("templates/base.html", broken),
                ("templates/page.html", page),
                ("content/post-1.md", plain),
            ],
            "templates/base.html",
        ),
        (
            &[
                ("templates/base.html", broken),
                (
                    "templates/middle.html",
                    "{% extends \"base\" %}{% block main %}{% endblock %}",
                ),
                ("templates/page.html", &page.replace("base", "middle")),
                ("content/post-1.md", plain),
            ],
            "templates/base.html",
        ),
        (
            &[
                BASE,
                ("templates/page.html", calls),
                (
                    "snippets/card.html",
                    &format!("{{# takes photo #}}{{{{ photo }}}}{broken}"),
                ),
                ("content/post-1.md", calling),
            ],
            "snippets/card.html",
        ),
        (
            &[
                BASE,
                PAGE,
                ("snippets/card.html", &format!("{{{{ body }}}}{broken}")),
                ("content/post-1.md", calling),
            ],
            "snippets/card.html",
        ),
    ];
    for (files, at) in cases {
        let built = site(files);
        let told: Vec<_> = built
            .template_problems()
            .into_iter()
            .map(|(name, file, _)| (name, file))
            .collect();
        assert_eq!(told, [("malformed-template", at.to_owned())], "{at}");
        assert_eq!(built.render("post-1.md").problems, [], "{at}");
    }
}

#[test]
fn a_block_the_extended_template_hasnt_got_is_told() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        BASE,
        (
            "templates/page.html",
            "{% extends \"base\" %}{% block content %}{{ page.content }}{% endblock %}",
        ),
    ]);
    assert_eq!(
        built.template_problems(),
        [(
            "text-never-shown",
            "templates/page.html".into(),
            "content".into()
        )]
    );
    assert_eq!(
        built.problems[0].kind,
        Kind::TextNeverShown {
            extended: "templates/base.html".into(),
            block: "content".into(),
            nearest: None,
        }
    );
}

const WRITTEN: (&str, &str) = (
    "content/post-1.md",
    "---\ntitle: Post 1\n---\n## Heading\n\nPost 1 says this.\n",
);

/// The kinds of problem building `post-1.md` tells.
fn kinds_building_post_1(files: &[(&str, &str)]) -> Vec<Kind> {
    let rendered = site(files).render("post-1.md");
    rendered.problems.into_iter().map(|p| p.kind).collect()
}

#[test]
fn a_layout_that_reads_none_of_a_pages_writing_is_told_at_its_head() {
    let built = site(&[
        WRITTEN,
        ("templates/page.html", "<h1>{{ page.title }}</h1>\n<hr>\n"),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html.as_deref(), Some("<h1>Post 1</h1>\n<hr>\n"));
    assert_eq!(
        rendered.problems[0].kind,
        Kind::WritingNeverShown {
            layout: "templates/page.html".into(),
            pages: 1,
        }
    );
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "writing-never-shown",
            "templates/page.html".into(),
            "<h1>{{ page.title }}</h1>".into()
        )]
    );
    assert_eq!(
        built.context(&rendered.problems[0]),
        [(
            Relation::Building(None),
            "content/post-1.md".into(),
            "---".into()
        )]
    );
}

/// `templates/page.html` told to show none of one page's writing.
fn hollow() -> Kind {
    Kind::WritingNeverShown {
        layout: "templates/page.html".into(),
        pages: 1,
    }
}

#[test]
fn a_layout_asking_for_the_writing_or_its_excerpt_shows_it() {
    for layout in [
        "{{ page.content }}",
        "{{ page.excerpt }}",
        "{% extends \"base\" %}{% block main %}{{ page.content }}{% endblock %}",
        "{% extends \"shows\" %}{% block main %}{{ page.title }}{% endblock %}",
        "{{ written(post=page) }}",
        "{% for post in site.pages %}{{ post.excerpt }}{% endfor %}",
    ] {
        let kinds = kinds_building_post_1(&[
            NO_HOME,
            WRITTEN,
            ("templates/page.html", layout),
            BASE,
            (
                "templates/shows.html",
                "{% block main %}{% endblock %}{{ page.content }}",
            ),
            ("snippets/written.html", "{{ post.content }}"),
        ]);
        assert_eq!(kinds, [], "{layout}");
    }
}

#[test]
fn a_layout_reading_the_writing_without_asking_for_it_shows_none() {
    for layout in [
        "{{ page.opening }}",
        "{{ page.words }}",
        "{{ page.maths }}",
        "{{ page.excerpt_marked }}",
        "{{ page.toc | length }}",
        "{% for post in site.pages %}{{ post.title }}{% endfor %}",
    ] {
        let kinds = kinds_building_post_1(&[NO_HOME, WRITTEN, ("templates/page.html", layout)]);
        assert_eq!(kinds, [hollow()], "{layout}");
    }
}

#[test]
fn a_base_reading_the_opening_for_a_description_shows_no_writing() {
    let base = (
        "templates/base.html",
        "<meta name=\"description\" content=\"{{ page.summary | default(page.opening) }}\">\
         {% block main %}{% endblock %}",
    );
    let extending =
        |main: &str| format!("{{% extends \"base\" %}}{{% block main %}}{main}{{% endblock %}}");
    let titled = extending("{{ page.title }}");
    let kinds = kinds_building_post_1(&[WRITTEN, base, ("templates/page.html", &titled)]);
    assert_eq!(kinds, [hollow()]);
    let excerpted = extending("{{ page.excerpt }}");
    let kinds = kinds_building_post_1(&[WRITTEN, base, ("templates/page.html", &excerpted)]);
    assert_eq!(kinds, []);
}

#[test]
fn a_layout_whose_templates_read_none_of_the_writing_is_the_one_told() {
    let kinds = kinds_building_post_1(&[
        WRITTEN,
        BASE,
        (
            "templates/page.html",
            "{% extends \"base\" %}{% block main %}{{ page.title }}{% endblock %}",
        ),
    ]);
    assert_eq!(kinds, [hollow()]);
}

#[test]
fn a_page_with_no_writing_is_never_told_of_a_layout_that_shows_none() {
    // The comment left open is told of itself.
    let open = Kind::UnmatchedTag {
        unmatched: crate::problem::Unmatched::CommentNeverClosed,
    };
    for (page, told) in [
        ("---\ntitle: Post 1\n---\n", None),
        ("---\ntitle: Post 1\n---\n\n  \n", None),
        (
            "---\ntitle: Post 1\n---\n<!-- To come. -->\n\n<!--\nAnd more.\n-->\n",
            None,
        ),
        ("---\ntitle: Post 1\n---\n<!-- Never closed.\n", Some(open)),
    ] {
        let kinds = kinds_building_post_1(&[
            ("content/post-1.md", page),
            ("templates/page.html", "{{ page.title }}"),
        ]);
        assert_eq!(kinds, Vec::from_iter(told), "{page}");
    }
}

#[test]
fn writing_after_a_comment_is_writing() {
    let kinds = kinds_building_post_1(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n<!-- To come. -->\nPost 1 says this.\n",
        ),
        ("templates/page.html", "{{ page.title }}"),
    ]);
    assert_eq!(kinds, [hollow()]);
}

#[test]
fn a_page_whose_layout_stopped_is_told_of_that_alone() {
    let built = site(&[WRITTEN, ("templates/page.html", "{{ page.summary }}")]);
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert_eq!(rendered.problems.len(), 1, "{:?}", rendered.problems);
    assert_ne!(rendered.problems[0].kind.name(), "writing-never-shown");
}

#[test]
fn an_empty_value_is_never_published() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\nsubtitle:\n---\n"),
        ("templates/page.html", "{{ page.subtitle }}"),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        rendered.problems[0].kind,
        Kind::NoValue {
            written: "page.subtitle".into(),
            lack: Lack::Empty,
            instead: Instead::Default("page.subtitle | default(\"\")".into()),
        }
    );
}

#[test]
fn the_site_answers_with_the_host_its_address_names() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\nurl: https://Example.org:8080/blog/\n",
        ),
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "{{ site.host }}"),
    ]);
    assert_eq!(built.html("post-1.md"), "example.org");
    let rendered = built.render("post-1.md");
    let url = Key::Setting(built.renderer.name_id("url"));
    assert!(rendered.reads.keys().contains(&url));

    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "{{ site.host | default('none') }}"),
    ]);
    assert_eq!(built.html("post-1.md"), "none");
}

#[test]
fn the_site_says_whether_it_takes_webmentions() {
    const ASKS: &str = "{% if site.webmentions %}on{% else %}off{% endif %}";
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\nurl: https://example.org/\n\
             webmentions: true\n",
        ),
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", ASKS),
    ]);
    assert_eq!(built.html("post-1.md"), "on");
    let rendered = built.render("post-1.md");
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
    let webmentions = Key::Setting(built.renderer.name_id("webmentions"));
    assert!(rendered.reads.keys().contains(&webmentions));

    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", ASKS),
    ]);
    assert_eq!(built.html("post-1.md"), "off");
    let rendered = built.render("post-1.md");
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
}

/// A page's layout and its writing each asking for search boxes: the page
/// has one first box and no two with one id, wherever the layout shows the
/// writing; another page showing the writing shows it numbered from the
/// first.
#[test]
fn a_pages_search_boxes_are_counted_across_its_layout_and_its_writing() {
    const BOX: &str =
        "{% set box = page.search_box() %}[{{ box.id }}{% if box.first %} first{% endif %}]";
    let shown = |layout: &str| {
        let built = site(&[
            (
                "content/post-1.md",
                "---\ntitle: Post 1\n---\n@find()\n\n@find()\n",
            ),
            ("content/index.md", "---\nlayout: all\n---\n"),
            ("snippets/find.html", BOX),
            ("templates/page.html", layout),
            (
                "templates/all.html",
                "{{ find() }}{% for post in site.pages %}{{ post.content }}{% endfor %}",
            ),
        ]);
        let own = built.render("post-1.md").html.expect("built");
        (own, built.html("index.md"))
    };
    let (own, list) = shown("{{ find() }}|{{ page.content }}|{{ find() }}");
    assert_eq!(
        own,
        "[statac-search first]|[statac-search-2]\n[statac-search-3]|[statac-search-4]"
    );
    assert_eq!(
        list,
        "[statac-search first][statac-search first]\n[statac-search-2]"
    );
    let (own, _) = shown("{{ page.content }}|{{ find() }}");
    assert_eq!(
        own,
        "[statac-search first]\n[statac-search-2]|[statac-search-3]"
    );
    // Shown twice, the writing is the same both times, and counted once.
    let (own, _) = shown("{{ find() }}|{{ page.content }}|{{ page.content }}|{{ find() }}");
    assert_eq!(
        own,
        "[statac-search first]|[statac-search-2]\n[statac-search-3]|\
         [statac-search-2]\n[statac-search-3]|[statac-search-4]"
    );
    // A layout that shows none of the writing counts none of its boxes.
    let (own, _) = shown("{% set words = page.words %}{{ find() }}");
    assert_eq!(own, "[statac-search first]");
}

#[test]
fn a_page_numbers_the_search_boxes_it_is_asked_for() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n@find()\n"),
        (
            "templates/page.html",
            "{% set a = page.search_box() %}{% set b = page.search_box() %}\
             {{ a.id }}{% if a.first %} first{% endif %} \
             {{ b.id }}{% if b.first %} first{% endif %} | {{ page.content }}",
        ),
        (
            "snippets/find.html",
            "{% set box = page.search_box() %}{{ box.id }}{% if box.first %} first{% endif %}",
        ),
        ("content/post-2.md", "---\nlayout: given\n---\n"),
        ("templates/given.html", "{{ page.search_box(1) }}"),
        ("content/post-3.md", "---\nlayout: bare\n---\n"),
        ("templates/bare.html", "{{ page.search_box }}"),
    ]);
    // The writing's box is numbered after the two the layout asked for
    // before showing it.
    assert_eq!(
        built.html("post-1.md"),
        "statac-search first statac-search-2 | statac-search-3"
    );
    let stopped = |path: &str| built.render(path).problems[0].kind.clone();
    assert_eq!(
        stopped("post-2.md"),
        Kind::WrongArgument {
            snippet: "page.search_box".into(),
            fault: ArgumentFault::NeedsNames { wanted: Vec::new() },
        }
    );
    assert_eq!(
        stopped("post-3.md"),
        Kind::WrongArgument {
            snippet: "page.search_box".into(),
            fault: ArgumentFault::Needs(Needed::Brackets),
        }
    );
}

#[test]
fn a_lists_feed_is_answered_by_the_feeds_the_build_publishes() {
    let temp = TempSite::new(&[
        (
            "statac.yaml",
            "language: en\ntitle: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n",
        ),
        ("content/index.md", "# Home\n"),
        ("content/post-1.md", "---\ndate: 2026-10-04\n---\n"),
        ("templates/home.html", "{{ page.list.feed }}"),
        ("templates/page.html", ""),
        ("templates/feed.xml", ""),
    ]);
    let run = crate::content::Run {
        today: Some(crate::date::Date::day_only(2026, 10, 4)),
        drafts: false,
        url: None,
    };
    let home = |published: bool| {
        let (mut site, assets, feeds) = crate::build::read(&temp.root, &run);
        assert_eq!(feeds.len(), 1);
        let feeds = if published { feeds } else { Vec::new() };
        let cards = crate::cards::Cards::default();
        let (templates, _) = read(
            &temp.root,
            &mut site.sources,
            &site.site,
            &site.settings,
            cards,
        );
        let renderer = Renderer::new(
            site.sources,
            site.settings,
            site.site,
            assets,
            templates,
            site.dates,
            feeds,
        );
        let (home, _) = renderer
            .site()
            .pages
            .iter()
            .find(|(_, page)| page.address.as_str() == "/")
            .expect("a home page");
        renderer.render(home).html.expect("built")
    };
    // Templates read the feeds the build decided on once, rather than
    // working each out again over every page of the site.
    assert_eq!(home(true), "/feed.xml");
    assert_eq!(home(false), "");
}

#[test]
fn the_code_stylesheet_and_the_feeds_are_named_by_their_addresses() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntitle: Site\nurl: https://example.org/blog/\nfeed_key: key\ntimezone: UTC\ncode_style: one-half\n",
        ),
        ("content/posts/post-1.md", "---\ndate: 2026-10-04\n---\n"),
        ("content/posts/index.md", "---\nfeed: atom.xml\n---\n"),
        (
            "templates/base.html",
            "{{ css('code.css') }}|{{ site.feed }}|{% if page.list is defined %}{{ page.list.feed }}{% endif %}",
        ),
        ("templates/page.html", "{% extends \"base\" %}"),
        ("templates/folder.html", "{% extends \"base\" %}"),
        ("templates/feed.xml", "{{ list.feed }}"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let html = built.html("posts/index.md");
    let parts: Vec<&str> = html.split('|').collect();
    assert!(
        parts[0].starts_with("<link rel=\"stylesheet\" href=\"/blog/code."),
        "{}",
        parts[0]
    );
    assert_eq!(&parts[1..], ["/blog/feed.xml", "/blog/posts/atom.xml"]);
    let reads = built.render("posts/index.md").reads;
    assert!(reads.keys().iter().any(|k| matches!(k, Key::Asset(_))));
    let face = built.id("posts/index.md");
    let feed = built.renderer.name_id("feed");
    assert!(reads.keys().contains(&Key::Field(face, feed)));
    // A page sending readers on gives its list no feed.
    let sends_on = built.renderer.name_id("redirect_to");
    assert!(reads.keys().contains(&Key::Field(face, sends_on)));
    let key = built.renderer.name_id("feed_key");
    assert!(reads.keys().contains(&Key::Setting(key)));
}

#[test]
fn a_feed_carries_code_uncoloured_where_its_page_colours_it() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntitle: Site\nurl: https://example.org\nfeed_key: key\ntimezone: UTC\n",
        ),
        ("content/index.md", "# Home\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\n---\n```rust\nlet a = 1 < 2;\n```\n",
        ),
        BASE,
        PAGE,
        ("templates/home.html", ""),
        (
            "templates/feed.xml",
            "{% for post in posts %}{{ post.content }}{% endfor %}|{{ 'fn f() {}' | highlight('rust') }}",
        ),
    ]);
    assert!(built.html("post-1.md").contains("<span class=\"hl-"));
    let feed = built.renderer.render_feed(&built.renderer.feeds()[0]);
    assert!(feed.problems.is_empty(), "{:?}", feed.problems);
    assert!(feed.coloured.is_empty());
    assert_eq!(
        feed.html.unwrap(),
        "<pre class=\"code\"><code class=\"language-rust\">let a = 1 &lt; 2;</code></pre>|fn f() {}"
    );
}

#[test]
fn code_is_coloured_in_the_sites_style_and_a_language_it_lacks_is_told_at_its_word() {
    let built = site(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n\n```rust\nfn main() {}\n```\n\n\
             - item\n\n  ~~~ cosole\n  $ ls\n  ~~~\n\n```text\nas it is\n```\n\n\
             ```\"An example\"\nas it is\n```\n\n```'An example'\nas it is\n```\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\n---\n\n```txt\nas it is\n```\n",
        ),
        BASE,
        PAGE,
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "unknown-code-language",
            "content/post-1.md".into(),
            "cosole".into()
        )]
    );
    assert_eq!(
        rendered.problems[0].kind,
        Kind::UnknownCodeLanguage {
            written: "cosole".into(),
            nearest: Some("console".into()),
        }
    );
    let html = rendered.html.unwrap();
    assert!(
        html.contains("<code class=\"language-rust\"><span class=\"hl-"),
        "{html}"
    );
    assert!(html.contains("<code class=\"language-cosole\">$ ls</code>"));
    assert!(html.contains("<code class=\"language-text\">as it is</code>"));
    // The classes follow the style, so a page with coloured code reads it.
    let style = built.renderer.name_id("code_style");
    assert!(rendered.reads.keys().contains(&Key::Setting(style)));
    let plain = built.render("post-2.md");
    assert!(plain.problems.is_empty());
    assert!(!plain.reads.keys().contains(&Key::Setting(style)));
}

#[test]
fn code_style_none_colours_nothing_tells_of_no_language_and_asks_for_no_stylesheet() {
    let built = site(&[
        ("statac.yaml", "title: Site\ncode_style: none\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n\n```rust\nfn main() {}\n```\n\n~~~ cosole\n$ ls\n~~~\n",
        ),
        (
            "templates/base.html",
            "<head>{{ css(\"code.css\") }}</head>{% block main %}{% endblock %}\
             <i>{{ \"let a\" | highlight(\"rust\") }}{{ \"b\" | highlight(\"cosole\") }}</i>",
        ),
        PAGE,
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(built.told(&rendered.problems), []);
    let html = rendered.html.unwrap();
    assert!(html.contains("<head></head>"), "{html}");
    assert!(html.contains("<code class=\"language-rust\">fn main() {}</code>"));
    assert!(html.contains("<code class=\"language-cosole\">$ ls</code>"));
    assert!(html.contains("<i>let ab</i>"), "{html}");
    let style = built.renderer.name_id("code_style");
    assert!(rendered.reads.keys().contains(&Key::Setting(style)));
}

#[test]
fn a_fence_opening_with_a_files_name_is_coloured_by_it_where_its_ending_names_a_language() {
    let built = site(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n\n```post-1.rs\nfn main() {}\n```\n\n\
             ```.gitignore\n/built\n```\n\n```Makefile\nall:\n```\n",
        ),
        BASE,
        PAGE,
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "unknown-code-language",
            "content/post-1.md".into(),
            ".gitignore".into()
        )]
    );
    let html = rendered.html.unwrap();
    assert!(
        html.contains(
            "<figure class=\"code\"><figcaption>post-1.rs</figcaption>\
             <pre><code class=\"language-rs\"><span class=\"hl-"
        ),
        "{html}"
    );
    assert!(
        html.contains(
            "<pre class=\"code\"><code class=\"language-.gitignore\">/built</code></pre>"
        ),
        "{html}"
    );
    assert!(
        html.contains("<pre class=\"code\"><code class=\"language-Makefile\"><span class=\"hl-"),
        "{html}"
    );
}

#[test]
fn the_table_of_contents_lists_the_headings_each_holding_those_beneath() {
    let built = site(&[
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n@toc()\n\n## One\n\n### One a\n\n## Two\n\n## One\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\n---\nNo headings.\n",
        ),
        (
            "content/post-3.md",
            "---\ntitle: Post 3\nlayout: other\n---\n# Привет\n",
        ),
        (
            "snippets/toc.html",
            "{% for s in page.toc recursive %}[{{ s.level }} {{ s.url }} {{ s.title }}{% if s.headings %}:{{ loop(s.headings) }}{% endif %}]{% endfor %}",
        ),
        (
            "templates/page.html",
            "{% if page.toc %}{{ page.toc | length }}{% endif %}{{ page.content }}",
        ),
        (
            "templates/other.html",
            "{{ page.toc[0].id }} {{ page.toc[0].url }}{% set shown = page.content %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "3[2 #one One:[3 #one-a One a]][2 #two Two][2 #one-1 One]\n\
         <h2 id=\"one\">One</h2>\n<h3 id=\"one-a\">One a</h3>\n\
         <h2 id=\"two\">Two</h2>\n<h2 id=\"one-1\">One</h2>"
    );
    assert_eq!(built.html("post-2.md"), "<p>No headings.</p>");
    assert_eq!(
        built.html("post-3.md"),
        "привет #%D0%BF%D1%80%D0%B8%D0%B2%D0%B5%D1%82"
    );
    let post = built.id("post-3.md");
    assert!(
        built
            .render("post-3.md")
            .reads
            .keys()
            .contains(&Key::Body(post))
    );
}

/// A list reads each post's excerpt, opening and words as a read of that
/// post's writing.
#[test]
fn a_posts_excerpt_opening_and_words_are_read_from_its_writing() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\n---\nOne *two*.\n\n<!--more-->\n\nThree.\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\n---\n![A picture](a.jpg)\n\nFour five.\n\nSix.\n",
        ),
        (
            "content/post-3.md",
            "---\ntitle: Post 3\ndate: 2026-10-03\n---\n- Seven.\n",
        ),
        (
            "templates/home.html",
            "{% for post in page.posts | sort(attribute='title') %}[{{ post.excerpt | default('none') }}|{{ post.excerpt_marked }}|{{ post.opening | default('none') }}|{{ post.words }}]{% endfor %}",
        ),
        ("templates/page.html", "{{ page.content }}"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let rendered = built.render("index.md");
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
    assert_eq!(
        rendered.html.expect("built"),
        "[<p>One <em>two</em>.</p>|True|One two.|3]\
         [<p>Four five.</p>|False|Four five.|3]\
         [none|False|none|1]"
    );
    for post in ["post-1.md", "post-2.md", "post-3.md"] {
        assert!(rendered.reads.keys().contains(&Key::Body(built.id(post))));
    }
}

/// Writing shown on another page keeps what each address in it means: one
/// read from the writing's own page, in Markdown, in HTML or written by a
/// snippet the writing calls, is spelled from the top of the site there,
/// and that page reads where the writing is published. On its own page
/// the writing is as built.
#[test]
fn writing_shown_elsewhere_keeps_what_its_addresses_mean() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\nurl: https://example.org/blog/\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/posts/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\n---\n\
             ![A](photo-1.jpg) [Post 2](../post-2/), [the note](#note), \
             <a href=\"a.pdf\">a</a>, [home](/blog/) @figure('b.jpg')\n\n## Note {#note}\n",
        ),
        (
            "content/posts/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\n---\n@first()\n",
        ),
        ("snippets/figure.html", "<img src=\"{{ photo }}\" alt=\"\">"),
        (
            "snippets/first.html",
            "{% for post in site.posts if post.title == 'Post 1' %}{{ post.excerpt }}{% endfor %}",
        ),
        (
            "templates/home.html",
            "{% for post in site.posts if post.title == 'Post 1' %}{{ post.content }}{% endfor %}",
        ),
        ("templates/page.html", "{{ page.content }}"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let written = |at: &str, a: &str, b: &str| {
        format!(
            "<p><img src=\"{at}photo-1.jpg\" alt=\"A\" loading=\"lazy\" decoding=\"async\"> \
             <a href=\"{a}\">Post 2</a>, <a href=\"{b}\">the note</a>, \
             <a href=\"{at}a.pdf\">a</a>, <a href=\"/blog/\">home</a> <img src=\"{at}b.jpg\" alt=\"\"></p>"
        )
    };
    let own = written("", "../post-2/", "#note");
    assert_eq!(
        built.html("posts/post-1.md"),
        format!("{own}\n<h2 id=\"note\">Note</h2>")
    );
    let at = "/blog/posts/post-1/";
    let shown = written(at, "/blog/posts/post-2/", "/blog/posts/post-1/#note");
    let home = built.render("index.md");
    assert!(home.problems.is_empty(), "{:?}", home.problems);
    assert_eq!(
        home.html.expect("built"),
        format!("{shown}\n<h2 id=\"note\">Note</h2>")
    );
    let url = |page| Key::Field(page, built.renderer.name_id("url"));
    let post = built.id("posts/post-1.md");
    assert!(home.reads.keys().contains(&url(post)));
    assert!(
        !built
            .render("posts/post-1.md")
            .reads
            .keys()
            .contains(&url(post))
    );
    // Shown in another post's writing, by a snippet it calls.
    let second = built.render("posts/post-2.md");
    assert!(second.problems.is_empty(), "{:?}", second.problems);
    assert_eq!(second.html.expect("built"), shown);
    assert!(second.reads.keys().contains(&url(post)));
}

/// Maths reaches the page and what a list shows of it as MathML;
/// `page.maths` says whether the writing holds any, read from that
/// writing; a heading's maths is listed and named as written. Malformed
/// maths is told at its place, and the page is not published.
#[test]
fn maths_is_mathml_and_page_maths_says_where_there_is_any() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/post-1.md",
            "---\ndate: 2026-10-01\n---\n# The $x^2$ bound\n\nArea $\\pi r^2$.\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\n---\nIt cost $5 and then $10 more.\n",
        ),
        (
            "content/post-3.md",
            "---\ntitle: Post 3\ndate: 2026-10-03\n---\nOne $\\fracc{1}{2}$ here.\n",
        ),
        (
            "templates/home.html",
            "{% for post in page.posts | sort(attribute='date') %}{% if not loop.last %}[{{ headline(post) }}|{{ post.maths }}|{{ post.excerpt }}]{% endif %}{% endfor %}",
        ),
        (
            "templates/page.html",
            "{{ page.maths }}|{% for h in page.toc %}{{ h.title }}{% endfor %}|{{ page.content }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let home = built.render("index.md");
    assert!(home.problems.is_empty(), "{:?}", home.problems);
    assert_eq!(
        home.html.expect("built"),
        "[The x^2 bound|True|<p>Area <math><mi>π</mi><msup><mi>r</mi><mn>2</mn></msup></math>.</p>]\
         [Post 2|False|<p>It cost $5 and then $10 more.</p>]"
    );
    for post in ["post-1.md", "post-2.md"] {
        assert!(home.reads.keys().contains(&Key::Body(built.id(post))));
    }
    assert_eq!(
        built.html("post-1.md"),
        "True|The x^2 bound|<h1 id=\"the-x2-bound\">The <math><msup><mi>x</mi><mn>2</mn></msup></math> bound</h1>\n\
         <p>Area <math><mi>π</mi><msup><mi>r</mi><mn>2</mn></msup></math>.</p>"
    );
    assert!(built.html("post-2.md").starts_with("False||<p>It cost $5"));
    let malformed = built.render("post-3.md");
    assert_eq!(malformed.html, None);
    assert_eq!(
        built.told(&malformed.problems),
        [(
            "malformed-maths",
            "content/post-3.md".to_owned(),
            "\\fracc".to_owned()
        )]
    );
}

/// A value the `markdown` filter reads is told what its writing would be
/// told, at its own place in its file however its YAML is written, with
/// the filter that read it; a value written in no file is told at the
/// filter. Its maths is MathML, as the writing's is.
#[test]
fn a_value_read_as_markdown_is_told_at_its_own_place() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\nsummary: Area $\\fracc{1}{2}$ here\n---\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\nsummary: |\n  One.\n\n  Two $\\fooo$.\n---\n",
        ),
        (
            "content/post-3.md",
            "---\ntitle: Post 3\ndate: 2026-10-03\nsummary: \"A \\\"b\\\" $\\\\fooo$\"\n---\n",
        ),
        (
            "content/post-4.md",
            "---\ntitle: Post 4\ndate: 2026-10-04\nsummary: \"Area \\x01here\"\n---\n",
        ),
        (
            "data/notes.yaml",
            "one: The $\\fooo$ note\ntwo: |\n  A note.\n\n  [^n]: Never shown.\n",
        ),
        (
            "templates/home.html",
            "{% for post in page.posts | sort(attribute='date') %}{{ post.title }}{{ post.summary | markdown }}{% endfor %}",
        ),
        (
            "templates/page.html",
            "{{ page.title }}|{{ site.data.notes.one | markdown }}|{{ \"$\\\\fooo$\" | markdown }}|{{ site.data.notes.two | markdown }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let home = built.render("index.md");
    assert_eq!(
        built.told(&home.problems),
        [
            (
                "malformed-maths",
                "content/post-1.md".to_owned(),
                "\\fracc".to_owned()
            ),
            (
                "malformed-maths",
                "content/post-2.md".to_owned(),
                "\\fooo".to_owned()
            ),
            (
                "malformed-maths",
                "content/post-3.md".to_owned(),
                "\\\\fooo".to_owned()
            ),
            (
                "control-character",
                "content/post-4.md".to_owned(),
                "\\x01".to_owned()
            ),
        ]
    );
    assert_eq!(
        built.context(&home.problems[0]),
        [
            (
                Relation::ReadAsMarkdown,
                "templates/home.html".into(),
                "post.summary | markdown".into()
            ),
            (
                Relation::Building(None),
                "content/index.md".into(),
                "---".into()
            ),
        ]
    );
    let page = built.render("post-4.md");
    assert_eq!(
        built.told(&page.problems),
        [
            (
                "malformed-maths",
                "data/notes.yaml".to_owned(),
                "\\fooo".to_owned()
            ),
            (
                "malformed-maths",
                "templates/page.html".to_owned(),
                "\"$\\\\fooo$\" | markdown".to_owned()
            ),
            (
                "footnote-never-shown",
                "data/notes.yaml".to_owned(),
                "[^n]".to_owned()
            ),
        ]
    );
    assert_eq!(
        built.context(&page.problems[1]),
        [(
            Relation::Building(None),
            "content/post-4.md".into(),
            "---".into()
        )]
    );
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        NO_HOME,
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummary: Area $x^2$\n---\n",
        ),
        ("templates/page.html", "{{ page.summary | markdown }}"),
    ]);
    assert_eq!(
        built.html("post-1.md"),
        "Area <math><msup><mi>x</mi><mn>2</mn></msup></math>"
    );
}

/// What is told only of a page's writing, such as another tool's call or
/// a cell past a table's columns, is not told of a value the `markdown`
/// filter reads. A note written twice
/// is, at the first note in the value's file, pointing at the second.
#[test]
fn a_value_read_as_markdown_is_told_only_what_a_value_is_told() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        NO_HOME,
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummary: \"{{< figure >}} and {% raw %}\"\n---\n",
        ),
        (
            "data/notes.yaml",
            "twice: |\n  A marker.[^n]\n\n  [^n]: The first.\n\n  [^n]: The second.\n\
             table: |\n  | A |\n  |---|\n  | 1 | Extra |\n",
        ),
        (
            "templates/page.html",
            "{{ page.summary | markdown }}|{{ site.data.notes.twice | markdown }}\
             {{ site.data.notes.table | markdown }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let rendered = built.render("post-1.md");
    let html = rendered.html.as_deref().unwrap();
    assert!(
        html.starts_with("{{&lt; figure &gt;}} and {% raw %}|"),
        "{html}"
    );
    assert!(html.contains("<td>1</td>"), "{html}");
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "definition-written-twice",
            "data/notes.yaml".to_owned(),
            "[^n]".to_owned()
        )]
    );
    let sources = built.renderer.sources();
    let first = rendered.problems[0].place.unwrap();
    let shown = rendered.problems[0].related()[0].place.unwrap();
    assert_eq!(sources.spelling(shown), "[^n]");
    assert_eq!(sources.path(shown.source), sources.path(first.source));
    assert!(first.span.start < shown.span.start);
}

/// HTML a value read as Markdown leaves unmatched is told at the tag in
/// the value's file, placed whether the value stands there as it is or
/// is read alone. A value of one paragraph is shown without its `<p>`, so
/// what the paragraph would close is told; in a value of more, the
/// paragraph closes it.
#[test]
fn html_a_value_read_as_markdown_leaves_unmatched_is_told_at_the_tag() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        NO_HOME,
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummary: A summary <span>in words\n---\n",
        ),
        (
            "data/notes.yaml",
            "box: |\n  <div class=\"box\">\n\n  A note in a box.\n\
             two: |\n  One <span>word.\n\n  Two.\n\
             stray: \"One.</section>\"\n\
             closed: \"<em>One</em>.\"\n",
        ),
        (
            "templates/page.html",
            "<p>{{ page.summary | markdown }}</p>{{ site.data.notes.box | markdown }}\
             {{ site.data.notes.two | markdown }}{{ site.data.notes.stray | markdown }}\
             {{ site.data.notes.closed | markdown }}",
        ),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        built.told(&rendered.problems),
        [
            (
                "unmatched-tag",
                "content/post-1.md".to_owned(),
                "<span>".to_owned()
            ),
            (
                "unmatched-tag",
                "data/notes.yaml".to_owned(),
                "<div class=\"box\">".to_owned()
            ),
            (
                "unmatched-tag",
                "data/notes.yaml".to_owned(),
                "</section>".to_owned()
            ),
        ]
    );
    let headlines: Vec<String> = rendered.problems.iter().map(Problem::headline).collect();
    assert_eq!(
        headlines,
        [
            "Unmatched tag: `<span>` is never closed",
            "Unmatched tag: `<div>` is never closed",
            "Unmatched tag: `</section>` closes nothing opened before it",
        ]
    );
    assert!(
        (rendered.html.as_deref().unwrap()).starts_with(
            "<p>A summary <span>in words</p><div class=\"box\">\n<p>A note in a box.</p>"
        ),
        "{:?}",
        rendered.html
    );
}

/// A page whose more marker Statac can't act on is told at the marker in
/// its file, and not published. A second marker is warned of, and the
/// page is published with its excerpt ending at the first.
#[test]
fn a_page_with_a_more_marker_statac_cant_act_on_stops() {
    let built = site(&[
        NO_HOME,
        ("content/post-1.md", "One.\n\n<!--More-->\n\nTwo.\n"),
        ("content/post-2.md", "> One.\n>\n> <!--more-->\n"),
        (
            "content/post-3.md",
            "One.\n\n<!--more-->\n\nTwo.\n\n<!--more-->\n",
        ),
        ("templates/page.html", "{{ page.excerpt }}"),
    ]);
    for (post, name, written) in [
        ("post-1.md", "other-spelling", "<!--More-->"),
        ("post-2.md", "wrong-place", "<!--more-->"),
    ] {
        let rendered = built.render(post);
        assert_eq!(rendered.html, None, "{post}");
        assert_eq!(
            built.told(&rendered.problems),
            [(name, format!("content/{post}"), written.to_owned())]
        );
    }
    let twice = built.render("post-3.md");
    assert_eq!(twice.html.as_deref(), Some("<p>One.</p>"));
    let told = built.told(&twice.problems);
    assert_eq!(
        told,
        [(
            "more-written-twice",
            "content/post-3.md".to_owned(),
            "<!--more-->".to_owned()
        )]
    );
    let place = twice.problems[0].place.unwrap();
    let text = built.renderer.sources().text(place.source);
    assert_eq!(
        place.span.start as usize,
        text.rfind("<!--more-->").unwrap()
    );
}

/// A title taken from the heading the writing opens with is Markdown, so
/// `headline` gives that heading's words, its punctuation set; a title in
/// the front matter is given as written.
#[test]
fn headline_gives_a_heading_title_as_a_reader_reads_it() {
    let built = site(&[
        NO_HOME,
        (
            "content/post-1.md",
            "# CSS Value: `currentColor` isn't *new*\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: \"Don't *panic*\"\n---\n# Heading\n",
        ),
        (
            "templates/page.html",
            "{{ headline(page) }}{% set shown = page.content %}",
        ),
    ]);
    assert_eq!(
        built.html("post-1.md"),
        "CSS Value: currentColor isn\u{2019}t new"
    );
    assert_eq!(built.html("post-2.md"), "Don&#x27;t *panic*");
}

/// `headline` calls a page by the first there is of: its `title`, the
/// site's `title` for the home page, the heading its writing opens with,
/// the name of the list it speaks for, its `summary` or opening words, its
/// date and its address. Only the last four are noted, once for each page.
#[test]
fn headline_calls_a_page_by_the_first_name_there_is() {
    let long = "---\ndate: 2026-01-05\n---\nTwo three four five six seven eight nine ten \
                eleven twelve thirteen, fourteen fifteen.\n";
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntitle: Site\ntimezone: UTC\npages.feed: false\nposts_per_page: 2\n",
        ),
        ("content/index.md", "The home page opens here.\n"),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\ntags: [Rust]\n---\n# Heading\n",
        ),
        (
            "content/post-2.md",
            "---\ndate: 2026-01-02\ntags: [Rust]\n---\n# Post *2*\n",
        ),
        (
            "content/post-3.md",
            "---\ndate: 2026-01-03\ntags: [rust]\n---\nPost 3 opens here.\n\n# Heading\n",
        ),
        (
            "content/post-4.md",
            "---\ndate: 2026-01-04\nsummary: \"Post  4\"\n---\nPost 4 opens here.\n",
        ),
        ("content/post-5.md", long),
        ("content/post-6.md", "---\ndate: 2026-01-06\n---\n"),
        ("content/notes/index.md", "The folder opens here.\n"),
        ("content/notes/note-1.md", "\n"),
        (
            "templates/page.html",
            "{{ headline(page) }}{% set shown = page.content %}",
        ),
        (
            "templates/home.html",
            "{{ headline(page) }}{% set shown = page.content %}",
        ),
        (
            "templates/folder.html",
            "{{ headline(page) }}{% set shown = page.content %}",
        ),
    ]);
    let called = |rendered: Rendered| {
        let notes: Vec<_> = rendered
            .problems
            .iter()
            .map(|problem| match &problem.kind {
                Kind::UntitledPage { by, .. } => *by,
                other => panic!("{other:?}"),
            })
            .collect();
        (rendered.html.expect("built"), notes)
    };
    let home = built.id("index.md");
    for (page, name, by) in [
        (home, "Site", None),
        (built.id("post-1.md"), "Post 1", None),
        (built.id("post-2.md"), "Post 2", None),
        (built.at("/tags/rust/"), "Rust", None),
        (built.id("notes/index.md"), "notes", None),
        (
            built.id("post-3.md"),
            "Post 3 opens here.",
            Some(Called::OpeningWords),
        ),
        (built.id("post-4.md"), "Post 4", Some(Called::Summary)),
        (
            built.id("post-5.md"),
            "Two three four five six seven eight nine ten eleven twelve thirteen\u{2026}",
            Some(Called::OpeningWords),
        ),
        (built.id("post-6.md"), "6 January 2026", Some(Called::Date)),
        (
            built.id("notes/note-1.md"),
            "/notes/note-1/",
            Some(Called::Address),
        ),
    ] {
        assert_eq!(
            called(built.renderer.render(page)),
            (name.to_owned(), Vec::from_iter(by)),
        );
    }
    // A list's later page is called what its first page is.
    assert_eq!(
        called(built.renderer.render_later(home, 2)),
        ("Site".to_owned(), Vec::new())
    );
}

/// A page with `untitled: true`, its own or its folder's, is called by its
/// title or opening heading where it has one, then its date, its `summary`
/// or opening words, and its address, and is never noted, whether a
/// template asks with `headline` or `page.og_title`. `untitled: false` on a
/// page in such a folder is called and noted as any page is.
#[test]
fn a_page_meant_to_go_untitled_is_called_by_its_date_first_and_never_noted() {
    let asks = "{{ headline(page) }}|{{ page.og_title }}{% set shown = page.content %}";
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntitle: Site\ntimezone: UTC\npages.feed: false\n\
             date_format: 31 Dec 2025, 15:04\n",
        ),
        NO_HOME,
        ("content/notes/index.md", "---\npages.untitled: true\n---\n"),
        (
            "content/notes/note-1.md",
            "---\ndate: 2026-01-01\n---\nNote 1 opens here.\n",
        ),
        (
            "content/notes/note-2.md",
            "---\ndate: 2026-01-02T09:05\nsummary: Note 2\n---\nNote 2 opens here.\n",
        ),
        (
            "content/notes/note-3.md",
            "---\ntitle: Note 3\ndate: 2026-01-03\n---\nNote 3 opens here.\n",
        ),
        (
            "content/notes/note-4.md",
            "---\ndate: 2026-01-04\n---\n# Note *4*\n",
        ),
        ("content/notes/note-5.md", "---\nsummary: Note 5\n---\n"),
        ("content/notes/note-6.md", "Note 6 opens here.\n"),
        ("content/notes/note-7.md", "\n"),
        (
            "content/notes/note-8.md",
            "---\ndate: 2026-01-08\nuntitled: false\n---\nNote 8 opens here.\n",
        ),
        (
            "content/post-1.md",
            "---\ndate: 2026-01-09\n---\nPost 1 opens here.\n",
        ),
        (
            "content/post-2.md",
            "---\ndate: 2026-01-10\nuntitled: true\n---\nPost 2 opens here.\n",
        ),
        ("templates/page.html", asks),
        ("templates/folder.html", asks),
    ]);
    for (page, name, by) in [
        ("notes/note-1.md", "1 Jan 2026", None),
        ("notes/note-2.md", "2 Jan 2026, 09:05", None),
        ("notes/note-3.md", "Note 3", None),
        ("notes/note-4.md", "Note 4", None),
        ("notes/note-5.md", "Note 5", None),
        ("notes/note-6.md", "Note 6 opens here.", None),
        ("notes/note-7.md", "/notes/note-7/", None),
        (
            "notes/note-8.md",
            "Note 8 opens here.",
            Some(Called::OpeningWords),
        ),
        ("notes/index.md", "notes", None),
        (
            "post-1.md",
            "Post 1 opens here.",
            Some(Called::OpeningWords),
        ),
        ("post-2.md", "10 Jan 2026", None),
    ] {
        let rendered = built.render(page);
        let notes: Vec<_> = rendered
            .problems
            .iter()
            .map(|problem| match &problem.kind {
                Kind::UntitledPage { by, .. } => *by,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            (rendered.html.as_deref(), notes),
            (Some(format!("{name}|{name}").as_str()), Vec::from_iter(by)),
            "{page}"
        );
    }
}

/// Whether a page is meant to go untitled is read where it has no name
/// of its own, so a change to it builds the page again.
#[test]
fn headline_records_reading_whether_a_page_is_meant_to_go_untitled() {
    let built = site(&[
        NO_HOME,
        ("content/post-1.md", "---\ndate: 2026-01-01\n---\nPost 1.\n"),
        (
            "templates/page.html",
            "{{ headline(page) }}{% set shown = page.content %}",
        ),
    ]);
    let rendered = built.render("post-1.md");
    let key = Key::Field(built.id("post-1.md"), built.renderer.name_id("untitled"));
    assert!(rendered.reads.keys().contains(&key));
}

/// The note names the page and is placed at its first line.
#[test]
fn a_page_called_by_its_opening_words_is_noted_at_its_first_line() {
    let built = site(&[
        NO_HOME,
        ("content/post-1.md", "---\ndate: 2026-01-01\n---\nPost 1.\n"),
        (
            "templates/page.html",
            "{{ headline(page) }}{% set shown = page.content %}",
        ),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        rendered.problems[0].kind,
        Kind::UntitledPage {
            page: Named::File("content/post-1.md".into()),
            by: Called::OpeningWords,
            name: "Post 1.".into(),
        }
    );
    assert_eq!(
        built.told(&rendered.problems),
        [("untitled-page", "content/post-1.md".into(), "---".into())]
    );
}

/// Naming a page by another's writing, or by a list, records what it read.
#[test]
fn headline_records_what_it_names_a_page_by() {
    let built = site(&[
        NO_HOME,
        ("content/post-1.md", "---\ndate: 2026-01-01\n---\nPost 1.\n"),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-01-02\n---\n",
        ),
        ("content/notes/index.md", "\n"),
        ("content/notes/note-1.md", "---\ntitle: Note 1\n---\n"),
        (
            "templates/page.html",
            "{% if page.next %}{{ headline(page.next) }}{% endif %}",
        ),
        ("templates/folder.html", "{{ headline(page) }}"),
    ]);
    let site = built.renderer.site();
    let field = |page, name| Key::Field(page, built.renderer.name_id(name));
    let post = built.id("post-1.md");
    let rendered = built.render("post-2.md");
    assert_eq!(rendered.html.as_deref(), Some("Post 1."));
    for key in [
        field(post, "title"),
        field(post, "summary"),
        Key::Body(post),
    ] {
        assert!(rendered.reads.keys().contains(&key), "{key:?}");
    }
    let folder = built.id("notes/index.md");
    let rendered = built.render("notes/index.md");
    assert_eq!(rendered.html.as_deref(), Some("notes"));
    assert!(rendered.reads.keys().contains(&field(folder, "list")));
    assert!(site.pages.get(folder).speaks_for.is_some());
}

#[test]
fn statacs_filters_and_functions_work_as_named() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\nurl: https://example.org\ndate_format: 31 December 2025\ntimezone: UTC\n\
             pages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-04\ntags: [Rust]\n---\nHello *there*.\n",
        ),
        (
            "content/post-2.md",
            "---\ndate: 2026-10-04 12:00\n---\n# Heading 2\n",
        ),
        ("assets/site.css", "p { margin: 0 }"),
        (
            "templates/page.html",
            "{{ css('site.css') }}|{{ page.url | absolute }}|{{ page.date }}|{{ page.date | moment }}|{{ page.content | plain }}|{{ '**a** b' | markdown }}|{{ 'one two three' | truncate(9, '...') }}|{{ headline(page) }}|{{ (site.posts | newest_first)[0].url }}|{{ site.posts | newest_date }}|{{ site.posts | tagged('rust') | length }}|{{ {'a': [1, true]} | json }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let html = built.html("post-1.md");
    let parts: Vec<&str> = html.split('|').collect();
    assert!(
        parts[0].starts_with("<link rel=\"stylesheet\" href=\"/site."),
        "{}",
        parts[0]
    );
    assert_eq!(
        &parts[1..],
        [
            "https://example.org/post-1/",
            "4 October 2026",
            "2026-10-04T00:00:00+00:00",
            "Hello there.",
            "<strong>a</strong> b",
            "one...",
            "Post 1",
            "/post-2/",
            "4 October 2026",
            "1",
            "{\"a\":[1,true]}",
        ]
    );
    let post = built.id("post-1.md");
    let reads = built.render("post-1.md").reads;
    assert!(reads.keys().iter().any(|k| matches!(k, Key::Asset(_))));
    let format = built.renderer.name_id("date_format");
    assert!(reads.keys().contains(&Key::Setting(format)));
    assert!(reads.keys().contains(&Key::Body(post)));
    assert_eq!(built.html("post-2.md").split('|').nth(7), Some("Heading 2"));
}

#[test]
fn a_filter_that_needs_the_sites_address_says_so() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "{{ page.url | absolute }}"),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        rendered.problems[0].kind,
        Kind::TemplateStopped {
            reason: Stopped::NoSiteAddress
        }
    );
    assert_eq!(built.told(&rendered.problems)[0].2, "page.url | absolute");
}

#[test]
fn a_snippet_that_calls_itself_is_stopped() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "{{ again() }}"),
        ("snippets/again.html", "{{ again() }}"),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        rendered.problems[0].kind,
        Kind::TemplateStopped {
            reason: Stopped::CallsItself("again".into())
        }
    );
}

/// Three loops over ten thousand, one inside another.
const ENDLESS: &str = "{% for a in range(10000) %}{% for b in range(10000) %}\
                       {% for c in range(10000) %}{% endfor %}{% endfor %}{% endfor %}";

fn ran_on(template: &str, steps: u64) -> Kind {
    Kind::TemplateStopped {
        reason: Stopped::RanOn {
            template: template.into(),
            steps,
        },
    }
}

#[test]
fn an_output_may_take_a_hundred_million_steps() {
    assert_eq!(STEPS, 100_000_000);
}

#[test]
fn a_layout_that_runs_on_is_stopped() {
    let built = site_taking(
        10_000,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            ("templates/page.html", ENDLESS),
        ],
    );
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert_eq!(rendered.problems.len(), 1, "{:?}", rendered.problems);
    assert_eq!(
        rendered.problems[0].kind,
        ran_on("templates/page.html", 10_000)
    );
    let told = built.told(&rendered.problems);
    assert_eq!(told[0].1, "templates/page.html");
}

#[test]
fn a_layout_that_runs_on_where_the_engine_names_no_place_is_stopped_alike() {
    let built = site_taking(
        10_000,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            (
                "templates/page.html",
                "{% for a in range(10000) %}{% for b in range(10000) %}\
                 {% set c = 1 %}{% endfor %}{% endfor %}",
            ),
        ],
    );
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert_eq!(rendered.problems.len(), 1, "{:?}", rendered.problems);
    assert_eq!(
        rendered.problems[0].kind,
        ran_on("templates/page.html", 10_000)
    );
    let told = built.told(&rendered.problems);
    assert_eq!(told[0].1, "templates/page.html");
}

#[test]
fn the_steps_of_every_snippet_called_count_to_the_page() {
    let built = site_taking(
        10_000,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            (
                "templates/page.html",
                "{% for n in range(1000) %}{{ step() }}{% endfor %}",
            ),
            ("snippets/step.html", "{% for n in range(20) %}{% endfor %}"),
        ],
    );
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert_eq!(rendered.problems.len(), 1, "{:?}", rendered.problems);
    assert_eq!(
        rendered.problems[0].kind,
        ran_on("templates/page.html", 10_000)
    );
    assert_eq!(
        built.told(&rendered.problems)[0],
        (
            "template-stopped",
            "templates/page.html".into(),
            "step()".into()
        )
    );
}

#[test]
fn a_snippet_a_snippet_calls_is_stopped_where_it_runs_on() {
    let built = site_taking(
        10_000,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            ("templates/page.html", "{{ outer() }}"),
            ("snippets/outer.html", "{{ inner() }}"),
            ("snippets/inner.html", ENDLESS),
        ],
    );
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.problems.len(), 1, "{:?}", rendered.problems);
    let problem = &rendered.problems[0];
    assert_eq!(problem.kind, ran_on("templates/page.html", 10_000));
    assert_eq!(built.told(&rendered.problems)[0].1, "snippets/inner.html");
    let called: Vec<(Relation, String, String)> = built
        .context(problem)
        .into_iter()
        .filter(|(relation, ..)| *relation == Relation::CalledFrom)
        .collect();
    assert_eq!(
        called,
        [
            (
                Relation::CalledFrom,
                "snippets/outer.html".into(),
                "inner()".into()
            ),
            (
                Relation::CalledFrom,
                "templates/page.html".into(),
                "outer()".into()
            ),
        ]
    );
}

#[test]
fn a_piece_of_the_writing_that_runs_on_is_stopped() {
    let built = site_taking(
        10_000,
        &[
            (
                "content/post-1.md",
                "---\ntitle: Post 1\n---\nA [link](/post-2/) and [another](/post-3/).\n",
            ),
            ("templates/page.html", "{{ page.content }}"),
            ("snippets/markdown/link.html", ENDLESS),
        ],
    );
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert_eq!(rendered.problems.len(), 1, "{:?}", rendered.problems);
    assert_eq!(
        rendered.problems[0].kind,
        ran_on("templates/page.html", 10_000)
    );
}

#[test]
fn the_steps_of_the_writing_count_to_its_page() {
    let files = [
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n@step()\n\n@step()\n\n@step()\n",
        ),
        (
            "templates/page.html",
            "{{ page.content }}{% for n in range(200) %}{% endfor %}",
        ),
        (
            "snippets/step.html",
            "{% for n in range(200) %}{% endfor %}",
        ),
    ];
    let taken = steps_taken(&files, "post-1.md");
    let built = site_taking(taken, &files);
    assert!(built.render("post-1.md").html.is_some());
    let built = site_taking(taken - 1, &files);
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert_eq!(
        rendered.problems[0].kind,
        ran_on("templates/page.html", taken - 1)
    );
}

/// The fewest steps the page at `path` builds in.
fn steps_taken(files: &[(&str, &str)], path: &str) -> u64 {
    let builds = |steps| site_taking(steps, files).render(path).html.is_some();
    let (mut fewest, mut most) = (1, 1_000_000);
    assert!(builds(most));
    while fewest < most {
        let middle = u64::midpoint(fewest, most);
        match builds(middle) {
            true => most = middle,
            false => fewest = middle + 1,
        }
    }
    fewest
}

#[test]
fn a_page_just_under_its_steps_builds() {
    let files = [
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "templates/page.html",
            "{% for n in range(3) %}{{ step() }}{% endfor %}",
        ),
        ("snippets/step.html", "{{ deeper() }}"),
        (
            "snippets/deeper.html",
            "{% for n in range(100) %}{% endfor %}",
        ),
    ];
    let taken = steps_taken(&files, "post-1.md");
    assert!(taken > 600, "{taken}");
    let built = site_taking(taken, &files);
    let rendered = built.render("post-1.md");
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
    assert!(rendered.html.is_some());
    let rendered = site_taking(taken - 1, &files).render("post-1.md");
    assert_eq!(
        rendered.problems[0].kind,
        ran_on("templates/page.html", taken - 1)
    );
}

#[test]
fn steps_running_out_on_many_pages_are_told_once_at_the_layout() {
    let temp = site_of_steps(
        10_000,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
            ("content/post-3.md", "---\ntitle: Post 3\n---\n"),
            NO_HOME,
            ("templates/page.html", ENDLESS),
        ],
    );
    let outcome = crate::build::run(&crate::build::Options {
        mode: crate::build::Mode::Check,
        site: temp.root.clone(),
        out: None,
        today: Some(crate::date::Date::day_only(2026, 10, 4)),
        drafts: false,
        url: None,
    });
    let stopped: Vec<&Problem> = outcome
        .problems
        .iter()
        .filter(|p| p.kind.name() == "template-stopped")
        .collect();
    assert_eq!(stopped.len(), 1, "{:?}", outcome.problems);
    assert_eq!(stopped[0].kind, ran_on("templates/page.html", 10_000));
    assert_eq!(stopped[0].built().count(), 3, "{:?}", stopped[0]);
}

#[test]
fn serve_stops_where_build_stops() {
    let temp = site_of_steps(
        10_000,
        &[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n@step()\n"),
            ("content/post-2.md", "---\ntitle: Post 2\n---\n"),
            NO_HOME,
            (
                "templates/page.html",
                "{{ page.content }}{% for n in range(1000) %}{{ step() }}{% endfor %}",
            ),
            ("snippets/step.html", "{% for n in range(5) %}{% endfor %}"),
        ],
    );
    let options = crate::build::Options {
        mode: crate::build::Mode::Build,
        site: temp.root.clone(),
        out: Some(temp.root.join("_fresh")),
        today: Some(crate::date::Date::day_only(2026, 10, 4)),
        drafts: false,
        url: None,
    };
    let built = crate::build::run(&options);
    let (mut session, round) = crate::serve::session::Session::start(&crate::build::Options {
        out: None,
        ..options.clone()
    });
    let stopped = |problems: &[Problem]| -> Vec<Problem> {
        problems
            .iter()
            .filter(|p| p.kind.name() == "template-stopped")
            .cloned()
            .collect()
    };
    assert_eq!(stopped(&built.problems).len(), 1, "{:?}", built.problems);
    assert_eq!(stopped(&round.outcome.problems), stopped(&built.problems));
    let round = session.rebuild();
    assert_eq!(stopped(&round.outcome.problems), stopped(&built.problems));
}

#[test]
fn every_page_is_built_alike_on_any_thread() {
    let mut files = vec![BASE, PAGE];
    let texts: Vec<(String, String)> = (1..=40)
        .map(|n| {
            (
                format!("content/post-{n}.md"),
                format!("---\ntitle: Post {n}\n---\nPost {n}.\n"),
            )
        })
        .collect();
    files.extend(texts.iter().map(|(p, t)| (p.as_str(), t.as_str())));
    let built = site(&files);
    use rayon::prelude::*;
    let ids: Vec<PageId> = (0..40).map(PageId::from_index).collect();
    let pages: Vec<String> = ids
        .par_iter()
        .map(|&id| built.renderer.render(id).html.unwrap())
        .collect();
    for (id, html) in ids.iter().zip(&pages) {
        let n = built.renderer.site().pages.get(*id).path.clone();
        let n = n.trim_start_matches("post-").trim_end_matches(".md");
        assert_eq!(*html, format!("<title>Post {n}</title><p>Post {n}.</p>"));
    }
}

#[test]
fn a_block_tag_takes_its_line_with_it() {
    let built = site(&[
        ("content/index.md", "---\ntitle: Home\n---\n"),
        BASE,
        PAGE,
        (
            "templates/home.html",
            "{% extends \"base\" %}{% block main %}<ul>\n  {% for n in [1, 2] %}\n  <li>{{ n }}</li>\n  {% endfor %}\n</ul>{% endblock %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<title>Home</title><ul>\n  <li>1</li>\n  <li>2</li>\n</ul>"
    );
}

/// A call's words are set as the writing around it is, its quotes, dashes
/// and ellipses; a value naming a file, an address, a path or an id
/// reaches the snippet as typed; and with `typography: false` none is set.
#[test]
fn a_calls_words_are_set_and_its_names_left_as_typed() {
    let page = "---\ntitle: Post 1\n---\n@show(a=\"It's 'here' -- now...\", b='a--b.jpg', \
                c=\"https://a.org/it's\", d='id--x', e=\"it's\", f=\"/a/it's\", \
                g=\"sentence ends.\")\n";
    let snippet = (
        "snippets/show.html",
        "{{ a }}|{{ b }}|{{ c }}|{{ d }}|{{ e }}|{{ f }}|{{ g }}",
    );
    let set = site(&[("content/post-1.md", page), BASE, PAGE, snippet]);
    assert!(set.problems.is_empty(), "{:?}", set.problems);
    assert_eq!(
        set.html("post-1.md"),
        "<title>Post 1</title>It\u{2019}s \u{2018}here\u{2019} \u{2013} now\u{2026}|a--b.jpg|\
         https://a.org/it&#x27;s|id--x|it\u{2019}s|/a/it&#x27;s|sentence ends."
    );
    let typed = site(&[
        ("statac.yaml", "language: en\ntypography: false\n"),
        ("content/post-1.md", page),
        BASE,
        PAGE,
        snippet,
    ]);
    assert!(
        typed
            .html("post-1.md")
            .starts_with("<title>Post 1</title>It&#x27;s &#x27;here&#x27; -- now...|"),
        "{}",
        typed.html("post-1.md")
    );
}

impl Built {
    /// The page standing at `address`, such as a page Statac made.
    fn at(&self, address: &str) -> PageId {
        self.renderer
            .site()
            .pages
            .iter()
            .find(|(_, page)| page.address.as_str() == address)
            .map(|(id, _)| id)
            .unwrap_or_else(|| panic!("nothing at {address}"))
    }

    fn html_at(&self, address: &str) -> String {
        let rendered = self.renderer.render(self.at(address));
        assert!(
            rendered.problems.is_empty(),
            "{address}: {:?}",
            rendered.problems
        );
        rendered.html.expect("built")
    }
}

/// A site with posts in a folder, a tag spelt two ways, a series and a
/// list a page declares.
fn listed(templates: &[(&'static str, &'static str)]) -> Built {
    let mut files = vec![
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        ("content/blog/index.md", "---\ntitle: Blog\n---\n"),
        (
            "content/blog/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\ntags: [Rust, Go]\nseries: Series 1\n---\n",
        ),
        (
            "content/blog/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-02-01\ntags: [rust]\nseries: Series 1\n---\n",
        ),
        (
            "content/about.md",
            "---\ntitle: About\nlayout: plain\n---\n",
        ),
        (
            "content/picks.md",
            "---\ntitle: Picks\nposts: site.posts | tagged(\"Go\")\n---\n",
        ),
        ("templates/plain.html", "{{ page.title }}"),
    ];
    files.extend_from_slice(templates);
    site(&files)
}

#[test]
fn every_kind_of_list_answers_the_same_names() {
    let built = listed(&[
        (
            "templates/page.html",
            "{{ page.tags | join(',') }}|{{ 'RUST' in page.tags }}|{{ 'Python' in page.tags }}|\
             {% for tag in page.tags %}{{ tag.name }}={{ tag.url }}={{ tag.word }};{% endfor %}|\
             {{ page.series.name }}:{% for p in page.series.posts %}{{ p.title }};{% endfor %}|\
             {{ page.folder.name }}:{{ page.folder.word }}|\
             {% if page.list is defined %}{{ page.list.word }}{% else %}none{% endif %}|\
             {{ page.tags | json }}",
        ),
        (
            "templates/folder.html",
            "{{ page.list.word }}:{{ page.list.name }}:{{ page.list.title }}:{{ page.list.url }}:\
             {% for p in page.posts %}{{ p.title }};{% endfor %}:{{ page.title is defined }}",
        ),
        (
            "templates/home.html",
            "{% for t in site.tags %}{{ t }}({{ t.posts | length }});{% endfor %}|\
             {% for s in site.series %}{{ s.name }};{% endfor %}|\
             {% for p in site.posts %}{{ p.title }};{% endfor %}|\
             {{ site.pages | length }}|{{ page.list.word }}|\
             {% for p in site.posts | where('title', 'Post 1') %}{{ p.url }}{% endfor %}|\
             {{ site.pages | tagged('go') | length }}|{{ site.pages | in('blog') | length }}|\
             {% for p in page.list.pages %}{{ p.title }};{% endfor %}",
        ),
    ]);
    assert_eq!(
        built.told(&built.problems),
        [(
            "spelled-two-ways",
            "content/blog/post-2.md".into(),
            "rust".into()
        )]
    );
    assert_eq!(
        built.html("blog/post-1.md"),
        "Rust,Go|True|False|Rust=/tags/rust/=tag;Go=/tags/go/=tag;|Series 1:Post 1;Post 2;|\
         blog:folder|none|[\"Rust\",\"Go\"]"
    );
    assert_eq!(
        built.html("blog/index.md"),
        "folder:blog:Blog:/blog/:Post 2;Post 1;:True"
    );
    assert_eq!(
        built.html_at("/tags/rust/"),
        "tag:Rust:Rust:/tags/rust/:Post 2;Post 1;:False"
    );
    assert_eq!(
        built.html_at("/series/series-1/"),
        "series:Series 1:Series 1:/series/series-1/:Post 1;Post 2;:False"
    );
    assert_eq!(
        built.html("picks.md"),
        "chosen:Picks:Picks:/picks/:Post 1;:True"
    );
    assert_eq!(
        built.html("index.md"),
        "Go(1);Rust(2);|Series 1;|Post 2;Post 1;|9|site|/blog/post-1/|1|2|Post 2;Post 1;About;"
    );
}

#[test]
fn a_top_folder_with_no_index_is_built_with_the_folder_template() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/notes/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\n",
        ),
        (
            "content/notes/2026/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-02-01\n---\n",
        ),
        ("templates/page.html", "{{ page.title }}"),
        (
            "templates/folder.html",
            "{{ page.list.name }}:{{ page.list.word }}:{{ page.content }}:\
             {% for p in page.posts %}{{ p.title }};{% endfor %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html_at("/notes/"), "notes:folder::Post 2;Post 1;");
    // A folder inside another has a page only by its own `index.md`.
    let inner = built
        .renderer
        .site()
        .pages
        .iter()
        .any(|(_, page)| page.address.as_str() == "/notes/2026/");
    assert!(!inner);
}

#[test]
fn a_missing_title_is_offered_the_headline_only_on_a_page_statac_made() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ndate: 2026-01-01\ntags: [Rust]\n---\nPost 1 opens here.\n",
        ),
        ("templates/page.html", "{{ page.title }}"),
        ("templates/folder.html", "{{ page.title }}"),
    ]);
    let made = built.renderer.render(built.at("/tags/rust/"));
    assert_eq!(made.html, None);
    assert_eq!(
        made.problems[0].kind,
        Kind::NoValue {
            written: "page.title".into(),
            lack: Lack::OnPage(Named::ListPage(Word::Tag, "Rust".into())),
            instead: Instead::Headline("headline(page)".into()),
        }
    );
    let authors = built.render("post-1.md");
    assert_eq!(
        authors.problems[0].kind,
        Kind::NoValue {
            written: "page.title".into(),
            lack: Lack::OnPage("content/post-1.md".into()),
            instead: Instead::Default("page.title | default(\"\")".into()),
        }
    );
}

#[test]
fn a_site_with_no_index_has_its_home_page_built_with_the_home_template() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/about.md", "---\ntitle: About\n---\n"),
        (
            "content/notes/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\n---\n",
        ),
        ("templates/page.html", "{{ page.title }}"),
        ("templates/folder.html", "{{ page.title }}"),
        (
            "templates/home.html",
            "{{ page.list.word }}:{{ page.content }}:{% if page.title %}titled{% endif %}:\
             {% for p in page.posts %}{{ p.title }};{% endfor %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html_at("/"), "site:::Post 1;");
}

#[test]
fn a_list_says_which_way_its_posts_run_or_nothing_where_dates_dont_place_them() {
    let order = |files: &[(&str, &str)], at: &str| {
        let mut all = vec![
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\npages.feed: false\n",
            ),
            ("templates/page.html", ""),
            ("templates/home.html", ""),
            ("templates/folder.html", "{{ page.list.order }}"),
        ];
        all.extend_from_slice(files);
        let built = site(&all);
        built.html_at(at)
    };
    let post = "---\ndate: 2026-10-01\nseries: Series 1\n---\n";
    let posts = [("content/blog/post-1.md", post)];
    assert_eq!(order(&posts, "/blog/"), "newest first");
    assert_eq!(order(&posts, "/series/series-1/"), "oldest first");
    let oldest = ("content/blog/index.md", "---\norder: oldest first\n---\n");
    assert_eq!(order(&[posts[0], oldest], "/blog/"), "oldest first");
    let by_hand = ("content/blog/index.md", "---\norder: [post-1]\n---\n");
    assert_eq!(order(&[posts[0], by_hand], "/blog/"), "");
    let numbered = [("content/blog/01-post-1.md", post)];
    assert_eq!(order(&numbered, "/blog/"), "");
}

#[test]
fn the_site_lists_its_folders_that_have_a_page_by_address() {
    let post = "---\ndate: 2026-10-01\ntags: [Tag 1]\n---\n";
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/notes/post-1.md", post),
        ("content/blog/post-1.md", post),
        ("content/blog/2026/post-2.md", post),
        ("content/blog/2026/index.md", "---\ntitle: Year\n---\n"),
        ("content/hidden/index.md", "---\nindex_page: false\n---\n"),
        ("content/hidden/post-1.md", post),
        ("templates/page.html", ""),
        ("templates/folder.html", ""),
        (
            "templates/home.html",
            "{% for folder in site.folders %}{{ folder.url }};{% endfor %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html_at("/"), "/blog/;/blog/2026/;/notes/;");
}

#[test]
fn a_page_speaking_for_no_list_has_no_list() {
    let built = listed(&[("templates/page.html", "{{ page.list.word }}")]);
    let rendered = built.render("blog/post-1.md");
    assert_eq!(
        built.told(&rendered.problems),
        [("no-value", "templates/page.html".into(), "page.list".into())]
    );
}

#[test]
fn reading_a_list_is_recorded_by_the_list() {
    let built = listed(&[(
        "templates/page.html",
        "{% for tag in page.tags %}{{ tag.posts | length }}{% endfor %}",
    )]);
    let post = built.id("blog/post-1.md");
    let site = built.renderer.site();
    let language = site.pages.get(post).language;
    let rust = site
        .spelt(language, &crate::site::ListKey::Tag("Rust".into()))
        .unwrap();
    let reads = built.render("blog/post-1.md").reads;
    let tags = built.renderer.name_id("tags");
    assert!(reads.keys().contains(&Key::Field(post, tags)));
    assert!(reads.keys().contains(&Key::Members(rust)));
}

#[test]
fn a_tag_with_no_page_has_no_address() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\ntags.pages: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\ntags: [Rust]\n---\n",
        ),
        (
            "templates/page.html",
            "{% for tag in page.tags %}{{ tag.url }}{% endfor %}",
        ),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        built.told(&rendered.problems),
        [("no-value", "templates/page.html".into(), "tag.url".into())]
    );
}

#[test]
fn site_data_reaches_each_file_by_its_folders_and_name() {
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        (
            "data/people.yaml",
            "- name: Person 1\n  role: Role 1\n- name: Person 2\n  count: 2\n",
        ),
        (
            "data/lists/links.json",
            "{\"link-1\": {\"url\": \"/a/\"}, \"42\": \"n\"}",
        ),
        ("data/unread.yaml", "a: 1\n"),
        (
            "templates/page.html",
            "{% for p in site.data.people %}{{ p.name }} {{ p.count | default(0) + 1 }};{% endfor %}\
             {{ site.data.lists.links[\"link-1\"].url }} {{ site.data.lists.links[42] }}\
             {% if \"people\" in site.data %} people{% endif %}\
             {% for name in site.data.lists %} {{ name }}{% endfor %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "Person 1 1;Person 2 3;/a/ n people links"
    );
    let data = &built.renderer.site().data;
    let read: Vec<&str> = built
        .render("post-1.md")
        .reads
        .keys()
        .iter()
        .filter_map(|key| match key {
            Key::Data(id) => Some(data.file(*id).path.as_str()),
            _ => None,
        })
        .collect();
    assert!(read.contains(&"data/people.yaml"), "{read:?}");
    assert!(read.contains(&"data/lists/links.json"), "{read:?}");
    assert!(!read.contains(&"data/unread.yaml"), "{read:?}");
}

#[test]
fn a_missing_data_value_names_the_file_or_folder() {
    let files = [
        ("data/people.yaml", "person-1:\n  name: Person 1\n"),
        ("data/lists/links.yaml", "- url: /a/\n"),
    ];
    let cases = [
        (
            "{{ site.data.people.person_2 }}",
            "site.data.people.person_2",
            Lack::InDataFile("data/people.yaml".into()),
        ),
        (
            "{{ site.data.people[\"person-1\"].role }}",
            "site.data.people[\"person-1\"].role",
            Lack::InDataFile("data/people.yaml".into()),
        ),
        (
            "{{ site.data.peple }}",
            "site.data.peple",
            Lack::InDataFolder("data/".into()),
        ),
        (
            "{{ site.data.lists.link }}",
            "site.data.lists.link",
            Lack::InDataFolder("data/lists/".into()),
        ),
    ];
    for (template, written, lack) in cases {
        let built = site(&[
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            files[0],
            files[1],
            ("templates/page.html", template),
        ]);
        let rendered = built.render("post-1.md");
        assert_eq!(rendered.html, None, "{template}");
        let Kind::NoValue {
            written: found,
            lack: found_lack,
            ..
        } = &rendered.problems[0].kind
        else {
            panic!("{template}: {:?}", rendered.problems);
        };
        assert_eq!((found.as_str(), found_lack), (written, &lack), "{template}");
    }
}

#[test]
fn pages_made_from_data_are_built_with_the_layout_their_page_names() {
    let built = site(&[
        (
            "content/people.md",
            "---\ntitle: People\neach: data.people\npermalink: /people/{key}/\nlayout: person\n---\n",
        ),
        (
            "data/people.yaml",
            "person-1:\n  title: Person 1\n  role: Role 1\nperson-2:\n  title: Person 2\n",
        ),
        (
            "templates/person.html",
            "{{ page.key }}: {{ page.title }}, {{ page.role }}",
        ),
        (
            "templates/folder.html",
            "{{ page.title }}:{% for p in page.list.pages %} {{ p.url }}{% endfor %}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("people.md"),
        "People: /people/person-1/ /people/person-2/"
    );
    assert_eq!(
        built.html("people.md/person-1"),
        "person-1: Person 1, Role 1"
    );
    let rendered = built.render("people.md/person-2");
    let problem = &rendered.problems[0];
    assert_eq!(
        problem.kind,
        Kind::NoValue {
            written: "page.role".into(),
            lack: Lack::OnPage("data/people.yaml: person-2".into()),
            instead: Instead::Default("page.role | default(\"\")".into()),
        }
    );
    assert_eq!(
        built.context(problem),
        [(
            Relation::Building(None),
            "data/people.yaml".into(),
            "person-2".into()
        )]
    );
}

#[test]
fn a_page_made_for_a_post_reaches_the_post_as_page_post() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/cards.md",
            "---\neach: site.posts\npermalink: /cards/{slug}/\nlayout: card\nindex_page: false\n---\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-10-01\n---\n",
        ),
        (
            "templates/card.html",
            "{{ page.post.title }}, {{ page.post.date.year }}, {{ page.post.url }}, {{ page.url }}, \
             {{ page.title is defined }}",
        ),
        PAGE,
        BASE,
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("cards.md/post-1.md"),
        "Post 1, 2026, /post-1/, /cards/post-1/, False"
    );
    let rendered = built.render("cards.md/post-1.md");
    let keys = rendered.reads.keys();
    let (card, post) = (built.id("cards.md/post-1.md"), built.id("post-1.md"));
    let name = |name: &str| built.renderer.name_id(name);
    assert!(keys.contains(&Key::Field(card, name("post"))));
    assert!(keys.contains(&Key::Field(post, name("title"))));
    assert!(keys.contains(&Key::Field(post, name("date"))));

    // A page made for no post has no `page.post`.
    let rendered = built.render("post-1.md");
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
}

#[test]
fn a_nested_field_reaches_templates_as_declared() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: Europe/London\ndate_format: 31 December 2025\nfields.book.published: date\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nbook:\n  author: Author 1\n  published: 2026-10-04\n---\n",
        ),
        (
            "templates/page.html",
            "{{ page.book.author }}, {{ page.book.published }}, {{ page.book.published.year }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "Author 1, 4 October 2026, 2026");
}

/// Three posts born in a year, a month and on a day, and one with a date
/// declared, in a site writing dates as `31 December 2025`.
const BORN: [(&str, &str); 5] = [
    (
        "statac.yaml",
        "language: en\ntimezone: Europe/London\ndate_format: 31 December 2025\nfields.born: when\n\
         fields.met: date\n",
    ),
    (
        "content/post-1.md",
        "---\ntitle: Post 1\nborn: 1850-03\nmet: 2026-10-04\n---\n",
    ),
    ("content/post-2.md", "---\ntitle: Post 2\nborn: 1850\n---\n"),
    ("content/post-3.md", "---\ntitle: Post 3\nborn: 1851\n---\n"),
    (
        "content/post-4.md",
        "---\ntitle: Post 4\nborn: 1850-03-12\n---\n",
    ),
];

#[test]
fn a_declared_date_answers_its_year_month_and_day() {
    let mut files = BORN.to_vec();
    files.push(NO_HOME);
    files.push((
        "templates/page.html",
        "{% if page.met is defined %}{{ page.met }}: {{ page.met.year }} {{ page.met.month }} \
         {{ page.met.day }}{% endif %}",
    ));
    let built = site(&files);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "4 October 2026: 2026 10 4");
}

#[test]
fn a_when_answers_what_was_written_of_it() {
    let mut files = BORN.to_vec();
    files.push(NO_HOME);
    files.push((
        "templates/page.html",
        "{{ page.born }}: {{ page.born.year }} {{ page.born.month | default(\"-\") }} \
         {{ page.born.day | default(\"-\") }}",
    ));
    let built = site(&files);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-2.md"), "1850: 1850 - -");
    assert_eq!(built.html("post-1.md"), "March 1850: 1850 3 -");
    assert_eq!(built.html("post-4.md"), "12 March 1850: 1850 3 12");
}

#[test]
fn a_when_asked_for_a_part_it_wasnt_written_with_is_missing() {
    let mut files = BORN.to_vec();
    files.push(NO_HOME);
    files.push(("templates/page.html", "{{ page.born.day }}"));
    let built = site(&files);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-4.md"), "12");
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "no-value",
            "templates/page.html".into(),
            "page.born.day".into()
        )]
    );
}

#[test]
fn a_part_of_a_date_reaches_a_filter_as_the_text_it_is_printed_as() {
    let mut files = BORN.to_vec();
    files.push(NO_HOME);
    files.push(("templates/page.html", "{{ page.born | json }}"));
    let built = site(&files);
    assert_eq!(built.html("post-1.md"), "\"March 1850\"");
    assert_eq!(built.html("post-4.md"), "\"1850-03-12\"");
    let mut files = BORN.to_vec();
    files.push(NO_HOME);
    files.push(("templates/page.html", "{{ page.born | date }}"));
    let built = site(&files);
    assert_eq!(built.html("post-4.md"), "12 March 1850");
    assert_eq!(
        built.render("post-2.md").problems[0].kind,
        Kind::TemplateStopped {
            reason: Stopped::NotADate("date".into())
        }
    );
}

#[test]
fn whens_are_sorted_by_the_calendar_among_dates() {
    let mut files = BORN.to_vec();
    files.push(("content/index.md", "---\ntitle: Home\n---\n"));
    files.push((
        "templates/home.html",
        "{% for p in site.pages | selectattr(\"born\", \"defined\") | sort(attribute=\"born\") %}\
         {{ p.born }}; {% endfor %}|\
         {% for p in site.pages | selectattr(\"born\", \"defined\") | sort(attribute=\"born\", \
         reverse=true) %}{{ p.born }}; {% endfor %}",
    ));
    files.push(("templates/page.html", "{{ page.title }}"));
    let built = site(&files);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "1850; March 1850; 12 March 1850; 1851; |1851; 12 March 1850; March 1850; 1850; "
    );
}

#[test]
fn a_nested_when_and_an_items_when_answer_their_year() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\nfields.book.published: when\nfields.born: when\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nbook:\n  published: 1850-03\n---\n",
        ),
        (
            "content/people.md",
            "---\ntitle: People\neach: data.people\npermalink: /people/{key}/\nlayout: person\n---\n",
        ),
        (
            "data/people.yaml",
            "person-1:\n  title: Person 1\n  born: 1850\n",
        ),
        (
            "templates/page.html",
            "{{ page.book.published }}, {{ page.book.published.year }}",
        ),
        (
            "templates/person.html",
            "{{ page.born }}, {{ page.born.year }}",
        ),
        ("templates/folder.html", "{{ page.title }}"),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("post-1.md"), "March 1850, 1850");
    assert_eq!(built.html("people.md/person-1"), "1850, 1850");
}

#[test]
fn bytes_that_arent_text_stop_a_template_only_where_it_shows_them() {
    let files = [
        (
            "data/comments.yaml",
            "- name: Name 1\n  note: !!binary Tm90ZSAx\n  bucket: !!binary /w==\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsecret: !!binary /w==\n---\n",
        ),
    ];
    let cases = [
        (
            "{% for c in site.data.comments %}{{ c.name }}, {{ c.note }}{% set b = c.bucket %}{% endfor %}",
            None,
        ),
        (
            "{% for c in site.data.comments %}{{ c.bucket }}{% endfor %}",
            Some("c.bucket"),
        ),
        (
            "{{ site.data.comments | first | json }}",
            Some("site.data.comments | first | json"),
        ),
        (
            "{% for c in site.data.comments %}{{ c.bucket | upper }}{% endfor %}",
            Some("c.bucket | upper"),
        ),
    ];
    for (template, stopped_at) in cases {
        let built = site(&[
            files[0],
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            ("templates/page.html", template),
        ]);
        let rendered = built.render("post-1.md");
        match stopped_at {
            None => assert_eq!(
                rendered.html.as_deref(),
                Some("Name 1, Note 1"),
                "{template}"
            ),
            Some(at) => {
                assert_eq!(rendered.html, None, "{template}");
                assert_eq!(
                    rendered.problems[0].kind,
                    Kind::TemplateStopped {
                        reason: Stopped::NotText
                    },
                    "{template}"
                );
                assert_eq!(built.told(&rendered.problems)[0].2, at, "{template}");
            }
        }
    }
    let built = site(&[
        files[1],
        ("templates/page.html", "{{ page.title }}{{ page.secret }}"),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(rendered.html, None);
}

/// A folder of five posts, each tagged, whose page shows two at a time.
fn paged(templates: &[(&'static str, &'static str)]) -> Built {
    let mut files = vec![
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/blog/index.md",
            "---\ntitle: Blog\nsummary: Every post\nposts_per_page: 2\nmood: calm\n\
             date: 2026-01-01\n---\nThe introduction.\n",
        ),
        (
            "content/blog/post-1.md",
            "---\ntitle: Post 1\ndate: 2026-01-01\ntags: [Rust, Go]\n---\n",
        ),
        (
            "content/blog/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-02-01\ntags: [Rust]\n---\n",
        ),
        (
            "content/blog/post-3.md",
            "---\ntitle: Post 3\ndate: 2026-03-01\ntags: [Go, Rust]\n---\n",
        ),
        (
            "content/blog/post-4.md",
            "---\ntitle: Post 4\ndate: 2026-04-01\ntags: [C]\n---\n",
        ),
        (
            "content/blog/post-5.md",
            "---\ntitle: Post 5\ndate: 2026-05-01\ntags: [Go]\n---\n",
        ),
    ];
    files.extend_from_slice(templates);
    site(&files)
}

impl Built {
    /// The page `number` of the list `path` speaks for, built.
    fn later_html(&self, path: &str, number: usize) -> String {
        let rendered = self.renderer.render_later(self.id(path), number);
        assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
        rendered.html.expect("built")
    }
}

#[test]
fn a_lists_pages_each_show_their_stretch_and_walk_each_other() {
    let built = paged(&[(
        "templates/folder.html",
        "{{ page.number }}/{{ page.list.pager | length }}|\
         {% for p in page.posts %}{{ p.title }};{% endfor %}|\
         {% if page.previous %}{{ page.previous.url }}{% endif %}|\
         {% if page.next %}{{ page.next.url }}{% endif %}|\
         {% for p in page.list.pager %}{{ p.number }}={{ p.url }};{% endfor %}|\
         {{ page.title }}|{{ page.summary }}|{{ page.mood }}|{{ page.url }}|\
         {{ page.date is defined }}|{{ page.content }}",
    )]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let pager = "1=/blog/;2=/blog/page/2/;3=/blog/page/3/;";
    assert_eq!(
        built.html("blog/index.md"),
        format!(
            "1/3|Post 5;Post 4;||/blog/page/2/|{pager}|Blog|Every post|calm|/blog/|True|\
             <p>The introduction.</p>"
        )
    );
    // A later page carries what its first says of itself, but not its
    // writing or its date.
    assert_eq!(
        built.later_html("blog/index.md", 2),
        format!(
            "2/3|Post 3;Post 2;|/blog/|/blog/page/3/|{pager}|Blog|Every post|calm|\
             /blog/page/2/|False|"
        )
    );
    assert_eq!(
        built.later_html("blog/index.md", 3),
        format!("3/3|Post 1;|/blog/page/2/||{pager}|Blog|Every post|calm|/blog/page/3/|False|")
    );
}

#[test]
fn a_later_page_reads_its_own_stretch() {
    let built = paged(&[(
        "templates/folder.html",
        "{% for p in page.posts %}{{ p.title }}{% endfor %}",
    )]);
    let page = built.id("blog/index.md");
    let list = built.renderer.site().pages.get(page).speaks_for.unwrap();
    let reads = built.renderer.render_later(page, 2).reads;
    assert!(reads.keys().contains(&Key::ListPage(list, 2)));
    assert!(!reads.keys().contains(&Key::ListPage(list, 1)));
    assert!(!reads.keys().contains(&Key::Body(page)));
}

#[test]
fn a_lists_first_page_alone_is_told_its_writing_is_never_shown() {
    let built = paged(&[("templates/folder.html", "{{ page.title }}")]);
    let page = built.id("blog/index.md");
    let told = |rendered: Rendered| -> Vec<&'static str> {
        let problems = rendered.problems.iter();
        problems.map(|problem| problem.kind.name()).collect()
    };
    assert_eq!(told(built.renderer.render(page)), ["writing-never-shown"]);
    assert!(told(built.renderer.render_later(page, 2)).is_empty());
}

#[test]
fn a_post_walks_its_folder_and_has_no_number() {
    let built = paged(&[(
        "templates/page.html",
        "{% if page.previous %}{{ page.previous.title }}{% endif %}<{{ page.title }}>\
         {% if page.next %}{{ page.next.title }}{% endif %}",
    )]);
    assert_eq!(built.html("blog/post-3.md"), "Post 4<Post 3>Post 2");
    assert_eq!(built.html("blog/post-5.md"), "<Post 5>Post 4");
    assert_eq!(built.html("blog/post-1.md"), "Post 2<Post 1>");
    let post = built.id("blog/post-3.md");
    let reads = built.render("blog/post-3.md").reads;
    assert!(reads.keys().contains(&Key::Neighbours(post)));

    let built = paged(&[("templates/page.html", "{{ page.number }}")]);
    let rendered = built.render("blog/post-3.md");
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "no-value",
            "templates/page.html".into(),
            "page.number".into()
        )]
    );
}

#[test]
fn related_gives_the_posts_sharing_most_tags_first() {
    let built = paged(&[(
        "templates/page.html",
        "{% for p in related(page) %}{{ p.title }};{% endfor %}",
    )]);
    assert_eq!(built.html("blog/post-1.md"), "Post 3;Post 5;Post 2;");
    assert_eq!(built.html("blog/post-4.md"), "");
    let post = built.id("blog/post-4.md");
    let site = built.renderer.site();
    let c = site
        .spelt(
            site.pages.get(post).language,
            &crate::site::ListKey::Tag("C".into()),
        )
        .unwrap();
    let reads = built.render("blog/post-4.md").reads;
    assert!(reads.keys().contains(&Key::Members(c)));

    let built = paged(&[(
        "templates/page.html",
        "{% for p in related(page) %}{{ p.titel }}{% endfor %}",
    )]);
    assert_eq!(
        built.template_problems(),
        [(
            "unknown-word",
            "templates/page.html".into(),
            "p.titel".into()
        )]
    );
}

#[test]
fn a_notes_way_back_says_where_it_goes_in_the_sites_language() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\nWords.[^1]\n\n[^1]: A note.\n",
        ),
        BASE,
        PAGE,
    ]);
    let html = built.html("post-1.md");
    assert!(
        html.contains(
            "<p>A note. <a href=\"#back-1\" class=\"back\" \
             aria-label=\"back to the text\">\u{21a9}</a></p>"
        ),
        "{html}"
    );
    let language = crate::site::LanguageId::from_index(0);
    assert!(
        built
            .render("post-1.md")
            .reads
            .keys()
            .contains(&Key::Language(language))
    );
}

mod building;
mod engine;
mod languages;
mod lists;
mod pieces;
mod shows;
mod translations;
mod walks;

/// `@url(...)` in a page's writing and in a value read as Markdown, and
/// `url(...)` in a template, are each the address under the site's path,
/// and what shows one reads the site's `url`.
#[test]
fn an_address_inside_the_site_is_under_the_sites_path() {
    let files = |url: &'static str| {
        [
            ("statac.yaml", url),
            NO_HOME,
            (
                "content/post-1.md",
                "---\ntitle: Post 1\n---\n[About](@url(/about/)) <a href=\"@url(/about/)\">@url(/about/)</a>\n",
            ),
            (
                "content/post-2.md",
                "---\ntitle: Post 2\nlayout: other\nsummary: \"[About](@url(/about/#team))\"\n---\n",
            ),
            (
                "content/post-3.md",
                "---\ntitle: Post 3\n---\n[About](/about/)\n",
            ),
            ("templates/page.html", "{{ page.content }}"),
            (
                "templates/other.html",
                "{{ page.summary | markdown }} <a href=\"{{ url(\"/about/?a=1&b=2\") }}\">About</a>",
            ),
        ]
    };
    let built = site(&files(
        "language: en\ntimezone: UTC\npages.feed: false\nurl: https://example.org/blog/\n",
    ));
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "<p><a href=\"/blog/about/\">About</a> <a href=\"/blog/about/\">/blog/about/</a></p>"
    );
    assert_eq!(
        built.html("post-2.md"),
        "<a href=\"/blog/about/#team\">About</a> <a href=\"/blog/about/?a=1&amp;b=2\">About</a>"
    );
    let url = Key::Setting(built.renderer.name_id("url"));
    let reads_it = |path: &str| built.render(path).reads.keys().contains(&url);
    assert!(reads_it("post-1.md"));
    assert!(reads_it("post-2.md"));
    assert!(!reads_it("post-3.md"));

    let built = site(&files("language: en\ntimezone: UTC\npages.feed: false\n"));
    assert_eq!(
        built.html("post-1.md"),
        "<p><a href=\"/about/\">About</a> <a href=\"/about/\">/about/</a></p>"
    );
    assert_eq!(
        built.html("post-2.md"),
        "<a href=\"/about/#team\">About</a> <a href=\"/about/?a=1&amp;b=2\">About</a>"
    );
}

#[test]
fn url_handed_anything_but_an_address_from_the_top_stops_the_template() {
    for written in [
        "url(\"about/\")",
        "url(\"//example.org/\")",
        "url(\"https://example.org/about/\")",
        "url(3)",
        "url(to=\"/about/\")",
        "url(page)",
    ] {
        let template = format!("<a href=\"{{{{ {written} }}}}\">About</a>");
        let built = site(&[
            NO_HOME,
            ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
            ("templates/page.html", &template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        let rendered = built.render("post-1.md");
        assert_eq!(rendered.html, None, "{written}");
        assert_eq!(
            rendered.problems[0].kind,
            Kind::TemplateStopped {
                reason: Stopped::NotAnAddress
            },
            "{written}"
        );
        assert_eq!(
            built.told(&rendered.problems),
            [(
                "template-stopped",
                "templates/page.html".into(),
                written.to_owned()
            )]
        );
    }
}

/// A value read as Markdown is told of an `@url(...)` handed something
/// else, as the writing is, at the value.
#[test]
fn an_address_inside_the_site_handed_something_else_is_told() {
    let built = site(&[
        NO_HOME,
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummary: \"[About](@url(about/))\"\n---\nSee @url(about/).\n",
        ),
        (
            "templates/page.html",
            "{{ page.summary | markdown }}{{ page.content }}",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let rendered = built.render("post-1.md");
    let found = crate::markdown::Malformed::NotOneAddress;
    for problem in &rendered.problems {
        assert_eq!(
            problem.kind,
            Kind::MalformedCall {
                found: found.clone()
            }
        );
    }
    assert_eq!(
        built.told(&rendered.problems),
        [
            (
                "malformed-call",
                "content/post-1.md".into(),
                "@url(about/)".into()
            ),
            (
                "malformed-call",
                "content/post-1.md".into(),
                "@url(about/)".into()
            ),
        ]
    );
    let mut lines: Vec<usize> = rendered
        .problems
        .iter()
        .map(|problem| {
            let place = problem.place.unwrap();
            let sources = built.renderer.sources();
            sources.line_number(place.source, place.span.start as usize)
        })
        .collect();
    lines.sort_unstable();
    assert_eq!(lines, [3, 5]);
}

/// `@url(...)` is Statac's own in the writing, so no snippet takes its
/// name; a site with none reads it all the same.
#[test]
fn a_snippet_named_url_is_told_as_a_reserved_name() {
    let built = site(&[
        NO_HOME,
        ("content/post-1.md", "---\ntitle: Post 1\n---\n"),
        ("templates/page.html", "{{ page.content }}"),
        ("snippets/url.html", "<a href=\"{{ to }}\">{{ to }}</a>\n"),
    ]);
    assert_eq!(
        built.template_problems(),
        [(
            "reserved-name",
            "snippets/url.html".into(),
            "<a href=\"{{ to }}\">{{ to }}</a>".into()
        )]
    );
    assert_eq!(
        built.problems[0].kind,
        Kind::ReservedName { name: "url".into() }
    );
}
