# Unmatched translation key

A page has a `translation_key` that no published page in another
language has, so it is linked to no translation.

## What you'll see {#what-youll-see}

```text
warning  Unmatched translation key: no page in another language has `abuot`
  Did you mean `about`?
  More: https://statac.dev/e/unmatched-translation-key/
```

## Why Statac warns

Statac builds the page as written. A key is there to link a page to its
translations, so one that links nothing is usually a key spelled one way
on one page and another way on its translation, or a translation still
to be written. A draft, or a post waiting for its day, that has the key
counts as having it: the page is linked once that one is published.

On a site in one language there is no other language, so no key links
anything.

## How to fix it

Write the same key on the page's translation in each other language.
Where the message offers a key from another language, check whether the
two were meant to be one. If the page has no translation, take the key
off.

## Example

`content/about.md` has:

```yaml
translation_key: about
```

and its English translation, `content/en/about-us.md`, has:

```yaml
translation_key: abuot
```

Spelled alike on both, the two are linked:

```yaml
translation_key: about
```
