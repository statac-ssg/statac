This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/front-matter/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 1 draft, with `draft: true`, so it isn't published by accident.
- 2 pages, into `content/` where they stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

The settings' `defaults` were written into each page they reached, as fields of its own.

`layout` was taken off 3 pages: each is built with the starter's template for its kind, as https://statac.dev/templates/ describes.

`toc: true` is now `contents: true` on 1 page, which the starter's post template reads to show a table of contents.

2 names in the old site's `categories` are now tags, each with `category_` before it, as `category_category-1`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Front matter
language: en
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
author: Name
old_tags:
  - one
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 6 old URLs there: 6 are published or redirected.

- 4 pages carry a `permalink` of their own, where no pattern fits.
- 2 pages carry `redirect_from`, for the other URLs that led to them.

## Left for you

### Settings with no equal

Statac has no equal for each of these settings or plugins, so nothing came across for them: https://statac.dev/settings/ says what Statac offers.

- `_config.yml`, line 4: `markdown: kramdown`

### Values Jekyll read as numbers

Jekyll read each of these as a number other than its figures say, as an older reading of YAML does, and Statac kept it as the text it is: write the number you mean.

- `_posts/2026-01-01-post-1.md`, line 24: `octal: 010`

### Fields renamed

Each of these fields has the name of one of Statac's built-in fields, which means something else there or holds another kind of value, so Statac put `old_` before its name: show it from a template under that name, or take it out.

- `_config.yml`, line 3: `tags: [one]`
- `_posts/2026-01-01-post-1.md`, line 8: `order: 3`
- `_posts/2026-01-01-post-1.md`, line 10: `slug: post-1`
- `_posts/2026-01-02-post-2.md`, line 5: `toc: 2`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
