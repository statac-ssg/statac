# Changes

What changed in each release of Statac, the newest first.

## Unreleased

### New

- `code_style: none` turns the colouring of code off: code is published
  as written with its language's class, no `code.css` is published,
  `css("code.css")` writes nothing, and a language Statac has no colours
  for is not warned of. For a site that colours code in the browser, or
  wants it plain.

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
