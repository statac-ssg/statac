# No pages in language

`language` in `statac.yaml` declares a language that has no folder under
`content/`. A page is in a language other than the main one by standing in
that language's folder, such as `content/en/` for `en`, so nothing on the
site is in this one.

## What you'll see {#what-youll-see}

```text
warning  No pages in `en`: there is no `content/en/` folder
  Put the pages written in `en` in `content/en/`.
  More: https://statac.dev/e/no-pages-in-language/
```

## Why Statac warns

Statac is sure what to build: every page outside the language folders is
in the main language, the first in the list. The language you declared
publishes nothing, which is worth knowing in case its pages are in a
folder named another way.

## How to fix it

Put the language's pages in a folder at the top of `content/` named
exactly as `language` writes it. If the site isn't in that language yet,
take it out of the list until it is.

## Example

In `statac.yaml`:

```yaml
language: [ar, en]
```

with the English pages in `content/english/`. Named for the language as
declared, the folder is `content/en/`:

```text
content/
  index.md
  en/
    index.md
    post-1.md
```
