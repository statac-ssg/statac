This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-templates/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/posts/`.
- 3 pages, into `content/` where they stood in the old site.
- 1 other file, copied as it was: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Eleventy's settings are a program, and Statac runs none of it: it read `.eleventy.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings name no folder in a `dir` Statac can read, so Statac took Eleventy's own: `input` as `.`, `output` as `_site`, `includes` as `_includes` and `data` as `_data`.

The settings copy nothing as it is with an `addPassthroughCopy` Statac can read, so Statac copied only the files beside a page that its content names.

The settings name no `markdownTemplateEngine`, so Statac read the template tags in the Markdown files as Liquid, as Eleventy does unless told otherwise.

The settings name no `htmlTemplateEngine`, so Statac read the template tags in the HTML files as Liquid, as Eleventy does unless told otherwise.

The settings don't set `breaks: true`, so Statac read a line end inside a paragraph as a space, as Eleventy's Markdown does unless told otherwise.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

The settings give Markdown no library of their own, so Statac took Eleventy's own, which publishes HTML in Markdown as it is, as Statac does.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings name no `excerpt_separator`, so Statac changed no excerpt marker.

3 pages had no date, which Eleventy gave the day each one's file was made; Statac gives each none, so each is a page and not a post.

The settings written in `statac.yaml`:

```yaml
title: Old
language: en
timezone: UTC
date_format: 31 December 2025
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 6 old URLs there: 5 are published or redirected.

- Every page lands where the old site published it, with nothing added.

Nothing in the new site is published at these old URLs. Give the page each should lead to a `redirect_from` naming it, or a `permalink`:

- `/archive/`, for `archive.md`

## Left for you

### Pages made by templates

Each of these was the old site's templates at work, a page written in a template language such as Nunjucks or one listing the old site's pages, so Statac didn't bring it across: Statac makes the home page and a page for each folder of posts and each tag itself, as https://statac.dev/lists/ describes.

- `archive.md`, line 4: `{% for post in collections.all %}`
- `feed.11ty.js`
- `index.njk`
- `parts/part-1.njk`

### Pages written in HTML

Each of these was written in HTML and came across as a Markdown file holding that HTML, which Statac publishes as it is: read each, since Markdown reads a line indented by four spaces after a blank line as code.

- `about.html`
- `contact.html`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `README.md`: a file `.eleventyignore` leaves out, which Eleventy doesn't read.
- `notes/`: a folder `.eleventyignore` leaves out, which Eleventy doesn't read.
- `file-3.txt`: a file Eleventy didn't publish, being neither a page nor copied by the settings.
- `posts/`: 2 files Eleventy didn't publish, being neither pages nor copied by the settings, nor named by a page beside them.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
