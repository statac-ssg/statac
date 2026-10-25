This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-skip-templating/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

`content/posts/index.md` cascades `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

The settings written in `statac.yaml`:

```yaml
title: Writing not run as a template
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
tags.pages: true
series.pages: true
typography: false
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 3 old URLs there: 3 are published or redirected.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Tags in the content left as written

Statac's Markdown runs none of Zola's tags, and these have no equal in it, so each is left as written and shows on the page as it stands: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `content/posts/post-1.md`, line 5: `{{ page.title }}`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
