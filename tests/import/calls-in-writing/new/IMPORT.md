This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/calls-in-writing/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

The settings give no `url`, so the new site has no `url` yet: add it to `statac.yaml` once it is known.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

Statac put a blank line before 1 `<pre>`, `<script>`, `<style>` or `<textarea>` tag that stood inside other HTML and held a blank line: its Markdown ends a block of HTML at a blank line unless one of those tags opens it, so what followed the blank line would have been read as Markdown.

Statac put a backslash before the `@` in 8 places it would have read as a snippet call, as `\@name(`, so each shows as written: see https://statac.dev/writing/#snippet-calls.

The settings written in `statac.yaml`:

```yaml
title: Calls in the writing
language: en
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for the one old URL there: it is published there.

- 1 page carries a `permalink` of its own, where no pattern fits.

The old site's built output isn't in `_site/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Includes now snippet calls

Each include is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/line_1.html`:
  - `_posts/2026-01-01-post-1.md`, line 26: `{% include line-1.html %}`
  - `_posts/2026-01-01-post-1.md`, line 28: `{% include line-1.html %}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_includes/`: the old site's includes; each one the content calls has a snippet of its name standing for it.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
