# URL gone

A URL the site published before now has nothing at it. A reader
following an old link or a bookmark, or a search engine, finds nothing
there.

## What you'll see {#what-youll-see}

```text
warning  URL gone: `/about/` was published before, and nothing is there now
  published.txt, line 1
   1 │ /about/
     │ ^^^^^^^
  Redirect it with `redirect_from: [/about/]` in the page that took its place, or take it out of `published.txt` if it is gone for good.
  More: https://statac.dev/e/url-gone/
```

## Why Statac warns

Each time it builds the site, and as `statac serve` runs, Statac adds
every URL it published to `published.txt` at the top of the site's
folder: each page from a file, each old URL in a `redirect_from`, each
feed, and each file published unchanged. Drafts and scheduled posts aren't
added until they are published. Commit the file with the site, so a
build on any machine knows the same URLs.

If a URL first appears while `statac serve` runs and goes again before
it stops, such as a page renamed while you were still writing it,
Statac takes it out of the file and says nothing.

When a page is renamed, moved, or given another `permalink`, its old
URL is no longer published, and nothing else would tell you. Statac
builds the site as usual, and warns at the URL's line in `published.txt`.

Statac doesn't keep the list pages it makes, such as a tag's, or
paginated pages, since they come and go with what is listed. Nor does it
keep the files in `assets/`, whose URLs change when their contents
do.

If the site has no `published.txt` yet, but `_site` still holds a site
Statac built, Statac reads the URLs in `_site` in its place. That is how
a site built by a Statac that kept no `published.txt` learns what an
upgrade no longer publishes. Each URL gone is told with "Published by an
earlier build, in `_site`", and the first build writes every URL from
`_site` into `published.txt`, paginated pages and tag pages among them,
so later warnings are at their lines. Statac leaves out the files it
names by their contents, and the script that reads a search index.

## How to fix it

If the page moved, add its old URL to the page's `redirect_from` in its
new place, and the old URL then redirects to it.

If it is gone for good, take its line out of `published.txt`, and
Statac stops warning about it. In an editor with Statac's extension,
the warning offers to do that for you.

If every URL Statac kept under a folder is gone, such as when you take
away a whole part of the site, Statac gives one warning for them all, at
the outermost folder they share, and the editor offers to take all their
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
