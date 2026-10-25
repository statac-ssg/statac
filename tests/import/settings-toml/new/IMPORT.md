This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/settings-toml/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site's home page is `content/index.md`, built with `templates/home.html`, which lists the posts below its content.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings written in `statac.yaml`:

```yaml
title: Settings in TOML
url: https://example.org
language: en
timezone: Europe/London
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
tags.pages: true
series.pages: true
author: Name
book:
  isbn: 0-000-00000-0
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 2 old URLs there: 2 are published or redirected.

- 1 page carries a `permalink` of its own, where no pattern fits.

## Left for you

### Settings with no equal

Statac has no equal for each of these settings or plugins, so nothing came across for them: https://statac.dev/settings/ says what Statac offers.

- `_config.toml`, line 5: `markdown: kramdown`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
