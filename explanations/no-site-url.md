# No site URL

The site has no full URL yet (`url` in `statac.yaml`), so Statac has left
something that needs one out of the built site: a page that says
`needs: url`, the feeds of the site's lists, the social cards, or the
site's webmentions.

## What you'll see {#what-youll-see}

```text
note  No site URL: social cards are left out until the site has a `url`
  Add the site's full URL to `statac.yaml`, as `url: https://example.org/`.
  More: https://statac.dev/e/no-site-url/
```

## Why Statac says so

Some files are useless without the site's full URL. A feed reader
follows each post's link from the feed file, away from the site, and a
sitemap lists every page by its full URL. Rather than publish these
files with URLs that lead nowhere, Statac leaves them out until
`statac.yaml` says where the site lives. It reports each one it leaves
out, since nothing else would tell you it is missing.

A page asks to wait like this with `needs: url` in its front matter.
Feeds always wait. Social cards wait as well, because a link preview is
given a card's full URL. Until there is a `url`, no card is drawn and
`page.card` has no value. Webmentions wait too. A webmention refers to
the page it is about by its full URL, so without a `url` Statac can't
tell which page that is, and `statac refresh` doesn't fetch them.

## How to fix it

Add the site's full URL to `statac.yaml` when you know it. Everything
that was waiting is published from then on.

## Example

`content/sitemap.md`:

```yaml
---
permalink: /sitemap.xml
layout: sitemap.xml
needs: url
---
```

Once `statac.yaml` has this line, the sitemap is published:

```yaml
url: https://example.org/
```
