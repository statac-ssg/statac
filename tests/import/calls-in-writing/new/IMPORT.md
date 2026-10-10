This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/calls-in-writing/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

Statac put a blank line before 1 `<pre>`, `<script>`, `<style>` or `<textarea>` tag that stood inside other HTML and held a blank line: its Markdown ends a block of HTML at a blank line unless one of those tags opens it, so what followed the blank line would have been read as Markdown.

Statac put a backslash before the `@` in 8 places it would have read as a snippet call, as `\@name(`, so each shows as written: see https://statac.dev/writing/#snippet-calls.

The settings written in `statac.yaml`:

```yaml
title: Calls in the writing
language: en
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for the one old address there: it is published there.

- 1 page carries a `permalink` of its own, where no pattern fits.

## Left for you

### Includes now snippet calls

Each include is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/line_1.html`:
  - `_posts/2026-01-01-post-1.md`, line 26: `{% include line-1.html %}`
  - `_posts/2026-01-01-post-1.md`, line 28: `{% include line-1.html %}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_includes/`: the old site's includes; each one the writing calls has a snippet of its name standing for it.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
