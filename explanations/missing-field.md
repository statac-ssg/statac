# Missing field

A page is missing a field that `statac.yaml` declares `required`, either for
every page or for pages of the page's `type`. A page made from an item in
`data/` is missing it when the item is.

A page with `each:` makes a page for each item of a table, or for each
post, and needs a `permalink:` pattern to give each one its address.

A card's settings in `templates/` need a `background` and a `font`, and
each text on the card needs all four of its fields: `text`, `place`,
`size` and `colour`, as `title.text`.

## What you'll see {#what-youll-see}

```text
error  Missing field: `isbn`
  Add `isbn:`, which `statac.yaml` requires on pages of type `book`.
  More: https://statac.dev/e/missing-field/
```

## Why Statac stops

The rule is yours: you said a page can't be published without this field.
Statac holds the site to it. A card can't be drawn without its background,
its fonts, or a text's place, size and colour, and Statac won't guess
them.

## How to fix it

Add the field to the page, or take `required` out of the declaration if
the field is optional after all. For a page with `each:`, add a pattern
such as `permalink: /people/{key}/`, or `permalink: /cards/{name}/` for a
page for each post. In the card's settings, add the line the message names.

## Example

In `statac.yaml`:

```yaml
types.book.isbn: required text
```

In `content/books/book-1.md`:

```yaml
---
title: Book 1
type: book
isbn: "0000000001"
---
```
