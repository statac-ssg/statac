# Missing words

Statac writes a few words of its own beside a page's writing: what a screen
reader says for the link leading back from a footnote to the text, and the
name of each kind of callout, such as "Note" or "Warning". They come from
the page's language, and this language has none for them. Statac has these
words in English only. For any other language they come from the site's own
file in `languages/`. So the note is for a language with no such file, for a
file that lacks the line, or for one that gives the line with nothing in it.

`statac new` writes the file for a site in another of the languages Statac
has words for, with these words in English for you to translate.

The message names the language and counts the pages. It is said once for
each language's footnotes and once for its callouts, however many pages
there are.

## What you'll see {#what-youll-see}

```text
note  Missing words: the footnotes of 1 page in `fr` have no words for the way back
  Add them to `languages/fr.yaml`, translating the English line `back_to_the_text: back to the text`.
  More: https://statac.dev/e/missing-words/
```

## Why Statac says so

The pages are built and nothing is left out. A footnote's link back is
written with no label, so a screen reader has only its arrow to read. A
callout's `title` is handed to `snippets/markdown/callout.html` empty.

Nothing is said where the words would not be shown. The note on footnotes is
given unless the site's own `snippets/markdown/footnotes.html` never reads
`label`. The note on callouts is given only where the site has a
`snippets/markdown/callout.html` that reads `title`: with no such file
Statac writes a callout with no name, and a file that never reads `title`
has names of its own, as when a site keeps them with its other words in
`data/`.

Once you have understood it, it can be quietened.

## How to fix it

Write the words in the language's file in `languages/`, named after its
code. The file adds to the words Statac has for the language: a line it
gives takes the place of Statac's, and every line it leaves out stays as it
was. For a language Statac has no words for, the file is all the words
there are.

`back_to_the_text` is the label of a footnote's link back. `callouts` is
five names, for note, tip, important, warning and caution, in that order.
A callout's name is shown by `snippets/markdown/callout.html`, where it
prints `title`.

## Example

In `languages/fr.yaml`, the lines as `statac new` writes them, in English,
each to be translated into the site's language:

```yaml
back_to_the_text: back to the text
callouts: [Note, Tip, Important, Warning, Caution]
```
