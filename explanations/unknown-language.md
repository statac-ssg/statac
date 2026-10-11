# Unknown language

The site's `language` is set to a language Statac has no words for, and
the site's `languages/` folder has no file for it either. Statac comes
with the words for a few languages. For any other, the site provides its
own.

## What you'll see {#what-youll-see}

```text
warning  Unknown language: Statac has no words for `sw`
  Write its months, weekdays and quotation marks in `languages/sw.yaml`.
  More: https://statac.dev/e/unknown-language/
```

## Why Statac warns

Statac is sure which language you mean, so the build continues. Without
the words, dates are written in figures, such as `2025-12-31`, and quotation
marks stay as they were typed. A `date_format` that writes a month or a
weekday in words stops the build, since there are no words to write it
with.

## How to fix it

Add a file to `languages/` named after the language's code, containing
the language's words and quotation marks. Every line in it is optional.

## Example

In `statac.yaml`:

```yaml
language: sw
```

A file `languages/sw.yaml` with the words for Swahili:

```yaml
months: [Januari, Februari, Machi, Aprili, Mei, Juni,
  Julai, Agosti, Septemba, Oktoba, Novemba, Desemba]
weekdays: [Jumatatu, Jumanne, Jumatano, Alhamisi, Ijumaa, Jumamosi, Jumapili]
quotes: “ ” ‘ ’
```
