This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-languages-folders/old` by `statac import`, which changed nothing there. It has no templates, and doesn't build until you add `templates/folder.html`: see https://statac.dev/templates/. Adding `templates/folder.html` and `templates/index.xml` builds its pages and feeds.

## What came across

- 2 posts, into `content/posts/` and `content/fr/posts/`.
- 2 pages, into `content/` where they stood in the old site.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

The settings declare 2 languages, so the new site is in each, the main one, `en`, first: each other language's pages are in its own folder in `content/`, where they stand at the same place as the page they translate, as https://statac.dev/languages/#a-site-in-several-languages describes.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/index.xml` once you add it, and `feed_ids_before` and `feed_ids_from: hugo` keep each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. Copy the docs' RSS template as `templates/index.xml`: see https://statac.dev/feeds/#the-template. The feed Hugo made for each tag has no equal.

Hugo made a list of each folder at the top of the content folder, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/en/posts/`, `content/fr/posts/index.md` for `content/fr/posts/`.

The settings written in `statac.yaml`:

```yaml
title: Languages by folder
url: https://example.org
language: [en, fr]
timezone: UTC
fr.title: Langues par dossier
feed_ids_before: 2026-10-06
feed_ids_from: hugo
tags.pages: true
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 11 old URLs there: 11 are published or redirected.

- Every page lands at the URL Statac worked out for it, with nothing added.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

The old site published a page listing every one of its `tags` at `/tags/`, and the new site has none.

## Left for you

### Feeds made by templates

Readers subscribed to the old site's feed at each of these URLs, and the template that made it didn't come across, so nothing is there now. A list's `feed:` line can put that list's feed at a URL of its own, as `feed: /index.rss`: see https://statac.dev/feeds/.

- Hugo's feed for each tag and each list of that kind, 1 of them, such as `/tags/tag-1/index.xml`: Statac gives such a list a feed only where a list page of yours asks for one with `feed:`.

## Not converted

Statac left these behind, and the new site has nothing of them:

- `content/notes.txt`: a file in no language's folder, which Hugo doesn't read.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
