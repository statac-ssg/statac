# Missing template

A page needs a template that isn't in `templates/`. The message gives the
name of the file Statac looked for. Either `layout:` or `extends` named
the template, or the page is a list page Statac makes itself, such as a
tag's page.

Statac builds a page with the first of these that applies:

1. the template named by the page's own `layout:`, such as `layout: post`;
2. the template named by its `type:`, if `templates/` has one;
3. a `layout:` cascaded from its folder or `statac.yaml`, such as
   `pages.layout: post`, unless the page is a list page;
4. `home` for the home page, `folder` for any other list page, and `page`
   for the rest.

A page with content of its own that names no template doesn't need the
one Statac chose for it. If a home page or list page has no template of
its own, Statac builds it with `page`, and the `built-with-page-template`
note says so. If the site has no template for a page at all, Statac
publishes it as its content alone, and the `built-without-template` note
says so.

A template's `{% extends "base" %}` refers to another template in the same
way. Each name is the file's name in `templates/`, with or without
`.html`: `base` and `base.html` are the same template.

## What you'll see {#what-youll-see}

```text
error  Missing template: there is no `templates/psot.html`
  content/posts/index.md, line 2
   2 │ pages.layout: psot
     │               ^^^^
  Did you mean `post`?
  More: https://statac.dev/e/missing-template/
```

## Why Statac stops

A template named by `layout:` or `extends` is the one you chose, so Statac
doesn't build the page another way. A list page Statac makes itself has no
content of its own, so without its template there is nothing to publish.

## How to fix it

Add the template to `templates/`, or correct the name. If there is a
template with a similar name, Statac suggests it. To build a page with a
different template, set `layout:` on the page, or `pages.layout:` in its
folder's `index.md`.

A list page Statac makes itself, such as a tag's page, has no front
matter where you could write `layout:`, and a cascaded `layout:` doesn't
reach a list page. Add `templates/folder.html` to build these pages, or
`templates/home.html` for the home page. To build one page your own way,
write a page of your own at its URL, such as `content/tags/rust.md` for
the tag `rust`, and Statac builds that page in its place.

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
