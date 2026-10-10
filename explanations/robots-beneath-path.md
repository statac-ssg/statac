# Robots beneath a path

The site lives beneath a path of its host, such as
`https://example.org/blog/`, and publishes a `robots.txt`, which lands
beneath that path too, at `https://example.org/blog/robots.txt`.

## What you'll see {#what-youll-see}

```text
note  Robots beneath a path: crawlers read only `https://example.org/robots.txt`
  Copy its lines into the `robots.txt` at the top of the host to have them read.
  More: https://statac.dev/e/robots-beneath-path/
```

## Why Statac notes it

Crawlers read `robots.txt` at the top of a host and nowhere else, so what
this one says is never read. Statac publishes it all the same, since it
is yours, and tells you once.

## How to fix it

Copy the lines you want read into the `robots.txt` at the top of the
host, wherever that is kept. A line naming the sitemap works from there:

```
Sitemap: https://example.org/blog/sitemap.xml
```

If the page is there only for crawlers, you can take it out of the site.
Once you have understood the note, `quiet:` stops it being shown.

## Example

`statac.yaml`:

```yaml
url: https://example.org/blog/
```

`content/robots.md`, published at `https://example.org/blog/robots.txt`:

```yaml
---
permalink: /robots.txt
layout: robots.txt
---
```
