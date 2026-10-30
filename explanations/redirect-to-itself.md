# Redirect to itself

A page's `redirect_to` is set to the page's own URL, or to a URL whose
redirects lead back to the page. A page with `redirect_to` publishes only
a redirect at its URL, so a reader arriving there would be redirected
back to the same redirect, again and again.

The same goes for an old URL in a page's `redirect_from` that is the
page's own URL: the redirect would be published where the page is.
Renaming a page and then renaming it back can leave one there.

If a `redirect_to` points at another redirect on the site (an old URL in
a page's `redirect_from`, or a page with its own `redirect_to`), Statac
follows it to where readers would end up. If that leads back to the page,
Statac reports the error on every page in the loop, each at its own
`redirect_to`.

## What you'll see {#what-youll-see}

```text
error  Redirect to itself: `/post-1/` is this page's own URL
  content/post-1.md, line 2
   2 │ redirect_to: /post-1/
     │              ^^^^^^^^
  Point `redirect_to` at the URL readers should reach.
  More: https://statac.dev/e/redirect-to-itself/
```

## Why Statac stops

Statac can't tell where readers were meant to go, and publishing the
redirect would leave its URL unusable.

## How to fix it

**A `redirect_to` that leads back to the page.** Point `redirect_to` at
the URL where the content is now. If the page should be published at its
own URL after all, remove `redirect_to`. In a loop, redirect at least
one page to a URL that isn't a redirect.

**The page's own URL in `redirect_from`.** Take the URL out of
`redirect_from`. The page's other old URLs still redirect to it.

## Example

In `content/post-1.md`, published at `/post-1/`:

```yaml
---
redirect_to: /post-1/
---
```

Redirect it to where the content is now:

```yaml
---
redirect_to: /posts/post-1/
---
```

Two pages that redirect to each other. In `content/post-1.md`:

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

Redirect one of them to where the content is now:

```yaml
---
redirect_to: /posts/post-1/
---
```

A page renamed from `content/about-us.md` to `content/about.md` and back
again, published at `/about/`:

```yaml
---
redirect_from: [/about/, /about-us/]
---
```

Keep only the URL it is no longer published at:

```yaml
---
redirect_from: [/about-us/]
---
```
