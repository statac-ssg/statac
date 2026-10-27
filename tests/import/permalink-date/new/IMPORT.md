This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/permalink-date/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 4 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

3 names in the old site's `categories` are now tags, each with `category_` before it, as `category_one`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Permalink date
url: https://example.org
language: en
timezone: UTC
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 5 old URLs there: 5 are published or redirected.

- 5 pages carry a `permalink` of their own, where no pattern fits.

Each URL was also looked for in the old site's own built output, `_site/`.

## Left for you

### Pages written in HTML

Each of these was written in HTML and came across as a Markdown file holding that HTML, which Statac publishes unchanged: read each, since Markdown reads a line indented by four spaces after a blank line as code.

- `_posts/2026-01-04-post-4.html`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_site/`: the old site's built output, read only to check the URLs.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
