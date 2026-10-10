This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-config-toml/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Hugo made a list of each folder at the top of the writing, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

The settings written in `statac.yaml`:

```yaml
title: Settings in config.toml
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

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old addresses there: 4 are published there, or sent on from there.

- Every page lands where the old site published it, with nothing added.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Statac left these behind, and the new site has nothing of them:

- `config.yaml`: another settings file, which Hugo reads only where the one before it isn't there.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
