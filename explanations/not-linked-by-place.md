# Not linked by place

A page with no `translation_key` stands where a page in another language
with one stands, such as `content/en/about.md` beside a
`content/about.md` that has `translation_key: about`. The two aren't
linked as translations.

## What you'll see {#what-youll-see}

```text
warning  Not linked by place: `content/about.md` has `translation_key: about`
  To link the two, give this page `translation_key: about` too.
  More: https://statac.dev/e/not-linked-by-place/
```

## Why Statac warns

Pages in different languages at the same path under their languages'
folders are translations of each other. A page with a
`translation_key` is linked by its key alone, though, so the page
standing in its place is left out. Statac builds both pages as they
are; it tells you because two pages standing in one place are usually
meant to be one page's translations.

## How to fix it

To link them, give the page without a key the same key as the other.
If the two aren't translations of each other, rename one of them.

## Example

`content/about.md` has:

```yaml
translation_key: about
```

and `content/en/about.md` has no key. Give it the same one:

```yaml
translation_key: about
```
