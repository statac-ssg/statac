This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-categories/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 2 posts, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Eleventy's settings are a program, and Statac runs none of it: it read `.eleventy.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings name no folder in a `dir` Statac can read, so Statac took Eleventy's own: `input` as `.`, `output` as `_site`, `includes` as `_includes` and `data` as `_data`.

The settings have no `addPassthroughCopy` Statac can read, so Statac copied only the files beside a page that its content names.

The settings name no `markdownTemplateEngine`, so Statac read the template tags in the Markdown files as Liquid, as Eleventy does unless told otherwise.

The settings don't set `breaks: true`, so Statac read a line end inside a paragraph as a space, as Eleventy's Markdown does unless told otherwise.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

The settings give Markdown no library of their own, so Statac took Eleventy's own, which publishes HTML in Markdown unchanged, as Statac does.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings name no `excerpt_separator`, so Statac changed no excerpt marker.

2 names in the old site's `categories` are now tags, each with `category_` before it, as `category_Group 1`, so they can still be told from the site's own tags. To rename one across the site, change it on each page that has it.

The settings written in `statac.yaml`:

```yaml
title: Old
language: en
timezone: UTC
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 2 old URLs there: 2 are published or redirected.

- Every page lands at the URL Statac worked out for it, with nothing added.

The old site's built output isn't in `_site/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Pages made by templates

Each of these was the old site's templates at work, a page written in a template language such as Nunjucks or one listing the old site's pages, so Statac didn't bring it across: Statac makes a list page where the site asks for one, with an `index.md` in a folder, `templates/home.html` for the home page, or `tags.pages: true`, as https://statac.dev/lists/ describes.

- `categories.njk`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
