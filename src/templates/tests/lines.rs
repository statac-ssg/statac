use super::*;

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
fn a_bodys_indent_for_the_templates_sake_is_left_out_of_the_page() {
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
        (
            "content/post-2.md",
            "---\ntitle: Post 2\ndate: 2026-10-02\n---\n",
        ),
        ("snippets/card.html", "<li>\n  {{ post.title }}\n</li>\n"),
        (
            "templates/home.html",
            "<ul>\n  <li><a href=\"/\">Home</a></li>\n  {% for post in page.posts %}\n      <li>{{ post.title }}</li>\n      {{ card(post) }}\n  {% endfor %}\n</ul>\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<ul>\n  <li><a href=\"/\">Home</a></li>\n  <li>Post 2</li>\n  <li>\n    Post 2\n  </li>\n  <li>Post 1</li>\n  <li>\n    Post 1\n  </li>\n</ul>\n"
    );
}

#[test]
fn a_blank_line_left_of_its_blocks_tag_stays_blank_with_tabs_or_spaces() {
    for (indent, step) in [("      ", "  "), ("\t\t", "\t")] {
        let template = format!(
            "<main>\n{indent}{{% if true %}}\n{indent}{step}<p>a</p>\n\n\
             {indent}{step}{{% if true %}}\n{indent}{step}{step}<p>b</p>\n\
             {indent}{step}{{% endif %}}\n{indent}{{% endif %}}\n</main>\n"
        );
        let built = site(&[
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\npages.feed: false\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("templates/home.html", &template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        assert_eq!(
            built.html("index.md"),
            format!("<main>\n{indent}<p>a</p>\n\n{indent}<p>b</p>\n</main>\n"),
            "{template:?}"
        );
    }
}

#[test]
fn a_body_written_at_an_indented_call_gives_up_the_indent_its_lines_share() {
    let cases = [
        (
            "<div>\n  {% macro m() %}\n    <p>a</p>\n    <p>b</p>\n  {% endmacro %}\n  {{ m() }}\n</div>\n",
            "<div>\n  <p>a</p>\n  <p>b</p>\n</div>\n",
        ),
        (
            "<div>\n  {% set s %}\n    <p>a</p>\n    <p>b</p>\n  {% endset %}\n  {{ s }}\n</div>\n",
            "<div>\n  <p>a</p>\n  <p>b</p>\n</div>\n",
        ),
        // Its lines keep their indent one to another.
        (
            "{% macro m() %}\n  <li>\n    a\n  </li>\n{% endmacro %}\n<div>\n  <ul>\n    {{ m() }}\n  </ul>\n</div>\n",
            "<div>\n  <ul>\n    <li>\n      a\n    </li>\n  </ul>\n</div>\n",
        ),
        // Called at the left, it is written as typed.
        (
            "{% macro m() %}\n  <li>a</li>\n  <li>b</li>\n{% endmacro %}\n<ul>\n{{ m() }}\n</ul>\n",
            "<ul>\n  <li>a</li>\n  <li>b</li>\n</ul>\n",
        ),
        // A snippet's lines are no block's body.
        (
            "<div>\n  {{ card() }}\n</div>\n",
            "<div>\n    <p>a</p>\n    <p>b</p>\n</div>\n",
        ),
    ];
    for (template, page) in cases {
        let built = site(&[
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\npages.feed: false\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("snippets/card.html", "  <p>a</p>\n  <p>b</p>\n"),
            ("templates/home.html", template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        assert_eq!(built.html("index.md"), page, "{template}");
    }
}

#[test]
fn what_a_call_or_filter_writes_at_an_indented_line_gives_up_the_indent_its_lines_share() {
    let section =
        "{% macro m() %}\n<section>\n  <p>{{ caller() }}</p>\n</section>\n{% endmacro %}\n";
    let cases = [
        (
            "<div>\n  {% filter upper %}\n    <p>a</p>\n  {% endfilter %}\n</div>\n".to_owned(),
            "<div>\n  <P>A</P>\n</div>\n",
        ),
        // Its lines keep their indent one to another.
        (
            "<div>\n  {% filter upper %}\n    <ul>\n      <li>a</li>\n    </ul>\n  \
             {% endfilter %}\n</div>\n"
                .to_owned(),
            "<div>\n  <UL>\n    <LI>A</LI>\n  </UL>\n</div>\n",
        ),
        (
            "<ul>\n  {% for e in [1, 2] %}\n    {% filter upper %}\n      <li>a</li>\n    \
             {% endfilter %}\n  {% endfor %}\n</ul>\n"
                .to_owned(),
            "<ul>\n  <LI>A</LI>\n  <LI>A</LI>\n</ul>\n",
        ),
        (
            format!("{section}<div>\n  {{% call m() %}}{{% endcall %}}\n</div>\n"),
            "<div>\n  <section>\n    <p></p>\n  </section>\n</div>\n",
        ),
        (
            format!("{section}<div>\n  {{% call m() %}}\n  {{% endcall %}}\n</div>\n"),
            "<div>\n  <section>\n    <p></p>\n  </section>\n</div>\n",
        ),
        // At the left, each is written as typed.
        (
            "<div>\n{% filter upper %}\n  <p>a</p>\n{% endfilter %}\n</div>\n".to_owned(),
            "<div>\n  <P>A</P>\n</div>\n",
        ),
        (
            format!("{section}<div>\n{{% call m() %}}{{% endcall %}}\n</div>\n"),
            "<div>\n<section>\n  <p></p>\n</section>\n</div>\n",
        ),
        // Inside `<pre>`, spaces show.
        (
            "<div>\n  <pre>\n  {% filter upper %}\n    a\n  {% endfilter %}\n  </pre>\n</div>\n"
                .to_owned(),
            "<div>\n  <pre>\n    A\n  </pre>\n</div>\n",
        ),
    ];
    for (template, page) in cases {
        let built = site(&[
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\npages.feed: false\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("templates/home.html", &template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        assert_eq!(built.html("index.md"), page, "{template}");
    }
}

#[test]
fn a_body_held_back_is_laid_out_line_by_line_as_the_page_is() {
    let paragraphs = "{% macro inner() %}\n<p>a</p>\n<p>b</p>\n{% endmacro %}\n";
    let lined = "<div>\n  <section>\n    <p>a</p>\n    <p>b</p>\n  </section>\n</div>\n";
    let branch = |x: bool| {
        let shown = if x { "    <p>b</p>\n" } else { "" };
        format!("<div>\n  <section>\n{shown}  </section>\n</div>\n")
    };
    let cases = [
        // A value written inside a macro's body, at its own line.
        (
            format!(
                "{paragraphs}{{% macro outer() %}}\n<section>\n  {{{{ inner() }}}}\n</section>\n\
                 {{% endmacro %}}\n<div>\n  {{{{ outer() }}}}\n</div>\n"
            ),
            lined.to_owned(),
        ),
        // A call's body, written by `caller()`, gives up the indent it has
        // under its tag.
        (
            "{% macro m() %}\n<section>\n  {{ caller() }}\n</section>\n{% endmacro %}\n<div>\n  \
             {% call m() %}\n    <p>a</p>\n    <p>b</p>\n  {% endcall %}\n</div>\n"
                .to_owned(),
            lined.to_owned(),
        ),
        // A branch not taken leaves no line of spaces, in a macro's body
        // and in a set block's.
        (
            "{% macro m(x) %}\n<section>\n  {% if x %}<p>b</p>{% endif %}\n</section>\n\
             {% endmacro %}\n<div>\n  {{ m(true) }}\n</div>\n"
                .to_owned(),
            branch(true),
        ),
        (
            "{% macro m(x) %}\n<section>\n  {% if x %}<p>b</p>{% endif %}\n</section>\n\
             {% endmacro %}\n<div>\n  {{ m(false) }}\n</div>\n"
                .to_owned(),
            branch(false),
        ),
        (
            "{% set x = true %}\n{% set s %}\n<section>\n  {% if x %}<p>b</p>{% endif %}\n\
             </section>\n{% endset %}\n<div>\n  {{ s }}\n</div>\n"
                .to_owned(),
            branch(true),
        ),
        (
            "{% set x = false %}\n{% set s %}\n<section>\n  {% if x %}<p>b</p>{% endif %}\n\
             </section>\n{% endset %}\n<div>\n  {{ s }}\n</div>\n"
                .to_owned(),
            branch(false),
        ),
        // A macro called from inside a loop.
        (
            format!(
                "{paragraphs}{{% macro item(e) %}}\n<li>\n  {{{{ inner() }}}}\n</li>\n\
                 {{% endmacro %}}\n<ul>\n  {{% for e in [1, 2] %}}\n  {{{{ item(e) }}}}\n  \
                 {{% endfor %}}\n</ul>\n"
            ),
            "<ul>\n  <li>\n    <p>a</p>\n    <p>b</p>\n  </li>\n  <li>\n    <p>a</p>\n    \
             <p>b</p>\n  </li>\n</ul>\n"
                .to_owned(),
        ),
        // A macro calling a macro calling a macro.
        (
            format!(
                "{paragraphs}{{% macro middle() %}}\n<div>\n  {{{{ inner() }}}}\n</div>\n\
                 {{% endmacro %}}\n{{% macro outer() %}}\n<section>\n  {{{{ middle() }}}}\n\
                 </section>\n{{% endmacro %}}\n<main>\n  {{{{ outer() }}}}\n</main>\n"
            ),
            "<main>\n  <section>\n    <div>\n      <p>a</p>\n      <p>b</p>\n    </div>\n  \
             </section>\n</main>\n"
                .to_owned(),
        ),
        // Lines inside `<pre>` are left as typed.
        (
            "{% macro m() %}\n<section>\n  <pre>one\n  two\nthree</pre>\n</section>\n\
             {% endmacro %}\n<div>\n  {{ m() }}\n</div>\n"
                .to_owned(),
            "<div>\n  <section>\n    <pre>one\n  two\nthree</pre>\n  </section>\n\
             </div>\n"
                .to_owned(),
        ),
        // A value inside a filter's body, and a filter inside a macro's.
        (
            format!(
                "{paragraphs}<div>\n  {{% filter upper %}}\n    <section>\n      \
                 {{{{ inner() }}}}\n    </section>\n  {{% endfilter %}}\n</div>\n"
            ),
            "<div>\n  <SECTION>\n    <P>A</P>\n    <P>B</P>\n  </SECTION>\n</div>\n".to_owned(),
        ),
        (
            "{% macro m() %}\n<section>\n  {% filter upper %}\n    <p>a</p>\n    <p>b</p>\n  \
             {% endfilter %}\n</section>\n{% endmacro %}\n<div>\n  {{ m() }}\n</div>\n"
                .to_owned(),
            "<div>\n  <section>\n    <P>A</P>\n    <P>B</P>\n  </section>\n</div>\n".to_owned(),
        ),
    ];
    for (template, page) in cases {
        let built = site(&[
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\npages.feed: false\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("templates/home.html", &template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        assert_eq!(built.html("index.md"), page, "{template}");
    }
}

#[test]
fn a_body_held_back_lands_where_it_is_called_however_the_block_defining_it_moves() {
    let base = "<body>\n  <main>\n    {% block content %}{% endblock %}\n  </main>\n</body>\n";
    let list = "  {% macro m() %}\n  <ul>\n    {% for i in [1, 2] %}\n      <li>{{ i }}</li>\n    {% endfor %}\n  </ul>\n  {% endmacro %}\n";
    let cases = [
        // Called from an indented line, its lines give up the indent they
        // share for the line's.
        (
            "  <div>\n    {{ m() }}\n  </div>\n",
            "<body>\n  <main>\n    <div>\n      <ul>\n        <li>1</li>\n        <li>2</li>\n      </ul>\n    </div>\n  </main>\n</body>\n",
        ),
        // Called at the left of the block, which lands its left where the
        // block stands, the call there with it.
        (
            "  <div>\n{{ m() }}\n  </div>\n",
            "<body>\n  <main>\n      <div>\n      <ul>\n        <li>1</li>\n        <li>2</li>\n      </ul>\n      </div>\n  </main>\n</body>\n",
        ),
        // Called among other things on a line, it is written as typed,
        // less only what its own loop gives up.
        (
            "  <div>{{ m() }}</div>\n",
            "<body>\n  <main>\n    <div>  <ul>\n    <li>1</li>\n    <li>2</li>\n  </ul>\n</div>\n  </main>\n</body>\n",
        ),
    ];
    for (calling, page) in cases {
        // Defined inside the block, or at the top of the template.
        for child in [
            format!(
                "{{% extends \"base.html\" %}}\n{{% block content %}}\n{list}{calling}{{% endblock %}}\n"
            ),
            format!(
                "{{% extends \"base.html\" %}}\n{list}{{% block content %}}\n{calling}{{% endblock %}}\n"
            ),
        ] {
            let built = site(&[
                (
                    "statac.yaml",
                    "language: en\ntimezone: UTC\npages.feed: false\n",
                ),
                ("content/index.md", "---\ntitle: Home\n---\n"),
                ("templates/base.html", base),
                ("templates/home.html", &child),
            ]);
            assert!(built.problems.is_empty(), "{:?}", built.problems);
            assert_eq!(built.html("index.md"), page, "{child}");
        }
    }
}

#[test]
fn a_loop_closing_its_bodys_last_line_writes_each_pass_on_lines_of_its_own_where_indented() {
    let recursive = "{% set items = [{\"t\": \"a\", \"c\": [{\"t\": \"b\", \"c\": []}, {\"t\": \"c\", \"c\": []}]}, {\"t\": \"d\", \"c\": []}] %}\n<ul>\n  {% for item in items recursive %}\n  <li>{{ item.t }}{% if item.c %}\n    <ul>\n      {{ loop(item.c) }}\n    </ul>{% endif %}</li>{% endfor %}\n</ul>\n";
    let cases = [
        (
            "<div>\n  <p>a</p>\n  {% for i in [1, 2] %}\n  <p>{{ i }}</p>{% endfor %}\n</div>\n",
            "<div>\n  <p>a</p>\n  <p>1</p>\n  <p>2</p>\n</div>\n",
        ),
        (
            "<div>\n  <p>a</p>\n  {% for i in [] %}\n  <p>{{ i }}</p>{% endfor %}\n</div>\n",
            "<div>\n  <p>a</p>\n</div>\n",
        ),
        (
            "<ul>\n  {% for i in [1, 2] %}\n  <li>\n    {{ i }}\n  </li>{% endfor %}\n</ul>\n",
            "<ul>\n  <li>\n    1\n  </li>\n  <li>\n    2\n  </li>\n</ul>\n",
        ),
        (
            "<div>\r\n  {% for i in [1, 2] %}\r\n  <p>{{ i }}</p>{% endfor %}\r\n</div>\r\n",
            "<div>\r\n  <p>1</p>\r\n  <p>2</p>\r\n</div>\r\n",
        ),
        // A recursive loop's deeper levels go in as far as its call.
        (
            recursive,
            "<ul>\n  <li>a\n    <ul>\n      <li>b</li>\n      <li>c</li>\n    </ul></li>\n  <li>d</li>\n</ul>\n",
        ),
        // Unindented, or where the spaces show, the passes run together as
        // typed, where a line ending between them would show.
        (
            "<p>\n{% for i in [1, 2] %}\n<b>{{ i }}</b>{% endfor %}\n</p>\n",
            "<p>\n<b>1</b><b>2</b>\n</p>\n",
        ),
        (
            "<p>\n{% for i in [] %}\n<b>{{ i }}</b>{% endfor %}\n</p>\n",
            "<p>\n</p>\n",
        ),
        (
            "<pre>\n  {% for i in [1, 2] %}\n  {{ i }}{% endfor %}\n</pre>\n",
            "<pre>\n  1  2\n</pre>\n",
        ),
        // More after the end tag on its line is left as typed.
        (
            "<p>\n  {% for i in [1, 2] %}\n  <b>{{ i }}</b>{% endfor %} after\n</p>\n",
            "<p>\n  <b>1</b>  <b>2</b> after\n</p>\n",
        ),
    ];
    for (body, page) in cases {
        // Straight into the page, and in a body the engine holds back.
        let held = format!("{{% set s %}}\n{body}{{% endset %}}\n{{{{ s }}}}\n");
        for template in [body, held.as_str()] {
            let built = site(&[
                (
                    "statac.yaml",
                    "language: en\ntimezone: UTC\npages.feed: false\n",
                ),
                ("content/index.md", "---\ntitle: Home\n---\n"),
                ("templates/home.html", template),
            ]);
            assert!(built.problems.is_empty(), "{:?}", built.problems);
            assert_eq!(built.html("index.md"), page, "{template}");
        }
    }
}

#[test]
fn a_body_held_back_keeps_the_lines_round_a_branch_as_the_page_does() {
    let templates = [
        "<div>\n  {% if a %}\n  {% if b %}\n  <p>x</p>{% endif %}{% endif %}\n</div>\n",
        // After a line whose line ending goes with the outer branch.
        "<div>\n  <p>w</p>{% if a %}\n  {% if b %}\n  <p>x</p>{% endif %}{% endif %}\n  <p>y</p>\n</div>\n",
        // Closing one branch and dividing another.
        "<div>\n  {% if a %}\n  <p>w</p>\n  {% if b %}\n  <p>x</p>{% endif %}{% else %}\n  <p>y</p>{% endif %}\n</div>\n",
        // A blank line, or a line ending with a comment, before the tag
        // closing a branch.
        "<div>\n  <p>w</p>{% if a %}\n  <p>x</p>\n\n  {% endif %}\n  <p>y</p>\n</div>\n",
        "<div>\n  <p>w</p>{% if b %}\n  <p>x</p>{# x #}\n  {% endif %}\n  <p>y</p>\n</div>\n",
    ];
    for template in templates {
        for (a, b) in [(true, true), (true, false), (false, true), (false, false)] {
            let set = format!("{{% set a = {a} %}}{{% set b = {b} %}}\n");
            let page = |template: &str| {
                let built = site(&[
                    (
                        "statac.yaml",
                        "language: en\ntimezone: UTC\npages.feed: false\n",
                    ),
                    ("content/index.md", "---\ntitle: Home\n---\n"),
                    ("templates/home.html", template),
                ]);
                assert!(built.problems.is_empty(), "{:?}", built.problems);
                built.html("index.md")
            };
            // Built as it is straight into the page.
            let straight = page(&format!("{set}{template}"));
            assert!(!straight.contains("></"), "{straight}");
            let held = page(&format!(
                "{set}{{% set s %}}\n{template}{{% endset %}}\n{{{{ s }}}}\n"
            ));
            assert_eq!(held, straight, "{template} {a} {b}");
        }
    }
}

#[test]
fn a_snippet_put_among_other_things_on_a_line_keeps_the_spaces_it_starts_with() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "snippets/items.html",
            "{% for item in [\"a\", \"b\"] %}\n    <b>{{ item }}</b>\n{% endfor %}\n",
        ),
        (
            "templates/home.html",
            "<p>Items:{{ items() }}</p>\n<div>\n  {{ items() }}\n</div>\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<p>Items:    <b>a</b>\n<b>b</b>\n</p>\n<div>\n  <b>a</b>\n  <b>b</b>\n</div>\n"
    );
}

#[test]
fn a_block_written_in_another_templates_place_lines_up_where_it_lands() {
    let base = (
        "templates/base.html",
        "<body>\n  <main>\n    {% block main %}{% endblock %}\n  </main>\n  <aside>\n  {% block aside %}\n    <p>Default</p>\n  {% endblock %}\n  </aside>\n</body>\n",
    );
    // Written flush left, under its tag, or at the depth it lands at.
    for body in [
        "<h1>{{ page.title }}</h1>\n<div>\n  <p>a</p>\n</div>\n{{ page.content }}\n",
        "  <h1>{{ page.title }}</h1>\n  <div>\n    <p>a</p>\n  </div>\n  {{ page.content }}\n",
        "    <h1>{{ page.title }}</h1>\n    <div>\n      <p>a</p>\n    </div>\n    {{ page.content }}\n",
    ] {
        let built = site(&[
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\npages.feed: false\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\nOne.\n\nTwo.\n"),
            base,
            (
                "templates/home.html",
                &format!(
                    "{{% extends \"base\" %}}\n{{% block main %}}\n{body}{{% endblock %}}\n{{% block aside %}}\n<p>Mine</p>\n{{% endblock %}}\n"
                ),
            ),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        assert_eq!(
            built.html("index.md"),
            "<body>\n  <main>\n    <h1>Home</h1>\n    <div>\n      <p>a</p>\n    </div>\n    <p>One.</p>\n    <p>Two.</p>\n  </main>\n  <aside>\n    <p>Mine</p>\n  </aside>\n</body>\n",
            "{body}"
        );
    }
}

#[test]
fn super_writes_the_replaced_body_where_it_is_called() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "templates/base.html",
            "<main>\n  {% block main %}\n  <p>Base</p>\n  {% endblock %}\n</main>\n",
        ),
        (
            "templates/home.html",
            "{% extends \"base\" %}\n{% block main %}\n<div>\n  {{ super() }}\n</div>\n{% endblock %}\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<main>\n  <div>\n    <p>Base</p>\n  </div>\n</main>\n"
    );
}

#[test]
fn a_recursive_loops_deeper_levels_go_in_as_far_as_its_call() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "templates/home.html",
            "<ul>\n  {% for item in [{\"t\": \"a\", \"c\": [{\"t\": \"b\", \"c\": [{\"t\": \"c\", \"c\": []}]}]}] recursive %}\n  <li>\n    {{ item.t }}\n    {% if item.c %}\n    <ul>\n      {{ loop(item.c) }}\n    </ul>\n    {% endif %}\n  </li>\n  {% endfor %}\n</ul>\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<ul>\n  <li>\n    a\n    <ul>\n      <li>\n        b\n        <ul>\n          <li>\n            c\n          </li>\n        </ul>\n      </li>\n    </ul>\n  </li>\n</ul>\n"
    );
}

#[test]
fn a_line_holding_more_than_tags_keeps_its_indent_and_line_end() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "snippets/figure.html",
            "<figure>{{ \"<img>\" | safe }}\n{% if caption is defined %}<figcaption>{{ caption }}</figcaption>{% endif %}\n</figure>\n",
        ),
        (
            "snippets/joined.html",
            "<b>\n{%- if true %}\n  {% if true %}<i>a</i>{% endif %}\n{%- endif %}\n</b>\n",
        ),
        ("snippets/among.html", "{% if true %}<i>a</i>{% endif %}\n"),
        (
            "templates/home.html",
            "<div>\n  {% if page.title %}<p>yes</p>{% endif %}\n  {% if not page.title %}<p>no</p>{% endif %}\n  <p>after</p>\n  {{ figure() }}\n  {{ figure(caption=\"c\") }}\n  {{ joined() }}\n  <p>{{ among() }}.</p>\n</div>\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<div>\n  <p>yes</p>\n  <p>after</p>\n  <figure><img>\n  </figure>\n  <figure><img>\n  <figcaption>c</figcaption>\n  </figure>\n  <b><i>a</i></b>\n  <p><i>a</i>.</p>\n</div>\n"
    );
}

#[test]
fn a_line_ending_with_a_tag_and_holding_more_keeps_its_line_end() {
    let following = "<div>\n    <p>{{ page.title }}</p>{% if page.summary %}\n    <p>{{ page.summary }}</p>{% endif %}\n    <p>after</p>\n</div>\n";
    let alone = "<div>\n    <p>{{ page.title }}</p>\n    {% if page.summary %}\n    <p>{{ page.summary }}</p>{% endif %}\n    <p>after</p>\n</div>\n";
    for template in [following, alone] {
        for (summary, built) in [
            (
                "\nsummary: Summary\n",
                "<div>\n    <p>Home</p>\n    <p>Summary</p>\n    <p>after</p>\n</div>\n",
            ),
            ("\n", "<div>\n    <p>Home</p>\n    <p>after</p>\n</div>\n"),
        ] {
            let page = format!("---\ntitle: Home{summary}---\n");
            let site = site(&[
                (
                    "statac.yaml",
                    "language: en\ntimezone: UTC\npages.feed: false\n",
                ),
                ("content/index.md", page.as_str()),
                ("templates/home.html", template),
            ]);
            assert!(site.problems.is_empty(), "{:?}", site.problems);
            assert_eq!(site.html("index.md"), built, "{template}{summary}");
        }
    }
}

#[test]
fn a_recursive_loop_ending_a_line_with_a_tag_keeps_its_line_ends() {
    let items = "[{\"t\": \"a\", \"c\": [{\"t\": \"b\", \"c\": []}]}, {\"t\": \"c\", \"c\": []}]";
    for opening in ["\n    {% if item.c %}", "{% if item.c %}"] {
        let template = format!(
            "<ul>\n  {{% for item in {items} recursive %}}\n  <li>\n    {{{{ item.t }}}}{opening}\n    <ul>\n      {{{{ loop(item.c) }}}}\n    </ul>{{% endif %}}\n  </li>\n  {{% endfor %}}\n</ul>\n"
        );
        let built = site(&[
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\npages.feed: false\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("templates/home.html", template.as_str()),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        assert_eq!(
            built.html("index.md"),
            "<ul>\n  <li>\n    a\n    <ul>\n      <li>\n        b\n      </li>\n    </ul>\n  </li>\n  <li>\n    c\n  </li>\n</ul>\n",
            "{template}"
        );
    }
}

/// Builds `template` with `@x` taken as a branch's condition and `@l` as a
/// loop's list, giving the page.
fn built_with(template: &str, x: &str, l: &str) -> String {
    let template = template.replace("@x", x).replace("@l", l);
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        ("templates/home.html", template.as_str()),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    built.html("index.md")
}

#[test]
fn a_branch_not_written_leaves_the_line_before_it_its_line_end() {
    // A condition, a list and the page a template builds from them.
    type Builds<'a> = &'a [(&'a str, &'a str, &'a str)];
    let cases: &[(&str, Builds)] = &[
        // The branch's end tag alone on its line.
        (
            "<p>a</p>{% if @x %}\n<p>b</p>\n{% endif %}\n<p>c</p>\n",
            &[
                ("true", "[]", "<p>a</p>\n<p>b</p>\n<p>c</p>\n"),
                ("false", "[]", "<p>a</p>\n<p>c</p>\n"),
            ],
        ),
        // Blank lines as typed, round the branch and in it.
        (
            "<p>a</p>{% if @x %}\n\n<p>b</p>\n\n{% endif %}\n\n<p>c</p>\n",
            &[
                ("true", "[]", "<p>a</p>\n\n<p>b</p>\n\n\n<p>c</p>\n"),
                ("false", "[]", "<p>a</p>\n\n<p>c</p>\n"),
            ],
        ),
        (
            "<div>\n  <p>a</p>{% if @x %}\n  <p>b</p>\n  {% endif %}\n  <p>c</p>\n</div>\n",
            &[
                (
                    "true",
                    "[]",
                    "<div>\n  <p>a</p>\n  <p>b</p>\n  <p>c</p>\n</div>\n",
                ),
                ("false", "[]", "<div>\n  <p>a</p>\n  <p>c</p>\n</div>\n"),
            ],
        ),
        (
            "<p>a</p>{% if @x %}\n<p>b</p>\n{% else %}\n<p>c</p>\n{% endif %}\n<p>d</p>\n",
            &[
                ("true", "[]", "<p>a</p>\n<p>b</p>\n<p>d</p>\n"),
                ("false", "[]", "<p>a</p>\n<p>c</p>\n<p>d</p>\n"),
            ],
        ),
        (
            "<p>a</p>{% if @x %}\n<p>b</p>{% else %}\n<p>c</p>{% endif %}\n<p>d</p>\n",
            &[
                ("true", "[]", "<p>a</p>\n<p>b</p>\n<p>d</p>\n"),
                ("false", "[]", "<p>a</p>\n<p>c</p>\n<p>d</p>\n"),
            ],
        ),
        (
            "<p>a</p>{% for i in @l %}\n<p>{{ i }}</p>\n{% endfor %}\n<p>c</p>\n",
            &[
                ("true", "[1, 2]", "<p>a</p>\n<p>1</p>\n<p>2</p>\n<p>c</p>\n"),
                ("true", "[]", "<p>a</p>\n<p>c</p>\n"),
            ],
        ),
        (
            "<p>a</p>{% for i in @l %}\n<p>{{ i }}</p>\n{% else %}\n<p>none</p>\n{% endfor %}\n<p>c</p>\n",
            &[
                ("true", "[1, 2]", "<p>a</p>\n<p>1</p>\n<p>2</p>\n<p>c</p>\n"),
                ("true", "[]", "<p>a</p>\n<p>none</p>\n<p>c</p>\n"),
            ],
        ),
        // Branches one after another.
        (
            "<p>a</p>{% if @x %}\n<p>b</p>\n{% endif %}\n{% if @l %}\n<p>c</p>\n{% endif %}\n<p>d</p>\n",
            &[
                ("true", "[1]", "<p>a</p>\n<p>b</p>\n<p>c</p>\n<p>d</p>\n"),
                ("true", "[]", "<p>a</p>\n<p>b</p>\n<p>d</p>\n"),
                ("false", "[1]", "<p>a</p>\n<p>c</p>\n<p>d</p>\n"),
                ("false", "[]", "<p>a</p>\n<p>d</p>\n"),
            ],
        ),
        (
            "<p>a</p>{% if @x %}\n<p>b</p>{% for i in @l %}\n<p>{{ i }}</p>\n{% endfor %}\n{% endif %}\n<p>c</p>\n",
            &[
                (
                    "true",
                    "[1, 2]",
                    "<p>a</p>\n<p>b</p>\n<p>1</p>\n<p>2</p>\n<p>c</p>\n",
                ),
                ("true", "[]", "<p>a</p>\n<p>b</p>\n<p>c</p>\n"),
                ("false", "[1, 2]", "<p>a</p>\n<p>c</p>\n"),
            ],
        ),
        (
            "<p>a</p>{% if @x %}{% if @l %}\n<p>b</p>\n{% endif %}\n{% endif %}\n<p>c</p>\n",
            &[
                ("true", "[1]", "<p>a</p>\n<p>b</p>\n<p>c</p>\n"),
                ("true", "[]", "<p>a</p>\n<p>c</p>\n"),
                ("false", "[1]", "<p>a</p>\n<p>c</p>\n"),
            ],
        ),
        // A body the engine holds back.
        (
            "{% macro m(x) %}\n<p>a</p>{% if x %}\n<p>b</p>{% else %}\n<p>c</p>{% endif %}\n<p>d</p>{% if x %}\n<p>e</p>{% endif %}\n<p>f</p>\n{% endmacro %}\n{{ m(@x) }}\n",
            &[
                (
                    "true",
                    "[]",
                    "<p>a</p>\n<p>b</p>\n<p>d</p>\n<p>e</p>\n<p>f</p>\n",
                ),
                ("false", "[]", "<p>a</p>\n<p>c</p>\n<p>d</p>\n<p>f</p>\n"),
            ],
        ),
        (
            "{% macro m(x) %}\n  <p>a</p>{% if x %}\n  <p>b</p>\n  {% endif %}\n  <p>c</p>\n{% endmacro %}\n{{ m(@x) }}\n",
            &[
                ("true", "[]", "  <p>a</p>\n  <p>b</p>\n  <p>c</p>\n"),
                ("false", "[]", "  <p>a</p>\n  <p>c</p>\n"),
            ],
        ),
        (
            "{% set s %}\n<p>a</p>{% if @x %}\n<p>b</p>\n{% else %}\n<p>c</p>\n{% endif %}\n<p>d</p>\n{% endset %}\n{{ s }}\n",
            &[
                ("true", "[]", "<p>a</p>\n<p>b</p>\n<p>d</p>\n"),
                ("false", "[]", "<p>a</p>\n<p>c</p>\n<p>d</p>\n"),
            ],
        ),
        (
            "{% macro w() %}{{ caller() }}{% endmacro %}\n{% call w() %}\n<p>a</p>{% for i in @l %}\n<p>{{ i }}</p>\n{% endfor %}\n<p>c</p>\n{% endcall %}\n",
            &[
                ("true", "[1, 2]", "<p>a</p>\n<p>1</p>\n<p>2</p>\n<p>c</p>\n"),
                ("true", "[]", "<p>a</p>\n<p>c</p>\n"),
            ],
        ),
        (
            "{% macro m(x) %}\n  <p>a</p>\n  {% if x %}\n  <p>b</p>{% else %}\n  <p>c</p>{% endif %}\n  <p>d</p>\n{% endmacro %}\n{{ m(@x) }}\n",
            &[
                ("true", "[]", "  <p>a</p>\n  <p>b</p>\n  <p>d</p>\n"),
                ("false", "[]", "  <p>a</p>\n  <p>c</p>\n  <p>d</p>\n"),
            ],
        ),
        (
            "{% set s %}\n  <p>a</p>\n  {% for i in @l %}\n  <p>{{ i }}</p>{% endfor %}\n  <p>c</p>\n{% endset %}\n{{ s }}\n",
            &[
                (
                    "true",
                    "[1, 2]",
                    "  <p>a</p>\n  <p>1</p>\n  <p>2</p>\n  <p>c</p>\n",
                ),
                ("true", "[]", "  <p>a</p>\n  <p>c</p>\n"),
            ],
        ),
        (
            "{% filter upper %}\na\n{% if @x %}\nb{% endif %}\nc\n{% endfilter %}\n",
            &[("true", "[]", "A\nB\nC\n"), ("false", "[]", "A\nC\n")],
        ),
        // One branch after another, and one inside another.
        (
            "{% macro m(x) %}\n<p>c</p>{% if x %}\n<p>d</p>\n{% endif %}\n{% if x %}\n<p>e</p>{% endif %}\n<p>f</p>\n{% endmacro %}\n{{ m(@x) }}\n",
            &[
                ("true", "[]", "<p>c</p>\n<p>d</p>\n<p>e</p>\n<p>f</p>\n"),
                ("false", "[]", "<p>c</p>\n<p>f</p>\n"),
            ],
        ),
        (
            "{% macro m(a, b) %}\n<p>x</p>\n{% if a %}\n{% if b %}\n<p>y</p>{% endif %}\n<p>z</p>{% endif %}\n<p>w</p>\n{% endmacro %}\n{{ m(@x, @l) }}\n",
            &[
                ("true", "[1]", "<p>x</p>\n<p>y</p>\n<p>z</p>\n<p>w</p>\n"),
                ("true", "[]", "<p>x</p>\n<p>z</p>\n<p>w</p>\n"),
                ("false", "[1]", "<p>x</p>\n<p>w</p>\n"),
                ("false", "[]", "<p>x</p>\n<p>w</p>\n"),
            ],
        ),
        // The branch ending the body, which writes a line end either way.
        (
            "{% macro m(x) %}\n<p>a</p>{% if x %}\n<p>b</p>\n{% endif %}\n{% endmacro %}\n{{ m(@x) }}<p>z</p>\n",
            &[
                ("true", "[]", "<p>a</p>\n<p>b</p>\n<p>z</p>\n"),
                ("false", "[]", "<p>a</p>\n<p>z</p>\n"),
            ],
        ),
        // After a blank line or a line ending with a tag, as in the page.
        (
            "{% macro m(x) %}\n<p>a</p>{% if x %}\n<p>b</p>\n\n{% endif %}\n<p>c</p>\n{% endmacro %}\n{{ m(@x) }}\n",
            &[
                ("true", "[]", "<p>a</p>\n<p>b</p>\n\n<p>c</p>\n"),
                ("false", "[]", "<p>a</p>\n<p>c</p>\n"),
            ],
        ),
        (
            "{% macro m(x) %}\n<p>a</p>{% if x %}\n<p>b</p>{# b #}\n{% endif %}\n<p>c</p>\n{% endmacro %}\n{{ m(@x) }}\n",
            &[
                ("true", "[]", "<p>a</p>\n<p>b</p>\n<p>c</p>\n"),
                ("false", "[]", "<p>a</p>\n<p>c</p>\n"),
            ],
        ),
        (
            "<ul>\n{% for i in @l %}\n  <li>{{ i }}</li>{% if @x %}\n  <li>more</li>\n  {% endif %}\n{% endfor %}\n</ul>\n",
            &[
                (
                    "true",
                    "[1, 2]",
                    "<ul>\n  <li>1</li>\n  <li>more</li>\n  <li>2</li>\n  <li>more</li>\n</ul>\n",
                ),
                (
                    "false",
                    "[1, 2]",
                    "<ul>\n  <li>1</li>\n  <li>2</li>\n</ul>\n",
                ),
                ("false", "[]", "<ul>\n</ul>\n"),
            ],
        ),
        // Inside `<pre>` and `<script>` the line end shows or means something.
        (
            "<pre>\na{% if @x %}\nb{% endif %}\nc\n{% if @x %}\nd{% endif %}\ne{% if @x %}\nf\n{% endif %}\ng</pre>\n",
            &[
                ("true", "[]", "<pre>\na\nb\nc\nd\ne\nf\ng</pre>\n"),
                ("false", "[]", "<pre>\na\nc\ne\ng</pre>\n"),
            ],
        ),
        (
            "<script>\n  go(); // a{% if @x %}\n  stop();{% endif %}\n  end();\n</script>\n",
            &[
                (
                    "true",
                    "[]",
                    "<script>\n  go(); // a\n  stop();\n  end();\n</script>\n",
                ),
                (
                    "false",
                    "[]",
                    "<script>\n  go(); // a\n  end();\n</script>\n",
                ),
            ],
        ),
    ];
    for &(template, cases) in cases {
        for &(x, l, built) in cases {
            assert_eq!(built_with(template, x, l), built, "{template} {x} {l}");
        }
    }
}

#[test]
fn a_tag_takes_the_spaces_round_it_whatever_they_are_written_with() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        ("content/index.md", "---\ntitle: Home\n---\n"),
        (
            "content/data.md",
            "---\ntitle: Data\npermalink: /data.json\nlayout: data.json\n---\n",
        ),
        // Line endings of two characters, a comment, `raw`, and a
        // no-break space before a tag.
        (
            "templates/home.html",
            "<div>\r\n  {# a note #}\r\n  {% raw %}\r\n  {{ as typed }}\r\n  {% endraw %}\r\n\
             \u{a0}{% if true %}\r\n  <p>a</p>\r\n\u{a0}{% endif %}\r\n</div>\r\n",
        ),
        // A file whose lines are written as typed.
        (
            "templates/data.json",
            "{% set a = 1 %}\n{\n  {% if true %}\n  \"a\": {{ a }}\n  {% endif %}\n}\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("index.md"),
        "<div>\r\n  {{ as typed }}\r\n  <p>a</p>\r\n</div>\r\n"
    );
    assert_eq!(built.html("data.md"), "{\n  \"a\": 1\n}\n");
}

#[test]
fn a_body_held_back_is_indented_as_a_browser_reads_its_markup() {
    let cases = [
        // No spaces go inside `<pre>` after a comment closed by `--!>`.
        (
            "{% macro m() %}\n<!-- a --!>\n<pre>\nx\n</pre>\n<p>y</p>\n{% endmacro %}\n<div>\n  {{ m() }}\n</div>\n",
            "<div>\n  <!-- a --!>\n  <pre>\nx\n</pre>\n  <p>y</p>\n</div>\n",
        ),
        // An apostrophe in an attribute's value written without quotes
        // leaves the lines after it to take the indent.
        (
            "{% macro m() %}\n<p><a title=it's>a</a></p>\n<p>b</p>\n<p>c</p>\n{% endmacro %}\n<div>\n  {{ m() }}\n</div>\n",
            "<div>\n  <p><a title=it's>a</a></p>\n  <p>b</p>\n  <p>c</p>\n</div>\n",
        ),
    ];
    for (template, page) in cases {
        let built = site(&[
            (
                "statac.yaml",
                "language: en\ntimezone: UTC\npages.feed: false\n",
            ),
            ("content/index.md", "---\ntitle: Home\n---\n"),
            ("templates/home.html", template),
        ]);
        assert!(built.problems.is_empty(), "{:?}", built.problems);
        assert_eq!(built.html("index.md"), page, "{template}");
    }
}

#[test]
fn a_template_writing_plain_text_keeps_its_bodys_indent() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/notes.md",
            "---\ntitle: Notes\npermalink: /notes.txt\nlayout: notes.txt\n---\n",
        ),
        (
            "templates/notes.txt",
            "- {{ page.title }}\n{% for line in [\"a\", \"b\"] %}\n    - {{ line }}\n{% endfor %}\n",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(built.html("notes.md"), "- Notes\n    - a\n    - b\n");
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
