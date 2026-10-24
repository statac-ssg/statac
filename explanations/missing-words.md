# Missing words

Statac adds a few words of its own to a page's content: what a screen
reader reads out for a footnote's back link, and the name of each kind of
callout, such as "Note" or "Warning". These words come from the page's
language, and this language has no translation for them. Statac has these
words in English only. For any other language, their translations come
from the site's own file in `languages/`. So you see this note for a
language with no such file, for a file without the line, or for a file
where the line is empty.

For a site in one of the other languages Statac has translations for,
`statac new` writes this file, with these words in English for you to
translate.

The message shows the language and the number of pages. It appears once
for each language's footnotes and once for its callouts, however many
pages there are.

## What you'll see {#what-youll-see}

```text
note  Missing words: the footnotes of 1 page in `fr` have no translation for the back link
  Add them to `languages/fr.yaml`, translating the English line `back_to_the_text: back to the text`.
  More: https://statac.dev/e/missing-words/
```

## Why Statac says so

The pages are still built, with nothing left out. A footnote's back link
is written without a label, so a screen reader can only read out its
arrow. A callout's `title` is passed to `snippets/markdown/callout.html`
empty.

Statac gives no note if the translations wouldn't be shown anyway. The
note on footnotes is given unless the site's own
`snippets/markdown/footnotes.html` never reads `label`. The note on
callouts is given only if the site has a `snippets/markdown/callout.html`
that reads `title`. Without that file, Statac writes a callout with no
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
