# No feed key

The site has lists with feeds, and `statac.yaml` has no `feed_key`.

## What you'll see {#what-youll-see}

```text
error  No feed key: `statac.yaml` has no `feed_key`
  Add `feed_key: key` to `statac.yaml` and never change it: feed readers know each post by an id made from it.
  More: https://statac.dev/e/no-feed-key/
```

## Why Statac stops

A feed reader knows each post by an id, and shows a post whose id it
hasn't seen as new. Statac makes each post's id from the site's
`feed_key` and the post's own identity, the same on every build, so a
post never comes back as new in readers. Without a key there is nothing
to make the ids from, and ids made up for one build would change on the
next, so Statac stops rather than publish a feed that does that.

Statac never writes the key into `statac.yaml` itself: the key has to
travel with every copy of the site, so it belongs in a file you keep.

## How to fix it

Statac offers a freshly made key. Add the line it shows to
`statac.yaml`, and build again. Once a feed has been published with it,
never change it: every id would change with it.

For no feeds at all, write `pages.feed: false` in `statac.yaml` instead.

## Example

```yaml
title: Site
url: https://example.org/
```

With the line Statac offered:

```yaml
title: Site
url: https://example.org/
feed_key: 0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20
```
