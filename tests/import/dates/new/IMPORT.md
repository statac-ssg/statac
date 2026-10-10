This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/dates/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 7 posts, into `content/posts/`.
- 2 drafts, each with `draft: true`, so none is published by accident.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

1 date had a time and no offset, which Jekyll read as UTC whatever the site's time zone: it is written with `Z`, so it means the same moment.

The settings written in `statac.yaml`:

```yaml
title: Dates
language: en
timezone: Europe/London
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 7 old addresses there: 7 are published there, or sent on from there.

- `content/posts/index.md` hands `pages.permalink: /{year}/{month}/{day}/{slug}.html` to every page in `content/posts/`, which puts 4 of them at their old addresses.
- 3 pages carry a `permalink` of their own, where no pattern fits.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
