# URL gone

A URL the site published before now has nothing at it. A reader
following an old link or a bookmark, or a search engine, finds nothing
there.

## What you'll see {#what-youll-see}

```text
warning  URL gone: `/about/` was published before, and nothing is there now
  Redirect it with `redirect_from: [/about/]` in the page that took its place, or take it out of `published.txt` if it is gone for good.
  More: https://statac.dev/e/url-gone/
```

## Why Statac warns

Each time it builds the site, and as `statac serve` runs, Statac adds
every URL it published to `published.txt` at the top of the site's
folder: each page from a file, each old URL in a `redirect_from`, each
feed, and each file published as it is. Drafts and scheduled posts aren't
added until they are published. Commit the file with the site, so a
build on any machine knows the same URLs.

A URL that first appears while `statac serve` runs, and goes again
before it stops, such as a page renamed while you were still writing
it, is taken out of the file with nothing said.

When a page is renamed, moved, or given another `permalink`, its old
URL is no longer published, and nothing else would tell you. Statac
builds the site as usual, and warns at the URL's line in `published.txt`.

List pages that Statac makes, such as a tag's, and paginated pages
aren't kept, since they come and go with what is listed. Nor are the
files in `assets/`, whose URLs change with what they hold.

## How to fix it

If the page moved, add its old URL to the `redirect_from` of the page
as it is now, and the old URL redirects to it.

If it is gone for good, take its line out of `published.txt`, and
Statac stops warning about it. In an editor with Statac's extension,
the warning offers to do that for you.

Where every URL Statac kept under a folder is gone, as when a whole
part of the site is taken away, they are one warning, under the
outermost folder they share, and the editor offers to take all their
lines out at once.

Once you have understood the warning, you can quieten it by name.

## Example

`content/about.md` was published at `/about/`, and was renamed
`content/about-us.md`, so it is now published at `/about-us/`.
Redirect the old URL to it:

```yaml
---
title: About us
redirect_from: [/about/]
---
```
