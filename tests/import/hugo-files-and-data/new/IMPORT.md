This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-files-and-data/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.
- 5 other files, copied as they were: files published as they are into `static/` at the same path, so their addresses stay, and data files into `data/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

The starter's `content/robots.md` was left out: the old site's `static/robots.txt` stands at its address.

`data/links.toml` was written in TOML, which Statac doesn't read in `data/`, so it is written as YAML, holding the same values, in `data/links.yaml`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Hugo made a list of each folder at the top of the writing, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

The settings written in `statac.yaml`:

```yaml
title: Files and data
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
feed_ids_before: 2026-10-06
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old addresses there: 4 are published there, or sent on from there.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Pages in formats Statac doesn't read

Hugo read each of these pages in a format other than Markdown or HTML, such as AsciiDoc or Org, which Statac doesn't, so it didn't come across: write it in Markdown.

- `content/posts/post-2.adoc`

### Data files left behind

Statac reads data files written in YAML or JSON, and writes those in TOML as YAML, so these weren't copied: each is in another format, or its TOML stops reading at the line given. Write each as YAML or JSON in `data/`.

- `data/broken.toml`, line 1: `broken = `
- `data/table.csv`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `archetypes/`: the old site's patterns for new pages, which Statac has no equal for.
- `assets/`: the files Hugo's templates worked on, such as stylesheets, which the starter's `assets/site.css` takes the place of.
- `go.mod`: a file Hugo doesn't publish.
- `i18n/`: the words the old site's templates showed in each language, which the starter's `data/words.yaml` takes the place of.
- `layouts/`: the old site's layouts, which the starter's templates take the place of.
- `resources/`: the files Hugo kept from earlier builds.
- `themes/`: the old site's themes, which the starter's templates, snippets and stylesheet take the place of.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
