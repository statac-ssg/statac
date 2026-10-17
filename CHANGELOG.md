# Changes

What changed in each release of Statac, the newest first.

## Unreleased

### Fixed

- A branch in the body of a `macro`, a `{% set %}` block, a
  `{% call %}` or a `{% filter %}` keeps the lines round it apart,
  whether or not it is written, where a tag alone on its line opens or
  closes it, as `<p>a</p>{% if x %}` with `{% endif %}` on the line
  after the branch, or `{% if x %}` on the line after `<p>a</p>` with
  `<p>b</p>{% endif %}`. The line before the branch ran into the line
  after it where the branch wasn't written, or the branch's last line
  ran into the line after it where it was.
- The body of a `macro`, or of a `{% set %}` block, written by its call
  or its name alone on an indented line, as `  {{ card() }}`, starts at
  that line's indent, its lines keeping their indent one to another. It
  kept the indent its lines had in the template on top of the line's,
  as a body indented under its tag inside a `<div>`. Written from a line
  at the left, the body is written as typed.
- What a `{% call %}` or a `{% filter %}` writes, where its tag stands
  alone on an indented line outside `<pre>`, `<textarea>`, `<script>`
  and `<style>`, starts at that line's indent, its lines keeping their
  indent one to another. A filter's lines kept the indent they had in
  the template in place of the line's, and a call's started at the
  left. Where the tag is at the left, or shares its line, what it writes
  is written as typed.
- A value written alone on an indented line inside the body of a
  `macro`, a `{% call %}`, a `{% filter %}` or a `{% set %}` block, such
  as `  {{ card() }}` or `  {{ caller() }}`, lines up at that line's
  indent as it does elsewhere, with no blank line added after it. A
  body written by `caller()` gives up the indent its lines have under
  the `{% call %}` tag. A line in such a body that comes to nothing, as
  `  {% if x %}<p>b</p>{% endif %}` where `x` is false, is left out, where
  it left a line of spaces.
- A loop whose tag stands alone on its line and whose end tag closes
  the last line of its body, as `  <li>{{ i }}</li>{% endfor %}` on the
  line after `{% for i in items %}`, writes each pass on lines of its
  own where that body is indented. The passes ran together on one line,
  each with the body's indent. Where the body is at the left, or inside
  `<pre>`, `<textarea>`, `<script>` or `<style>`, or the template writes
  plain text, the passes run together as typed, since a line ending
  between them could change the page.
- A line in the body of a `macro`, a `{% set %}` block, a `{% call %}`
  or a `{% filter %}` whose tags at its end close two branches, as
  `<p>x</p>{% endif %}{% endif %}`, keeps the line after it apart. It
  ran into the line after it where it was written.
- "Ran `after_build`, which added N files and changed M" counts a file
  the command wrote again at its length straight after the build wrote
  it. Where the disk's times move in steps, as by a few thousandths of a
  second on Linux, such a file kept its time and went uncounted.
- `serve` shows a file of the site, such as one under `static/`, saved
  again at its length straight after a build copied it, from the next
  build on. Where the disk's times move in steps, such a file kept its
  time, and serve kept the copy from before the save until the file
  changed again.

## 0.4.0 - 2026-10-17

### New

- `statac new --no-starter` and `statac import --no-starter` make a site
  without the starter's templates, snippets, stylesheet and pages, for an
  author who writes their own: it holds `statac.yaml` and `.gitignore`,
  besides what an import brings. `statac new` asks this last of its
  questions, so answers piped in for the other four still answer them.
  `statac new --update` says once that a site has none of the starter's
  files, in place of naming each, and `--missing` writes
  `data/words.yaml` where it isn't there, with the other files the site
  lacks.
- A `permalink` written on a folder's own `index.md`, the home page's
  included, publishes the page and its list at that address, as it does
  any other page, and the list's later pages and its feed follow it. It
  was set aside, the page staying at the folder's address. A `permalink`
  handed down from a folder above still leaves a folder's own page where
  it is, and the folder's other files stay at the folder's address.
- `statac import` keeps a list at the address the old site gave it,
  where that isn't its folder's, as with a Hugo section's `url` or a
  pattern in `permalinks.section`, or a Zola folder whose name
  Statac spells otherwise: the folder's `index.md` carries that address
  as its `permalink`, and the list's later pages and its feed follow it.
  The list stood at its folder's address, sending readers on from the old
  one, and a list Hugo made with no `_index.md` was left with nothing at
  its old address.
- A table takes an id and classes from a line of braces directly under
  its last row, as `{#speed .comparison}`, written as a heading's are,
  and Statac writes them on its `<table>`. Braces holding anything else,
  or escaped as `\{`, stay a row of the table, as they were. An id given
  twice, on a table or a heading, is told as a heading's always was.
- A site can write tables its own way with `snippets/markdown/table.html`,
  handed `head`, the heading row's cells, and `rows`, each a list of
  cells, every cell with its `body` and its column's `align`, besides
  the table's `id` and `classes`. Without the file, Statac writes a
  table as it does now.

### Fixed

- `statac import` from Hugo looks for each list's later pages at the
  addresses Hugo gave them, and sends readers on from the first,
  `/posts/page/1/`, to the list. A list under `uglyURLs` stays at its
  folder's address, where Hugo has published it since version 0.148,
  and sends readers on from `/posts.html`, where earlier versions did,
  unless the old site's built output has no such file; without the built
  output, the report says that nothing answers those versions' later
  pages, `/posts/page/2.html`. Neither kind of later page was looked for
  or named.
- `statac import` sends readers on from each numbered page of a list or
  a tag that it finds in the old site's built output, such as
  `/posts/page/2.html` or `/tags/rust/page/1/`, to the new list's page of
  the same number. From Hugo, it also looks for a tag's later pages where
  Statac publishes them, and under `uglyURLs` sends readers on from a
  tag's page, such as `/tags/rust.html`, and from `/tags.html`, where
  Hugo before 0.148 listed every tag. Nothing answered these.
- `statac import` from Hugo sends readers on to each tag's page and
  each series' from the address `[permalinks]` gave it, where Statac has
  no pattern to match, as `/tags/rust/posts/`, and from each of its
  later pages it finds in the old site's built output; without the
  built output, the report names one of those later pages, which
  nothing answers. The old address was only listed as one nothing
  answered, and the later pages were never named. Under `uglyURLs`, a
  series' page, such as `/series/rust.html`, sends readers on too, where
  nothing answered it.
- A code block opened with a caption in quotes and no language, as
  `` ```"An example" ``, is no longer warned of as a language Statac has
  no colours for. The block was built rightly; only the warning was
  wrong.
- A broken link a template writes from a value, as `href="{{ address }}"`,
  is told at the line of the template that wrote it wherever the line
  is. It was told at the page alone, or at another line, where the line
  was inside a loop, opened a block or came after the page's writing.
- A broken link a snippet writes from a value is told at the snippet's
  line, whether the writing calls the snippet or a template does. It was
  told where the template shows the page's writing, or at another line
  of the snippet.
- Why Sass stopped is told in Statac's own words, with a suggestion
  where there is a good one: `` Sass stopped: no variable called `$link` ``
  where it was `Sass stopped: undefined variable`. What an `@error` in
  the site's Sass says is quoted exactly, where it lost its capital and
  its full stop. A value CSS has no way to write, as `1px * 1px`, is
  told by the stylesheet alone where Sass gives no place for it. It was
  told at the stylesheet's first line.
- What stops a template is told in Statac's own words, with a
  suggestion where there is a good one: `` Malformed template: `{% if %}`
  is never closed ``, with `` Close it with `{% endif %}`. ``, where it
  was `Malformed template: unexpected end of input, expected end of
  block`, and `` Template stopped: can't use `+` on text and a number ``.
- A template that can't be read is told once, at the template. It was
  told again at each template that extends it and at each call of it.
- Where the template engine gives a line but no place, as for a
  `{% block main required %}` that nothing fills, what stopped it is
  told at its tag on that line. It was told at the page alone.
- `{% extends "base.html" %}` takes the template `base`, as
  `layout: base.html` does. It was told as a template that doesn't
  exist.
- A feed carries code as written, with its language's class, and
  without the markup that colours it, since a feed reader loads no
  stylesheet to colour it with.
- Haxe is coloured after a string in single quotes. The rest of the
  line was left uncoloured, and later lines could lose colours too.
- Builds of one site run one at a time. A build started while another
  runs, such as `statac build` beside `statac serve`, says that it is
  waiting and builds once the other has finished. Two at once could stop
  with `Can't write`, leave no built site, or warn `cache-not-written`.
- A division by zero in a template, with `/`, `//` or `%`, stops the
  template, told at the sum. `/` wrote `inf` or `nan` on the page and
  nothing was said.
- A minus before a field, as `{{ -page.date }}`, makes the field's value
  negative, and stops the template at it where the value isn't a
  number. It was told as an unknown field called `-page.date`.
- A function that takes no names, handed one, as `url(to="/about/")`,
  is told as having no argument called it. It was told as handed a
  value that isn't an address.
- The lines of a block indented under its tag only to read the
  template, as a loop's body under `{% for %}` level with the line
  before it, are written at the indent of the lines round them. They
  were written that much further in. A body the markup indents, one
  step inside the `<ul>` its tag stands level with, is written as typed,
  as is any body where Statac can't tell which was meant. Pages look
  the same; only their source changes.
- A block's body written in place of another template's `{% block %}`
  lines up where it lands: its lines give up the indent they have in
  common and take the indent of the line the `{% block %}` tags share,
  or of the body they replace. It was written as typed, wherever that
  left it. The body `{{ super() }}` writes lines up at that line's
  indent, and a `{% block %}`'s own body gives up indent it has only for
  the template's sake, as a loop's does. Each deeper level of a
  recursive loop goes in as far as its `{{ loop() }}` stands inside its
  `{% for %}` tag, where only the level's first line moved.
- A line that opens with a tag and holds more than tags, as
  `{% if page.summary %}<p>{{ page.summary }}</p>{% endif %}`, keeps its
  indent, and keeps its line end if it also closes with a tag and a
  later line writes something. Both were left out, so the line was
  written at the left and ran into the next. A line whose tags write
  nothing between them is still not written.
- A line that holds more than tags and closes with one, as
  `<p>{{ page.title }}</p>{% if page.summary %}` or a recursive loop's
  `</ul>{% endif %}`, keeps its line end where a later line writes
  something, whether or not it opens with a tag, inside `<pre>` and
  `<script>` as anywhere. It was left out, so the next line ran into
  it. Where the branch after such a line isn't written, as an
  `{% if %}` that is false or a `{% for %}` over nothing, the tag
  closing it on a line of its own ends the line instead, which ran
  into the line after the branch.
- Two `statac serve` sessions of one site at once each build into a
  folder of their own, the second into `.statac/serve-2`. Both built
  into `.statac/serve`, so each could show pages the other wrote, such
  as drafts where only one was showing them.
- `statac serve` says when the system keeps it from answering the
  browser, as when no file handles are left: "Can't answer the browser:
  too many open files." It says so once, and again only after it has
  answered in between. It dropped the connection with nothing said.
- `statac serve` answers each request a program sends on one connection
  before the first is answered. It answered the first alone.
- The bar `statac serve` adds to each page, and its details, keep the
  size the reader's browser gives text, whatever font size the site sets
  on the page's `html`. A site setting `html { font-size: 62.5% }` drew
  them at about 9 pixels.
- `loop` in a loop's own `if`, as in `{% for x in xs if loop.first %}`,
  is told as an unknown name when the template is read, at the `loop`,
  with `` `loop` is set only inside the loop: test it with an `{% if %}`
  there. `` It was accepted, and the build stopped at the first page
  with no value for it, suggesting a `default` that stopped it again.

### Changed

- The template language follows Jinja more closely, so a template may
  write some things differently:
  - `title` starts a word with a capital only after a space, a hyphen or
    an opening bracket: `node.js` becomes `Node.js` where it became
    `Node.Js`, and `at&t` becomes `At&t` where it became `At&T`.
  - `round` takes a value halfway between two to the even one:
    `2.5 | round` is `2.0`, where it was `3.0`.
  - `//` and `%` round down for a negative divisor: `7 // -2` is `-4`
    and `7 % -2` is `-1`, where they were `-3` and `1`.
  - A list or map written on a page as it stands quotes its text in
    single quotes, `['a']`, where it used double.
  - `true` and `false` count as numbers to `is number` and `sum`, and
    the `upper` and `lower` tests pass only text with a letter in it.

## 0.3.0 - 2026-10-12

### New

- `code_style: none` turns the colouring of code off: code is published
  as written with its language's class, no `code.css` is published,
  `css("code.css")` writes nothing, and a language Statac has no colours
  for is not warned of. For a site that colours code in the browser, or
  wants it plain.
- A note, `loop-over-every-page`, where a template loops over a list of
  the whole site's pages on so many pages that the build's work grows
  with the square of the site. `check` shows the loop and, where the loop
  keeps pages by a field, the filter to write in its place, which Statac
  works out once for the site.

### Fixed

- A text in quotes in front matter, settings or data that runs onto a
  line one space deeper than its field is read, as YAML reads it, where
  it was refused as a quote never closed. A later line no deeper than
  the field is told at that line, with the spaces that mend it.
- A large site whose layout chooses pages from `site.pages` by a field,
  as the starter's menu does with `selectattr("menu")`, builds in a
  fraction of the time: 10,000 posts in 2 seconds where 0.2.0 took 17.
- A large site whose layout asks for a list of pages on every page and
  takes a few of them, or counts them, as `site.posts[:5]` and
  `tag.posts | length` do, no longer builds slower with the square of
  its size: with 8,000 posts, 1 second where it took 3 to 10.
- A large site whose layout looks through every page on every page no
  longer takes memory with the square of its size: with 8,000 posts,
  under 1 GB where it took 4 GB, and 39 GB where it sorted them.
- A large site whose layout chooses from the site's posts or pages on
  every page, or puts them in order, with `tagged`, `in`, `where`,
  `selectattr`, `rejectattr`, `sort` or `newest_first`, no longer builds
  slower with the square of its size: with 8,000 posts, 1 second where
  it took 6 to 90.
- `related(page)`, which the starter's posts show three of, no longer
  slows a large site by the square of its size: 32,000 posts build in
  3.7 seconds where they took 6.9.

### Changed

- Coloured code is marked with the kind of each piece, such as
  `hl-keyword` or `hl-comment`, in place of a numbered class, and is
  marked the same whatever `code_style` is. `code.css` gives each kind
  one rule, and a site changes a colour by setting its property, such as
  `--code-comment`, in a stylesheet of its own. Rules a site wrote for
  the numbered classes no longer apply.
- Code is coloured by another library, so some pieces take another
  colour than before, and Statac no longer builds the Oniguruma and
  Zstandard C libraries or carries code under the EUPL.
- A site whose own `snippets/markdown/code.html` never shows `body`
  builds faster: Statac no longer colours code that file can't show.

## 0.2.0 - 2026-10-11

### New

- A new site has a page listing every tag, at `/tags/`, with an "All
  tags" link from each tag's page, and an About page to adopt or copy.
- A page joins the starter's menu with `menu: true` in its front matter.
- HTML in the writing that is left open, or closes what it never opened,
  is warned of at its tag and published as written.
- `site.webmentions` tells a template whether the site takes webmentions.
- An install script for Linux and macOS:
  `curl -fsSL https://get.statac.dev | sh`.

### Fixed

- `statac new --update` and `check` take a file that no longer resembles
  any version of the starter's as the author's own, and say nothing of a
  newer one.
- `check` no longer stops a template that only pages with `needs: url`
  use from reading the site's address.
- `check` no longer lists a file as writing JavaScript where the page's
  scripts all come from another file.
- A site in a right-to-left language Statac has no words for is built
  right to left.
- A tag whose `>` starts the next line of HTML is read without a fault.
- A more marker inside an element the writing's HTML holds open is
  refused.
- The starter's snippets treat `pictured=false` and `nested=false` as
  not given.
- A line of many unclosed `$` or bare addresses no longer slows a build.
- Importing a Hugo site keeps its feeds where `outputs` and
  `outputFormats` put them, leaves a page's own fields beneath `params`
  where they bear one of Hugo's names, and speaks of the later pages
  wherever the site has more posts than a page shows.
- Importing a Zola site answers a list's `/page/1/` from the list's own
  page.

### Changed

- What `statac new --update` shows of a changed file, the explanations
  of problems and the opening lines of each starter file are reworded.

## 0.1.0 - 2026-10-10

The first release.
