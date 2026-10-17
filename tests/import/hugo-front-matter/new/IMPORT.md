This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-front-matter/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 2 drafts, each with `draft: true`, so none is published by accident.
- 1 page, into `content/` where it stood in the old site.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

Hugo ordered the pages of `content/posts/` by their `weight`, so `content/posts/index.md` lists them in that order with `order:`, and `weight` was taken off them.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Hugo made a list of each folder at the top of the writing, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

`layout` and `type` were taken off 1 page: each is built with the starter's template for its kind, as https://statac.dev/templates/ describes.

1 name in the old site's `categories` is now a tag, with `category_` before it, as `category_Category 1`, so it can still be told from the site's own tags. To rename it across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Front matter
url: https://example.org
language: en-GB
timezone: Europe/London
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
feed_ids_before: 2026-10-06
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 10 old addresses there: 10 are published there, or sent on from there.

- 2 pages carry a `permalink` of their own, where no pattern fits.
- 1 page carries `redirect_from`, for the other addresses that led to it.
- 1 page at the top of `content/`, `content/categories-category-1.md`, sends readers on from the old page of a tag, a category or a series to the page Statac makes for it, with `permalink` and `redirect_to`.

The old site published a page listing every one of its `categories` at `/categories/`, and the starter has none.

## Left for you

### Pages published on a day of their own

Hugo published each of these pages from its `publishDate`, a day other than its `date`, and Statac publishes a page on its `date`, so the line was taken off: where the page should wait, write that day as its `date`.

- `content/posts/post-2.md`, line 8: `publishDate: 2026-01-05`

### Pages published until a day

Hugo stopped publishing each of these pages after its `expiryDate`, which Statac has no equal for, so the line was taken off: those already past that day came across with `draft: true`, and the others stay published until you give them the same.

- `content/posts/post-3.md`, line 5: `expiryDate: 2026-02-01`

### Feeds made by templates

Readers subscribed to the old site's feed at each of these addresses, and the template that made it didn't come across, so nothing is there now. A list's `feed:` line can put that list's feed at an address of its own, as `feed: /index.rss`, and the starter's feed template writes Atom: see https://statac.dev/feeds/.

- Hugo's feed for each tag and each list of that kind, 2 of them, such as `/tags/tag-1/index.xml`: Statac gives such a list a feed only where a page of yours at its address asks for one with `feed:`.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
