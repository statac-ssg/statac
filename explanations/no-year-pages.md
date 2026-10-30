# No year pages

`years.pages: true` in `statac.yaml` asks for a page for each year, but
no page has a date, so there is no year to publish a page for. If
`years.folder` names a folder, only the pages in that folder count.

## What you'll see {#what-youll-see}

```text
warning  No year pages: no page has a date
  statac.yaml, line 2
   2 │ years.pages: true
     │              ^^^^
  Give a page a `date` to publish a page for its year.
  More: https://statac.dev/e/no-year-pages/
```

## Why Statac warns

A year's page lists the posts dated in that year, and a page is a post
only if it has a `date`. Statac builds the rest of the site as usual,
but the setting you wrote publishes nothing, and nothing else would
tell you.

## How to fix it

Give the posts a `date` in their front matter. Each year a post is
dated in then gets its page, at `/2026/` for 2026 unless
`years.permalink` says otherwise.

If `years.folder` names a folder, check that it is the folder the posts
are in. A post filed in a folder inside it counts too.

If the site needs no year pages yet, take `years.pages` out of
`statac.yaml`.

## Example

In `statac.yaml`:

```yaml
years.pages: true
years.folder: notes
```

with every dated post in `content/blog/` and none in `content/notes/`.
To list the posts in `content/blog/` under their years, the setting is:

```yaml
years.folder: blog
```
