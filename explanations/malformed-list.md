# Malformed list

Statac can't read a page's `posts:` line, which chooses the posts the
page lists.

## What you'll see {#what-youll-see}

```text
error  Malformed list: a list starts from `site.posts`, not `site.pages`
  Start it with `site.posts`, such as `site.posts | tagged("Rust")`.
  More: https://statac.dev/e/malformed-list/
```

## Why Statac stops

A `posts:` line starts from `site.posts`, which is every post on the
site, and narrows it down with filters, each written after a `|`:

- `tagged("Rust")` keeps the posts with that tag.
- `in("blog")` keeps the posts in that folder, including those in folders inside it.
- `where("field", "value")` keeps the posts whose field has that value.

Templates use the same filters. If a line starts from anything else,
uses another filter, or leaves a bracket or a quote open, Statac can't
tell which posts it means. The build stops, and the message points to the
place in the line.

## How to fix it

Start the line with `site.posts`, and write each filter with its values in
brackets and each value in quotes. The page's `order:` sets the order the
posts are listed in. It is newest first by default, or `oldest first` if
the page says so.

## Example

`content/rust.md`:

```yaml
---
title: Rust
posts: site.posts | tagged(Rust
---
```

With the bracket closed:

```yaml
---
title: Rust
posts: site.posts | in("blog") | tagged("Rust")
---
```
