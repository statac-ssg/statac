# Two lists

A page has both `each:` and `posts:`. Each line chooses the list the page
speaks for: `posts:` the posts it lists, `each:` the pages it makes. A
page speaks for one list. The message points at the second line, and
shows where the first was written.

## What you'll see {#what-youll-see}

```text
error  Two lists: this page has both `posts:` and `each:`
  Move one of them to a page of its own.
  More: https://statac.dev/e/two-lists/
```

## Why Statac stops

Statac can't tell which of the two lists you meant the page to speak for,
and choosing one would leave the other out without a word.

## How to fix it

Move one of the lines to a page of its own, with what goes with it: a
`permalink:` and a `layout:` go with `each:`.

## Example

In `content/rust.md`:

```yaml
---
title: Rust
posts: site.posts | tagged("Rust")
each: site.posts | tagged("Rust")
permalink: /cards/{name}/
---
```

Keep the list here, and make the pages from another page,
`content/cards.md`:

```yaml
---
each: site.posts | tagged("Rust")
permalink: /cards/{name}/
index_page: false
---
```
