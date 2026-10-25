This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/markdown-forms/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 2 posts, into `content/posts/`.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

The settings give no `url`, so the new site has no `url` yet: add it to `statac.yaml` once it is known.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

Statac put a blank line after 1 footnote definition that had a link definition on the next line: Jekyll read such a line as a link definition, and Statac's Markdown reads it as more of the footnote.

The settings written in `statac.yaml`:

```yaml
title: Markdown forms
language: en
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 2 old URLs there: 2 are published or redirected.

- `content/posts/index.md` cascades `pages.permalink: /{year}/{month}/{day}/{slug}.html` to every page in `content/posts/`, which puts 2 of them at their old URLs.

## Left for you

### Tables of contents written in Markdown

Statac's Markdown reads `{:toc}` as words: take it and the list above it out. A template can show the page's headings from `page.toc`, as https://statac.dev/templates/#what-a-template-sees describes.

- `_posts/2026-01-01-post-1.md`, line 7: `{:toc}`

### Attribute lists

Statac's Markdown reads `{: ...}` as words, so each shows on the page as written: read each page that has one, and take it out or write the HTML it stood for.

- `_posts/2026-01-01-post-1.md`, line 10: `{: .class-1}`
- `_posts/2026-01-01-post-1.md`, line 12: `A word with an attribute list{:.class-2} inside a line.`
- `_posts/2026-01-02-post-2.md`, line 5: `{: .class-3}`

### Extension blocks

Statac's Markdown reads `{::name}` as words, so each shows on the page as written: read each page that has one, and take it out or write what it stood for, such as an HTML comment for `{::comment}`.

- `_posts/2026-01-01-post-1.md`, line 14: `{::comment}`
- `_posts/2026-01-01-post-1.md`, line 16: `{:/comment}`

### Maths between two dollar signs inside a line

Jekyll set these inside the line, and Statac sets maths between `$$` apart on a line of its own: read each page that has one, and write the maths with one `$` either side to keep it inside the line.

- `_posts/2026-01-01-post-1.md`, line 18: `Maths $$x^2$$ inside a line.`

### Markdown inside HTML

Jekyll read the Markdown inside each of these tags, as `markdown="1"` asks, and Statac's Markdown leaves what is inside HTML as it is, so the Markdown there shows as written: take off the HTML round it, or write what is inside as HTML.

- `_posts/2026-01-02-post-2.md`, line 7: `<div markdown="1">`
- `_posts/2026-01-02-post-2.md`, line 11: `<figure class="a" markdown='block'>`
- `_posts/2026-01-02-post-2.md`, line 15: `A <span markdown=span>*span*</span> inside a line.`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
