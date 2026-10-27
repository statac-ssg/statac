This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-settings-double/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/posts/`.
- 3 other files, copied unchanged: the files to publish into `static/` at the same path, so their URLs stay, and data files into `data/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Eleventy's settings are a program, and Statac runs none of it: it read `.eleventy.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings' `dir` names `input` as `src`, `output` as `dist`, `includes` as `parts` and `data` as `info`.

The settings copy `src/img` and `fonts` as they are with `addPassthroughCopy`, so Statac copied 2 files into `static/`, where each keeps its URL.

The settings' `markdownTemplateEngine` is `njk`, so Statac read the template tags in the Markdown files as Nunjucks.

The settings' Markdown had `breaks: true`, which made each line end inside a paragraph a line break, so Statac ended 1 line with `\`, which Statac reads as a line break.

The settings' Markdown had `linkify: true`, which linked bare URLs; Statac links those starting `www.`, `http://` or `https://` too, and 1 other is no longer linked, listed under Left for you.

The settings' Markdown had `html: true`, so Eleventy published HTML in Markdown unchanged, as Statac does.

The settings use `markdown-it-attrs`, which read attributes in braces, as `{.class}`; Statac reads them only at the end of a heading, so 1 elsewhere shows as written, listed under Left for you.

The settings' `excerpt_separator`, `<!-- excerpt -->`, is now `<!--more-->`, Statac's excerpt marker, in 1 place.

The settings written in `statac.yaml`:

```yaml
title: Old
language: en
timezone: UTC
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for the one old URL there: it is published there.

- Every page lands at the URL Statac worked out for it, with nothing added.

The old site's built output isn't in `dist/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### URLs no longer linked

The old site's Markdown linked each of these, written without `www.` or `http`, and Statac doesn't, so each shows as words: write it as a link, as `<https://example.org>`, where one is meant.

- `src/posts/post-1.md`, line 9: `example.org`

### Attributes in braces

The old site's Markdown read each of these as attributes of what stands before it, and Statac reads them only at the end of a heading, so each shows as written: take it out, or write the HTML it stood for.

- `src/posts/post-1.md`, line 12: `{.wide}`

### Template tags left as written

Statac's Markdown runs no template tags and these have no equal in it, so each appears on the page as written: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `src/posts/post-1.md`, line 14: `{# A Nunjucks comment. #}`

### Settings Statac didn't read

Eleventy's settings file is a program, and Statac ran none of it, so it doesn't know what each of these did: read each, and write what the new site needs of it, in its templates or snippets, as https://statac.dev/templates/ describes.

- `.eleventy.js`, line 8: `addPlugin(pluginRss)`
- `.eleventy.js`, line 9: `addFilter("readableDate")`
- `.eleventy.js`, line 10: `addCollection("featured")`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `src/parts/`: the old site's includes and layouts; each include the content calls has a snippet of its name standing for it.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
