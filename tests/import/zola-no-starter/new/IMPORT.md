This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-no-starter/old` by `statac import --no-starter`, which changed nothing there. It has none of the starter's templates, snippets, stylesheet or pages, so it won't build until you add templates of your own: see https://statac.dev/templates/. Its pages and feeds need `templates/feed.xml`, `templates/folder.html`, `templates/home.html`, `templates/page.html` and `templates/redirect.html`.

## What came across

- 1 post, into `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.
- 1 other file, copied as it was: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page had no content of its own, so `content/index.md` holds only front matter: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page, unless its list page gives `paginate_by`, so `posts_per_page: 0` in `statac.yaml` does the same.

The settings written in `statac.yaml`:

```yaml
title: No starter
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
typography: false
posts_per_page: 0
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 4 are published or redirected.

- 1 page carries `redirect_from`, for the other URLs that led to it.

## Left for you

### Tags now snippet calls

Each of these tags is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/note.html`:
  - `content/posts/post-1.md`, line 8: `{{ note() }}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `templates/`: the old site's templates.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
