This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-first-pages/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 2 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`: each list page that gave it says `posts_per_page`, and the first of its paginated pages, which Zola redirected to the list, is in the list page's `redirect_from`.

The old site's home page showed its posts 1 to a page. `content/index.md` gives no `posts_per_page`, so its own list stays on one page.

The settings written in `statac.yaml`:

```yaml
title: First pages
url: https://example.org
language: en
timezone: UTC
typography: false
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 7 old URLs there: 7 are published or redirected.

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
