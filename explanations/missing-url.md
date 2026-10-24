# Missing URL

`statac refuse` was given no URL. It turns away either the webmention
from one URL or everything from one sender, so it needs a full URL to
know which.

## What you'll see {#what-youll-see}

```text
error  Missing URL: `statac refuse` needs the full URL of a webmention or a sender
  Give the full URL after the command, as `statac refuse https://a.example/post-1/`.
  More: https://statac.dev/e/missing-url/
```

## Why Statac stops

Without a URL, there is nothing to refuse.

## How to fix it

Give the full URL after `statac refuse`. This is either the URL of the
page that sent the webmention, as given in `url` in its file in
`fetched/mentions/`, or the URL of the sender's site.

## Example

```text
statac refuse
```

To turn away the webmention from `https://a.example/post-1/`:

```text
statac refuse https://a.example/post-1/
```
