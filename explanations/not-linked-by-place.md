# Not linked by place

A page with no `translation_key` is in the same place as a page in
another language that has one. For example, `content/en/about.md` has no
key, and `content/about.md` has `translation_key: about`. The two aren't
linked as translations.

## What you'll see {#what-youll-see}

```text
warning  Not linked by place: `content/about.md` has `translation_key: about`
  content/en/about.md, line 1
   1 │ ---
     │ ^^^
  To link the two, give this page `translation_key: about` too.
  More: https://statac.dev/e/not-linked-by-place/
```

## Why Statac warns

Pages in different languages at the same path under their languages'
folders are translations of each other. But Statac links a page with a
`translation_key` by its key alone, so it leaves out the page in the
same place. Statac still builds both pages. It warns you
because two pages in the same place are usually meant to be translations
of one page.

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
