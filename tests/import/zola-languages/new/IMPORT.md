This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-languages/old` by `statac import`, which changed nothing there. It has no templates, and doesn't build until you add `templates/folder.html`: see https://statac.dev/templates/. Adding `templates/feed.xml` and `templates/folder.html` builds its pages and feeds.

## What came across

- 4 posts, into `content/fr/posts/` and `content/posts/`.
- 4 pages, into `content/` where they stood in the old site.
- 2 other files, copied unchanged: the files to publish into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page is `content/index.md`.

The settings declare 2 languages, so the new site is in each, the main one, `en`, first: each other language's pages are in its own folder in `content/`, where they stand at the same place as the page they translate, as https://statac.dev/languages/#a-site-in-several-languages describes.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

Zola's feeds, at `/atom.xml` and `/fr/atom.xml`, each stay at the same URL, as `feed:` on each list page says, built with `templates/feed.xml` once you add it, and `feed_ids_before` and `feed_ids_from: zola` keep each post's id the URL Zola gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids.

The settings written in `statac.yaml`:

```yaml
title: Languages
url: https://example.org
language: [en, fr]
timezone: UTC
fr.title: Langues
feed_ids_before: 2026-10-06
feed_ids_from: zola
tags.pages: true
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 12 old URLs there: 12 are published or redirected.

- 1 page carries a `permalink` of its own, where no pattern fits.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

The old site published a page listing every one of its `tags` at `/fr/tags/`, and the new site has none.

The old site published a page listing every one of its `tags` at `/tags/`, and the new site has none.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
