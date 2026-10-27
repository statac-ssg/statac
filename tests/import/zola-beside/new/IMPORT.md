This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-beside/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 5 posts, into `content/fr/posts/` and `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.
- 6 other files, copied unchanged: the files to publish into `static/` at the same path, so their URLs stay, and data files into `data/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

The settings declare 2 languages, so the new site is in each, the main one, `en`, first: each other language's pages are in its own folder in `content/`, where they stand at the same place as the page they translate, as https://statac.dev/dates-and-languages/#a-site-in-several-languages describes.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

The settings written in `statac.yaml`:

```yaml
title: Files beside pages
url: https://example.org
language: [en, fr]
timezone: UTC
fr.title: Fichiers
typography: false
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 7 old URLs there: 7 are published or redirected.

- 1 page carries a `permalink` of its own, where no pattern fits.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Links to pages that didn't come across

Each of these names a page by its file, and Statac couldn't find that file in the old site, or Zola didn't publish it, so the import left it unchanged: link to the page's URL instead.

- `content/posts/post-3.fr.md`, line 5: `{{ get_url(path="@/posts/post-2/file-4.txt") }}`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
