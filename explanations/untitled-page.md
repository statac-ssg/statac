# Untitled page

A template asked for a page's name, either with `headline(page)` or as the
`title` of a redirect, but the page has no name written: it has no `title`,
and no `# heading` at the top of its content. Statac called it by the
first of these that it has: its `summary`, its opening words, its date or
its URL.

## What you'll see {#what-youll-see}

```text
note  Untitled page: `content/post-1.md` is called by its opening words, `Post 1 opens…`
  Give it a `title`, or open its content with a `# heading`. A folder whose pages go without titles says `pages.untitled: true`.
  More: https://statac.dev/e/untitled-page/
```

## Why Statac says so

A feed entry and a browser tab need some words to show, so a page with no
name is called by the first of these that it has. Statac never makes up a
name such as "Untitled". The home page is called by the site's `title`,
and a list page, such as a folder's or a tag's, is called by the list's
name, so Statac doesn't show this note for either of them. None
of the other choices is a name you wrote, so Statac says once for each
page which one it used. Once you have understood the note, you can quieten
it.

## How to fix it

If the page is meant to have no name, nothing is wrong. If every page in a
folder is meant to have no name, as short notes often are, say so in the
folder's `index.md`:

```yaml
pages.untitled: true
```

Statac then calls each of those pages by its date, and doesn't show the
note for them. To do the same for a single page, give it `untitled: true`.

To name the page, give it a `title` in its front matter, or open its
content with a `# heading`. If a page has no `title` of its own, that
heading is its title.

If the note is about the home page of a site with no `title` and no
`content/index.md`, add `title:` to `statac.yaml`.

## Example

`content/post-1.md` has a date and a paragraph, and nothing else:

```markdown
---
date: 2026-10-04
---
Post 1 opens with this paragraph.
```

In a feed, it is called `Post 1 opens with this paragraph.` To name it:

```markdown
---
title: Post 1
date: 2026-10-04
---
Post 1 opens with this paragraph.
```
