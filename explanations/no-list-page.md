# No list page

A folder, a tag or a series has posts, but nothing asks Statac to publish
a list page for it, so it has none. The message names the first such list
and counts the rest of its kind.

## What you'll see {#what-youll-see}

```text
note  No list page: `content/notes/` has no `index.md`
  Add `content/notes/index.md` to publish one.
  More: https://statac.dev/e/no-list-page/
```

## Why Statac says so

Statac publishes only the pages you ask for. A folder's list page is its
`index.md`. A tag or a series has a list page if `statac.yaml` says
`tags.pages: true` or `series.pages: true`, or if one of your pages is
published at its URL. Without one of these, the list still works in
templates, through `site.folders`, `site.tags` and `page.tags`, but
nothing is published at its URL and `url` has no value on it.

Statac before 0.7.0 made these pages without being asked, so a site
made with it may link to pages that are no longer there. `statac check`
says so once for the folders and once each for the tags and the series,
and only if the site has `templates/folder.html`, the template that
built them. It says nothing about a list for which you chose no page,
with `tags.pages: false` or `series.pages: false`.

`statac build` and `statac serve` don't show this note or count it.

## How to fix it

If the site should have the list page, ask for it. For a folder, add an
`index.md` to it. It can be empty, or hold the list's title and an
introduction. For the tags or the series, add the setting to
`statac.yaml`.

If the site needs no list page, nothing is wrong. Quieten the note by
name and it is no longer shown.

## Example

A folder of notes with no `index.md`:

```
content/
  index.md
  notes/
    note-1.md
    note-2.md
```

With `content/notes/index.md` added, its list page is published at
`/notes/`, built with `templates/folder.html`:

```yaml
---
title: Notes
---
```

A page for each tag, in `statac.yaml`:

```yaml
tags.pages: true
```
