//! The pieces of the writing a site writes itself, in `snippets/markdown/`.

use super::*;

/// A piece file printing every name it is handed, each as `name=value`.
fn every_name(piece: crate::markdown::Piece) -> (String, String) {
    let names: Vec<String> = piece
        .handed()
        .iter()
        .map(|name| match *name {
            "notes" => "notes={% for note in notes %}[{{ note.number }} {{ note.shown }} \
                        {{ note.id }} \
                        {{ note.back }} {{ note.body }}]{% endfor %}"
                .to_owned(),
            "photo" => "photo={% if photo %}{{ photo.src }}{% endif %}".to_owned(),
            _ => format!("{name}={{{{ {name} }}}}"),
        })
        .collect();
    (
        format!("snippets/markdown/{}.html", piece.name()),
        format!("<{} {}>", piece.name(), names.join(" ")),
    )
}

#[test]
fn every_piece_is_written_through_its_file_handed_what_the_writing_says() {
    let files: Vec<(String, String)> = crate::markdown::Piece::ALL
        .into_iter()
        .map(every_name)
        .collect();
    let mut site_files: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    site_files.extend([
        (
            "statac.yaml",
            "language: en\nurl: https://example.org\nalso_at: [example.net]\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n## A *heading*\n\n\
             [In](/about/) [own](https://example.net/a) [out](https://other.org \"Other\") \
             ![A picture](a.png \"Shown\") `x < y`[^1]\n\n\
             ```rust example.rs\nlet x = 1 < 2;\n```\n\n\
             > [!TIP]\n> Words.\n\n[^1]: A note.\n",
        ),
        BASE,
        PAGE,
    ]);
    let built = site(&site_files);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let rendered = built.render("post-1.md");
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
    assert_eq!(
        rendered.html.unwrap(),
        "<title>Post 1</title>\
         <heading level=2 id=a-heading classes= body=A <em>heading</em>>\n\
         <p>\
         <link address=/about/ title= body=In outside=False> \
         <link address=https://example.net/a title= body=own outside=False> \
         <link address=https://other.org title=Other body=out outside=True> \
         <image address=a.png alt=A picture title=Shown width= height= photo=> \
         <inline_code code=x &lt; y>\
         <footnote number=1 shown=1 id=back-1 note=note-1></p>\n\
         <code code=let x = 1 &lt; 2; body=<span class=\"hl-4k\">let</span> \
         <span class=\"hl-52\">x</span> = <span class=\"hl-5z\">1</span> &lt; \
         <span class=\"hl-5z\">2</span>; language=rust caption=example.rs>\n\
         <callout kind=tip title=Tip body=<p>Words.</p>>\n\
         <footnotes notes=[1 1 note-1 back-1 <p>A note.</p>] label=back to the text>"
    );
    // Each piece's file is read by the page using it.
    let pieces = rendered
        .reads
        .keys()
        .iter()
        .filter(|key| match key {
            Key::Template(id) => built
                .renderer
                .sources()
                .path(built.renderer.template_source(*id))
                .starts_with("snippets/markdown"),
            _ => false,
        })
        .count();
    assert!(pieces >= crate::markdown::Piece::ALL.len(), "{pieces}");
}

/// A value read as Markdown is written through the files that need
/// nothing of a page's own writing: not a heading's, a picture's or a
/// footnote's.
#[test]
fn a_value_read_as_markdown_is_written_through_the_pieces_that_need_no_page() {
    let files: Vec<(String, String)> = crate::markdown::Piece::ALL
        .into_iter()
        .map(every_name)
        .collect();
    let mut site_files: Vec<(&str, &str)> = files
        .iter()
        .map(|(path, text)| (path.as_str(), text.as_str()))
        .collect();
    site_files.extend([
        (
            "statac.yaml",
            "language: en\nurl: https://example.org\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummary: \"A \\\"quoted\\\" [link](https://other.org)\"\ndeck: |\n  \
             ## A heading\n\n  \
             [In](/about/) ![A picture](a.png) `x < y`[^1]\n\n  \
             ```rust\n  let x = 1;\n  ```\n\n  \
             > [!TIP]\n  > Words.\n\n  [^1]: A note.\n---\n",
        ),
        (
            "templates/page.html",
            "{{ page.summary | markdown }}\n{{ page.deck | markdown }}",
        ),
    ]);
    let built = site(&site_files);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let rendered = built.render("post-1.md");
    assert!(rendered.problems.is_empty(), "{:?}", rendered.problems);
    assert_eq!(
        rendered.html.unwrap(),
        "A \u{201c}quoted\u{201d} <link address=https://other.org title= body=link outside=True>\n\
         <h2>A heading</h2>\n\
         <p><link address=/about/ title= body=In outside=False> \
         <img src=\"a.png\" alt=\"A picture\" loading=\"lazy\" decoding=\"async\"> \
         <inline_code code=x &lt; y>\
         <sup class=\"footnote\"><a href=\"#note-1\" id=\"back-1\">1</a></sup></p>\n\
         <code code=let x = 1; body=let x = 1; language=rust caption=>\n\
         <callout kind=tip title=Tip body=<p>Words.</p>>\n\
         <section class=\"footnotes\">\n  <ol>\n    <li id=\"note-1\">\n      \
         <p>A note. <a href=\"#back-1\" class=\"back\">\u{21a9}</a></p>\n    </li>\n  </ol>\n\
         </section>"
    );
}

/// A piece's file that stops in a value is told once, and the value
/// written as Statac writes it.
#[test]
fn a_piece_that_stops_in_a_value_is_told_once() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\nsummary: \"[One](/a/) and [two](/b/)\"\n---\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\nsummary: Two\ndeck: Two\n---\n",
        ),
        ("templates/page.html", "{{ page.summary | markdown }}"),
        (
            "snippets/markdown/link.html",
            "<a href=\"{{ address }}\">{{ page.deck }}</a>",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let rendered = built.render("post-1.md");
    assert_eq!(
        built.told(&rendered.problems),
        [(
            "no-value",
            "snippets/markdown/link.html".into(),
            "page.deck".into()
        )]
    );
    assert_eq!(
        built.context(&rendered.problems[0])[0],
        (
            Relation::CalledFrom,
            "content/post-1.md".into(),
            "[One](/a/)".into()
        )
    );
    assert_eq!(rendered.html, None);
}

/// An address written bare is a link like any other, and leaves the site
/// by where it points, `http://` and all for a `www.` address.
#[test]
fn an_address_written_bare_is_written_through_the_link_file() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\nurl: https://www.example.org\nalso_at: [example.net]\ntimezone: UTC\n\
             pages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\nwww.example.org/a https://example.net/b \
             https://other.org/c, name@example.org and [written](https://other.org/c).\n",
        ),
        BASE,
        PAGE,
        (
            "snippets/markdown/link.html",
            "<link {{ address }} {{ title }} {{ outside }}>{{ body }}</link>",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    assert_eq!(
        built.html("post-1.md"),
        "<title>Post 1</title><p>\
         <link http://www.example.org/a  False>www.example.org/a</link> \
         <link https://example.net/b  False>https://example.net/b</link> \
         <link https://other.org/c  True>https://other.org/c</link>, \
         <link mailto:name@example.org  False>name@example.org</link> and \
         <link https://other.org/c  True>written</link>.</p>"
    );
}

#[test]
fn a_piece_writing_nothing_leaves_nothing_and_one_writing_lines_is_laid_out() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n> [!NOTE]\n> ## Inside\n\n## Gone\n\nWords.\n",
        ),
        BASE,
        PAGE,
        (
            "snippets/markdown/heading.html",
            "{% if level == 2 and id == \"inside\" %}<h2 id=\"{{ id }}\">{{ body }}</h2>\n\
             <a href=\"#{{ id }}\">#</a>{% endif %}\n",
        ),
    ]);
    assert_eq!(
        built.html("post-1.md"),
        "<title>Post 1</title><aside class=\"callout note\">\n  \
         <h2 id=\"inside\">Inside</h2>\n  <a href=\"#inside\">#</a>\n</aside>\n<p>Words.</p>"
    );
}

#[test]
fn a_file_in_the_pieces_folder_that_is_no_piece_is_told_with_the_one_meant() {
    let built = site(&[
        ("content/post-1.md", "Words.\n"),
        BASE,
        PAGE,
        ("snippets/markdown/headings.html", "<h1>{{ body }}</h1>"),
        ("snippets/markdown/notes.txt", "Notes."),
    ]);
    let told: Vec<_> = built
        .problems
        .iter()
        .map(|p| (p.headline(), p.suggestion()))
        .collect();
    assert_eq!(
        told,
        [
            (
                "Unknown file: `snippets/markdown/headings.html`".to_owned(),
                Some("Did you mean `snippets/markdown/heading.html`?".to_owned())
            ),
            (
                "Unknown file: `snippets/markdown/notes.txt`".to_owned(),
                None
            ),
        ]
    );
}

#[test]
fn a_piece_is_checked_against_the_names_it_is_handed() {
    let built = site(&[
        ("content/post-1.md", "Words.\n"),
        BASE,
        PAGE,
        (
            "snippets/markdown/link.html",
            "<a href=\"{{ adress }}\">{{ body }}</a>{{ page.title }}",
        ),
    ]);
    assert_eq!(
        built.template_problems(),
        [(
            "unknown-word",
            "snippets/markdown/link.html".into(),
            "adress".into()
        )]
    );
    // No call reaches a piece.
    let built = site(&[
        ("content/post-1.md", "---\ntitle: Post 1\n---\n@link('x')\n"),
        BASE,
        PAGE,
        ("snippets/markdown/link.html", "<a>{{ body }}</a>"),
    ]);
    let rendered = built.render("post-1.md");
    assert_eq!(
        built.told(&rendered.problems),
        [("unknown-word", "content/post-1.md".into(), "link".into())]
    );
}

#[test]
fn a_piece_that_stops_is_told_once_and_the_page_written_as_statac_writes_it() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n## One\n\n## Two\n",
        ),
        (
            "content/post-2.md",
            "---\ntitle: Post 2\nsummary: Two\n---\n",
        ),
        BASE,
        PAGE,
        (
            "snippets/markdown/heading.html",
            "<h{{ level }}>{{ page.summary }}</h{{ level }}>",
        ),
    ]);
    assert!(built.problems.is_empty(), "{:?}", built.problems);
    let rendered = built.render("post-1.md");
    let told = built.told(&rendered.problems);
    assert_eq!(
        told,
        [(
            "no-value",
            "snippets/markdown/heading.html".into(),
            "page.summary".into()
        )]
    );
    let context = built.context(&rendered.problems[0]);
    assert_eq!(
        context[0],
        (
            Relation::CalledFrom,
            "content/post-1.md".into(),
            "## One".into()
        )
    );
    assert_eq!(rendered.html, None);
}

#[test]
fn a_picture_alone_written_as_a_block_stands_in_place_of_its_paragraph() {
    let built = site(&[
        (
            "statac.yaml",
            "language: en\ntimezone: UTC\npages.feed: false\n",
        ),
        (
            "content/post-1.md",
            "---\ntitle: Post 1\n---\n![One](1.png \"Titled\")\n\n![Two](2.png)\n\n\
             Words ![Three](3.png \"Titled\").\n\n[![Four](4.png \"Titled\")](/a/)\n\n\
             > ![Five](5.png \"Titled\")\n",
        ),
        BASE,
        PAGE,
        (
            "snippets/markdown/image.html",
            "{% if title %}<figure><img src=\"{{ address }}\" alt=\"{{ alt }}\">\n\
             <figcaption>{{ title }}</figcaption></figure>\
             {% else %}<img src=\"{{ address }}\" alt=\"{{ alt }}\">{% endif %}\n",
        ),
    ]);
    assert_eq!(
        built.html("post-1.md"),
        "<title>Post 1</title>\
         <figure><img src=\"1.png\" alt=\"One\">\n<figcaption>Titled</figcaption></figure>\n\
         <p><img src=\"2.png\" alt=\"Two\"></p>\n\
         <p>Words <figure><img src=\"3.png\" alt=\"Three\">\n\
         <figcaption>Titled</figcaption></figure>.</p>\n\
         <p><a href=\"/a/\"><figure><img src=\"4.png\" alt=\"Four\">\n\
         <figcaption>Titled</figcaption></figure></a></p>\n\
         <blockquote>\n  \
         <figure><img src=\"5.png\" alt=\"Five\">\n  <figcaption>Titled</figcaption></figure>\n\
         </blockquote>"
    );
}

/// A footnote's marker the site writes itself is left out of the excerpt
/// and the word count, as Statac's own is.
#[test]
fn a_footnote_marker_the_site_writes_is_left_out_of_the_excerpt() {
    let built = site(&[
        NO_HOME,
        ("content/post-1.md", "One two.[^a]\n\n[^a]: Three.\n"),
        (
            "snippets/markdown/footnote.html",
            "<sup>[{{ number }}]</sup>",
        ),
        ("templates/page.html", "{{ page.excerpt }}|{{ page.words }}"),
    ]);
    assert_eq!(built.html("post-1.md"), "<p>One two.</p>|3");
}
