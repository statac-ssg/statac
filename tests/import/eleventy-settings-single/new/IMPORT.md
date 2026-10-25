This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-settings-single/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/`.
- 2 other files, copied as they were: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Eleventy's settings are a program, and Statac runs none of it: it read `eleventy.config.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings' `dir` names `input` as `site`, `output` as `built`, `includes` as `layouts` and `data` as `data`.

The settings copy `files` as it is with `addPassthroughCopy`, so Statac copied 1 file into `static/`, where it keeps its URL.

The settings' `markdownTemplateEngine` is `liquid`, so Statac read the template tags in the Markdown files as Liquid.

The settings don't set `breaks: true`, so Statac read a line end inside a paragraph as a space, as Eleventy's Markdown does unless told otherwise.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

The settings' Markdown had `html: false`, so Eleventy showed HTML in Markdown as text, and Statac publishes it as HTML: 3 places are listed under Left for you.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings' `excerpt_separator`, `<!-- cut -->`, is now `<!--more-->`, Statac's excerpt marker, in 1 place.

The settings written in `statac.yaml`:

```yaml
title: Old
language: en
timezone: UTC
date_format: 31 December 2025
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for the one old URL there: it is published there.

- Every page lands where the old site published it, with nothing added.

## Left for you

### HTML the old site showed as text

The old site's Markdown had HTML turned off and showed each of these as text, and Statac publishes HTML in Markdown as it is: write each as you mean it, or put a backslash before its `<` to show it as text.

- `site/post-1.md`, line 8: `<b>`
- `site/post-1.md`, line 8: `</b>`
- `site/post-1.md`, line 10: `<div>A block of HTML.</div>`

### Tags now snippet calls

Each of these tags of the old site's own is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/year.html`:
  - `site/post-1.md`, line 12: `{% year %}`

### Settings Statac didn't read

Eleventy's settings file is a program, and Statac ran none of it, so it doesn't know what each of these did: read each, and write what the new site needs of it, in its templates or snippets, as https://statac.dev/templates/ describes.

- `eleventy.config.js`, line 5: `addShortcode('year')`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `site/layouts/`: the old site's includes and layouts; each include the content calls has a snippet of its name standing for it.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
