# Content not published

A page with `redirect_to` has content after its front matter. A page
that redirects publishes only the redirect, so the content doesn't appear
anywhere.

## What you'll see {#what-youll-see}

```text
warning  Content not published: `content/post-1.md` redirects
  content/post-1.md, line 4
   4 │ The first post.
     │ ^^^^^^^^^^^^^^^
  Move it to the page it redirects to, or take out `redirect_to` to publish it.
  More: https://statac.dev/e/content-not-published/
```

## Why Statac warns

Statac is sure the redirect is intended, because `redirect_to` says so.
It publishes the redirect and tells you, because content that is never
published may be content you want to keep.

## How to fix it

Move the content to the page it redirects to, and leave only front
matter on the page that redirects. If the page should be published
here after all, remove `redirect_to`.

## Example

In `content/post-1.md`:

```markdown
---
redirect_to: /posts/post-1/
---
The first post.
```

With the content moved to `content/posts/post-1.md`:

```markdown
---
redirect_to: /posts/post-1/
---
```
