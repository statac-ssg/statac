This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-settings-fetched/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Eleventy's settings are a program, and Statac runs none of it: it read `eleventy.config.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings name no folder in a `dir` Statac can read, so Statac took Eleventy's own: `input` as `.`, `output` as `_site`, `includes` as `_includes` and `data` as `_data`.

The settings copy nothing as it is with an `addPassthroughCopy` Statac can read, so Statac copied only the files beside a page that its content names.

The settings name no `markdownTemplateEngine`, so Statac read the template tags in the Markdown files as Liquid, as Eleventy does unless told otherwise.

The settings' Markdown had `breaks: true`, which made each line end inside a paragraph a line break, so Statac ended 1 line with `\`, which Statac reads as a line break.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

Markdown's settings were read in `config/markdown.js`, which the settings file fetches.

The settings' Markdown had `html: true`, so Eleventy published HTML in Markdown as it is, as Statac does.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings name no `excerpt_separator`, so Statac changed no excerpt marker.

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

Statac worked out where the old site published each page, read the new site, and looked for the one old URL there: it is published there.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Settings Statac didn't read

Eleventy's settings file is a program, and Statac ran none of it, so it doesn't know what each of these did: read each, and write what the new site needs of it, in its templates or snippets, as https://statac.dev/templates/ describes.

- `eleventy.config.js`, line 1: `import { markdownLib } from "./config/markdown.js"`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `config/`: 1 file Eleventy didn't publish, being neither pages nor copied by the settings, nor named by a page beside them.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
