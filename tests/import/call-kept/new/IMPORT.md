This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/call-kept/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.
- 1 other file, copied unchanged: the files to publish into `static/` at the same path, so their URLs stay, and data files into `data/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings written in `statac.yaml`:

```yaml
title: Site 1
url: https://example.org
language: en
timezone: UTC
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 3 old URLs there: 3 are published or redirected.

- 2 pages carry a `permalink` of their own, where no pattern fits.
- 1 page carries `redirect_from`, for the other URLs that led to it.

The old site's built output isn't in `_site/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Settings with no equal

Statac has no equal for each of these settings or plugins, so nothing came across for them: https://statac.dev/settings/ says what Statac offers.

- `_config.yml`, line 4: `plugins: jekyll-sitemap`

### Includes now snippet calls

Each include is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/note.html`:
  - `_posts/2026-01-01-post-1.md`, line 8: `{% include note.html %}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_includes/`: the old site's includes; each one the content calls has a snippet of its name standing for it.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
