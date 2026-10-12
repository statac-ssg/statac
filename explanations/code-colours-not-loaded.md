# Code colours not loaded

A page shows a block of code that Statac coloured, but nothing the site
builds asks for `code.css`, the stylesheet that contains the colours. The
code is published marked up for colour, but with no colours defined, so
readers see it all in one colour.

Statac reports this once for the whole site, at the first coloured block
on the first page that has one.

## What you'll see {#what-youll-see}

```text
warning  Code colours not loaded: no template asks for `code.css`
  Add `{{ css("code.css") }}` inside a template's `<head>`.
  More: https://statac.dev/e/code-colours-not-loaded/
```

## Why Statac warns

Statac colours code by giving each piece of it a class for the kind of
thing it is, such as `hl-keyword`, and writes the colour of each kind
into `code.css`. A page shows the colours only if it loads that
stylesheet, which a template does with `css("code.css")`. If no template
does, the stylesheet is published but no page uses it. Nothing on the
page looks broken, so this is easy to miss.

Statac decides from what the build actually read. If anything it built
asked for `code.css`, with `css` or with `asset`, on any page, it says
nothing. The same check applies if you have an `assets/code.css` of your
own.

## How to fix it

Ask for the stylesheet inside the `<head>` of your layout (the template
your other templates extend), so that every page loads it.

If the colours reach your pages in a way Statac can't see, such as rules
copied into another stylesheet, or if you want code in one colour,
quieten this warning with `quiet: [code-colours-not-loaded]` in
`statac.yaml`. To show a single block without colour, give `text` as its
language, or give no language.

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
