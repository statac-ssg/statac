# Nested setting

One of Statac's settings is written nested under its first part, such as
`layout:` indented under `pages:`, or a text on a card is written as
`title:` with `text:` under it. Statac's own settings have only one
spelling, written flat on one line: `pages.layout: post`, `title.text:`.

## What you'll see {#what-youll-see}

```text
error  Nested setting: `layout` under `pages`
  content/notes/index.md, line 4
   4 │   layout: note
     │   ^^^^^^
  Write it flat: `pages.layout: note`.
  More: https://statac.dev/e/nested-setting/
```

## Why Statac stops

Custom fields can be nested, and templates reach them through that
nesting. Statac's settings keep their one flat spelling, so every site
writes them the same way, and anyone reading a site's files only has to
learn them once.

## How to fix it

Write the setting on one line, as the message shows.

## Example

In `content/notes/index.md`:

```yaml
---
title: Notes
pages:
  layout: note
---
```

Written flat:

```yaml
---
title: Notes
pages.layout: note
---
```
