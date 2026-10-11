# Feed not written

A list has a feed, but Statac didn't write it. The message names the
list and gives the reason, which is one of these:

- The site has no `title`, which the feed uses as its name.
- There is no `templates/feed.xml` to build the feed with.
- One of your pages is published at the feed's address.
- A `feed:` line asks for a feed of a list where nothing is dated, so the
  feed would have nothing in it.

## What you'll see {#what-youll-see}

```text
warning  Feed not written: the site's own list needs `templates/feed.xml`
  Add the template to publish it, or write `pages.feed: false` in `statac.yaml` for no feeds.
  More: https://statac.dev/e/feed-not-written/
```

## Why Statac warns

The site's own list and each folder at the top of `content/` have a feed
unless their page turns it off. Any other list has a feed only if its page
asks for one with `feed:`. Statac has no built-in feed. The file a feed
reader reads is built from your template, so you decide what it says.
If something a feed needs is missing, Statac publishes the rest of the
site without the feed, and warns once. If one of your pages is at the
feed's address, the page is published as you wrote it, and the feed is
left out.

## How to fix it

Add what is missing: a `title:` line in `statac.yaml`, or the template.

A feed is built with `templates/feed.xml`. If the feed's address ends in
another extension, Statac uses the template with that name if you have
one, such as `templates/feed.json` for `/feed.json`. The template is
handed `list` (the list), `posts` (its posts, newest first) and `page`
(the page that speaks for the list).

To turn off every feed, hand `feed: false` down to every page in
`statac.yaml`:

```yaml
pages.feed: false
```

If a page is at the feed's address, give the feed a different address
with `feed:` on the list's page, such as `feed: atom.xml`, or move the
page.

If nothing in the list is dated, give one of its pages a `date:`, or
remove the `feed:` line. If the line is handed down to several such
lists, such as `pages.feed: true`, Statac warns once, at that line, and
names the first of the lists.

## Example

A site with posts but no `templates/feed.xml`. The site's own list has a
feed, but there is no template to build it with:

```
templates/
  base.html
  page.html
```

With a template, the feed is published at `/feed.xml`:

```
templates/
  base.html
  feed.xml
  page.html
```

A folder of notes with no dates, whose page asks for a feed. In
`content/notes/index.md`:

```yaml
---
title: Notes
feed: true
---
```

Remove the `feed:` line, and the folder has no feed and no warning.
