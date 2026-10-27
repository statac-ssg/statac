This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-pages/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 7 posts, into `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.
- 1 other file, copied unchanged: the files to publish into `static/` at the same path, so their URLs stay, and data files into `data/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

The settings written in `statac.yaml`:

```yaml
title: Pages
url: https://example.org
language: en
timezone: UTC
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 12 old URLs there: 12 are published or redirected.

- 2 pages carry a `permalink` of their own, where no pattern fits.
- 2 pages carry `redirect_from`, for the other URLs that led to them.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

Statac shows 1 image beside a page through resized copies it makes, and doesn't publish the image's own file, so nothing answers at its old URL, `/posts/post-1/photo-1.jpg`. Statac publishes an image's own file where a page links to it, or where its sidecar file, such as `photo-1.jpg.yaml`, holds `optimise: false`. With `optimise_images: false` in `statac.yaml`, Statac makes no resized copies and publishes each image unchanged: see https://statac.dev/images/.

## Left for you

Nothing: everything Statac read came across.

## Not converted

Statac left these behind, and the new site has nothing of them:

- `content/posts/data.xlsx`: a file the settings' `ignored_content` names, which Zola doesn't read.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
