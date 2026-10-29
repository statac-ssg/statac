# Not stamped

A value in a page's front matter is a stamp, such as `=today` or
`=monday 09:00`. A stamp is one of Statac's words that stands for a date,
and is there until the real date is written in its place.

## What you'll see {#what-youll-see}

```text
error  Not stamped: `date` is still `=monday 09:00`
  content/post-1.md, line 3
   3 │ date: =monday 09:00
     │       ^^^^^^^^^^^^^
  Run `statac stamp` to write the date in its place, or `statac serve`, which writes it on every save.
  More: https://statac.dev/e/not-stamped/
```

## Why Statac stops

A stamp means a different day depending on when Statac reads it, but a
build of the same files is always the same. So `statac build` and
`statac check` never choose the date. Statac can't publish a page with
the date `=monday` either. Instead, `statac stamp` or `statac serve` writes
the date into the file once, and from then on the file itself says which
day it is.

## How to fix it

Run `statac stamp` in the site's folder. It replaces every
stamp in the site with its date, and tells you what it wrote.
`statac serve` does the same when it starts and on every save, so a stamp
never stays in a site being served for long.

To publish a stamp's word itself, write the value as a block. Statac
always reads a block as text:

```yaml
title: |-
  =today
```

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
date: =monday 09:00
---
```

After running `statac stamp` on Tuesday 6 October 2026:

```yaml
---
title: Post 1
date: 2026-10-12 09:00
---
```
