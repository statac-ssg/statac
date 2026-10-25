This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-no-starter/old` by `statac import --no-starter`, which changed nothing there. It has none of the starter's templates, snippets, stylesheet or pages, so it won't build until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.
- 1 other file, copied as it was: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Eleventy's settings are a program, and Statac runs none of it: it read `.eleventy.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings name no folder in a `dir` Statac can read, so Statac took Eleventy's own: `input` as `.`, `output` as `_site`, `includes` as `_includes` and `data` as `_data`.

The settings copy `file-1.txt` as it is with `addPassthroughCopy`, so Statac copied 1 file into `static/`, where it keeps its URL.

The settings name no `markdownTemplateEngine`, so Statac read the template tags in the Markdown files as Liquid, as Eleventy does unless told otherwise.

The settings don't set `breaks: true`, so Statac read a line end inside a paragraph as a space, as Eleventy's Markdown does unless told otherwise.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

The settings give Markdown no library of their own, so Statac took Eleventy's own, which publishes HTML in Markdown as it is, as Statac does.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings name no `excerpt_separator`, so Statac changed no excerpt marker.

1 page had no date, which Eleventy gave the day its file was made; Statac gives it none, so it is a page and not a post.

The old site's `_redirects` redirected 1 old URL to a page, each now in that page's `redirect_from`.

The settings written in `statac.yaml`:

```yaml
title: Old
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 3 old URLs there: 3 are published or redirected.

- 1 page carries `redirect_from`, for the other URLs that led to it.

## Left for you

### Includes now snippet calls

Each include is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/note.html`:
  - `posts/post-1.md`, line 7: `{% include "note.html" %}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_includes/`: the old site's includes and layouts; each include the content calls has a snippet of its name standing for it.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
