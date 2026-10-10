This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/baseurl/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.
- 1 other file, copied as it was: files published as they are into `static/` at the same path, so their addresses stay, and data files into `data/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

The starter's `content/404.md` was left out: the old site's `404.md` stands at its address.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

The settings written in `statac.yaml`:

```yaml
title: Under a path
url: https://example.org/blog
language: en
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old addresses there: 4 are published there, or sent on from there.

- `content/posts/index.md` hands `pages.permalink: /{year}/{month}/{day}/{slug}.html` to every page in `content/posts/`, which puts 2 of them at their old addresses.
- 2 pages carry a `permalink` of their own, where no pattern fits.
- The site is under a path, `/blog`, as its `url` says, so every address starts with it, and links in the writing were written with it: see https://statac.dev/addresses/.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
