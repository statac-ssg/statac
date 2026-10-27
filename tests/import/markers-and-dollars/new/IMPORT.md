This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/markers-and-dollars/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 4 posts, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

The settings give no `url`, so the new site has no `url` yet: add it to `statac.yaml` once it is known.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

Statac gave 1 `<!--more-->` marker a blank line each side, where it stood on a line of its own inside HTML: Statac ends a summary only at one between two blocks of Markdown. The summary ends where it did, and the lines after the marker, up to the next blank line, are read as Markdown now, where they were part of the HTML.

Statac put a backslash before 2 dollar signs it would have read as opening maths, as `\$`, which the old site didn't set as maths, so each shows as written: see https://statac.dev/writing/#maths.

The settings written in `statac.yaml`:

```yaml
title: Markers and dollars
language: en
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 4 are published or redirected.

- `content/posts/index.md` cascades `pages.permalink: /{year}/{month}/{day}/{slug}.html` to every page in `content/posts/`, which puts 4 of them at their old URLs.

The old site's built output isn't in `_site/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Summary markers taken out

Statac ends a summary only at `<!--more-->` on a line of its own between two blocks of Markdown, and one anywhere else stops the build. Each of these stood inside a line, a list, a quotation or an HTML element, so it was taken out, and the page's summary is its first paragraph: put `<!--more-->` on a line of its own where the summary should end.

- `_posts/2026-01-02-post-2.md`: `<!--more-->`
- `_posts/2026-01-03-post-3.md`: `<!--more-->`
- `_posts/2026-01-03-post-3.md`: `<!--more-->`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
