# Feed written twice

A list's `feed:` line names the same feed twice. Either the same name is
in the list twice, or two names lead to the same URL, such as `feed.xml` and
`/posts/feed.xml` on the list page of `content/posts/`. The message points
at the second one, and shows where the first is written.

## What you'll see {#what-youll-see}

```text
warning  Feed written twice: `feed.xml` and `/posts/feed.xml` are both at `/posts/feed.xml`
  The feed is published once; take one of them out of the `feed:` line.
  More: https://statac.dev/e/feed-written-twice/
```

## Why Statac warns

Only one file can be published at a URL. Both names ask for the same
feed, so Statac publishes it once, built with the template the first
name chooses, and warns you because you may have meant the second name
to be a feed of its own. A name ends up at the same URL as another when
one is written beside the list page and the other from the top of the
site, or when they differ only in capitals, which most disks ignore.

## How to fix it

Take one of the names out of the `feed:` line. If you meant two feeds,
such as an Atom feed and an RSS feed, give the second a name of its own,
such as `rss.xml`.

## Example

In `content/posts/index.md`:

```yaml
---
title: Posts
feed: [feed.xml, rss.xml, feed.xml]
---
```

With each feed named once:

```yaml
---
title: Posts
feed: [feed.xml, rss.xml]
---
```
