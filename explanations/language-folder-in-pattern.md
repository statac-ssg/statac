# Language folder in pattern

A URL pattern for one language's pages starts with that language's
folder. On a site in several languages, Statac puts `/en/`, the English
folder's own URL, in front of each pattern for English. This applies to
a `permalink:` on a page in `content/en/`, and to `tags.permalink` and
`series.permalink` for the English tags and series. So the pattern
`/{year}/{slug}/` puts an English post at `/en/2026/post-1/`. If the
pattern were written as `/en/{year}/{slug}/`, the folder would be in the
URL twice.

## What you'll see {#what-youll-see}

```text
warning  Language folder in pattern: `/en/{year}/{slug}/` starts with `/en/`, which Statac puts in front already
  content/en/index.md, line 3
   3 │ pages.permalink: /en/{year}/{slug}/
     │                  ^^^^^^^^^^^^^^^^^^
  Write it as `/{year}/{slug}/`.
  More: https://statac.dev/e/language-folder-in-pattern/
```

## Why Statac warns

Statac is sure what you meant: a URL with the folder in it once. It
drops the folder from the pattern, so it publishes the post at
`/en/2026/post-1/`. It still warns you, because on the main language's
pages, at the top of `content/`, it puts `/` in front of the same
pattern.

## How to fix it

Write the pattern without the language's folder. You can then write one
pattern for every language in `statac.yaml`, and Statac publishes each
language's pages under that language's own folder.

## Example

In `content/en/index.md`:

```yaml
pages.permalink: /en/{year}/{slug}/
```

Without the folder, Statac still publishes English posts at
`/en/2026/post-1/`:

```yaml
pages.permalink: /{year}/{slug}/
```
