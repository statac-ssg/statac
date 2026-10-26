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

A screen reader names each button by the words inside it. One with only
an icon inside it, or an image with no alt text, is read out as "button" and
nothing more, and a reader can't tell what it does. That is part of the
page Statac built, so Statac tells you. It is a note, and nothing is left
out of the site.

Statac reads each page once it is built. It reports a button written in
a template or snippet once, at its line, with the pages it is on, and
one written in the content at its place in the content. A screen reader
never reads out what is inside an element marked `aria-hidden="true"`,
so that can't give a button a name, and Statac never reports a button
inside such an element. Nor does it report one inside a `<template>`,
which is on no page until a script puts it there. Statac reports an
`<input type="button">` or `<input type="image">` with no words as
`missing-label`. If a button in the content shows only an image that
Statac already reports as `missing-alt-text`, Statac reports it once,
there.

## How to fix it

Put words inside the button that say what it does. If it shows only an
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

To stop Statac showing the note, add this to `statac.yaml`:

```yaml
quiet: [missing-button-text]
```
