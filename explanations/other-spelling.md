# Other spelling

A field or setting uses a word that Statac spells differently, or a
page's content has a marker such as `<!--more-->` spelled differently,
such as `<!-- More -->`. On a card, Statac uses `colour` and `centre`, never
`color` or `center`. A stamp in front matter is written `=today` or
`=monday 09:00`, never `=Today`, `=mon`, `=next monday` or `=monday 9am`.
Statac refuses a Markdown file that starts with a `+++` line, the way some
other tools write front matter, and doesn't build its page. Statac has one
spelling for each of its own words, and this isn't it.

Statac 0.7.0 renamed some of its own settings, fields and problem names.
Statac refuses an old name the same way, and offers the new one:
`later_pages:` becomes `pagination.permalink:`.

## What you'll see {#what-youll-see}

```text
error  Unknown field: `template`
  Statac calls this `layout`.
  More: https://statac.dev/e/other-spelling/
```

## Why Statac stops

Statac never accepts a second spelling of its own words, however clear
the meaning. If a site used both, everyone who reads its files would have
to learn both. The message gives Statac's spelling.

## How to fix it

Write the field, or the marker, as the message shows it.

Rewrite front matter between `+++` lines as YAML between `---` lines:
`title = 'Post 1'` becomes `title: Post 1`.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
template: post
---
```

Statac calls this `layout`:

```yaml
---
title: Post 1
layout: post
---
```
