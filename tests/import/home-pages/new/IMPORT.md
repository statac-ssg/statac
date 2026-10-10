This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/home-pages/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 5 posts, into `content/posts/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The old site's home page showed its posts 2 to a page. Statac's home page shows the newest of each folder itself, so `content/index.md` keeps its own list to one page with `posts_per_page: 0`, and the posts' own page shows them 2 to a page, as `posts_per_page` in `statac.yaml` says for every list: see https://statac.dev/lists/#pages-of-a-list.

The settings written in `statac.yaml`:

```yaml
title: Home pages
url: https://example.org
language: en
timezone: Europe/London
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
posts_per_page: 2
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 8 old addresses there: 8 are published there, or sent on from there.

- `content/posts/index.md` hands `pages.permalink: /{year}/{month}/{day}/{slug}.html` to every page in `content/posts/`, which puts 5 of them at their old addresses.
- 1 page at the top of `content/`, `content/page3.md`, sends readers on from an old later page of the home page to the page of `content/posts/` that holds the same posts, with `permalink` and `redirect_to`.
- 1 page at the top of `content/`, `content/page2.md`, sends readers on from `/page2/`, an old later page of the home page, to the page of `content/posts/` with the same number, which doesn't hold exactly the same posts, as happens where posts share a date.

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
