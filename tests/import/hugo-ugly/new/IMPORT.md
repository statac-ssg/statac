This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-ugly/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/. Its pages and feeds need `templates/feed.xml`.

## What came across

- 3 posts, into `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/feed.xml` once you add it, and `feed_ids_before` keeps each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

Hugo made a list of each folder at the top of the content folder, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

The settings written in `statac.yaml`:

```yaml
title: Ugly addresses
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
feed_ids_before: 2026-10-06
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 11 old URLs there: 11 are published or redirected.

- `content/posts/index.md` cascades `pages.permalink: /posts/{slug}.html` to every page in `content/posts/`, which puts 3 of them at their old URLs.
- 1 page carries a `permalink` of its own, where no pattern fits.
- 3 pages carry `redirect_from`, for the other URLs that led to them.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
