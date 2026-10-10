# Nested setting

One of Statac's settings is written nested under its first part, such as
`layout:` indented under `pages:`, or a text on a card is written as
`title:` with `text:` beneath it. Statac's own settings have one
spelling, written flat on one line: `pages.layout: post`, `title.text:`.

## What you'll see {#what-youll-see}

```text
error  Nested setting: `layout` under `pages`
  Write it flat: `pages.layout: post`.
  More: https://statac.dev/e/nested-setting/
```

## Why Statac stops

Your own fields may nest, and reach templates the same way. Statac's
settings keep the one flat spelling, so every site writes them alike and
anyone reading a site's files learns them once.

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
