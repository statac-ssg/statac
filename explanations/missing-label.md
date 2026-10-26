# Missing label

A form control on a built page, an `<input>`, `<select>` or `<textarea>`,
has no label: no `<label for>` names its `id`, no `<label>` is around it,
and it has no `aria-label`, `aria-labelledby` or `title`.

## What you'll see {#what-youll-see}

```text
note  Missing label: the `<input>` named `q` has no label
  Put it inside a `<label>` saying what it is for, or give it an `aria-label`.
  More: https://statac.dev/e/missing-label/
```

## Why Statac says so

A screen reader names a form control by its label. With none, a reader
hears only "edit text" or "combo box", and can't tell what to type or
choose. A `placeholder` isn't a label: it goes once the reader starts
typing, and not every screen reader reads it. That is part of the page
Statac built, so Statac tells you. It is a note, and nothing is left out
of the site.

Statac reads each page once it is built. It reports a control written in
a template or snippet once, at its line, with the pages it is on, and
one written in the content at its place in the content.

Some controls need no `<label>`. An `<input type="hidden">` is never
shown, and a browser labels `<input type="submit">` and
`<input type="reset">` itself. An `<input type="button">` is labelled by
its `value`, and an `<input type="image">` by its `alt`. A control marked
`disabled`, such as the box beside an item of a task list, takes nothing
in. A screen reader never reads out a control marked `aria-hidden="true"`,
or one inside an element marked so, and Statac never reports it. Nor
does it report one inside a `<template>`, which is on no page until a
script puts it there.

## How to fix it

Put the control inside a `<label>` with words saying what it is for, or
give the control an `id` and write a `<label for>` naming it. If the
words would show twice, such as beside a search box with a button saying
"Search", give the control an `aria-label` instead.

Once you have understood the note, you can quieten it by name.

## Example

In `templates/search.html`:

```html
<input type="search" name="q">
```

Put it inside a `<label>`, and the note goes away:

```html
<label>Search <input type="search" name="q"></label>
```

To stop Statac showing the note, add this to `statac.yaml`:

```yaml
quiet: [missing-label]
```
