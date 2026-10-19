# Unusable address

An address can't be written to disk, or a `permalink` pattern can't make
one. A name in an address can't contain `< > : " | ? * \ % #` or a control
character, can't end with a space or a dot, and can't be `.`, `..` or a
name Windows reserves for a device, such as `con`. Statac applies these
rules to every site, so a site that builds on one computer builds on all
of them.

A pattern can use `{year}`, `{month}`, `{day}`, `{folder}` and `{name}`,
and `{slug}`, which is the page's own name made suitable for an address.
It can also use a field declared in `statac.yaml`, such as `{isbn}` for
`fields.isbn: text` or `{book.author}` for `fields.book.author: text`,
and each page it gives an address must have that field. The pieces of
a date need a page that has a date. The pattern of a page with `each:`
can also use any field of the items or posts it makes pages for, such
as `{key}` or `{isbn}`, and each item or post must have the fields the
pattern uses.

## What you'll see {#what-youll-see}

```text
error  Unusable address: there is no piece `{yaer}`
  Did you mean `{year}`?
  More: https://statac.dev/e/unusable-address/
```

## Why Statac stops

A page that can't be written would be missing from the site, and one
that ends up somewhere you didn't mean would be at the wrong address.

## How to fix it

Do what the message suggests: remove the character, correct the piece's
name, or give the page a date.

## Example

In `content/notes/index.md`:

```yaml
---
pages.permalink: /notes/{yaer}/{name}/
---
```

The piece is called `{year}`:

```yaml
---
pages.permalink: /notes/{year}/{name}/
---
```
