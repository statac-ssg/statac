This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-config-toml/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/. Adding `templates/rss.xml` builds its feeds.

## What came across

- 1 post, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

Zola's feed, at `/rss.xml`, stays at the same URL, as `feed:` on each list page says, built with `templates/rss.xml` once you add it, and `feed_ids_before` and `feed_ids_from: zola` keep each post's id the URL Zola gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. Copy the docs' RSS template as `templates/rss.xml`: see https://statac.dev/feeds/#the-template.

The settings written in `statac.yaml`:

```yaml
title: Config
url: https://example.org
language: en
timezone: UTC
typography: false
feed_ids_before: 2026-10-06
feed_ids_from: zola
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 2 old URLs there: 2 are published or redirected.

- Every page lands at the URL Statac worked out for it, with nothing added.

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
