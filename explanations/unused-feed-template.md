# Unused feed template

The site has a feed template, such as `templates/feed.xml`, but no list
asks for a feed, so the template builds nothing.

## What you'll see {#what-youll-see}

```text
note  Unused feed template: no list asks for a feed
  Write `feed: true` in `statac.yaml` for the site's feed, built with `templates/feed.xml`.
  More: https://statac.dev/e/unused-feed-template/
```

## Why Statac says so

No list has a feed unless a `feed:` line asks for one. A feed template
on a site where nothing asks is likely a line you meant to write, and
nothing else would show that the site publishes no feed, so Statac
tells you once.

## How to fix it

For the site's feed, write `feed: true` in `statac.yaml`. On a site in
several languages, this gives each language's own list a feed. A
`feed:` line in the site's list page, `content/index.md`, is used
instead of the setting.

For a folder's feed, write `feed: true` in the front matter of its list
page, the `index.md` in the folder. To give every list page a feed,
cascade the line from `statac.yaml` with `pages.feed: true`.

The message names `templates/feed.xml` where the site has it. A site
with only another feed template, such as `templates/feed.json`, is
offered a line naming a feed it builds, such as `feed: feed.json`.

If you don't want any feeds, remove the template, or quieten the note
with `quiet: [unused-feed-template]` in `statac.yaml`.

## Example

A site with a feed template:

```
templates/
  base.html
  feed.xml
  page.html
```

With this line in `statac.yaml`, the site's own list has a feed at
`/feed.xml`, built with the template:

```yaml
title: Site
url: https://example.org/
feed_key: 0b6d0c4e-2f7a-8d51-9a8e-3c1f5e7a9b20
feed: true
```
