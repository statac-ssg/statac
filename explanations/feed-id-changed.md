# Feed id changed

A post in a feed is now published at a different URL from the one the
last build used, and it has no `id:` of its own. The message shows its
old URL.

## What you'll see {#what-youll-see}

```text
warning  Feed id changed: this post was published at `/posts/post-1/`
  Add `id: fc370633-88f0-8092-9f96-7749dbfff0eb` to its front matter to keep it the same post in feed readers.
  More: https://statac.dev/e/feed-id-changed/
```

## Why Statac warns

A feed reader identifies each post by an id, and shows any post whose id
it hasn't seen before as new. A post with no `id:` gets an id made from
its URL, so moving the post changes its id, and everyone following
the feed sees it again as a new post.

Each time `statac build` publishes the site, Statac records in
`.statac/feeds` where each post in a feed was published. It recognises a
post by its file. The file can be at the same path, or, if the file has
moved too, it can have exactly the same contents as before, or have the
old URL in its `redirect_from:`. If a post is moved and edited at the
same time, with no `redirect_from:`, Statac can't tell it from a new
post, and says nothing about it.

A post made from a data file, by a page with `each:`, Statac knows by
that page and its item: an item with a name by its name, and an item in
a list by its fields being written exactly as before, or by the old
URL in its `redirect_from:`. Two items in a list written alike are
both taken as new.

## How to fix it

Add the `id:` line that the message suggests to the post's front matter.
For a post made from a data file, the line goes in the item's record in
the data file. It is the id the post had at its old URL, so feed
readers keep treating it as the same post. From then on, the id stays
with the post wherever it moves.

Statac warns on every build until the line is there. If you want readers
to see the post as new after all, delete `.statac/feeds`. Statac then
forgets where every post was published, and records them afresh on the
next build.

Statac never writes the line itself unless you ask it to, with
`write_feed_ids: true` in `statac.yaml`. With that set, `statac serve`
and `statac stamp` write it for you, and the warning goes away. They
never write into a data file, so a post made from one always takes the
line by hand.

## Example

`content/posts/post-1.md` was published at `/posts/post-1/`, and now
sets a different URL:

```yaml
---
title: Post 1
date: 2026-01-01
permalink: /writing/post-1/
---
```

With the line Statac suggested:

```yaml
---
title: Post 1
date: 2026-01-01
permalink: /writing/post-1/
id: fc370633-88f0-8092-9f96-7749dbfff0eb
---
```
