This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-under-path/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Eleventy's settings are a program, and Statac runs none of it: it read `.eleventy.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

The settings' `pathPrefix` puts the old site under `/blog`, and each URL through the `url` filter is written with `/blog` in front, as Eleventy wrote it. Statac puts a site under a path only by its `url`, and the new site has none yet. Add the full URL as `url` in `statac.yaml`, such as `url: https://example.org/blog`. Until then, `statac check` says nothing is published at those links: leave them as they are, even where it offers another URL.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings name no folder in a `dir` Statac can read, so Statac took Eleventy's own: `input` as `.`, `output` as `_site`, `includes` as `_includes` and `data` as `_data`.

The settings copy nothing as it is with an `addPassthroughCopy` Statac can read, so Statac copied only the files beside a page that its content names.

The settings name no `markdownTemplateEngine`, so Statac read the template tags in the Markdown files as Liquid, as Eleventy does unless told otherwise.

The settings don't set `breaks: true`, so Statac read a line end inside a paragraph as a space, as Eleventy's Markdown does unless told otherwise.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

The settings give Markdown no library of their own, so Statac took Eleventy's own, which publishes HTML in Markdown as it is, as Statac does.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings name no `excerpt_separator`, so Statac changed no excerpt marker.

1 page had no date, which Eleventy gave the day its file was made; Statac gives it none, so it is a page and not a post.

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

Statac worked out where the old site published each page, read the new site, and looked for each of the 2 old URLs there: 2 are published or redirected.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Template tags left as written

Statac's Markdown runs no template tags and these have no equal in it, so each is left as written and shows on the page as it stands: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `posts/post-1.md`, line 19: `{{ page.url | url }}`
- `posts/post-1.md`, line 21: `{{ "/about/" | url | upcase }}`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
