# Field written two ways

On one page, one of your own fields is written with a dot in its name, such
as `book.author:`. On another page it is nested, as `book:` with `author:`
beneath it. YAML reads these as two different fields.

## What you'll see {#what-youll-see}

```text
warning  Field written two ways: `book.author` is also nested under `book`
  Nest it under `book`, so templates read it as `page.book.author`.
  More: https://statac.dev/e/field-written-two-ways/
```

## Why Statac warns

A template that uses `page.book.author` finds the nested field and misses
the dotted one, so on some pages it would show nothing. Statac keeps both
fields as you wrote them, and the warning shows where the nested one is.

## How to fix it

Nest the field on every page, so templates read it the same way
everywhere.

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
