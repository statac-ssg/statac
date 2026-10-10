This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/permalink-none/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

The old site's home page showed its posts 1 to a page. Statac's home page shows the newest of each folder itself, so `content/index.md` keeps its own list to one page with `posts_per_page: 0`, and the posts' own page shows them 1 to a page, as `posts_per_page` in `statac.yaml` says for every list: see https://statac.dev/lists/#pages-of-a-list.

1 name in the old site's `categories` is now a tag, with `category_` before it, as `category_one`, so it can still be told from the site's own tags. To rename it across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Permalink none
language: en
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
posts_per_page: 1
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old addresses there: 4 are published there, or sent on from there.

- 2 pages carry a `permalink` of their own, where no pattern fits.
- 1 page at the top of `content/`, `content/page-2.md`, sends readers on from an old later page of the home page to the page of `content/posts/` that holds the same posts, with `permalink` and `redirect_to`.

## Left for you

### Pages made by templates

Each of these pages was mostly the old site's templates at work, such as a home page or an archive listing the posts, so Statac didn't bring it across: Statac makes the home page and a page for each folder of posts and each tag itself, as https://statac.dev/lists/ describes.

- `index.html`, line 4: `{% for post in paginator.posts %}`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
