This site was imported from a Zola site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-tags/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page, unless the list's own page gives `paginate_by`, so `posts_per_page: 0` in `statac.yaml` does the same.

Zola made a feed for each tag, as the settings' `taxonomies` asked, which has no equal in Statac: a tag has a feed only where a page of yours speaks for it.

Zola showed the posts of each tag some at a time, as the settings' `taxonomies` asked, and Statac shows them all on one page, so the later pages of a tag aren't made.

3 names in the old site's `categories` are now tags, each with `category_` before it, as `category_Notes`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

1 name in the old site's `authors` is now a tag, with `authors_` before it, as `authors_Name 1`, so it can still be told from the site's own tags. To rename it across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Tags
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
posts_per_page: 0
tags.slugs:
  Rust: Rust
  Straße: Straße
tags.permalink: /Tags/{slug}/
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 9 old addresses there: 9 are published there, or sent on from there.

- 4 pages at the top of `content/`, such as `content/categories-Notes.md`, send readers on from the old page of a tag, a category or a series to the page Statac makes for it, each with `permalink` and `redirect_to`.

The old site published a page listing every one of its `categories` at `/categories/`, and the starter has none.

The old site published a page listing every one of its `authors` at `/authors/`, and the starter has none.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
