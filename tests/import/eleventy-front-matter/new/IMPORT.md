This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-front-matter/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 3 posts, into `content/posts/`.
- 3 pages, into `content/` where they stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Eleventy's settings are a program, and Statac runs none of it: it read `.eleventy.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings name no folder in a `dir` Statac can read, so Statac took Eleventy's own: `input` as `.`, `output` as `_site`, `includes` as `_includes` and `data` as `_data`.

The settings copy nothing as it is with an `addPassthroughCopy` Statac can read, so Statac copied only the files beside a page that its content names.

The settings name no `markdownTemplateEngine`, so Statac read the template tags in the Markdown files as Liquid, as Eleventy does unless told otherwise.

The settings don't set `breaks: true`, so Statac read a line end inside a paragraph as a space, as Eleventy's Markdown does unless told otherwise.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

The settings give Markdown no library of their own, so Statac took Eleventy's own, which publishes HTML in Markdown as it is, as Statac does.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings name no `excerpt_separator`, so Statac changed no excerpt marker.

`layout` was taken off 1 page or folder: each is built with the starter's template for its kind, as https://statac.dev/templates/ describes.

Statac took 1 control character out of the front matter of 1 page: Eleventy published each as nothing, and Statac's reader stops at one.

`templateEngineOverride` was taken off 1 page, once the content was read in the language it named.

3 pages had no date, which Eleventy gave the day each one's file was made; Statac gives each none, so each is a page and not a post.

The settings written in `statac.yaml`:

```yaml
title: Old
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 6 old URLs there: 6 are published or redirected.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Front matter written as a program

Eleventy ran the front matter of each of these as JavaScript, and Statac runs none, so the page came across with none: write its fields in YAML between `---` lines.

- `posts/post-3.md`, line 1: `---js`

### Dates Eleventy took from the file

Eleventy dated each of these pages by its file, as the words say, and Statac can't know that day, so `date` was taken off and the page is dated by its name or not at all: write the day it was published, as `2026-10-04`.

- `posts/post-4.md`, line 4: `date: Last Modified`

### Eleventy's fields kept as custom fields

Statac has no equal for each of these fields of Eleventy's own, so each came across as a custom field and does nothing: take it out, or show it from a template.

- `eleventyComputed`:
  - `page-1.md`, line 3: `eleventyComputed: {title: "{{ title }} again"}`
- `eleventyExcludeFromCollections`:
  - `posts/post-1.md`, line 8: `eleventyExcludeFromCollections: true`
- `eleventyNavigation`:
  - `posts/post-1.md`, line 9: `eleventyNavigation: {key: Post 1, order: 2}`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
