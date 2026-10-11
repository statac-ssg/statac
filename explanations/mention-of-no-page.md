# Mention of no page

A mention in `fetched/mentions/` is for an address on the site where
nothing is published. No page is at that address, and no page lists it in
its `redirect_from`. So no page shows the mention.

## What you'll see {#what-youll-see}

```text
warning  Mention of no page: nothing is published at `https://example.org/post-1/`
  If the page moved, add the old address to its `redirect_from`.
  More: https://statac.dev/e/mention-of-no-page/
```

## Why Statac warns

A mention is a message from another site saying that it linked to one of
your pages. It gives the page by its address. `statac refresh` keeps every
mention of an address on the site, whatever is at that address on the
day, because a draft or a post waiting for its day may be published there
later. When the site is built, each mention belongs to the page at its
address. If there is no page there, usually because a page has moved or
been removed, Statac warns and leaves the mention where it is. It doesn't
remove it.

A draft, or a post waiting for its day, counts as a page here. Its
mentions wait with it, and Statac doesn't warn about them.

## How to fix it

If the page moved, add its old address to its `redirect_from`. Its
mentions then belong to it again, and readers who follow the old link
are redirected to the page. If the page has gone for good, delete the
mention's file in `fetched/mentions/`. The mention's line in
`fetched/record` stops it being fetched again.

Once you have understood the warning, you can quieten it by name.

## Example

In `fetched/mentions/2028899.yaml`:

```yaml
target: "https://example.org/post-1/"
```

The page, now at `/posts/post-1/`, with its old address added:

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
