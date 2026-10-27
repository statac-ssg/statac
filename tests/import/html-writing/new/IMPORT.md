This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/html-writing/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 2 posts, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

The settings give no `url`, so the new site has no `url` yet: add it to `statac.yaml` once it is known.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

Statac put a blank line before 6 `<pre>`, `<script>`, `<style>` or `<textarea>` tags that stood inside other HTML and held a blank line: its Markdown ends a block of HTML at a blank line unless one of those tags opens it, so what followed the blank line would have been read as Markdown.

Statac put a blank line before 2 code blocks that stood inside HTML, so each is read as code and not as part of the HTML.

The settings written in `statac.yaml`:

```yaml
title: HTML writing
language: en
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 2 old URLs there: 2 are published or redirected.

- `content/posts/index.md` cascades `pages.permalink: /{year}/{month}/{day}/{slug}.html` to every page in `content/posts/`, which puts 2 of them at their old URLs.

The old site's built output isn't in `_site/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Pages written in HTML

Each of these was written in HTML and came across as a Markdown file holding that HTML, which Statac publishes unchanged: read each, since Markdown reads a line indented by four spaces after a blank line as code.

- `_posts/2026-01-01-post-1.html`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
