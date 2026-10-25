This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-settings/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.
- 1 other file, copied as it was: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

The starter's `content/robots.md` was left out: the old site's `static/robots.txt` is published at its URL.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings' `[markdown]` opened each link to another site with its own `target` or `rel`, so `snippets/markdown/link.html` writes each link the same.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

`content/posts/index.md` cascades `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

The settings written in `statac.yaml`:

```yaml
title: Settings
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
tags.pages: true
series.pages: true
description: A site whose settings come across.
author: Name
github: name
menu:
  - name: Posts
    url: /posts/
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 3 old URLs there: 3 are published or redirected.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Settings with no equal

Statac has no equal for each of these settings, so nothing came across for them: https://statac.dev/settings/ says what Statac offers.

- `zola.toml`, line 5: `compile_sass: true`
- `zola.toml`, line 6: `build_search_index: true`
- `zola.toml`, line 9: `markdown.highlight_code: true`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `sass/`: the old site's Sass stylesheets, which the starter's `assets/site.css` takes the place of.
- `templates/`: the old site's templates, which the starter's templates take the place of.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
