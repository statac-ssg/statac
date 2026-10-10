# Mention of no page

A mention in `fetched/mentions/` is of an address on the site where
nothing is published: no page stands there, and no page's
`redirect_from` names it. No page shows it.

## What you'll see {#what-youll-see}

```text
warning  Mention of no page: nothing is published at `https://example.org/post-1/`
  If the page moved, add the old address to its `redirect_from`.
  More: https://statac.dev/e/mention-of-no-page/
```

## Why Statac warns

A mention is another site saying it linked to one of your pages, by the
page's address. `statac refresh` keeps every mention of an address on the
site, whatever stands there that day, since a draft or a post waiting
for its day may be published there later. When the site is built, each
mention belongs to the page at its address. Where there is none, mostly
because a page has moved or gone, Statac warns and leaves the mention
where it is: it is never removed.

A draft, or a post waiting for its day, counts as a page here: its
mentions wait with it, and nothing is said of them.

## How to fix it

If the page moved, add its old address to its `redirect_from`. Its
mentions then belong to it again, and readers following the old link
are sent on. If the page has gone for good, delete the mention's file in
`fetched/mentions/`. Its line in `fetched/record` keeps it from being
fetched again.

Once you have understood it, the warning can be quietened by name.

## Example

In `fetched/mentions/2028899.yaml`:

```yaml
target: "https://example.org/post-1/"
```

The page now at `/posts/post-1/`, with its old address:

```yaml
---
title: Post 1
redirect_from: [/post-1/]
---
```

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [mention-of-no-page]
```
