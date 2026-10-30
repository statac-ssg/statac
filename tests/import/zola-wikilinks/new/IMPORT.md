This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-wikilinks/old` by `statac import`, which changed nothing there. It has no templates, and doesn't build until you add `templates/folder.html`: see https://statac.dev/templates/. Adding `templates/folder.html` builds its pages.

## What came across

- 5 posts, into `content/fr/posts/` and `content/posts/`.
- 1 draft, with `draft: true`, so it isn't published by accident.
- 5 pages, into `content/` where they stood in the old site.
- 6 other files, copied unchanged: the files to publish into `static/` at the same path, so their URLs stay, and data files into `data/`.

No page of the old site's came across as its home pages, so `content/index.md` and `content/fr/index.md` have no content: write the home pages there.

The settings declare 2 languages, so the new site is in each, the main one, `en`, first: each other language's pages are in its own folder in `content/`, where they stand at the same place as the page they translate, as https://statac.dev/languages/#a-site-in-several-languages describes.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

The settings written in `statac.yaml`:

```yaml
title: Wikilinks
url: https://example.org
language: [en, fr]
timezone: UTC
fr.title: Liens
typography: false
tags.pages: true
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 12 old URLs there: 12 are published or redirected.

- 1 page carries `redirect_from`, for the other URLs that led to it.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

The old site published a page listing every one of its `tags` at `/tags/`, and the new site has none.

## Left for you

### Wikilinks now links

Statac's Markdown doesn't read wikilinks, so the import wrote each of these as an ordinary link to the URL of the page or file Zola linked it to.

- `content/posts/post-1.fr.md`, line 5: `[[post-2]]`
- `content/posts/post-1.fr.md`, line 5: `[[about.fr]]`
- `content/posts/post-1.md`, line 10: `[[about]]`
- `content/posts/post-1.md`, line 10: `[[posts/post-2]]`
- `content/posts/post-1.md`, line 10: `[[posts/_index|the posts]]`
- `content/posts/post-1.md`, line 12: `[[post-2|Post 2]]`
- `content/posts/post-1.md`, line 12: `[[old-about]]`
- `content/posts/post-1.md`, line 14: `[[post-2#heading-2]]`
- `content/posts/post-1.md`, line 14: `[[#heading-1|this heading]]`
- `content/posts/post-1.md`, line 16: `[[file-1.txt]]`
- `content/posts/post-1.md`, line 16: `[[posts/post-3/file 2.txt|File 2]]`
- `content/posts/post-1.md`, line 16: `![[image-1.svg|Image 1]]`
- `content/posts/post-2.md`, line 7: `[[pages/page-1]]`
- `content/posts/post-2.md`, line 7: `[[notes/page-1|Page 1]]`
- `content/posts/post-3/index.fr.md`, line 5: `[[file-1.txt]]`
- `content/posts/post-3/index.md`, line 5: `[[file-1.txt]]`

### Wikilinks the import left unchanged

Zola looks up the page or file a wikilink names by its path, an alias or its file's name. For each of these, Statac found no page or file Zola published by that name, or more than one, so the import left it unchanged and it appears on the page as written: link to the page's URL instead.

- `content/posts/post-1.md`, line 18: `[[post-4]]`
- `content/posts/post-1.md`, line 18: `[[page-1]]`
- `content/posts/post-1.md`, line 18: `[[draft-1]]`
- `content/posts/post-1.md`, line 18: `[[tags/tag-1]]`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
