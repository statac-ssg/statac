# Unmatched translation key

A page has a `translation_key` that no published page in another
language has, so it isn't linked to any translation.

## What you'll see {#what-youll-see}

```text
warning  Unmatched translation key: no page in another language has `abuot`
  Did you mean `about`?
  More: https://statac.dev/e/unmatched-translation-key/
```

## Why Statac warns

Statac builds the page as written. A key is there to link a page to its
translations, so a key that links nothing usually means one of two
things. Either it is spelled differently on the page and on its
translation, or the translation hasn't been written yet. A draft, or a
scheduled post, still counts as having its key. The page is
linked to it once it is published.

On a site in only one language there is no other language, so no key
links anything.

## How to fix it

Write the same key on the page's translation in each other language.
If the message suggests a key from another language, check whether the
two keys were meant to be the same. If the page has no translation,
remove the key.

## Example

`content/about.md` has:

```yaml
translation_key: about
```

and its English translation, `content/en/about-us.md`, has:

```yaml
translation_key: abuot
```

Once the key is spelled the same on both, the two pages are linked:

```yaml
translation_key: about
```
