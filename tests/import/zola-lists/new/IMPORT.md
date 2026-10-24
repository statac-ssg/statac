This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-lists/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 15 posts, into `content/by-date/` and `content/by-none/` and `content/by-slug/` and `content/by-title/` and `content/by-update/` and `content/by-weight/` and `content/hidden/` and `content/outer/inner/`.
- 10 pages, into `content/` where they stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola listed the pages of `content/by-date/` oldest first, as its `paginate_reversed` asked, so `content/by-date/index.md` says `order: oldest first`.

Zola ordered the pages of `content/by-slug/` by their slugs, A to Z, so `content/by-slug/index.md` lists them in that order with `order:`.

Zola ordered the pages of `content/by-title/` by their titles, A to Z, so `content/by-title/index.md` lists them in that order with `order:`.

Zola ordered the pages of `content/by-update/` by their `updated` dates, newest first, so `content/by-update/index.md` lists them in that order with `order:`.

Zola ordered the pages of `content/by-weight/` by their `weight`, lightest first, so `content/by-weight/index.md` lists them in that order with `order:`, and `weight` was taken off them.

Zola gave the pages of `content/by-none/` no order of its own, as no `sort_by` asked for one, and Statac lists them newest first.

Zola shows all of a list's pages on one page, unless its list page gives `paginate_by`, so `posts_per_page: 0` in `statac.yaml` does the same, and each list page that gave it says `posts_per_page`; the first of its paginated pages, which Zola redirected to the list, is in the list page's `redirect_from`.

The old site's home page showed its posts 2 to a page. Statac's home page shows the newest of each folder itself, so `content/index.md` keeps its own list to one page with `posts_per_page: 0`.

`content/by-date/index.md`, `content/by-none/index.md`, `content/by-slug/index.md`, `content/by-title/index.md`, `content/by-update/index.md`, `content/by-weight/index.md`, `content/hidden/index.md` and `content/outer/index.md` each cascade `pages.layout: post`, so the pages beneath them, every one dated, are built with the starter's template for posts.

`in_search_index` was taken off 1 page, `content/by-none/_index.md`: it said whether Zola's search index held the page, and Statac makes no search index.

`insert_anchor_links` was taken off 1 page, `content/by-update/_index.md`: it put a link beside each heading, which a snippet of yours in `snippets/markdown/heading.html` can do, as https://statac.dev/snippets/#pieces-of-the-writing describes.

`template` was taken off 1 page, `content/by-weight/_index.md`: it named the old site's template for the page, which the starter's templates take the place of.

`page_template` was taken off 1 page, `content/by-weight/_index.md`: it named the old site's template for the list's pages, which the starter's templates take the place of.

`render` was taken off 1 page, `content/hidden/_index.md`: with `render = false` Zola published the list's pages but no page for the list itself; `index_page: false` on the list page does that in Statac.

`transparent` was taken off 1 page, `content/outer/inner/_index.md`: Zola listed the folder's pages among those of the folder above it, as Statac's list of a folder holds every post beneath it already.

The starter's `content/posts/index.md` was left out: nothing of the old site lands in `content/posts/`, and it would have been an empty list.

The settings written in `statac.yaml`:

```yaml
title: Lists
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
posts_per_page: 0
pagination.permalink: "{list}p/{n}/"
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 28 old URLs there: 28 are published or redirected.

- 2 pages carry `redirect_from`, for the other URLs that led to them.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
