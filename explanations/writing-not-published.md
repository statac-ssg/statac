# Writing not published

A page with `redirect_to` has writing after its front matter. A page
that sends readers on publishes only the redirect, so the writing doesn't
appear anywhere.

## What you'll see {#what-youll-see}

```text
warning  Writing not published: `content/post-1.md` sends readers on
  Move it to where readers are sent, or take out `redirect_to` to publish it.
  More: https://statac.dev/e/writing-not-published/
```

## Why Statac warns

Statac is sure the redirect is intended, because `redirect_to` says so.
It publishes the redirect and tells you, because writing that is never
published may be writing you want to keep.

## How to fix it

Move the writing to the page readers are sent to, and leave only front
matter on the page that sends them on. If the page should be published
here after all, remove `redirect_to`.

## Example

In `content/post-1.md`:

```markdown
---
redirect_to: /posts/post-1/
---
The first post.
```

With the writing moved to `content/posts/post-1.md`:

```markdown
---
redirect_to: /posts/post-1/
---
```
