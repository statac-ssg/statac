# Wrong kind of value

A field or setting holds a different kind of value from the one it takes:
words where it takes `true` or `false`, a list where it takes text, a date
written another way, and so on. A list in a language file holds a set
number of words: 12 months, January first, or 7 weekdays, Monday first.
`each:` takes `site.posts`, or a table in `data/`: a list of items, or
items by name, each item `name: value` lines. On a card, `padding` is a
number from 0 to 314, a text's size is a number from 1 to 630 or `up to`
one, its colour is `#RGB` or `#RRGGBB` or a template, and a font is a
file ending `.ttf`, `.otf` or `.ttc`. The message says what it takes.

## What you'll see {#what-youll-see}

```text
error  Wrong kind of value: `draft` is `true` or `false`, not `perhaps`
  More: https://statac.dev/e/wrong-kind-of-value/
```

## Why Statac stops

Statac can't tell what you meant by the value, and guessing could publish
a page that isn't the one you wrote. Where another way of writing has one
clear meaning, such as `yes` for `true` or `13/04/2026` for a date, Statac
shows how to write it.

## How to fix it

Write the kind of value the message names. Dates are written year, month
and day, as `2026-10-04`, with a time after them if you want one.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
draft: yes
date: 13/04/2026
---
```

Written as Statac reads them:

```yaml
---
title: Post 1
draft: true
date: 2026-04-13
---
```
