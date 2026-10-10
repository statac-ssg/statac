This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-file-names/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.
- 4 other files, copied as they were: files published as they are into `static/` at the same path, so their addresses stay, and data files into `data/`.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Hugo summed up 1 page with its opening paragraphs, as many as made its `summaryLength` of words, and Statac takes the first paragraph alone: where a summary should run further, put `<!--more-->` on a line of its own where it should end.

The settings written in `statac.yaml`:

```yaml
title: File names
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

Nothing answers the old addresses of the files given another name, and a file can't send readers on:

- `/posts/post-1/File%25201.txt`, for `content/posts/post-1/File%201.txt`
- `/posts/post-1/File%25202.txt`, for `content/posts/post-1/File%202.txt`
- `/posts/post-1/a%23b.txt`, for `content/posts/post-1/a#b.txt`
- `/Plan%25201.pdf`, for `static/Plan%201.pdf`

## Left for you

### Files given another name

Statac can't publish a file under the name each of these had, so each has one it can: a `%` and the two figures after it became the character they spell, as a browser reads them, and any other character an address or a disk won't take became `-`. Links in the writing that named the file reach it under its new name. Its old address is listed under Addresses, and nothing answers it: a file can't send readers on.

- `content/posts/post-1/File%201.txt`: `content/posts/post-1/File 1.txt`
- `content/posts/post-1/File%202.txt`: `content/posts/post-1/File 2.txt`
- `content/posts/post-1/a#b.txt`: `content/posts/post-1/a-b.txt`
- `static/Plan%201.pdf`: `static/Plan 1.pdf`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
