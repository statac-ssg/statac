# Feed not written

A list has a feed, and Statac wrote none for it. Statac names the list,
and says what stood in the way: the site has no `title`, which names the
feed; there is no `templates/feed.xml` to build it with; a page of
yours is published at the feed's address; or a `feed:` line asks for a
feed of a list with nothing dated, which has nothing to carry.

## What you'll see {#what-youll-see}

```text
warning  Feed not written: the site's own list needs `templates/feed.xml`
  Add the template to publish it, or write `pages.feed: false` in `statac.yaml` for no feeds.
  More: https://statac.dev/e/feed-not-written/
```

## Why Statac warns

The site's own list and each folder at the top of `content/` have a feed
unless their page says otherwise, and any other list has one where its
page asks with `feed:`. Statac ships no feed of its own: the file a feed
reader reads is built with your template, so that what it says is yours.
Where something a feed needs is missing, Statac publishes the rest of the
site without the feed, and says so once. A page of yours at the feed's
address is published as you wrote it, and the feed is left out.

## How to fix it

Add what is missing: a `title:` line in `statac.yaml`, or the template.

A feed is built with `templates/feed.xml`, or for a feed whose address
ends in another extension, the template of that name where you have one:
`templates/feed.json` for `/feed.json`. The template is handed `list`,
the list, `posts`, its posts newest first, and `page`, the page speaking
for the list.

For no feeds at all, hand every page `feed: false` in `statac.yaml`:

```yaml
pages.feed: false
```

Where a page stands at the feed's address, give the feed another with
`feed:` on the list's page, such as `feed: atom.xml`, or move the page.

Where the list has nothing dated, give a page in it a `date:`, or take
the `feed:` line out. A line handed down to several such lists, such as
`pages.feed: true`, is told once, at the line, naming the first of them.

## Example

A site with posts and no `templates/feed.xml`: the site's own list has a
feed, and nothing to build it with.

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

A folder of notes with no dates, whose page asks for a feed, in
`content/notes/index.md`:

```yaml
---
title: Notes
feed: true
---
```

Take the line out, and the folder goes without a feed and without a
word.
