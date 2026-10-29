# Missing title

A page built with a template has no `<title>`, or its `<title>` is empty.

## What you'll see {#what-youll-see}

```text
note  Missing title: the page has no `<title>`
  templates/page.html, line 3
   3 │ <head>
     │ ^^^^^
  While building: content/index.md
  Add a `<title>` naming the page, such as `<title>{{ page.title }}</title>`.
  More: https://statac.dev/e/missing-title/
```

## Why Statac says so

A page's `<title>` is what its browser tab says, what a bookmark and a
search result are called, and the first thing a screen reader reads out
when the page opens. With none, a reader hears the page's URL, or
nothing, and can't tell which page they are on. That is part of the page
Statac built, so Statac tells you. It is a note, and nothing is left out
of the site.

Statac reads each page once it is built. If the `<title>` is missing,
Statac reports it once, at the `<head>` the template writes, or at the
top of the page's layout if there is no `<head>`, with the pages it is
missing from. It reports an empty one at the `<title>` itself. A
`<title>` inside an `<svg>` names the image, not the page. A page built
without a template is published as its content alone, with no `<head>`,
so Statac never reports it.

## How to fix it

Add a `<title>` to the `<head>` of the template the page is built with,
or of the template it extends, naming the page. If it is there but
empty on some pages, give those pages a `title`.

Once you have understood the note, you can quieten it by name.

## Example

In `templates/page.html`:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
</head>
```

Add a `<title>`, and the note goes away:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>{{ page.title }}</title>
</head>
```

To stop Statac showing the note, add this to `statac.yaml`:

```yaml
quiet: [missing-title]
```
