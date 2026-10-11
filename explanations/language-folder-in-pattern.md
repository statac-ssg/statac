# Language folder in pattern

An address pattern for one language's pages starts with that language's
folder. On a site in several languages, the patterns for English are read
from `/en/`, the English folder's own address. This applies to a
`permalink:` on a page in `content/en/`, and to `tags.permalink` and
`series.permalink` for the English tags and series. So the pattern
`/{year}/{slug}/` puts an English post at `/en/2026/post-1/`. If the
pattern were written as `/en/{year}/{slug}/`, the folder would be in the
address twice.

## What you'll see {#what-youll-see}

```text
warning  Language folder in pattern: `/en/{year}/{slug}/` is read from `/en/` already
  Write it as `/{year}/{slug}/`.
  More: https://statac.dev/e/language-folder-in-pattern/
```

## Why Statac warns

Statac is sure what you meant: an address with the folder in it once. It
reads the pattern without the folder, so the post is published at
`/en/2026/post-1/`. It still warns you, because the same pattern is read
from `/` on the main language's pages, at the top of `content/`.

## How to fix it

Write the pattern without the language's folder. You can then write one
pattern for every language in `statac.yaml`, and each language's pages are
published beneath that language's own folder.

## Example

In `content/en/index.md`:

```yaml
pages.permalink: /en/{year}/{slug}/
```

Without the folder, English posts are still published at
`/en/2026/post-1/`:

```yaml
pages.permalink: /{year}/{slug}/
```
