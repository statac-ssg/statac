# Writing not published

A page with `redirect_to` has writing after its front matter. A page
sending readers on publishes only its redirect, so the writing appears
nowhere.

## What you'll see {#what-youll-see}

```text
warning  Writing not published: `content/post-1.md` sends readers on
  Move it to where readers are sent, or take out `redirect_to` to publish it.
  More: https://statac.dev/e/writing-not-published/
```

## Why Statac warns

Statac is sure the redirect is meant: `redirect_to` says so. It publishes
the redirect and tells you, because writing that is never published may
be writing you want kept.

## How to fix it

Move the writing to the page readers are sent to, and leave the page
sending them on with front matter alone. If the page should be published
here after all, take `redirect_to` out.

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
