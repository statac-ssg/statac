# Unknown word

A word appears where Statac knows every word that can be written, and this
isn't one of them: a setting under `tags.` or `series.`, a kind in a field
declaration, a setting in a language file, a time zone, a table in `data/`
that `each:` names, a place on a card, a stamp in front matter such as
`=tomorrow`, or a name in a template.

In a template, Statac knows every name: `page` and `site`, the fields pages
and settings have, the names the template sets itself, its filters and
tests, the snippets in `snippets/`, and the tags and folders the site has.

## What you'll see {#what-youll-see}

```text
error  Unknown setting: `tags.permalnk`
  Did you mean `tags.permalink`?
  More: https://statac.dev/e/unknown-word/
```

## Why Statac stops

These words are Statac's own, the site's, or the names of the world's time
zones, so one it doesn't know is a slip, and it can't tell what you meant.
When a known word is close, Statac offers it.

## How to fix it

Write the word Statac offers, or one from the list on its page in the
documentation. A time zone is named for a place, as `Europe/London` or
`America/Chicago` is. In a template, check the spelling against the field in
the front matter, the setting in `statac.yaml`, or the file in `snippets/`.

A stamp is `=today`, `=now`, `=tomorrow`, `=yesterday` or a weekday from
`=monday` to `=sunday`. A time after it is written with two figures for
the hour, a colon and two for the minute, as `=monday 09:00` is.

## Example

In `statac.yaml`:

```yaml
tags.permalnk: /topics/{slug}/
fields.isbn: txt
timezone: Europe/Londn
```

Statac's words are `tags.permalink` and `text`, and the zone is
`Europe/London`:

```yaml
tags.permalink: /topics/{slug}/
fields.isbn: text
timezone: Europe/London
```

In `templates/post.html`:

```html
<h1>{{ page.titel }}</h1>
```

The field is `title`:

```html
<h1>{{ page.title }}</h1>
```
