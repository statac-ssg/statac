# Missing iframe title

An `<iframe>` on a built page has no `title`, and no `aria-label` or
`aria-labelledby` either.

## What you'll see {#what-youll-see}

```text
note  Missing iframe title: the `<iframe>` showing `https://example.org/video/1` has no `title`
  Give it a `title` saying what it shows.
  More: https://statac.dev/e/missing-iframe-title/
```

## Why Statac says so

An `<iframe>` shows another page inside this one, such as a video or a
map. A screen reader names it by its `title`. With none, a reader hears
only that there is a frame, or its URL, and has to go inside to find
out what it is. That is part of the page Statac built, so Statac tells
you. It is a note, and nothing is left out of the site.

Statac reads each page once it is built. It reports an `<iframe>`
written in a template or snippet once, at its line, with the pages it is
on, and one written in the content at its place in the content. A screen
reader never reads out an `<iframe>` marked `aria-hidden="true"`, or one
inside an element marked so, and Statac never reports it. Nor does it
report one inside a `<template>`, which is on no page until a script puts
it there.

## How to fix it

Give the `<iframe>` a `title` that says what it shows, in a few words.

Once you have understood the note, you can quieten it by name.

## Example

In `content/post-1.md`:

```html
<iframe src="https://example.org/video/1"></iframe>
```

Give it a `title`, and the note goes away:

```html
<iframe src="https://example.org/video/1" title="Video 1"></iframe>
```

To stop Statac showing the note, add this to `statac.yaml`:

```yaml
quiet: [missing-iframe-title]
```
