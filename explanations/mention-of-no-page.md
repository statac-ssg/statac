# Mention of no page

A webmention in `fetched/mentions/` is for a URL on the site where
nothing is published. No page is at that URL, and no page lists it in its
`redirect_from`. So no page shows the webmention.

## What you'll see {#what-youll-see}

```text
warning  Mention of no page: nothing is published at `https://example.org/post-1/`
  If the page moved, add the old URL to its `redirect_from`.
  More: https://statac.dev/e/mention-of-no-page/
```

## Why Statac warns

A webmention is a message from another site saying that it linked to one
of your pages. It names the page by its URL. `statac refresh` keeps every
webmention of a URL on the site, whatever is at that URL on the day,
because a draft or a scheduled post may be published there later. When
Statac builds the site, it attaches each webmention to the page at its
URL. If
there is no page there, usually because a page has moved or been removed,
Statac warns and leaves the webmention where it is. It doesn't remove it.

A draft, or a scheduled post, counts as a page here. Its webmentions wait
with it, and Statac doesn't warn about them.

## How to fix it

If the page moved, add its old URL to its `redirect_from`. Its
webmentions then belong to it again, and readers who follow the old link
are redirected to the page. If the page has gone for good, delete the
webmention's file in `fetched/mentions/`. The webmention's line in
`fetched/record` stops it being fetched again.

Once you have understood the warning, you can quieten it by name.

## Example

In `fetched/mentions/2028899.yaml`:

```yaml
target: "https://example.org/post-1/"
```

The page, now at `/posts/post-1/`, with its old URL added:

```yaml
---
title: Post 1
redirect_from: [/post-1/]
---
```

To stop the warning being shown, in `statac.yaml`:

```yaml
quiet: [mention-of-no-page]
```
