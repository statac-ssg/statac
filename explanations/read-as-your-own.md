# Read as your own

A name Statac keeps as one of your own is one slip from a word Statac or
the site knows: one character added, left out or changed, two neighbours
swapped, or the same word in other capitals. Statac reads it as your own,
as it would any other name, and says so.

It looks in three places. A key in front matter, on a page, in a
`pages.` line or on an item of a table, is compared with Statac's words
for a page and the fields `statac.yaml` declares. A name in `statac.yaml`
with no dot in it is compared with Statac's settings. A `type` is compared
with the types `statac.yaml` declares fields for, unless it is one of
them or a template of that name is in `templates/`.

A name of fewer than four characters is never told, and nor is a field
`statac.yaml` declares.

## What you'll see {#what-youll-see}

```text
warning  Read as your own field: `titel`
  Did you mean `title`? To keep `titel`, declare it in `statac.yaml` as `fields.titel: text`.
  More: https://statac.dev/e/read-as-your-own/
```

## Why Statac warns

A word written slightly wrong is read as something else without any sign.
`titel:` is a field of your own, so the page has no title; `posts_per_pge:`
is a value of your own, so lists keep their usual length. The page builds,
and the mistake shows only on the page itself, if anyone looks. Statac
can't be sure the name isn't meant, so it reads it as written and names
the word it is near.

## How to fix it

If the name is a slip, write the word Statac offers.

If the field is meant, declare it in `statac.yaml`, as `fields.titel: text`,
and Statac knows it is yours. A setting or a type that is meant can keep
its name, and the warning can be quietened by name.

## Example

In `content/post-1.md`:

```yaml
---
titel: Post 1
---
```

Statac's word is `title`:

```yaml
---
title: Post 1
---
```
