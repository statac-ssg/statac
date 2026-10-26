This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-tags-in-writing/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 3 posts, into `content/posts/`.
- 1 draft, with `draft: true`, so it isn't published by accident.
- 1 page, into `content/` where it stood in the old site.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

Statac put a backslash before the `@` in 1 place it would have read as a snippet call, as `\@name(`, so it shows as written: see https://statac.dev/writing/#snippet-calls.

The settings written in `statac.yaml`:

```yaml
title: Tags in the writing
url: https://example.org
language: en
timezone: UTC
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 4 are published or redirected.

- Every page lands at the URL Statac worked out for it, with nothing added.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Settings with no equal

Statac has no equal for each of these settings, so nothing came across for them: https://statac.dev/settings/ says what Statac offers.

- `config.toml`, line 4: `markdown.render_emoji: true`

### Shortcodes left as written

Statac's Markdown runs none of Zola's shortcodes, and these have no equal in it, so each appears on the page as written: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `content/posts/post-1.md`, line 27: `{% aside(kind='tip') %}`
- `content/posts/post-1.md`, line 27: `{% end %}`
- `content/posts/post-1.md`, line 29: `{{ unknown_1(values=[1, 2]) }}`

### Links to pages that didn't come across

Each of these names a page by its file, and Statac couldn't find that file in the old site, or Zola didn't publish it, so it is left as written: link to the page's URL instead.

- `content/posts/post-1.md`, line 33: `@/posts/post-4.md`
- `content/posts/post-1.md`, line 33: `@/posts/post-9.md`

### Options on code taken off

Statac reads the rest of a code block's first line as its caption, so Zola's options after a comma, such as `,linenos`, were taken off each of these: the code is shown without them.

- `content/posts/post-1.md`, line 39: ```` ```rust,linenos,hl_lines=1 ````

### Shortcodes inside code

Zola ran these even inside code, and Statac shows code exactly as written, so each now shows as written: write by hand what it made, where that was meant.

- `content/posts/post-1.md`, line 40: `{{ youtube(id="in-code") }}`
- `content/posts/post-1.md`, line 43: `{{ youtube(id="in-span") }}`

### Shortcodes now snippet calls

Each of these shortcodes is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/youtube.html`:
  - `content/posts/post-1.md`, line 9: `{{ youtube(id="abcdefghijk") }}`
  - `content/posts/post-1.md`, line 13: `{{ youtube(id="abcdefghijk", autoplay=true) }}`
- `snippets/vimeo.html`:
  - `content/posts/post-1.md`, line 11: `{{ vimeo(id="123456789") }}`
- `snippets/gist.html`:
  - `content/posts/post-1.md`, line 15: `{{ gist(url="https://gist.github.com/user/0123456789abcdef", file="a.rs") }}`
- `snippets/streamable.html`:
  - `content/posts/post-1.md`, line 17: `{{ streamable(id="92ok4") }}`
- `snippets/note.html`:
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

- `templates/`: the old site's templates.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
