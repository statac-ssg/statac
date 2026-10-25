# No feed key

Lists on the site ask for feeds, and `statac.yaml` has no `feed_key`.

## What you'll see {#what-youll-see}

```text
error  No feed key: `statac.yaml` has no `feed_key`
  Add `feed_key: key` to `statac.yaml` and never change it: feed readers know each post by an id made from it.
  More: https://statac.dev/e/no-feed-key/
```

## Why Statac stops

A feed reader identifies each post by an id, and shows any post with an id
it hasn't seen before as new. Statac makes each post's id from the site's
`feed_key` and the post's own identity, so the id is the same on every
build and a post never reappears as new in readers. Without a key, Statac
has nothing to make the ids from. Ids made up for one build would change on
the next, so Statac stops rather than publish a feed whose ids change.

A build never writes the key into `statac.yaml`: only `statac import`
does, for a site whose feeds come across with it. The key has to go with
every copy of the site, so it belongs in a file you keep.

## How to fix it

The message offers a newly made key. Add the line it shows to
`statac.yaml`, and build again. Once you have published a feed with the
key, never change it, because every id would change too.

If you don't want any feeds, take out the `feed:` lines that ask for
them instead.

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
