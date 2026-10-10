# Same translation

Two pages in one language would both be the translation of the same
pages in the site's other languages. Either both have the same
`translation_key`, or both stand where a page in another language
stands, such as `content/en/about.md` and `content/en/about.html` beside
`content/about.md`. The message points at the second, and shows the
first.

## What you'll see {#what-youll-see}

```text
error  Same translation: two pages in `en` have `translation_key: about`
  Give each page in `en` its own key.
  More: https://statac.dev/e/same-translation/
```

## Why Statac stops

A page has one translation in each language. With two candidates in
one language, Statac can't know which one a reader switching language
should reach, and a guess would send them to the wrong page.

## How to fix it

Where both have one key, give each page its own key, or move the one in
the wrong language to its own language's folder. Where both stand in one
place, rename one of them, or give the one that isn't the translation a
`translation_key` of its own, which links it by its key alone.

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
