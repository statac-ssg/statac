# Untitled page

A template asked for the name of a page, with `headline(page)` or as the
`title` of a redirect, and the page has none written: no `title`, and no
`# heading` at the top of its writing. Statac called it by its `summary`,
its opening words, its date or its address, whichever came first.

## What you'll see {#what-youll-see}

```text
note  Untitled page: `content/post-1.md` is called by its opening words, `Post 1 opens…`
  Give it a `title`, or open its writing with a `# heading`. A folder whose pages go without titles says `pages.untitled: true`.
  More: https://statac.dev/e/untitled-page/
```

## Why Statac says so

A feed's entry and a browser's tab need words to show, so a page with no
name is called by the first thing there is to call it by, and Statac
never makes up a name such as "Untitled". The home page is called by the
site's `title`, and a page speaking for a list, such as a folder or a
tag, by the list's name, so neither is mentioned here. The others are
none of them a name you wrote, so Statac says once for each page which
one it used. Once you have understood it, it can be quietened.

## How to fix it

If the page is meant to go without a name, nothing is wrong. Where every
page in a folder is meant to, as short notes are, say so in the folder's
`index.md`:

```yaml
pages.untitled: true
```

Statac then calls each of them by its date, and says nothing of them. One
page says the same for itself with `untitled: true`.

To name it, give it a `title` in its front matter, or open its writing
with a `# heading`, which is its title where it has no `title` of its own.

For the home page of a site with no `title` and no `content/index.md`,
add `title:` to `statac.yaml`.

## Example

`content/post-1.md` has a date and a paragraph, and nothing else:

```markdown
---
date: 2026-10-04
---
Post 1 opens with this paragraph.
```

A feed calls it `Post 1 opens with this paragraph.` To name it:

```markdown
---
title: Post 1
date: 2026-10-04
---
Post 1 opens with this paragraph.
```
