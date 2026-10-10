# Missing template

A page needs a template that `templates/` doesn't have. The message names
the file Statac looked for.

A page is built with the first of these:

1. the template its own `layout:` names, as `layout: post`;
2. the template its `type:` names, where `templates/` has one;
3. a `layout:` handed down from its folder or `statac.yaml`, as
   `pages.layout: post`, unless the page speaks for a list;
4. `home` for the home page, `folder` for any other page that speaks for
   a list, and `page` for the rest.

A template's `{% extends "base" %}` names another the same way. Each name
is the file's name in `templates/` without `.html`.

## What you'll see {#what-youll-see}

```text
error  Missing template: there is no `templates/folder.html`
  Add the template, or choose another with `layout:`.
  More: https://statac.dev/e/missing-template/
```

## Why Statac stops

With no template, there is nothing to build the page with.

## How to fix it

Add the template to `templates/`, or correct the name. When a template with
a similar name is there, Statac offers it. To build a page with another
template, write `layout:` on the page, or `pages.layout:` on its folder's
`index.md`.

## Example

In `content/posts/index.md`:

```yaml
---
pages.layout: psot
---
```

If the file is `templates/post.html`, write:

```yaml
---
pages.layout: post
---
```
