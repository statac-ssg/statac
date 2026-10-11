This site was imported from a Hugo site, as well as Statac can. It is a best effort: the writing, its settings and its addresses came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-aliases-kept/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site had no home page of its own writing, so the starter's home page stays, listing the posts: `content/index.md`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list's own page. Each is now an Atom feed at the same address, written by `templates/feed.xml`, as `feed: index.xml` on each list's page says, and `feed_ids_before` keeps each post's id the address Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` hands `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

The settings written in `statac.yaml`:

```yaml
title: Aliases
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

Statac worked out where the old site published each page, read the new site, and looked for each of the 8 old addresses there: 8 are published there, or sent on from there.

- 2 pages carry `redirect_from`, for the other addresses that led to them.

These old addresses can't be kept: the new site can't send readers on from them, so each was left out of its page's `redirect_from`.

- `/tags/tag-1/`, for `content/posts/post-1.md`: the page for the tag `tag-1` is published there
- `/old/post-1-??/`, for `content/posts/post-1.md`: `?` can't be in a name
- `/posts/`, for `content/posts/post-1.md`: `content/posts/index.md` is published there
- `/posts/post-2/`, for `content/posts/post-1.md`: `content/posts/post-2.md` is published there
- `/old/post-1/`, for `content/posts/post-2.md`: `content/posts/post-1.md` sends readers on from there already

## Left for you

### Feeds made by templates

Readers subscribed to the old site's feed at each of these addresses, and the template that made it didn't come across, so nothing is there now. A list's `feed:` line can put that list's feed at an address of its own, as `feed: /index.rss`, and the starter's feed template writes Atom: see https://statac.dev/feeds/.

- Hugo's feed for each tag and each list of that kind, 1 of them, such as `/tags/tag-1/index.xml`: Statac gives such a list a feed only where a page of yours at its address asks for one with `feed:`.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
