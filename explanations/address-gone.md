# Address gone

An address the site published before now has nothing at it. A reader
following an old link or a bookmark, or a search engine, finds nothing
there.

## What you'll see {#what-youll-see}

```text
warning  Address gone: `/about/` was published before, and nothing answers there now
  Send readers on with `redirect_from: [/about/]` in the page that took its place, or take this line out if it is gone for good.
  More: https://statac.dev/e/address-gone/
```

## Why Statac warns

Each time it builds the site, Statac adds every address it published to
`published.txt` at the top of the site: each page from a file, each old
address in a `redirect_from`, each feed, and each file published as it
is. Commit the file with the site, so a build on any machine knows the
same addresses.

When a page is renamed, moved, or given another `permalink`, its old
address stops answering, and nothing else would tell you. Statac builds
the site as usual, and warns at the address's line in `published.txt`.

Lists' pages that Statac makes, such as a tag's, and the pages of a
list after its first aren't kept, since they come and go with what is
listed. Nor are the files in `assets/`, whose addresses change with
what they hold.

## How to fix it

If the page moved, add its old address to the `redirect_from` of the
page as it is now, and readers are sent on to it.

If it is gone for good, take its line out of `published.txt`, and
Statac stops warning about it.

Once you have understood the warning, you can quieten it by name.

## Example

`content/about.md` was published at `/about/`, and was renamed
`content/about-us.md`, so it is now published at `/about-us/`. Send
readers on from the old address:

```yaml
---
title: About us
redirect_from: [/about/]
---
```
