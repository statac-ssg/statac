# Missing address

`statac refuse` was given no address. It turns away the webmention at an
address, or everything from a sender, so it needs to know which.

## What you'll see {#what-youll-see}

```text
error  Missing address: `statac refuse` needs the address of a webmention or a sender
  Give the address after the command, as `statac refuse https://a.example/post-1/`.
  More: https://statac.dev/e/missing-address/
```

## Why Statac stops

Without an address, there is nothing to refuse.

## How to fix it

Give the address after `statac refuse`: the address of the page that
sent the webmention, as its file in `fetched/mentions/` gives it in
`url`, or the address of the sender's site.

## Example

```text
statac refuse
```

To turn away the webmention from `https://a.example/post-1/`:

```text
statac refuse https://a.example/post-1/
```
