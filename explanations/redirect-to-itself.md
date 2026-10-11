# Redirect to itself

A page's `redirect_to` is set to the page's own address, or to an address
whose redirects lead back to the page. A page with `redirect_to` publishes
only a redirect at its address, so a reader arriving there would be sent
back to the same redirect, again and again.

If a `redirect_to` points at another redirect on the site (an old address
in a page's `redirect_from`, or a page with its own `redirect_to`), Statac
follows it to where readers would end up. If that leads back to the page,
Statac reports the error on every page in the loop, each at its own
`redirect_to`.

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

Point `redirect_to` at the address where the writing is now. If the page
should be published at its own address after all, remove `redirect_to`.
In a loop, send readers from at least one page to an address that isn't
a redirect.

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

Two pages that send readers to each other. In `content/post-1.md`:

```yaml
---
redirect_to: /post-2/
---
```

And in `content/post-2.md`:

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
