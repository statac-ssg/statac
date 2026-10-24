# Missing template

A page needs a template that isn't in `templates/`. The message gives the
name of the file Statac looked for.

Statac builds a page with the first of these that applies:

1. the template named by the page's own `layout:`, such as `layout: post`;
2. the template named by its `type:`, if `templates/` has one;
3. a `layout:` cascaded from its folder or `statac.yaml`, such as
   `pages.layout: post`, unless the page is a list page;
4. `home` for the home page, `folder` for any other list page, and `page`
   for the rest.

A template's `{% extends "base" %}` refers to another template in the same
way. Each name is the file's name in `templates/`, with or without
`.html`: `base` and `base.html` are the same template.

## What you'll see {#what-youll-see}

```text
error  Missing template: there is no `templates/folder.html`
  Add the template, or choose another with `layout:`.
  More: https://statac.dev/e/missing-template/
```

## Why Statac stops

With no template, there is nothing to build the page with.

## How to fix it

Add the template to `templates/`, or correct the name. If there is a
template with a similar name, Statac suggests it. To build a page with a
different template, set `layout:` on the page, or `pages.layout:` in its
folder's `index.md`.

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
