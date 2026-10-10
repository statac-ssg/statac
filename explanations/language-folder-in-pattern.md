# Language folder in pattern

An address pattern for a language's pages opens with that language's
folder. On a site in several languages, a `permalink:` on a page in
`content/en/`, and `tags.permalink` and `series.permalink` for the English
tags and series, are read from `/en/`, the folder's own address: the
pattern `/{year}/{slug}/` lands an English post at `/en/2026/post-1/`.
Written as `/en/{year}/{slug}/`, the folder would be in the address twice.

## What you'll see {#what-youll-see}

```text
warning  Language folder in pattern: `/en/{year}/{slug}/` is read from `/en/` already
  Write it as `/{year}/{slug}/`.
  More: https://statac.dev/e/language-folder-in-pattern/
```

## Why Statac warns

Statac is sure what was meant: an address with the folder in it once. It
reads the pattern without the folder, so the post lands at
`/en/2026/post-1/`, and says so, because the same pattern is read from
`/` on the main language's pages at the top of `content/`.

## How to fix it

Write the pattern without the language's folder. A pattern for every
language, written once in `statac.yaml`, then lands each language's pages
beneath its own folder.

## Example

In `content/en/index.md`:

```yaml
pages.permalink: /en/{year}/{slug}/
```

Without the folder, English posts still land at `/en/2026/post-1/`:

```yaml
pages.permalink: /{year}/{slug}/
```
