# One tag, not two

A page's `tags` are written on one line with commas between them and no
brackets around them, as `tags: hills, water`. To YAML that is one piece
of text, so the page has one tag, called `hills, water`. The same goes for
`pages.tags` handed down, and for `tags` on an item of a table.

## What you'll see {#what-youll-see}

```text
warning  One tag, not two: `tags: hills, water` makes a single tag called `hills, water`
  For two tags, write `tags: [hills, water]`.
  More: https://statac.dev/e/one-tag-not-two/
```

## Why Statac warns

Statac reads what YAML reads, and makes the one tag, with its own page and
its own place in the list of tags. That is rarely what was meant, and
nothing on the page shows it until someone looks for the tag. Text in
quotes, as `tags: "hills, water"`, is plainly meant as one tag and is
never told.

## How to fix it

For several tags, put brackets around them, as the message shows, or
write each on a line of its own beginning `- `. For one tag with a comma
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
