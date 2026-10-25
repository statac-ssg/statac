This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-aliases-kept/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/. Its pages and feeds need `templates/feed.xml` and `templates/folder.html`.

## What came across

- 2 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/feed.xml` once you add it, and `feed_ids_before` keeps each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

The settings written in `statac.yaml`:

```yaml
title: Aliases
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
feed_ids_before: 2026-10-06
tags.pages: true
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 8 old URLs there: 8 are published or redirected.

- 2 pages carry `redirect_from`, for the other URLs that led to them.

These old URLs can't be kept: the new site can't redirect from them, so each was left out of its page's `redirect_from`.

- `/tags/tag-1/`, for `content/posts/post-1.md`: the page for the tag `tag-1` is published there
- `/old/post-1-??/`, for `content/posts/post-1.md`: `?` can't be in a name
- `/posts/`, for `content/posts/post-1.md`: `content/posts/index.md` is published there
- `/posts/post-2/`, for `content/posts/post-1.md`: `content/posts/post-2.md` is published there
- `/old/post-1/`, for `content/posts/post-2.md`: `content/posts/post-1.md` already redirects from there

The old site published a page listing every one of its `tags` at `/tags/`, and the new site has none.

## Left for you

### Feeds made by templates

Readers subscribed to the old site's feed at each of these URLs, and the template that made it didn't come across, so nothing is there now. A list's `feed:` line can put that list's feed at a URL of its own, as `feed: /index.rss`: see https://statac.dev/feeds/.

- Hugo's feed for each tag and each list of that kind, 1 of them, such as `/tags/tag-1/index.xml`: Statac gives such a list a feed only where a list page of yours asks for one with `feed:`.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
