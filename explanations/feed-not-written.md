# Feed not written

A list asks for a feed, but Statac didn't write it. The message names the
list and gives the reason, which is one of these:

- The site has no `title`, which the feed uses as its name.
- There is no template to build the feed with, such as `templates/feed.xml`.
- One of your pages is published at the feed's URL.
- A `feed:` line asks for a feed of a list where nothing is dated, so the
  feed would have nothing in it.
- The list has no list page, such as a folder with no `index.md`, or its
  list page has `index_page: false`. A feed links to its list page and
  takes its id from it, so Statac doesn't write a feed with no page to
  link to.

## What you'll see {#what-youll-see}

```text
warning  Feed not written: the site's own list needs `templates/feed.xml`
  Add the template to publish it, or take out the `feed:` line asking for it.
  More: https://statac.dev/e/feed-not-written/
```

## Why Statac warns

A list has a feed only if a `feed:` line asks for one: on its list
page, cascaded to that page, or, for the site's own list, in
`statac.yaml`. Statac has no built-in feed. The file a feed reader reads
is built from your template, so you decide what it says. If something a
feed needs is missing, Statac publishes the rest of the site without the
feed, and warns once. If one of your pages is at the feed's URL, Statac
publishes the page as you wrote it, and leaves the feed out.

## How to fix it

**No site `title`.** Add a `title:` line to `statac.yaml`.

**No template.** Add one of the templates the message names. It names
them in the order Statac looks for them, and any one of them builds the
feed. [Write the feed's template](https://statac.dev/feeds/#the-template)
has an Atom template and an RSS template to copy, and says which
template builds each feed and what the template is handed.

A list can have several feeds, such as an Atom feed and an RSS feed:
`feed: [feed.xml, rss.xml]` builds `feed.xml` with `templates/feed.xml`
and `rss.xml` with `templates/rss.xml`. Without `templates/rss.xml`,
Statac builds `rss.xml` with `templates/feed.xml`, and warns with
[feed format mismatch](https://statac.dev/e/feed-format-mismatch/)
if that builds it in another format than its name says. Statac leaves a
feed out only if none of its templates exists, and warns once for each
such set of templates, naming the first list that needs it.

**A page at the feed's URL.** Give the feed a different URL with
`feed:` on the list page, such as `feed: atom.xml`, or move the page.

**A list with nothing dated.** Give one of its pages a `date:`, or
remove the `feed:` line. If the line cascades to several such lists,
such as `pages.feed: true`, Statac warns once, at that line, and names
the first of the lists.

**No list page, or `index_page: false`.** Add an `index.md` to the
list's folder, or for the site's own list, `content/index.md` or
`templates/home.html`. If its list page has `index_page: false`, take
that line out to publish the page. Or remove the `feed:` line. Statac warns once for all the lists
without a page, naming the first.

If you don't want the feed, take out the `feed:` line that asks for it.

## Example

Here a site with posts has `feed: true` in `statac.yaml`, but no
`templates/feed.xml`. The site's own list asks for a feed, but there is
no template to build it with:

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

Here a folder of notes has no dates, and its list page asks for a feed.
In
`content/notes/index.md`:

```yaml
---
title: Notes
feed: true
---
```

Remove the `feed:` line, and the folder has no feed and no warning.
