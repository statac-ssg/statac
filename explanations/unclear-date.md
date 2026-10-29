# Unclear date

A date is written with slashes or dots, so it could be read two ways:
`3/4/2026` is 3 April in some places and 4 March in others.

## What you'll see {#what-youll-see}

```text
error  Unclear date: `3/4/2026` could be 3 April or 4 March
  content/post-1.md, line 3
   3 │ date: 3/4/2026
     │       ^^^^^^^^
  Write it as `2026-04-03` or `2026-03-04`.
  More: https://statac.dev/e/unclear-date/
```

## Why Statac stops

Statac can't know which you meant, and either guess could put a post on
the wrong day.

## How to fix it

Write the date year first, such as `2026-04-03`. The message gives both
readings in that form, so you can choose one.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
date: 3/4/2026
---
```

For 3 April:

```yaml
---
title: Post 1
date: 2026-04-03
---
```
