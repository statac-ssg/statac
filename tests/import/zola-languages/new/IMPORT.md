This site was imported from a Zola site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-languages/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 4 posts, into `content/fr/posts/` and `content/posts/`.
- 4 pages, into `content/` where they stood in the old site.
- 2 other files, copied as they were: files published as they are into `static/` at the same path, so their addresses stay, and data files into `data/`.

The old site's home page is `content/index.md`, built with `templates/home.html`, which lists the posts below its writing.

The settings declare 2 languages, so the new site is in each, the main one, `en`, first: each other language's pages are in its own folder in `content/`, where they stand at the same place as the page they translate, as https://statac.dev/dates-and-languages/#a-site-in-several-languages describes.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page, unless the list's own page gives `paginate_by`, so `posts_per_page: 0` in `statac.yaml` does the same.

Zola's feeds, at `/atom.xml` and `/fr/atom.xml`, are each now Statac's Atom feed at the same address, written by `templates/feed.xml`, as `feed:` on each list's page says, and `feed_ids_before` keeps each post's id the address Zola gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids.

`content/fr/posts/index.md` and `content/posts/index.md` each hand `pages.layout: post`, so the pages beneath them, every one dated, are built with the starter's template for posts.

The settings written in `statac.yaml`:

```yaml
title: Languages
url: https://example.org
language: [en, fr]
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
fr.title: Langues
posts_per_page: 0
feed_ids_before: 2026-10-06
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 12 old addresses there: 12 are published there, or sent on from there.

- 1 page carries a `permalink` of its own, where no pattern fits.

The old site published a page listing every one of its `tags` at `/fr/tags/`, and the starter has none.

The old site published a page listing every one of its `tags` at `/tags/`, and the starter has none.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
