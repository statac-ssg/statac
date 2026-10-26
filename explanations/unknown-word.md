# Unknown word

Statac found a word it doesn't know in a place where it knows every word
that can be written. That place is one of these: a setting under `tags.`
or `series.`, a kind in a field declaration, a line in a language file,
a time zone, a table in `data/` that `each:` refers to, a place on a card,
a stamp in front matter such as `=tomorrow`, or a name in a template.

In a template, Statac knows every name that can be used: `page` and
`site`, the fields of pages and settings, the names the template sets
itself, the filters and tests, the snippets in `snippets/`, and the site's
tags and folders.

## What you'll see {#what-youll-see}

```text
error  Unknown setting: `tags.permalnk`
  Did you mean `tags.permalink`?
  More: https://statac.dev/e/unknown-word/
```

## Why Statac stops

These words are Statac's own, the site's own, or the names of the world's
time zones. So a word Statac doesn't know is a mistake, and it can't tell
what you meant. If a known word is close, Statac suggests it.

## How to fix it

Write the word Statac suggests, or one from the list on the relevant page
of the documentation. A time zone is named after a place, such as
`Europe/London` or `America/Chicago`. In a template, check the spelling against the field in
the front matter, the setting in `statac.yaml`, or the file in `snippets/`.

In a template, `loop` is set only inside a loop, between `{% for %}` and
`{% endfor %}`. A loop's own `if`, as in `{% for x in xs if loop.first %}`,
is read before `loop` is set, so test it with an `{% if %}` inside the
loop instead.

A stamp is `=today`, `=now`, `=tomorrow`, `=yesterday` or a weekday from
`=monday` to `=sunday`. To add a time after it, write two
digits for the hour, a colon, and two digits for the minute, as in
`=monday 09:00`.

## Example

In `statac.yaml`:

```yaml
tags.permalnk: /topics/{slug}/
fields.isbn: txt
timezone: Europe/Londn
```

The correct words are `tags.permalink` and `text`, and the time zone is
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
