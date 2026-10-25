This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-front-matter/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/. Its pages need `templates/folder.html`.

## What came across

- 2 posts, into `content/posts/`.
- 1 draft, with `draft: true`, so it isn't published by accident.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

`template` was taken off 1 page, `content/posts/post-1.md`: it named the old site's template for the page.

`in_search_index` was taken off 1 page, `content/posts/post-1.md`: it said whether Zola's search index held the page, and Statac makes no search index.

1 name in the old site's `categories` is now a tag, with `category_` before it, as `category_Notes`, so it can still be told from the site's own tags. To rename it across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Front matter
url: https://example.org
language: en
timezone: UTC
tags.pages: true
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 5 old URLs there: 5 are published or redirected.

- 1 page at the top of `content/`, `content/categories-notes.md`, redirects from the old page of a tag, a category or a series to the page Statac makes for it, with `permalink` and `redirect_to`.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

The old site published a page listing every one of its `tags` at `/tags/`, and the new site has none.

The old site published a page listing every one of its `categories` at `/categories/`, and the new site has none.

## Left for you

### Settings with no equal

Statac has no equal for each of these settings, so nothing came across for them: https://statac.dev/settings/ says what Statac offers.

- `config.toml`, line 3: `smart_punctuation: true`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
