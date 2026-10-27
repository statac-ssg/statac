# No language set

`statac.yaml` has no `language` setting, so Statac treats the site as
written in English.

## What you'll see {#what-youll-see}

```text
note  No language set: Statac reads the site as English
  Add the site's language to `statac.yaml`, such as `language: en`.
  More: https://statac.dev/e/no-language/
```

## Why Statac says so

Each page states its language with `lang` on its `<html>` element, and a
screen reader uses that to choose the voice it reads the page aloud in.
With no `language`, templates get `en` for it, so a site written in
another language is read aloud in an English voice. A browser also uses
the same value to decide whether to offer a translation, and how to break
words at the ends of lines.

The language also decides the translations Statac writes: the names of
months and weekdays in dates, the quotation marks, and text such as `back
to the text` beside a footnote. With no `language`, these are in English
too.

Statac shows this note once for the whole site, and only if a template
reads the language, through `page.language`, `site.language` or
`site.languages`. If no template writes it, no page says what language
it is in, and there is nothing to tell. A site in English shows
correctly either way, so nothing else would show that the line is
missing.

## How to fix it

Add the site's language to `statac.yaml`, using its code. If the site is
written in more than one language, list them all, with the main one
first.

## Example

A `statac.yaml` with no `language` line:

```yaml
title: Site 1
```

With the line added, the note goes away, and every page says it is in
English:

```yaml
title: Site 1
language: en
```
