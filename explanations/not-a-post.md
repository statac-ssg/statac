# Not a post

A page with a date is at the URL of a list, such as a tag's at
`/tags/rust/` or a folder's at `/notes/`. That makes it the list page: the
page that introduces the list and holds its settings.

## What you'll see {#what-youll-see}

```text
warning  Not a post: `content/tags/rust.md` is the list page for the tag `Rust`
  A list page is in no list; give it a `permalink` of its own to list it as a post.
  More: https://statac.dev/e/not-a-post/
```

## Why Statac warns

Any page at a list's URL is that list's list page. Its content
introduces the list, and its front matter contains the list's settings. A
list page is not one of the list's posts, and it isn't in any other list
or feed either. Its date suggests you wrote it as a post, so Statac
tells you it isn't listed as one, rather than leaving it out of every list
without a message. The page itself is published as usual.

## How to fix it

If the page is meant to introduce the list, take its date out, and the
warning goes away.

If it is a post, give it a URL of its own with `permalink:`, or rename
its file, so it is no longer at the list's URL.

## Example

`content/tags/rust.md` is published at `/tags/rust/`, the URL of the tag
`Rust`:

```yaml
---
title: Rust
date: 2026-10-04
---
```

To make it the tag's introduction, take out the date:

```yaml
---
title: Rust
---
```
