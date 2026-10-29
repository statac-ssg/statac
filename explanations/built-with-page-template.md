# Built with page template

A home page or list page has content of its own, and the site has no
template for that kind of page, so Statac built it with
`templates/page.html`. The message names the template it looked for,
says how many pages were built this way, and lists them.

## What you'll see {#what-youll-see}

```text
note  Built with page template: `templates/page.html` builds 1 page in place of `templates/home.html`
  While building: content/index.md
  Add `templates/home.html` to give it a template of its own.
  More: https://statac.dev/e/built-with-page-template/
```

## Why Statac says so

A page that names no template with `layout:` is built with
`templates/home.html` if it is the site's home page, with
`templates/folder.html` if it is any other list page, such as a folder's
`index.md`, and with `templates/page.html` otherwise. A small site often
needs only `templates/page.html`, so Statac builds a home page or list
page with content of its own with it if the site lacks the template for
its kind. If the site has no `templates/page.html` either, Statac
publishes the page as its content alone, and the `built-without-template`
note says so.

A list page Statac makes itself, such as a tag's page or the home page of
a site with no `content/index.md`, has no content of its own, so it still
needs its own template, and the build stops with `missing-template`
without it.

Statac says so once for each template it looked for, so you know which
template built the pages. Once you have understood the note, you can
quieten it.

## How to fix it

If `templates/page.html` suits these pages, nothing is wrong. Quieten the
note with `quiet: [built-with-page-template]` in `statac.yaml`.

To build them another way, add the template the message names. To build
one page with another template, give it `layout:`.

## Example

A site has `templates/page.html` and `content/index.md`, and no
`templates/home.html`, so the home page is built with
`templates/page.html`. To list the site's posts below its content, add
`templates/home.html`:

```html
<!doctype html>
<html lang="{{ page.language }}">
<meta charset="utf-8">
<title>{{ page.title }}</title>
{{ page.content }}
<ul>
{% for post in page.posts %}
  <li><a href="{{ post.url }}">{{ post.title }}</a></li>
{% endfor %}
</ul>
</html>
```
