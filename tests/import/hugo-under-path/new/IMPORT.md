This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-under-path/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.
- 2 other files, copied as they were: files published as they are into `static/` at the same path, so their addresses stay, and data files into `data/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

The starter's `content/robots.md` was left out: the old site's `static/robots.txt` stands at its address.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Hugo made a list of each folder at the top of the writing, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

Hugo summed up 1 page with its opening paragraphs, as many as made its `summaryLength` of words, and Statac takes the first paragraph alone: where a summary should run further, put `<!--more-->` on a line of its own where it should end.

3 places held a tag of the site's own that showed one of the site's values and nothing else, such as `{{< baseurl >}}`: each holds the value itself now, the site's address as the path from its top.

1 figure is now a call to the starter's `figure` snippet.

The settings written in `statac.yaml`:

```yaml
title: Under a path
url: https://example.org/blog
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

Statac worked out where the old site published each page, read the new site, and looked for each of the 6 old addresses there: 6 are published there, or sent on from there.

- The site is under a path, `/blog`, as its `url` says, so every address starts with it, and links in the writing were written with it: see https://statac.dev/addresses/.

Each address was also looked for in the old site's own built output, `public/`.

## Left for you

### Links to pages that didn't come across

Each of these names a page Statac couldn't find in the old site, so it is left as written: link to the page's address instead.

- `content/posts/post-1.md`, line 11: `{{< ref "post-9.md" >}}`

### Addresses the old built site doesn't have

Statac worked these addresses out by the old tool's rules, but the old site's built output has nothing there, so the rule was wrong for the page or the output is older than the writing: find where the old site published the page, and add that address to the page's `redirect_from`.

- `content/posts/post-2.md`: `/posts/post-2/`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `layouts/`: the old site's layouts, which the starter's templates take the place of.
- `public/`: the old site's built output, read only to check the addresses.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
