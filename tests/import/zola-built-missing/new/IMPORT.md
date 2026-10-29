This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-built-missing/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/. Adding `templates/feed.xml` builds its feeds.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

No page of the old site's came across as its home page, so `content/index.md` has no content: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

Zola's feed, at `/atom.xml`, stays at the same URL, as `feed:` on each list page says, built with `templates/feed.xml` once you add it, and `feed_ids_before` and `feed_ids_from: zola` keep each post's id the URL Zola gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids.

The settings written in `statac.yaml`:

```yaml
title: Built
url: https://example.org
language: en
timezone: UTC
feed_ids_before: 2026-10-06
feed_ids_from: zola
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 4 are published or redirected.

- 1 page carries `redirect_from`, for the other URLs that led to it.

Each URL was also looked for in the old site's own built output, `public/`.

The old site's built output also holds 1 page at a URL Statac didn't work out, and nothing in the new site is published there. Give the page it should lead to a `redirect_from` naming it:

- `/posts/`: 1 page, `/posts/post-one/`

## Left for you

### URLs the old built site doesn't have

Statac worked these URLs out by the old tool's rules, but the old site's built output has nothing there, so the rule was wrong for the page or the output is older than the content: find where the old site published the page, and add that URL to the page's `redirect_from`.

- `content/posts/post-1.md`: `/posts/post-1/`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `public/`: the old site's built output, read only to check the URLs.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
