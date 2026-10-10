# Not a web address

`statac refuse` was given an address that doesn't start `https://` or
`http://`. A webmention is kept only from a page on the web, so only
such an address can name one.

## What you'll see {#what-youll-see}

```text
error  Not a web address: `statac refuse` takes an address starting `https://` or `http://`, not `a.example`
  Did you mean `https://a.example`?
  More: https://statac.dev/e/not-a-web-address/
```

## Why Statac stops

An address without its start could name more than one place, and Statac
doesn't guess which.

## How to fix it

Give the whole address, as the browser shows it, or as the webmention's
file in `fetched/mentions/` gives it in `url` or `author_url`.

## Example

```text
statac refuse a.example
```

With the whole address:

```text
statac refuse https://a.example
```
