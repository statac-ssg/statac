# Unusable URL

A URL can't be written to disk, or a `permalink` pattern can't make one.
A name in a URL, between its slashes, can't contain `< > : " | ? * \ % #`
or a control character, can't end with a space or a dot, and can't be
`.`, `..` or a name Windows reserves for a device, such as `con`. Statac
applies these rules to every site, so a site that builds on one computer
builds on all of them.

A pattern can use the placeholders `{year}`, `{month}`, `{day}`,
`{folder}` and `{name}`, and `{slug}`, the page's slug. It can also use a
field declared in `statac.yaml`, such as `{isbn}` for
`fields.isbn: text` or `{book.author}` for `fields.book.author: text`,
and each page it gives a URL must have that field. The date placeholders
need a page that has a date. The pattern of a page with `each:` can also
use any field of the items or posts it makes pages for, such as `{key}`
or `{isbn}`, and each item or post must have the fields the pattern
uses.

## What you'll see {#what-youll-see}

```text
error  Unusable URL: there is no placeholder `{yaer}`
  Did you mean `{year}`?
  More: https://statac.dev/e/unusable-url/
```

## Why Statac stops

A page that can't be written would be missing from the site, and one
that ends up somewhere you didn't mean would be at the wrong URL.

## How to fix it

Do what the message suggests: remove the character, correct the
placeholder's name, or give the page a date.

## Example

In `content/notes/index.md`:

```yaml
---
pages.permalink: /notes/{yaer}/{name}/
---
```

The placeholder is called `{year}`:

```yaml
---
pages.permalink: /notes/{year}/{name}/
---
```
