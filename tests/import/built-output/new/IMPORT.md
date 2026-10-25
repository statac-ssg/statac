This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/built-output/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

2 names in the old site's `categories` are now tags, each with `category_` before it, as `category_linux`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Built output
url: https://example.org
language: en
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 6 old URLs there: 5 are published or redirected.

- 1 page carries a `permalink` of its own, where no pattern fits.
- 3 pages at the top of `content/`, such as `content/categories-linux.md`, redirect from the old page of a tag, a category or a series to the page Statac makes for it, each with `permalink` and `redirect_to`.

Each URL was also looked for in the old site's own built output, `_site/`.

Nothing in the new site is published at these old URLs. Give the page each should lead to a `redirect_from` naming it, or a `permalink`:

- `/archive.html`, for `archive.html`

The old site published a page listing every one of its `categories` at `/categories/`, and the starter has none.

The old site's built output also holds 6 pages at URLs Statac didn't work out, and nothing in the new site is published there. Give the page each should lead to a `redirect_from` naming it. By folder:

- `/`: 1 page, `/page-9.html`
- `/tag/`: 3 pages, `/tag/b.html`, `/tag/c.html` and `/tag/d.html`
- `/x/`: 2 pages, `/x/y/` and `/x/z.html`

## Left for you

### Pages made by templates

Each of these pages was mostly the old site's templates at work, such as a home page or an archive listing the posts, so Statac didn't bring it across: Statac makes the home page and a page for each folder of posts and each tag itself, as https://statac.dev/lists/ describes.

- `archive.html`, line 4: `{% for post in site.posts %}`
- `index.html`, line 3: `{% for post in site.posts %}`

### Feeds made by templates

Readers subscribed to the old site's feed at each of these URLs, and the template that made it didn't come across, so nothing is there now. A list's `feed:` line can put that list's feed at a URL of its own, as `feed: /index.rss`, and the starter's feed template writes Atom: see https://statac.dev/feeds/.

- `feed.xml`: `/feed.xml`
- `feeds/index.xml`: `/feeds/index.xml`
- `feeds/posts.rss`: `/feeds/posts.rss`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_site/`: the old site's built output, read only to check the URLs.
- `feed.xml`: a file with front matter that isn't a page, such as a stylesheet or a feed, which Jekyll built with its templates.
- `feeds/index.xml`: a file with front matter that isn't a page, such as a stylesheet or a feed, which Jekyll built with its templates.
- `feeds/map.xml`: a file with front matter that isn't a page, such as a stylesheet or a feed, which Jekyll built with its templates.
- `feeds/posts.rss`: a file with front matter that isn't a page, such as a stylesheet or a feed, which Jekyll built with its templates.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
