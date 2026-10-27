This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-files-and-data/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/. Adding `templates/index.xml` builds its feeds.

## What came across

- 1 post, into `content/posts/`.
- 5 other files, copied unchanged: the files to publish into `static/` at the same path, so their URLs stay, and data files into `data/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

`data/links.toml` was written in TOML, which Statac doesn't read in `data/`, so it is written as YAML, holding the same values, in `data/links.yaml`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/index.xml` once you add it, and `feed_ids_before` and `feed_ids_from: hugo` keep each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. Copy the docs' RSS template as `templates/index.xml`: see https://statac.dev/feeds/#the-template.

Hugo made a list of each folder at the top of the content folder, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

The settings written in `statac.yaml`:

```yaml
title: Files and data
url: https://example.org
language: en
timezone: UTC
feed_ids_before: 2026-10-06
feed_ids_from: hugo
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 4 are published or redirected.

- Every page lands at the URL Statac worked out for it, with nothing added.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

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
- `assets/`: the files Hugo's templates worked on, such as stylesheets.
- `go.mod`: a file Hugo doesn't publish.
- `i18n/`: the translations the old site's templates showed in each language.
- `layouts/`: the old site's layouts.
- `resources/`: the files Hugo kept from earlier builds.
- `themes/`: the old site's themes.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
