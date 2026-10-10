# Not a post

A page with a date stands at the address of a list, such as a tag's page
at `/tags/rust/` or a folder's at `/notes/`, so it speaks for that list.

## What you'll see {#what-youll-see}

```text
warning  Not a post: `content/tags/rust.md` speaks for the tag `Rust`
  It stands at the list's address, so no list holds it; give it a `permalink` of its own to list it as a post.
  More: https://statac.dev/e/not-a-post/
```

## Why Statac warns

Whatever page stands at a list's address speaks for the list: its writing
introduces the list, and its front matter carries the list's settings. A
page that speaks for a list is none of its posts, and is in no other list
or feed either. Its date says it was written as a post, so Statac tells
you it isn't listed as one, rather than leave it out of every list without
a word. The page itself is published as usual.

## How to fix it

If the page introduces the list, take its date out, and the warning goes.

If it is a post, give it an address of its own with `permalink:`, or
rename its file, so it no longer stands at the list's address.

## Example

`content/tags/rust.md` is published at `/tags/rust/`, the address of the
tag `Rust`:

```yaml
---
title: Rust
date: 2026-10-04
---
```

As the tag's introduction, with no date:

```yaml
---
title: Rust
---
```
