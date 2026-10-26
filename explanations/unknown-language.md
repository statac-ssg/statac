# Unknown language

The site's `language` is set to a language Statac has no translations
for, and the site's `languages/` folder has no file for it either. Statac
comes with translations for a few languages. For any other, the site
provides its own.

## What you'll see {#what-youll-see}

```text
warning  Unknown language: Statac has no translations for `sw`
  Write its months, weekdays and quotation marks in `languages/sw.yaml`.
  More: https://statac.dev/e/unknown-language/
```

## Why Statac warns

Statac is sure which language you mean, so the build continues. Without
translations, Statac writes dates in figures, such as `2025-12-31`,
whichever look a template names, and leaves quotation marks as they were
typed. A `date_format` that writes a month or a weekday in words stops
the build, since there are no names to write it with. Text direction
needs no translations: Statac still writes a page in Hebrew or Urdu
right to left.

## How to fix it

Add a file to `languages/` named after the language's code, containing
the language's months, weekdays and quotation marks. Every line in it is optional.

## Example

In `statac.yaml`:

```yaml
language: sw
```

A file `languages/sw.yaml` with the translations for Swahili:

```yaml
months: [Januari, Februari, Machi, Aprili, Mei, Juni,
  Julai, Agosti, Septemba, Oktoba, Novemba, Desemba]
weekdays: [Jumatatu, Jumanne, Jumatano, Alhamisi, Ijumaa, Jumamosi, Jumapili]
quotes: “ ” ‘ ’
```
