# Content never shown

A layout builds pages that have content, but never puts the content on
the page, so each of those pages is published without it. The message shows
the layout and how many pages it builds this way, and lists those pages.

A page's content is everything below its front matter. A layout puts it on
the page with `{{ page.content }}`.

## What you'll see {#what-youll-see}

```text
warning  Content never shown: 1 page built by `templates/hollow.html` has content it never shows
  templates/hollow.html, line 1
   1 │ <h1>{{ page.title }}</h1>
     │ ^^^^^^^^^^^^^^^^^^^^^^^^^
  While building: content/post-1.md
  Add `{{ page.content }}` where the content belongs.
  More: https://statac.dev/e/content-never-shown/
```

## Why Statac warns

Statac builds the pages as the layout says, and a layout that leaves out
the content may be what you meant, for a page of links or a home page that
only lists posts. Statac can't tell that apart from a layout where you
forgot `{{ page.content }}`. In that case, what you wrote would be missing
from the site with no message, so Statac warns once for each layout.

A layout counts as showing the content if it uses `page.content` or
`page.excerpt`, either directly or in a template it extends or a snippet
it calls. Values that only describe the content, such as `page.opening`,
`page.word_count` or `page.toc`, don't put any of it on the page, so they
don't count. A page with front matter and nothing below it, or only HTML
comments, has no content to show, so it is never counted.

## How to fix it

If the content belongs on the page, add `{{ page.content }}` to the
layout.

If it doesn't, take the content out of the pages the message lists, or
keep it and quieten the warning with `quiet: [content-never-shown]` in
`statac.yaml`.

## Example

`templates/hollow.html` shows a title and nothing else:

```html
<h1>{{ page.title }}</h1>
```

`content/post-1.md` is built with it:

```markdown
---
title: Post 1
layout: hollow
---
Post 1 says this.
```

The page is published with its title but without `Post 1 says this.` To
show the content:

```html
<h1>{{ page.title }}</h1>
{{ page.content }}
```
