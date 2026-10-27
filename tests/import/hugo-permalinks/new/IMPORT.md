This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-permalinks/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/. Adding `templates/index.xml` builds its feeds.

## What came across

- 5 posts, into `content/blog/` and `content/essays/` and `content/notes/2026/` and `content/posts/`.
- 6 pages, into `content/` where they stood in the old site.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/index.xml` once you add it, and `feed_ids_before` and `feed_ids_from: hugo` keep each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. Copy the docs' RSS template as `templates/index.xml`: see https://statac.dev/feeds/#the-template.

Hugo made a list of each folder at the top of the content folder, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/blog/index.md` for `content/blog/`, `content/essays/index.md` for `content/essays/`, `content/pages/index.md` for `content/pages/`, `content/posts/index.md` for `content/posts/`.

The settings written in `statac.yaml`:

```yaml
title: Permalinks
url: https://example.org
language: en
timezone: UTC
feed_ids_before: 2026-10-06
feed_ids_from: hugo
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 21 old URLs there: 21 are published or redirected.

- 4 pages carry a `permalink` of their own, where no pattern fits.

Each URL was also looked for in the old site's own built output, `public/`.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Statac left these behind, and the new site has nothing of them:

- `public/`: the old site's built output, read only to check the URLs.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
