# Malformed list

A page's `posts:` line, which chooses the posts the page lists, can't be
read.

## What you'll see {#what-youll-see}

```text
error  Malformed list: a list starts from `site.posts`, not `site.pages`
  Start it with `site.posts`, as `site.posts | tagged("Rust")`.
  More: https://statac.dev/e/malformed-list/
```

## Why Statac stops

A `posts:` line starts from every post on the site, `site.posts`, and
keeps some of them with filters, each after a `|`:

- `tagged("Rust")` keeps the posts with that tag.
- `in("blog")` keeps the posts in that folder, however deeply filed.
- `where("field", "value")` keeps the posts whose field holds that value.

These are the same filters templates use. A line that starts elsewhere,
uses another filter, or leaves a bracket or a quote open can't say which
posts it means, so the build stops at the place in the line.

## How to fix it

Start the line with `site.posts`, and write each filter with its values in
brackets, each value in quotes. The order the posts are listed in is the
page's `order:`, newest first unless it says `oldest first`.

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
