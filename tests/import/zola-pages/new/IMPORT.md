This site was imported from a Zola site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-pages/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 7 posts, into `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.
- 1 other file, copied as it was: files published as they are into `static/` at the same path, so their addresses stay, and data files into `data/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page, unless the list's own page gives `paginate_by`, so `posts_per_page: 0` in `statac.yaml` does the same.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

The settings written in `statac.yaml`:

```yaml
title: Pages
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
posts_per_page: 0
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 12 old addresses there: 12 are published there, or sent on from there.

- 2 pages carry a `permalink` of their own, where no pattern fits.
- 2 pages carry `redirect_from`, for the other addresses that led to them.

1 photograph beside a page is shown as copies Statac makes, and the file itself isn't published, so its old address, `/posts/post-1/photo-1.jpg`, is no longer answered. A photograph's own file is published where a page links to it, or where a file beside it, named after it with `.yaml` on the end, holds `optimise: false`; `optimise_images: false` in `statac.yaml` leaves every photograph as it is: see https://statac.dev/pictures/.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Statac left these behind, and the new site has nothing of them:

- `content/posts/data.xlsx`: a file the settings' `ignored_content` names, which Zola doesn't read.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
