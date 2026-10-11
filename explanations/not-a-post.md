# Not a post

A page with a date is at the address of a list, such as a tag's page at
`/tags/rust/` or a folder's page at `/notes/`. Because of that, it speaks
for the list.

## What you'll see {#what-youll-see}

```text
warning  Not a post: `content/tags/rust.md` speaks for the tag `Rust`
  It stands at the list's address, so no list holds it; give it a `permalink` of its own to list it as a post.
  More: https://statac.dev/e/not-a-post/
```

## Why Statac warns

Any page at a list's address speaks for the list. Its writing introduces
the list, and its front matter contains the list's settings. A page that
speaks for a list is not one of the list's posts, and it isn't in any other
list or feed either. Its date suggests you wrote it as a post, so Statac
tells you it isn't listed as one, rather than leaving it out of every list
without a message. The page itself is published as usual.

## How to fix it

If the page is meant to introduce the list, take its date out, and the
warning goes away.

If it is a post, give it an address of its own with `permalink:`, or
rename its file, so it is no longer at the list's address.

## Example

`content/tags/rust.md` is published at `/tags/rust/`, the address of the
tag `Rust`:

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
