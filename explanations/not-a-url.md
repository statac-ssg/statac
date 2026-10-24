# Not a URL

`statac refuse` was given something that isn't a full URL starting
`https://` or `http://`. Statac keeps webmentions only from pages on the
web, so only a URL like that can identify one.

## What you'll see {#what-youll-see}

```text
error  Not a URL: `statac refuse` takes a full URL starting `https://` or `http://`, not `a.example`
  Did you mean `https://a.example`?
  More: https://statac.dev/e/not-a-url/
```

## Why Statac stops

A URL without its scheme could mean more than one place, and Statac
doesn't guess which.

## How to fix it

Give the full URL, as the browser shows it, or as it appears in
`url` or `author_url` in the webmention's file in `fetched/mentions/`.

## Example

```text
statac refuse a.example
```

With the full URL:

```text
statac refuse https://a.example
```
