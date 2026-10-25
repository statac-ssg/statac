This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-lists/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 15 posts, into `content/by-date/` and `content/by-none/` and `content/by-slug/` and `content/by-title/` and `content/by-update/` and `content/by-weight/` and `content/hidden/` and `content/outer/inner/`.
- 10 pages, into `content/` where they stood in the old site.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola listed the pages of `content/by-date/` oldest first, as its `paginate_reversed` asked, so `content/by-date/index.md` says `order: oldest first`.

Zola ordered the pages of `content/by-slug/` by their slugs, A to Z, so `content/by-slug/index.md` lists them in that order with `order:`.

Zola ordered the pages of `content/by-title/` by their titles, A to Z, so `content/by-title/index.md` lists them in that order with `order:`.

Zola ordered the pages of `content/by-update/` by their `updated` dates, newest first, so `content/by-update/index.md` lists them in that order with `order:`.

Zola ordered the pages of `content/by-weight/` by their `weight`, lightest first, so `content/by-weight/index.md` lists them in that order with `order:`, and `weight` was taken off them.

Zola gave the pages of `content/by-none/` no order of its own, as no `sort_by` asked for one, and Statac lists them newest first.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`: each list page that gave it says `posts_per_page`, and the first of its paginated pages, which Zola redirected to the list, is in the list page's `redirect_from`.

The old site's home page showed its posts 2 to a page. `content/index.md` gives no `posts_per_page`, so its own list stays on one page.

`in_search_index` was taken off 1 page, `content/by-none/_index.md`: it said whether Zola's search index held the page, and Statac makes no search index.

`insert_anchor_links` was taken off 1 page, `content/by-update/_index.md`: it put a link beside each heading, which a snippet of yours in `snippets/markdown/heading.html` can do, as https://statac.dev/snippets/#pieces-of-the-writing describes.

`template` was taken off 1 page, `content/by-weight/_index.md`: it named the old site's template for the page.

`page_template` was taken off 1 page, `content/by-weight/_index.md`: it named the old site's template for the list's pages.

`render` was taken off 1 page, `content/hidden/_index.md`: with `render = false` Zola published the list's pages but no page for the list itself; `index_page: false` on the list page does that in Statac.

`transparent` was taken off 1 page, `content/outer/inner/_index.md`: Zola listed the folder's pages among those of the folder above it, as Statac's list of a folder holds every post beneath it already.

The settings written in `statac.yaml`:

```yaml
title: Lists
url: https://example.org
language: en
timezone: UTC
pagination.permalink: "{list}p/{n}/"
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 28 old URLs there: 28 are published or redirected.

- 2 pages carry `redirect_from`, for the other URLs that led to them.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
