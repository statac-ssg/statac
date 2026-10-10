This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-permalinks/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 5 posts, into `content/blog/` and `content/essays/` and `content/notes/2026/` and `content/posts/`.
- 6 pages, into `content/` where they stood in the old site.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/notes/index.md`, `content/blog/index.md`, `content/essays/index.md`, `content/posts/index.md` each hand `pages.layout: post`, so the pages beneath them, every one dated, are built with the starter's template for posts.

Hugo made a list of each folder at the top of the writing, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/blog/index.md` for `content/blog/`, `content/essays/index.md` for `content/essays/`, `content/pages/index.md` for `content/pages/`, `content/posts/index.md` for `content/posts/`.

The settings written in `statac.yaml`:

```yaml
title: Permalinks
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
feed_ids_before: 2026-10-06
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 21 old addresses there: 21 are published there, or sent on from there.

- 4 pages carry a `permalink` of their own, where no pattern fits.
- 2 pages carry `redirect_from`, for the other addresses that led to them.

Each address was also looked for in the old site's own built output, `public/`.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Statac left these behind, and the new site has nothing of them:

- `public/`: the old site's built output, read only to check the addresses.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
