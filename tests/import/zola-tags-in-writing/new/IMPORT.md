This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-tags-in-writing/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 3 posts, into `content/posts/`.
- 1 draft, with `draft: true`, so it isn't published by accident.
- 1 page, into `content/` where it stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page, unless its list page gives `paginate_by`, so `posts_per_page: 0` in `statac.yaml` does the same.

`content/posts/index.md` cascades `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Statac put a backslash before the `@` in 1 place it would have read as a snippet call, as `\@name(`, so it shows as written: see https://statac.dev/writing/#snippet-calls.

The settings written in `statac.yaml`:

```yaml
title: Tags in the writing
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
posts_per_page: 0
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 4 are published or redirected.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Tags in the content left as written

Statac's Markdown runs none of Zola's tags, and these have no equal in it, so each is left as written and shows on the page as it stands: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `content/posts/post-1.md`, line 13: `{{ youtube(id="abcdefghijk", autoplay=true) }}`
- `content/posts/post-1.md`, line 27: `{% aside(kind='tip') %}`
- `content/posts/post-1.md`, line 27: `{% end %}`
- `content/posts/post-1.md`, line 29: `{{ unknown_1(values=[1, 2]) }}`

### Links to pages that didn't come across

Each of these names a page by its file, and Statac couldn't find that file in the old site, or Zola didn't publish it, so it is left as written: link to the page's URL instead.

- `content/posts/post-1.md`, line 33: `@/posts/post-4.md`
- `content/posts/post-1.md`, line 33: `@/posts/post-9.md`

### Emoji written by name

Zola showed each of these as its emoji, as the settings' `render_emoji` asked, and Statac shows the name as written: read each page that has one, and write the emoji itself in its place.

- `content/posts/post-1.md`, line 37: `:smile:`

### Options on code taken off

Statac reads the rest of a code block's first line as its caption, so Zola's options after a comma, such as `,linenos`, were taken off each of these: the code is shown without them.

- `content/posts/post-1.md`, line 41: ```` ```rust,linenos,hl_lines=1 ````

### Tags inside code

Zola ran these even inside code, and Statac shows code exactly as written, so each now shows as it stands: write by hand what it made, where that was meant.

- `content/posts/post-1.md`, line 42: `{{ youtube(id="in-code") }}`
- `content/posts/post-1.md`, line 45: `{{ youtube(id="in-span") }}`

### Tags now snippet calls

Each of these tags is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/gist.html`:
  - `content/posts/post-1.md`, line 15: `{{ gist(url="https://gist.github.com/user/0123456789abcdef", file="a.rs") }}`
- `snippets/streamable.html`:
  - `content/posts/post-1.md`, line 17: `{{ streamable(id="92ok4") }}`
- `snippets/old_note.html`:
  - `content/posts/post-1.md`, line 19: `{{ note(text="A note of the site's own.") }}`
- `snippets/line_1.html`:
  - `content/posts/post-1.md`, line 21: `{{ line_1(n=3, on=false) }}`
- `snippets/aside.html`:
  - `content/posts/post-1.md`, line 23: `{% aside() %}`

### Summary markers taken out

Statac ends a summary only at `<!--more-->` on a line of its own between two blocks of Markdown, and one anywhere else stops the build. Each of these stood inside a line, a list, a quotation or an HTML element, so it was taken out, and the page's summary is its first paragraph: put `<!--more-->` on a line of its own where the summary should end.

- `content/posts/post-1.md`: `<!--more-->`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `templates/`: the old site's templates, which the starter's templates take the place of.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
