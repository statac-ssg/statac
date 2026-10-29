# Wrong kind of value

A field or setting has a different kind of value from the one it expects:
words where it expects `true` or `false`, a list where it expects text, a
date written another way, and so on.

A list in a language file has a set number of words: 12 months, starting
with January, or 7 weekdays, starting with Monday.

`each:` takes `site.posts` or a table in `data/`. A table is a list of
items, or items by name, and each item is a set of `name: value` lines.

On a card, `padding` is a number from 0 to 314. A text's size is a number
from 1 to 630, or `up to` such a number. Its colour is `#RGB`, `#RRGGBB`
or a template. A font is a file ending in `.ttf`, `.otf` or `.ttc`.

The message says what kind of value is expected.

## What you'll see {#what-youll-see}

```text
error  Wrong kind of value: `draft` is `true` or `false`, not `yes`
  content/post-1.md, line 3
   3 │ draft: yes
     │        ^^^
  Write it as `true`.
  More: https://statac.dev/e/wrong-kind-of-value/
```

## Why Statac stops

Statac can't tell what you meant by the value, and guessing could publish
a page that isn't the one you wrote. If another way of writing it has one
clear meaning, such as `yes` for `true` or `13/04/2026` for a date, Statac
shows how to write it.

## How to fix it

Write the kind of value the message asks for. Write dates as year, month
and day, such as `2026-10-04`, with a time after them if you want one.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
draft: yes
date: 13/04/2026
---
```

Written so that Statac can read them:

```yaml
---
title: Post 1
draft: true
date: 2026-04-13
---
```
