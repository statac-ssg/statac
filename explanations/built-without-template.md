# Built without template

The site has no template for some of its pages, so Statac published each
of them as its content alone. The message says how many pages were
published this way, and lists them. Redirects published without
`templates/redirect.html` are counted in a message of their own.

## What you'll see {#what-youll-see}

```text
note  Built without template: Statac publishes 2 pages as their content alone
  Add `templates/page.html` to build them with a template.
  More: https://statac.dev/e/built-without-template/
```

## Why Statac says so

A page that names no template with `layout:` is built with
`templates/home.html` if it is the site's home page, with
`templates/folder.html` if it is any other list page, and with
`templates/page.html` otherwise. A home page or list page with content of
its own falls back to `templates/page.html`. If the site has no
`templates/page.html`, nothing is left to build the page with.

Statac doesn't make up a template in its place. It publishes the page as
the HTML its content becomes, and nothing else: no doctype, no `<head>`,
no title and no stylesheet. That isn't a whole HTML document, but browsers
show it. Statac checks the links in it as it does on any other page.

If there is no `templates/redirect.html`, Statac publishes a redirect as a
refresh to the URL it redirects to and a link to that URL, and nothing
else:

```html
<meta http-equiv="refresh" content="0; url=/post-1/">
<a href="/post-1/">/post-1/</a>
```

A list page Statac makes itself, such as a tag's page or the home page of
a site with no `content/index.md`, has no content to publish, so it still
needs its template, and the build stops with `missing-template` without
it. So does a page that names its template with `layout:`.

Statac says once how many pages it published this way, so you know they
have no template. Once you have understood the note, you can quieten it.

## How to fix it

If the pages are meant to be their content alone, nothing is wrong.
Quieten the note with `quiet: [built-without-template]` in `statac.yaml`.

To build them with a template, add `templates/page.html`. It builds every
page that names no other template, from then on. For the redirects, add
`templates/redirect.html`. Statac hands it `to`, the URL the page
redirects to, and `title`, what the browser's tab shows meanwhile.

## Example

A site has `content/index.md` and `content/hello.md`, and no templates.
`content/hello.md` is:

```markdown
# Hello

The first page.
```

Statac publishes it as `hello/index.html`, which contains:

```html
<h1 id="hello">Hello</h1>
<p>The first page.</p>
```

To publish it as a whole HTML document, add `templates/page.html`:

```html
<!doctype html>
<html lang="{{ page.language }}">
<meta charset="utf-8">
<title>{{ page.title }}</title>
{{ page.content }}
</html>
```
