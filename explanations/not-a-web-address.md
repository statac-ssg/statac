# Not a web address

`statac refuse` was given an address that doesn't start with `https://`
or `http://`. Statac keeps webmentions only from pages on the web, so only
an address like that can identify one.

## What you'll see {#what-youll-see}

```text
error  Not a web address: `statac refuse` takes an address starting `https://` or `http://`, not `a.example`
  Did you mean `https://a.example`?
  More: https://statac.dev/e/not-a-web-address/
```

## Why Statac stops

An address without its beginning could mean more than one place, and
Statac doesn't guess which.

## How to fix it

Give the whole address, as the browser shows it, or as it appears in
`url` or `author_url` in the webmention's file in `fetched/mentions/`.

## Example

```text
statac refuse a.example
```

With the whole address:

```text
statac refuse https://a.example
```
