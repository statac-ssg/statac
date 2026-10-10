# Feed id changed

A post in a feed is published at another address than the last build
published it at, and has no `id:` of its own. The message shows the
address it had.

## What you'll see {#what-youll-see}

```text
warning  Feed id changed: this post was published at `/posts/post-1/`
  Add `id: fc370633-88f0-8092-9f96-7749dbfff0eb` to its front matter to keep it the same post in feed readers.
  More: https://statac.dev/e/feed-id-changed/
```

## Why Statac warns

A feed reader knows each post by an id, and shows a post whose id it
hasn't seen as new. A post with no `id:` has an id made from its address,
so moving it gives it another, and everyone following the feed is shown
it again as a new post.

Statac remembers where each post in a feed was published, in
`.statac/feeds`, each time `statac build` publishes the site. It knows a
post again by its file: at the same path, or, where the file has moved
too, holding exactly what it held, or with the old address in its
`redirect_from:`. A post moved and edited at once, with no
`redirect_from:`, is one it can't tell from a new post, and it says
nothing of it.

## How to fix it

Add the `id:` line the message offers to the post's front matter. It is
the id the post had at its old address, so feed readers go on seeing the
same post. An id stays with a post wherever it moves after that.

Statac warns on every build until the line is there. For readers to see
the post as new after all, delete `.statac/feeds`: Statac forgets where
every post was, and remembers them afresh on the next build.

Statac never writes the line itself unless you ask it to, with
`write_feed_ids: true` in `statac.yaml`. `statac serve` and
`statac stamp` then write it for you, and the warning goes.

## Example

`content/posts/post-1.md` was published at `/posts/post-1/`, and now
gives itself another address:

```yaml
---
title: Post 1
date: 2026-01-01
permalink: /writing/post-1/
---
```

With the line Statac offered:

```yaml
---
title: Post 1
date: 2026-01-01
permalink: /writing/post-1/
id: fc370633-88f0-8092-9f96-7749dbfff0eb
---
```
