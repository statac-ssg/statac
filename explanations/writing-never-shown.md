# Writing never shown

A layout builds pages that have writing and never asks for it, so each of
those pages is published without its writing. The message names the
layout, says how many pages it builds that way, and lists them.

A page's writing is everything below its front matter. A layout puts it on
the page with `{{ page.content }}`.

## What you'll see {#what-youll-see}

```text
warning  Writing never shown: 3 pages built by `templates/hollow.html` have writing it never shows
  Add `{{ page.content }}` where the writing belongs.
  More: https://statac.dev/e/writing-never-shown/
```

## Why Statac warns

The pages are built as the layout says, and a layout that leaves the
writing out may be what you meant: a page of links, say, or a home page
that only lists posts. Statac can't tell that from a layout whose
`{{ page.content }}` was forgotten, and what you wrote would be missing
from the site without a word, so it says so once for each layout.

A layout shows the writing where it asks for `page.content` or
`page.excerpt`, itself or in a template it extends or a snippet it calls.
What only describes the writing, such as `page.opening`, `page.words` or
`page.toc`, puts none of it on the page and doesn't count. A page with
front matter and nothing below it, or only HTML comments, has no writing
to show and is never counted.

## How to fix it

Where the writing belongs on the page, add `{{ page.content }}` to the
layout.

Where it doesn't, take the writing out of the pages the message lists, or
keep it and quieten the warning with `quiet: [writing-never-shown]` in
`statac.yaml`.

## Example

`templates/hollow.html` shows a title and nothing else:

```html
<h1>{{ page.title }}</h1>
```

`content/post-1.md` is built by it:

```markdown
---
title: Post 1
layout: hollow
---
Post 1 says this.
```

The page is published with its title and without `Post 1 says this.` To
show it:

```html
<h1>{{ page.title }}</h1>
{{ page.content }}
```
