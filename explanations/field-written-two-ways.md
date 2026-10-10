# Field written two ways

One of your own fields is written with a dot in its name on one page, such
as `book.author:`, and nested on another, as `book:` with `author:` beneath
it. To YAML these are two different fields.

## What you'll see {#what-youll-see}

```text
warning  Field written two ways: `book.author` is also nested under `book`
  Nest it under `book`, so templates read it as `page.book.author`.
  More: https://statac.dev/e/field-written-two-ways/
```

## Why Statac warns

A template asking for `page.book.author` finds the nested one and misses
the other, so some pages would show nothing there. Statac keeps both as you
wrote them and tells you, showing where the nested one is.

## How to fix it

Nest the field everywhere, so templates read it one way.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
book.author: Author 1
---
```

In `content/post-2.md`, the same field is nested. Nest it in `post-1.md`
too:

```yaml
---
title: Post 1
book:
  author: Author 1
---
```
