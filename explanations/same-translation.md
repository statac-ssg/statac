# Same translation

Two pages in the same language would both be the translation of the same
pages in the site's other languages. Either both have the same
`translation_key`, or both are in the same place as a page in another
language. For example, `content/en/about.md` and `content/en/about.html`
are both in the same place as `content/about.md`. The message points to
the second page, and shows the first.

## What you'll see {#what-youll-see}

```text
error  Same translation: two pages in `en` have `translation_key: about`
  content/en/about.md, line 2
   2 │ translation_key: about
     │                  ^^^^^
  First written: content/en/about-us.md, line 2
   2 │ translation_key: about
     │                  ^^^^^
  Give each page in `en` its own key.
  More: https://statac.dev/e/same-translation/
```

## Why Statac stops

A page has one translation in each language. If there are two candidates
in one language, Statac can't know which one to send a reader to when
they switch language, and a guess would send them to the wrong page.

## How to fix it

If both have the same key, give each page its own key, or move the one in
the wrong language to its own language's folder. If both are in the same
place, rename one of them, or give the one that isn't the translation a
`translation_key` of its own. Statac then links it to other pages by its
key only.

## Example

`content/en/about.md` and `content/en/about-us.md` both have:

```yaml
translation_key: about
```

Keep the key on the page that is the translation, and give the other
one its own:

```yaml
translation_key: about-us
```
