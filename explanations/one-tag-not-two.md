# One tag, not two

A page's `tags` are written on one line with commas between them and no
brackets around them, such as `tags: hills, water`. To YAML, that is a single
piece of text, so the page has one tag, called `hills, water`. The same
applies to `pages.tags` in a cascade, and to `tags` on an item in a table.

## What you'll see {#what-youll-see}

```text
warning  One tag, not two: `tags: hills, water` makes a single tag called `hills, water`
  content/post-1.md, line 3
   3 │ tags: hills, water
     │       ^^^^^^^^^^^^
  For two tags, write `tags: [hills, water]`.
  More: https://statac.dev/e/one-tag-not-two/
```

## Why Statac warns

Statac reads the line the same way YAML does, and makes the single tag,
with its own place in the list of tags, and its own page where the site
has tag pages. That is rarely
what was meant, and nothing on the page shows the mistake until someone
looks for the tag. Text in quotes, such as `tags: "hills, water"`, is
clearly meant as one tag, so Statac never warns about it.

## How to fix it

For several tags, put brackets around them, as the message shows, or
write each tag on its own line, starting with `- `. For one tag with a comma
in its name, put it in quotes.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
tags: hills, water
---
```

Two tags:

```yaml
---
title: Post 1
tags: [hills, water]
---
```
