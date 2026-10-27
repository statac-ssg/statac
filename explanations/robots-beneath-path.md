# Robots beneath a path

The site is published beneath a path on its host, such as
`https://example.org/blog/`, and it publishes a `robots.txt`. That file
ends up beneath the same path, at `https://example.org/blog/robots.txt`.

## What you'll see {#what-youll-see}

```text
note  Robots beneath a path: crawlers read only `https://example.org/robots.txt`
  Copy its lines into the `robots.txt` at the root of the host for crawlers to read them.
  More: https://statac.dev/e/robots-beneath-path/
```

## Why Statac says so

Crawlers read `robots.txt` only at the root of a host, so they never read
this one. Statac publishes it anyway, since it is your file, and tells
you once.

## How to fix it

Copy the lines you want crawlers to read into the `robots.txt` at the
root of the host, wherever that file is kept. A line giving the sitemap's
URL works from there:

```
Sitemap: https://example.org/blog/sitemap.xml
```

If the page exists only for crawlers, you can remove it from the site.
Once you have understood the note, you can stop it being shown with
`quiet:`.

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
