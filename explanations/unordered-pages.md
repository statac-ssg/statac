# Unordered pages

A list page sets its order as a list of names, `order: [day-one,
day-two]`, but the list has pages that aren't named there.

## What you'll see {#what-youll-see}

```text
warning  Unordered pages: `order` leaves out `day-four` and `day-five`
  Statac puts them after the named pages, in their usual order; name them in `order` to place them.
  More: https://statac.dev/e/unordered-pages/
```

## Why Statac warns

The pages named in `order` come first, in the order written. The pages
left out follow them, in the order they would otherwise have had: newest
first for posts, and A to Z by name for pages without a date. Statac
carries on, but warns you, because a page left out of the order was
probably added after the order was written.

## How to fix it

Add each page the warning lists to the `order:` list, in the place where
it belongs. A page is named by the last part of its URL, so
`content/days/day-four.md` is `day-four`.

## Example

In `content/days/index.md`:

```yaml
---
title: Days
order: [day-one, day-two]
---
```

The folder also has `day-three.md`. Add it to the list:

```yaml
order: [day-one, day-two, day-three]
```
