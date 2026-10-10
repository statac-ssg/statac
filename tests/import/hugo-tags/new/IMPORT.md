This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-tags/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 3 posts, into `content/posts/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Hugo made a list of each folder at the top of the writing, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

2 names in the old site's `categories` are now tags, each with `category_` before it, as `category_Category 1`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

1 name in the old site's `authors` is now a tag, with `authors_` before it, as `authors_Name 1`, so it can still be told from the site's own tags. To rename it across the site, change it on each page that has it.

1 page named more than one series, and a page is in one series at most in Statac: the first is its `series`, and each other is a tag with `series_` before it.

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
feed_ids_before: 2026-10-06
tags.slugs:
  C++: c++
  Café: café
tags.permalink: /topics/{slug}/
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## Addresses

Statac worked out where the old site published each page, read the new site, and looked for each of the 14 old addresses there: 14 are published there, or sent on from there.

- 4 pages at the top of `content/`, such as `content/categories-category-1.md`, send readers on from the old page of a tag or a category to the tag's page, each with `permalink` and `redirect_to`.

Each address was also looked for in the old site's own built output, `public/`.

The old site published a page listing every one of its `series` at `/series/`, and the starter has none.

The old site published a page listing every one of its `categories` at `/categories/`, and the starter has none.

The old site published a page listing every one of its `authors` at `/authors/`, and the starter has none.

## Left for you

### Feeds made by templates

Readers subscribed to the old site's feed at each of these addresses, and the template that made it didn't come across, so nothing is there now. A list's `feed:` line can put that list's feed at an address of its own, as `feed: /index.rss`, and the starter's feed template writes Atom: see https://statac.dev/feeds/.

- Hugo's feed for each tag and each list of that kind, 8 of them, such as `/tags/c++/index.xml`: Statac gives such a list a feed only where a page of yours at its address asks for one with `feed:`.

### Addresses the old built site doesn't have

Statac worked these addresses out by the old tool's rules, but the old site's built output has nothing there, so the rule was wrong for the page or the output is older than the writing: find where the old site published the page, and add that address to the page's `redirect_from`.

- `content/posts/post-1.md`: `/posts/post-1/`
- `content/posts/post-2.md`: `/posts/post-2/`
- `content/posts/post-3.md`: `/posts/post-3/`
- /posts/index.xml
- /index.xml
- /topics/c++/
- /topics/café/
- /topics/tag-1/
- /series/series-1/
- /posts/

## Not converted

Statac left these behind, and the new site has nothing of them:

- `public/`: the old site's built output, read only to check the addresses.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
