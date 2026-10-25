# Missing button text

A `<button>` on a built page has nothing a screen reader can read out as
its name: no words inside it, no image inside it with alt text, and no
`aria-label`, `aria-labelledby` or `title` on it.

## What you'll see {#what-youll-see}

```text
note  Missing button text: a `<button>` has nothing a screen reader can read out
  Give the `<img>` inside it an `alt` saying what the button does.
  More: https://statac.dev/e/missing-button-text/
```

## Why Statac says so

A screen reader names each button by the words it holds. One that holds
only an icon, or an image with no alt text, is read out as "button" and
nothing more, and a reader can't tell what it does. That is part of the
page Statac built, so Statac tells you. It is a note, and nothing is left
out of the site.

Statac reads each page as it is published. A button written in a template
or snippet is told once, at its line, with the pages it is on. One
written in the content is told in the content. What is inside an element
marked `aria-hidden="true"` is never read out, so it gives a button no
name, and a button inside one is never told. Nor is one inside a
`<template>`, which is on no page until a script puts it there. An
`<input type="button">` or `<input type="image">` with no words is told
as `missing-label`. A button that shows only an image Statac already
tells as `missing-alt-text` is told once, there: an image in the content,
or one a template shows with `page.image` where its `<img>` is written
with the image's `alt`.

## How to fix it

Put words inside the button that say what it does. Where it shows only an
icon, give the icon's `<img>` an `alt`, or give the button an
`aria-label`.

Once you have understood the note, you can quieten it by name.

## Example

In `templates/page.html`, a button shows only an icon:

```html
<button class="menu"><img src="/menu.svg" alt=""></button>
```

Give the image an `alt` saying what the button does, and the note goes
away:

```html
<button class="menu"><img src="/menu.svg" alt="Menu"></button>
```

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [missing-button-text]
```
