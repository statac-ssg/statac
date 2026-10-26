# Read as custom

A name that Statac treats as a custom field, setting or type is one slip
away from a name that Statac or the site knows: one character added,
missing or changed, two neighbouring characters swapped, or the same name
with different capitals. Statac reads it as custom, the same as any other
name, and warns you.

It checks in three places. It compares a key in front matter (on a
page, in a `pages.` cascade or on an item of a table) with a page's
built-in fields and with the fields `statac.yaml` declares. It compares a
name in `statac.yaml` with no dot in it with Statac's settings. It
compares a `type` with the types `statac.yaml` declares fields for,
unless it is one of them or `templates/` has a template of that name.

Statac never warns about a name shorter than four characters, or about
a field that `statac.yaml` declares.

## What you'll see {#what-youll-see}

```text
warning  Read as a custom field: `titel`
  Did you mean `title`? To keep `titel`, declare it in `statac.yaml` as `fields.titel: text`.
  More: https://statac.dev/e/read-as-custom/
```

## Why Statac warns

Statac reads a name written slightly wrong as something else, with no
sign that anything is wrong. `titel:` becomes a custom field, so the page has
no title. `posts_per_pge:` becomes a custom setting, so lists keep their
usual length. The page builds, and the mistake shows only on the
page itself, if anyone looks. Statac can't be sure you didn't mean the
name, so it reads it as written and tells you the name it is close to.

## How to fix it

If the name is a slip, write the name Statac suggests instead.

If you meant the field, declare it in `statac.yaml`, with a line
such as `fields.titel: text`, and Statac then knows it is a custom field. A
custom setting or type you meant can keep its name, and you can quieten
the warning by name.

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
