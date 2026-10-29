This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-tags/old` by `statac import`, which changed nothing there. It has no templates, and doesn't build until you add `templates/folder.html`: see https://statac.dev/templates/. Adding `templates/folder.html` and `templates/index.xml` builds its pages and feeds.

## What came across

- 3 posts, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` has no content: write the home page there.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/index.xml` once you add it, and `feed_ids_before` and `feed_ids_from: hugo` keep each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. Copy the docs' RSS template as `templates/index.xml`: see https://statac.dev/feeds/#the-template. The feed Hugo made for each tag has no equal.

Hugo made a list of each folder at the top of the content folder, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

2 names in the old site's `categories` are now tags, each with `category_` before it, as `category_Category 1`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

1 name in the old site's `authors` is now a tag, with `authors_` before it, as `authors_Name 1`, so it can still be told from the site's own tags. To rename it across the site, change it on each page that has it.

1 page named more than one series, and a page is in one series at most in Statac: the first is its `series`, and each other is a tag with `series_` before it.

The settings written in `statac.yaml`:

```yaml
title: Tags
url: https://example.org
language: en
timezone: UTC
feed_ids_before: 2026-10-06
feed_ids_from: hugo
tags.slugs:
  C++: c++
  Café: café
tags.pages: true
tags.permalink: /topics/{slug}/
series.pages: true
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 14 old URLs there: 14 are published or redirected.

- 4 pages at the top of `content/`, such as `content/categories-category-1.md`, redirect from the old page of a tag, a category or a series to the page Statac makes for it, each with `permalink` and `redirect_to`.

Each URL was also looked for in the old site's own built output, `public/`.

The old site published a page listing every one of its `series` at `/series/`, and the new site has none.

The old site published a page listing every one of its `categories` at `/categories/`, and the new site has none.

The old site published a page listing every one of its `authors` at `/authors/`, and the new site has none.

## Left for you

### Feeds made by templates

Readers subscribed to the old site's feed at each of these URLs, and the template that made it didn't come across, so nothing is there now. A list's `feed:` line can put that list's feed at a URL of its own, as `feed: /index.rss`: see https://statac.dev/feeds/.

- Hugo's feed for each tag and each list of that kind, 8 of them, such as `/topics/c++/index.xml`: Statac gives such a list a feed only where a list page of yours asks for one with `feed:`.

### URLs the old built site doesn't have

Statac worked these URLs out by the old tool's rules, but the old site's built output has nothing there, so the rule was wrong for the page or the output is older than the content: find where the old site published the page, and add that URL to the page's `redirect_from`.

- `content/posts/post-1.md`: `/posts/post-1/`
- `content/posts/post-2.md`: `/posts/post-2/`
- `content/posts/post-3.md`: `/posts/post-3/`

### Lists' and feeds' URLs the old built site doesn't have

Statac worked these URLs out by the old tool's rules, but the old site's built output has nothing there, so the rule was wrong for the site or the output is older than the content. Each is a list's or a feed's, not one page's, so there is no `redirect_from` to add: where the old site published the list at another URL, that URL is among the pages listed at URLs Statac didn't work out.

- `/posts/index.xml`
- `/index.xml`
- `/topics/c++/`
- `/topics/café/`
- `/topics/tag-1/`
- `/series/series-1/`
- `/posts/`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `public/`: the old site's built output, read only to check the URLs.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
