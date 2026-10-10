# Unordered pages

A list's page gives its order as a list of names, `order: [day-one,
day-two]`, and the list has pages that aren't named there.

## What you'll see {#what-youll-see}

```text
warning  Unordered pages: `order` leaves out `day-four` and `day-five`
  They follow in their usual order; name them in `order` to place them.
  More: https://statac.dev/e/unordered-pages/
```

## Why Statac warns

The pages it names come first, in the order written. The pages it leaves
out follow them, in the order they would have had: newest first for posts,
and A to Z by name for pages without a date. Statac goes on, and tells you,
because a page left out of the order was probably added after it was
written.

## How to fix it

Add each page the warning names to the `order:` list, where it belongs. A
page is named by the last part of its address, so `content/days/day-four.md`
is `day-four`.

## Example

`content/days/index.md`:

```yaml
---
title: Days
order: [day-one, day-two]
---
```

with `day-three.md` in the folder too. Name it:

```yaml
order: [day-one, day-two, day-three]
```
