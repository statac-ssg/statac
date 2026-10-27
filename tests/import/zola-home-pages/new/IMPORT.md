This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-home-pages/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 3 posts, into `content/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

The old site's home page showed its posts 2 to a page. `content/index.md` gives no `posts_per_page`, so its own list stays on one page, and nothing is published at the old home page's paginated pages now: they are listed under URLs.

The settings written in `statac.yaml`:

```yaml
title: Home pages
url: https://example.org
language: en
timezone: UTC
typography: false
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 6 old URLs there: 5 are published or redirected.

- 1 page carries `redirect_from`, for the other URLs that led to it.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

Nothing in the new site is published at these old URLs. Give the page each should lead to a `redirect_from` naming it, or a `permalink`:

- `/page/2/`

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
