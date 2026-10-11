# Two lists

A page has both `each:` and `posts:`. Each line chooses a list for the
page: `posts:` chooses the posts it lists, and `each:` chooses the pages
it makes. A page can have only one list. The message points to the second
line, and shows where the first one is.

## What you'll see {#what-youll-see}

```text
error  Two lists: this page has both `posts:` and `each:`
  Move one of them to a page of its own.
  More: https://statac.dev/e/two-lists/
```

## Why Statac stops

Statac can't tell which of the two lists you meant the page to have, and
choosing one would leave the other out without telling you.

## How to fix it

Move one of the lines to a page of its own, along with the lines that
belong with it. A `permalink:` and a `layout:` belong with `each:`.

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

Keep the `posts:` list here, and make the pages from a separate page,
`content/cards.md`:

```yaml
---
each: site.posts | tagged("Rust")
permalink: /cards/{name}/
index_page: false
---
```
