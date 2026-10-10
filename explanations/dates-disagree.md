# Dates disagree

A page's file name opens with a date, such as `2026-10-04-post-1.md`, and
its `date:` line gives another day.

## What you'll see {#what-youll-see}

```text
warning  Dates disagree: `date` is `2026-10-05`, the file name `2026-10-04`
  Change one so they match; Statac uses the `date` line.
  More: https://statac.dev/e/dates-disagree/
```

## Why Statac warns

The `date:` line is written on the page itself, so Statac uses it, and the
build goes on. It tells you, because one of the two is probably a mistake,
and the address of a post often holds its date.

## How to fix it

Change the file name or the `date:` line so they give the same day. If the
file name's date is right, you can remove the `date:` line: a date at the
start of the name dates the page.

## Example

`content/2026-10-04-post-1.md`:

```yaml
---
title: Post 1
date: 2026-10-05
---
```

If the post was written on 4 October:

```yaml
---
title: Post 1
date: 2026-10-04
---
```
