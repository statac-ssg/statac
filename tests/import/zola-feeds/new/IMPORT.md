This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-feeds/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/. Its pages and feeds need `templates/feed.xml`.

## What came across

- 2 posts, into `content/notes/` and `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

Zola's feeds, at `/posts/atom.xml`, `/posts/rss.xml`, `/atom.xml` and `/rss.xml`, each stay at the same URL, as `feed:` on each list page says, built with `templates/feed.xml` once you add it, and `feed_ids_before` and `feed_ids_from: zola` keep each post's id the URL Zola gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids.

The settings written in `statac.yaml`:

```yaml
title: Feeds
url: https://example.org
language: en
timezone: UTC
feed_ids_before: 2026-10-06
feed_ids_from: zola
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 8 old URLs there: 8 are published or redirected.

- Every page lands where the old site published it, with nothing added.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
