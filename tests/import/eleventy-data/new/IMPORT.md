This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-data/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 4 posts, into `content/notes/` and `content/posts/2026/` and `content/posts/`.
- 2 other files, copied as they were: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Eleventy's settings are a program, and Statac runs none of it: it read `.eleventy.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings name no folder in a `dir` Statac can read, so Statac took Eleventy's own: `input` as `.`, `output` as `_site`, `includes` as `_includes` and `data` as `_data`.

The settings copy nothing as it is with an `addPassthroughCopy` Statac can read, so Statac copied only the files beside a page that its content names.

The settings name no `markdownTemplateEngine`, so Statac read the template tags in the Markdown files as Liquid, as Eleventy does unless told otherwise.

The settings don't set `breaks: true`, so Statac read a line end inside a paragraph as a space, as Eleventy's Markdown does unless told otherwise.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

The settings give Markdown no library of their own, so Statac took Eleventy's own, which publishes HTML in Markdown as it is, as Statac does.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings name no `excerpt_separator`, so Statac changed no excerpt marker.

`layout` was taken off 4 pages and folders: each is built with the template for its kind, as https://statac.dev/templates/ describes.

The data each folder's own file cascaded to the pages beneath it is in `pages.` lines of `content/posts/index.md`, as https://statac.dev/front-matter/#handing-down describes; what can't cascade that way, as a `title` or a field inside a folder with no page of its own, is in each page's own front matter.

The settings written in `statac.yaml`:

```yaml
title: Old
language: en
timezone: UTC
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 4 are published or redirected.

- `content/posts/index.md` cascades `pages.permalink: /blog/{slug}/` to every page in `content/posts/`, which puts 3 of them at their old URLs.

The old site's built output isn't in `_site/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Data written as a program

Eleventy ran each of these JavaScript files for data, and Statac runs none, so what it gave didn't come across: write it as JSON or YAML, in `data/` for the whole site, or in the front matter of the pages it was for.

- `_data/menu.js`
- `notes/notes.11tydata.js`

### Data files left behind

Statac reads data files written in YAML or JSON only, so these weren't copied: write each as YAML or JSON in `data/`.

- `_data/notes.csv`

### Collections left out of tags

Eleventy reads the `tags` in a folder's data file as the collections the folder's pages belong to, as `"tags": "posts"` puts them in `collections.posts`, so these didn't become tags. Statac needs no collection to list a folder's pages: where one is also a tag readers should see, add it to the `tags` of each page.

- `posts/posts.json`, line 3: `tags: posts`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
