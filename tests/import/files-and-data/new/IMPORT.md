This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/files-and-data/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/posts/`.
- 1 draft, with `draft: true`, so it isn't published by accident.
- 2 pages, into `content/` where they stood in the old site.
- 5 other files, copied as they were: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page is `content/index.md`.

The settings give no `url`, so the new site has no `url` yet: add it to `statac.yaml` once it is known.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

`layout` was taken off 1 page: each is built with the template for its kind, as https://statac.dev/templates/ describes.

The settings written in `statac.yaml`:

```yaml
title: Files and data
language: en
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 3 are published or redirected.

- 2 pages carry a `permalink` of their own, where no pattern fits.

Nothing in the new site is published at these old URLs. Give the page each should lead to a `redirect_from` naming it, or a `permalink`:

- `/archive.html`, for `archive.html`

## Left for you

### Data files left behind

Statac reads data files written in YAML or JSON only, so these weren't copied: write each as YAML or JSON in `data/`.

- `_data/table.csv`

### Pages Jekyll didn't publish

Each of these is in a folder of pages Jekyll's settings kept from being published, so it came across with `draft: true`: take that line out of each you want published.

- `_drafts-kept/item-1.md`

### Files in `_posts` that aren't posts

Jekyll publishes a post only from a file with front matter named as `2026-10-04-title.md`, so it didn't publish these, and Statac left them: bring each across by hand if it is wanted.

- `_posts/post-2.md`

### Pages made by templates

Each of these pages was mostly the old site's templates at work, such as a home page or an archive listing the posts, so Statac didn't bring it across: Statac makes the home page and a page for each folder of posts and each tag itself, as https://statac.dev/lists/ describes.

- `archive.html`, line 5: `{% for post in site.posts %}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `Gemfile`: a file Jekyll leaves out of the site.
- `_sass/`: the old site's Sass.
- `assets/style.scss`: a file with front matter that isn't a page, such as a stylesheet or a feed, which Jekyll built with its templates.
- `feed.xml`: a file with front matter that isn't a page, such as a stylesheet or a feed, which Jekyll built with its templates.
- `notes.txt`: a file Jekyll leaves out of the site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
