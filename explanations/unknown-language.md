# Unknown language

The site's `language` names a language Statac has no words for, and the
site's `languages/` folder has no file for it either. Statac knows the
words of a few languages; for any other, the site writes its own.

## What you'll see {#what-youll-see}

```text
warning  Unknown language: Statac has no words for `sw`
  Write its months, weekdays and quotation marks in `languages/sw.yaml`.
  More: https://statac.dev/e/unknown-language/
```

## Why Statac warns

Statac is sure which language you mean, and the build goes on. Without
words, dates are written in figures, such as `2025-12-31`, and quotation
marks stay as they were typed. A `date_format` that writes a month or a
weekday in words stops the build, since there are no words to write it
with.

## How to fix it

Add a file named after the language's code to `languages/`, with the
words and marks it is written with. Every line is optional.

## Example

In `statac.yaml`:

```yaml
language: sw
```

A file `languages/sw.yaml` giving Swahili's words:

```yaml
months: [Januari, Februari, Machi, Aprili, Mei, Juni,
  Julai, Agosti, Septemba, Oktoba, Novemba, Desemba]
weekdays: [Jumatatu, Jumanne, Jumatano, Alhamisi, Ijumaa, Jumamosi, Jumapili]
quotes: “ ” ‘ ’
```
