# Missing link text

A link on a built page has nothing a screen reader can read out as its
name: no words inside it, no image inside it with alt text, and no
`aria-label`, `aria-labelledby` or `title` on it.

## What you'll see {#what-youll-see}

```text
note  Missing link text: the link to `https://example.org/` has nothing a screen reader can read out
  Give the `<img>` inside it an `alt` saying where the link leads.
  More: https://statac.dev/e/missing-link-text/
```

## Why Statac says so

A screen reader names each link by the words it holds. One that holds
only an icon, or an image with no alt text, is read out as "link" and
nothing more, or as its address, and a reader can't tell where it leads.
That is part of the page Statac built, so Statac tells you. It is a note,
and nothing is left out of the site.

Statac reads each page as it is published. A link written in a template
or snippet is told once, at its line, with the pages it is on. One
written in the content is told in the content. What is inside an element
marked `aria-hidden="true"` is never read out, so it gives a link no
name, and a link inside one is never told. Nor is one inside a
`<template>`, which is on no page until a script puts it there. A
`<button>` with no words is told as `missing-button-text`.

## How to fix it

Put words inside the link that say where it leads. Where it shows only an
icon, give the icon's `<img>` an `alt`, or give the link an `aria-label`.

Once you have understood the note, you can quieten it by name.

## Example

In `templates/page.html`, a link shows only an icon:

```html
<a href="https://example.org/"><img src="/icon.svg" alt=""></a>
```

Give the image an `alt` saying where the link leads, and the note goes
away:

```html
<a href="https://example.org/"><img src="/icon.svg" alt="Example"></a>
```

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [missing-link-text]
```
