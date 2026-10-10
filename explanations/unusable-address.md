# Unusable address

An address can't be written to disk, or a `permalink` pattern can't make
one. A name in an address can't hold `< > : " | ? * \ % #` or a control
character, can't end with a space or a dot, and can't be `.`, `..` or a
name Windows keeps for a device, such as `con`. Statac holds every site to
these rules, so a site that builds on one computer builds on all of them.

A pattern may ask for `{year}`, `{month}`, `{day}`, `{folder}` and
`{name}`, and for `{slug}`, the page's own name made ready for an
address. The pieces of a date need a page with a date. The pattern of a page with
`each:` may also ask for any field of the items or posts it makes pages
for, such as `{key}` or `{isbn}`, and each item or post needs the fields
it asks for.

## What you'll see {#what-youll-see}

```text
error  Unusable address: there is no piece `{yaer}`
  Did you mean `{year}`?
  More: https://statac.dev/e/unusable-address/
```

## Why Statac stops

A page that can't be written, or that lands somewhere you didn't mean,
would be missing from the site or at the wrong address.

## How to fix it

Do what the message suggests: take out the character, correct the piece's
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
