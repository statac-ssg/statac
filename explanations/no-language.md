# No language set

`statac.yaml` has no `language`, so Statac reads the site as written in
English.

## What you'll see {#what-youll-see}

```text
note  No language set: the site is read as English
  Add the site's language to `statac.yaml`, as `language: en`.
  More: https://statac.dev/e/no-language/
```

## Why Statac says so

A page says what language it is in, as `lang` on its `<html>`, and a
screen reader chooses the voice it reads the page aloud in by that. With
no `language`, templates have `en` for it, so a site written in another
language is read aloud in an English voice. A browser offers to
translate a page, and breaks its words at the ends of lines, by the same
value.

The language also decides the words Statac writes: the months and
weekdays of dates, the quotation marks, and words such as `back to the
text` beside a footnote. With no `language`, these are English too.

Statac says so once for the site, since a site in English looks right
either way and nothing else would show the line missing.

## How to fix it

Add the language the site is written in to `statac.yaml`, by its code.
A site written in more than one lists them all, the main one first.

## Example

A `statac.yaml` with no `language` line:

```yaml
title: Site 1
```

With the line, the note is gone, and every page says it is in English:

```yaml
title: Site 1
language: en
```
