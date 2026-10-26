# Feed format mismatch

A feed's file name says one format, but the feed is built as another,
such as `rss.xml` built as an Atom feed. A name says a format where it
is `rss.xml` or `atom.xml`, or ends in `.rss`, `.atom` or `.json`.

## What you'll see {#what-youll-see}

```text
warning  Feed format mismatch: `rss.xml` is built as an Atom feed
  It is built with `templates/feed.xml`; add `templates/rss.xml` to build it as an RSS feed.
  More: https://statac.dev/e/feed-format-mismatch/
```

## Why Statac warns

Statac has no feed of its own. Each feed is built with the template
named as its file, such as `templates/rss.xml` for `rss.xml`. If there
isn't one, Statac uses the template named `feed` with the feed's
extension, such as `templates/feed.json` for `feed.json`, and otherwise
`templates/feed.xml`. A list with `feed: [atom.xml, rss.xml]` and only
`templates/feed.xml` therefore publishes the same Atom feed twice, once
under the RSS feed's name, and nothing else would show it.

Statac tells the format from the feed it built: an `<rss>` root is RSS,
a `<feed>` root is Atom, and a JSON Feed names its version. A name such
as `feed.xml` or `index.xml` says no format, so a feed with that name is
never warned of.

## How to fix it

Add the template the message names, written in the format the feed's
name says. The docs have an Atom template and an RSS template to copy:
see [the template](https://statac.dev/feeds/#the-template).

If the feed's own template is the one building it, write that template
in the format its name says, or rename the feed in the `feed:` line.

If you meant both names to carry the same feed, quieten the warning with
`quiet: [feed-format-mismatch]` in `statac.yaml`.

## Example

In `content/index.md`:

```yaml
---
title: Home
feed: [atom.xml, rss.xml]
---
```

With only `templates/feed.xml`, an Atom template, both feeds are Atom.
With an RSS template beside it, `rss.xml` is RSS:

```
templates/
  feed.xml
  rss.xml
```
