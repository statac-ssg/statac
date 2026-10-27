# Missing translation

Statac adds a little text of its own to a page's content: what a screen
reader reads out for a footnote's back link, and the name of each kind of
callout, such as "Note" or "Warning". This text comes from the page's
language, and this language has no translation for it. Statac has these
lines in English only. For any other language, their translations come
from the site's own file in `languages/`. So Statac gives this note for a
language with no such file, for a file without the line, or for a file
whose line is empty.

For a site made in another language Statac knows the dates and
quotation marks of, `statac new` writes this file, with these lines in
English for you to translate.

The message shows the language and the number of pages. It appears once
for each language's footnotes and once for its callouts, however many
pages there are.

## What you'll see {#what-youll-see}

```text
note  Missing translation: the footnotes' back link, on 1 page in `fr`
  Add the line `back_to_the_text: back to the text` to `languages/fr.yaml`, translated.
  More: https://statac.dev/e/missing-translation/
```

## Why Statac says so

Statac still builds the pages, and leaves nothing out. It writes a
footnote's back link without a label, so a screen reader can only read
out its arrow. It passes a callout's `title` to
`snippets/markdown/callout.html` empty.

Statac gives no note if the translations wouldn't be shown anyway. It
gives the note on footnotes unless the site's own
`snippets/markdown/footnotes.html` never reads `label`. It gives the note
on callouts only if the site has a `snippets/markdown/callout.html` that
reads `title`. Without that file, Statac writes a callout with no
name. A file that never reads `title` has names of its own, for example
when a site keeps them with its other translations in `data/`.

Once you have understood the note, you can quieten it.

## How to fix it

Write the translations in the language's file in `languages/`, which is
named after the language's code. The file adds to the translations Statac
has for the language. Each line in the file replaces Statac's, and every
line it leaves out stays as it was. For a language Statac has no
translations for, the file supplies all of them.

`back_to_the_text` is the label of a footnote's back link. `callouts` is a
list of five names, for note, tip, important, warning and caution, in that
order. `snippets/markdown/callout.html` shows a callout's name where it
prints `title`.

## Example

In `languages/fr.yaml`, `statac new` writes these lines in English, for
you to translate into the site's language:

```yaml
back_to_the_text: back to the text
callouts: [Note, Tip, Important, Warning, Caution]
```
