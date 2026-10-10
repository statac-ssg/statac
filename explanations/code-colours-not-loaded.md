# Code colours not loaded

A page shows a block of code Statac coloured, and nothing the site builds
asks for `code.css`, the stylesheet that holds the colours. The code is
published marked for colour with no colours to show it in, so readers see
it in one colour.

Statac tells this once for the site, at the first coloured block of the
first page that shows one.

## What you'll see {#what-youll-see}

```text
warning  Code colours not loaded: no template asks for `code.css`
  Add `{{ css("code.css") }}` inside a template's `<head>`.
  More: https://statac.dev/e/code-colours-not-loaded/
```

## Why Statac warns

Statac colours code by giving each piece of it a class, such as `hl-4k`,
and writes what each class looks like into `code.css`. A page shows the
colours only where it loads that stylesheet, which a template does with
`css("code.css")`. Where no template does, the stylesheet is published
and no page uses it. Nothing on the page looks broken, so it is easy to
miss.

Statac goes by what the build read. Where anything it built asked for
`code.css`, with `css` or with `asset`, on any page, it says nothing. An
`assets/code.css` of your own is held to the same.

## How to fix it

Ask for the stylesheet inside the `<head>` of your layout, the template
your other templates extend, so every page loads it.

Where the colours reach your pages some way Statac can't see, such as
rules copied into another stylesheet, or you want code in one colour,
quieten this warning with `quiet: [code-colours-not-loaded]` in
`statac.yaml`. To show one block without colour, write `text` as its
language, or none.

## Example

In `content/post-1.md`:

````markdown
```yaml
title: Post 1
```
````

In `templates/base.html`:

```html
<head>
  <title>{{ page.title }}</title>
  {{ css("code.css") }}
</head>
```
