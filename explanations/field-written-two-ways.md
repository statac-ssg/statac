# Field written two ways

A custom field is written with a dot in its name on one page, such as
`book.author:`, and nested on another: `book:` with `author:` under it.
YAML reads these as two different fields.

## What you'll see {#what-youll-see}

```text
warning  Field written two ways: `book.author` is also nested under `book`
  content/post-1.md, line 3
   3 │ book.author: Author 1
     │ ^^^^^^^^^^^
  Nested here: content/post-2.md, line 4
   4 │   author: Author 2
     │   ^^^^^^
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
