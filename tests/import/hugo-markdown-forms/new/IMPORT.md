This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-markdown-forms/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/. Adding `templates/index.xml` builds its feeds.

## What came across

- 2 posts, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/index.xml` once you add it, and `feed_ids_before` and `feed_ids_from: hugo` keep each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. Copy the docs' RSS template as `templates/index.xml`: see https://statac.dev/feeds/#the-template.

Hugo made a list of each folder at the top of the content folder, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

Hugo summed up 1 page with its opening paragraphs, as many as made its `summaryLength` of words, and Statac takes the first paragraph alone: where a summary should run further, put `<!--more-->` on a line of its own where it should end.

The settings written in `statac.yaml`:

```yaml
title: Markdown forms
url: https://example.org
language: en
timezone: UTC
feed_ids_before: 2026-10-06
feed_ids_from: hugo
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 5 old URLs there: 5 are published or redirected.

- Every page lands at the URL Statac worked out for it, with nothing added.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Settings with no equal

Statac has no equal for each of these settings, so nothing came across for them: https://statac.dev/settings/ says what Statac offers.

- `hugo.toml`, line 8: `markup: {highlight: {style: monokai}}`

### HTML Hugo left out

Hugo's settings left HTML in Markdown out of the page, and Statac publishes it unchanged, so each of these now shows: read each page that has one, and take out what wasn't meant to show.

- `content/posts/post-1.md`, line 9: `<div class="note">HTML Hugo left out.</div>`
- `content/posts/post-1.md`, line 11: `<span>`
- `content/posts/post-1.md`, line 22: `<div class="box">`

### Attribute lists

Hugo's settings read `{.class}` below a block as the block's own, and Statac's Markdown reads it as words, so each shows on the page as written: read each page that has one, and take it out or write the HTML it stood for.

- `content/posts/post-1.md`, line 14: `{.wide}`

### Emoji written by name

Hugo showed each of these names as its emoji where it knew one by the name, as the settings' `enableEmoji` asked, and Statac shows the name as written: write the emoji itself in its place, wherever it stands.

- `:smile:`, 2 places
- `:+1:`, 1 place

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
