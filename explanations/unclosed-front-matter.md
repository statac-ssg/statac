# Unclosed front matter

A Markdown file starts with a `---` line, which opens front matter, but no
later `---` line closes it.

## What you'll see {#what-youll-see}

```text
error  Unclosed front matter: no `---` line ends it
  Add a `---` line after the last field.
  More: https://statac.dev/e/unclosed-front-matter/
```

## Why Statac stops

Without the closing line, Statac can't tell where the fields end and the
writing begins. Reading the whole file as fields would lose the writing,
and reading it all as writing would lose the fields.

## How to fix it

Add a `---` line on its own after the last field.

## Example

In `content/post-1.md`:

```markdown
---
title: Post 1
date: 2026-10-04

The first paragraph.
```

Close the front matter:

```markdown
---
title: Post 1
date: 2026-10-04
---

The first paragraph.
```
