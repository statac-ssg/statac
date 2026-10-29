This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-tags/old` by `statac import`, which changed nothing there. It has no templates, and doesn't build until you add `templates/folder.html`: see https://statac.dev/templates/. Adding `templates/folder.html` builds its pages.

## What came across

- 2 posts, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` has no content: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

Zola made a feed for each tag, as the settings' `taxonomies` asked, which has no equal in Statac: a tag has a feed only where a list page of yours asks for one with `feed:`.

Zola showed the posts of each tag some at a time, as the settings' `taxonomies` asked, and Statac shows them all on one page, so a tag's paginated pages aren't made.

3 names in the old site's `categories` are now tags, each with `category_` before it, as `category_Notes`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

1 name in the old site's `authors` is now a tag, with `authors_` before it, as `authors_Name 1`, so it can still be told from the site's own tags. To rename it across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Tags
url: https://example.org
language: en
timezone: UTC
tags.pages: true
tags.slugs:
  Rust: Rust
  Straße: Straße
tags.permalink: /Tags/{slug}/
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 9 old URLs there: 9 are published or redirected.

- 4 pages at the top of `content/`, such as `content/categories-Notes.md`, redirect from the old page of a tag, a category or a series to the page Statac makes for it, each with `permalink` and `redirect_to`.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

The old site published a page listing every one of its `Tags` at `/Tags/`, and the new site has none.

The old site published a page listing every one of its `categories` at `/categories/`, and the new site has none.

The old site published a page listing every one of its `authors` at `/authors/`, and the new site has none.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
