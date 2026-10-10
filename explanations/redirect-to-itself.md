# Redirect to itself

A page's `redirect_to` names the page's own address, or an address whose
redirects, followed, come back to the page. A page with `redirect_to`
publishes a redirect at its address and nothing else, so a reader
arriving there would be sent back to the same redirect, again and again.

Statac follows a `redirect_to` naming another redirect on the site, an
old address in a page's `redirect_from` or a page with its own
`redirect_to`, to where readers would end up. Where that comes back to
the page, every page in the ring is told, each at its own `redirect_to`.

## What you'll see {#what-youll-see}

```text
error  Redirect to itself: `/post-1/` is this page's own address
  Point `redirect_to` at the address readers should reach.
  More: https://statac.dev/e/redirect-to-itself/
```

## Why Statac stops

Statac can't tell where readers were meant to go, and publishing the
redirect would leave its address unusable.

## How to fix it

Point `redirect_to` at the address the writing is at now. If the page
should be published here after all, take `redirect_to` out. In a ring,
send readers from one page at least to somewhere that isn't a redirect.

## Example

In `content/post-1.md`, published at `/post-1/`:

```yaml
---
redirect_to: /post-1/
---
```

Send readers to where the writing is now:

```yaml
---
redirect_to: /posts/post-1/
---
```

Two pages sending readers to each other, `content/post-1.md`:

```yaml
---
redirect_to: /post-2/
---
```

and `content/post-2.md`:

```yaml
---
redirect_to: /post-1/
---
```

Send one of them to where the writing is now:

```yaml
---
redirect_to: /posts/post-1/
---
```
