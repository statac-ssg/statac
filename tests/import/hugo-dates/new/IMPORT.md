This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-dates/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 4 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each is now an Atom feed at the same URL, written by `templates/feed.xml`, as `feed: index.xml` on each list page says, and `feed_ids_before` keeps each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

Hugo made a list of each folder at the top of the content folder, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

Hugo took when 5 pages last changed from the file or from Git, which Statac doesn't read, so no `updated` was written from there.

The settings written in `statac.yaml`:

```yaml
title: Dates from names
url: https://example.org
language: en
timezone: Europe/London
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
tags.pages: true
series.pages: true
feed_ids_before: 2026-10-06
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 8 old URLs there: 8 are published or redirected.

- 1 page carries a `permalink` of its own, where no pattern fits.

## Left for you

### Dates Hugo took from the file or from Git

Hugo's settings took the date of each of these pages from the time its file was saved or from Git, which Statac doesn't read, so none was written: write its `date`.

- `content/posts/post-5.md`: `:git`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
