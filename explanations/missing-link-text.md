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

A screen reader names each link by the words inside it. One with only an
icon inside it, or an image with no alt text, is read out as "link" and
nothing more, or as its URL, and a reader can't tell where it leads.
That is part of the page Statac built, so Statac tells you. It is a note,
and nothing is left out of the site.

Statac reads each page once it is built. It reports a link written in a
template or snippet once, at its line, with the pages it is on, and one
written in the content at its place in the content. A screen reader
never reads out what is inside an element marked `aria-hidden="true"`,
so that can't give a link a name, and Statac never reports a link inside
such an element. Nor does it report one inside a `<template>`, which is
on no page until a script puts it there. Statac reports a `<button>`
with no words as `missing-button-text`. If a link in the content shows
only an image that Statac already reports as `missing-alt-text`, Statac
reports it once, there.

## How to fix it

Put words inside the link that say where it leads. If it shows only an
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

To stop Statac showing the note, add this to `statac.yaml`:

```yaml
quiet: [missing-link-text]
```
