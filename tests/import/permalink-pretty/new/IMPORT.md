This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/permalink-pretty/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 3 posts, into `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

The old site's home page showed its posts 1 to a page. `content/index.md` keeps its own list to one page with `posts_per_page: 0`, and the posts' list page shows them 1 to a page, as `posts_per_page` in `statac.yaml` says for every list: see https://statac.dev/lists/#pages-of-a-list.

The settings written in `statac.yaml`:

```yaml
title: Permalink pretty
language: en
date_format: 31 December 2025
posts_per_page: 1
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 8 old URLs there: 8 are published or redirected.

- `content/posts/index.md` cascades `pages.permalink: /{year}/{month}/{day}/{slug}/` to every page in `content/posts/`, which puts 3 of them at their old URLs.
- 1 page carries a `permalink` of its own, where no pattern fits.
- 2 pages at the top of `content/`, such as `content/page2.md`, redirect from the old paginated pages of the home page, each to the page of `content/posts/` that holds the same posts, with `permalink` and `redirect_to`.

Each URL was also looked for in the old site's own built output, `_site/`.

## Left for you

### Pages made by templates

Each of these pages was mostly the old site's templates at work, such as a home page or an archive listing the posts, so Statac didn't bring it across: Statac makes the home page and a page for each folder of posts and each tag itself, as https://statac.dev/lists/ describes.

- `index.html`, line 4: `{% for post in paginator.posts %}`

### URLs the old built site doesn't have

Statac worked these URLs out by the old tool's rules, but the old site's built output has nothing there, so the rule was wrong for the page or the output is older than the content: find where the old site published the page, and add that URL to the page's `redirect_from`.

- `_posts/2026-01-03-post-3.md`: `/2026/01/03/post-3/`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_site/`: the old site's built output, read only to check the URLs.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
