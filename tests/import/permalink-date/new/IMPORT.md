This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/permalink-date/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 4 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

3 names in the old site's `categories` are now tags, each with `category_` before it, as `category_one`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Permalink date
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 5 old URLs there: 5 are published or redirected.

- 5 pages carry a `permalink` of their own, where no pattern fits.

Each URL was also looked for in the old site's own built output, `_site/`.

## Left for you

### Pages written in HTML

Each of these was written in HTML and came across as a Markdown file holding that HTML, which Statac publishes as it is: read each, since Markdown reads a line indented by four spaces after a blank line as code.

- `_posts/2026-01-04-post-4.html`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_site/`: the old site's built output, read only to check the URLs.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
