This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-bundles/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.
- 2 other files, copied as they were: files published as they are into `static/` at the same path, so their addresses stay, and data files into `data/`.

The old site's home page is `content/index.md`, built with `templates/home.html`, which lists the posts below its writing.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Hugo summed up 1 page with its opening paragraphs, as many as made its `summaryLength` of words, and Statac takes the first paragraph alone: where a summary should run further, put `<!--more-->` on a line of its own where it should end.

1 figure is now a call to the starter's `figure` snippet.

The settings written in `statac.yaml`:

```yaml
title: Bundles
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

Statac worked out where the old site published each page, read the new site, and looked for each of the 6 old addresses there: 6 are published there, or sent on from there.

- 1 page carries a `permalink` of its own, where no pattern fits.

1 photograph beside a page is shown as copies Statac makes, and the file itself isn't published, so its old address, `/posts/post-1/photo-1.jpg`, is no longer answered. A photograph's own file is published where a page links to it, or where a file beside it, named after it with `.yaml` on the end, holds `optimise: false`; `optimise_images: false` in `statac.yaml` leaves every photograph as it is: see https://statac.dev/pictures/.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Statac left these behind, and the new site has nothing of them:

- `content/posts/post-2/notes.md`: writing kept among a page's files, which Hugo doesn't publish as a page.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
