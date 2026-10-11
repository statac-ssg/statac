# Read as your own

A name that Statac treats as one of your own is one slip away from a
word that Statac or the site knows: one character added, missing or
changed, two neighbouring characters swapped, or the same word with
different capitals. Statac reads it as your own, as it would any other
name, and warns you.

It checks in three places. A key in front matter (on a page, in a
`pages.` line or on an item of a table) is compared with Statac's words
for a page and with the fields `statac.yaml` declares. A name in
`statac.yaml` with no dot in it is compared with Statac's settings. A
`type` is compared with the types `statac.yaml` declares fields for,
unless it is one of them or `templates/` has a template of that name.

Statac never warns about a name shorter than four characters, or about
a field that `statac.yaml` declares.

## What you'll see {#what-youll-see}

```text
warning  Read as your own field: `titel`
  Did you mean `title`? To keep `titel`, declare it in `statac.yaml` as `fields.titel: text`.
  More: https://statac.dev/e/read-as-your-own/
```

## Why Statac warns

A word written slightly wrong is read as something else, with no sign
that anything is wrong. `titel:` becomes a field of your own, so the page
has no title. `posts_per_pge:` becomes a value of your own, so lists keep
their usual length. The page builds, and the mistake shows only on the
page itself, if anyone looks. Statac can't be sure you didn't mean the
name, so it reads it as written and tells you the word it is close to.

## How to fix it

If the name is a slip, write the word Statac suggests instead.

If you meant the field, declare it in `statac.yaml`, as
`fields.titel: text`, and Statac then knows it is yours. A setting or a
type you meant can keep its name, and you can quieten the warning by
name.

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
