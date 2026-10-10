# No site address

The site has no address yet, `url` in `statac.yaml`, and something that
can't do without one is left out of the built site: a page that says
`needs: url`, the feeds of the site's lists, the sharing cards, or the
site's webmentions.

## What you'll see {#what-youll-see}

```text
note  No site address: sharing cards are left out until the site has a `url`
  Add the site's address to `statac.yaml`, as `url: https://example.org/`.
  More: https://statac.dev/e/no-site-address/
```

## Why Statac says so

Some files mean nothing without the site's whole address. A feed reader
follows each post's link from a file of its own, away from the site, and a
sitemap lists every page by it. Rather than publish them with addresses
that lead nowhere, Statac leaves them out until `statac.yaml` says where
the site lives, and names each, since nothing else would show it missing.

A page asks for this with `needs: url` in its front matter; feeds always
wait. Sharing cards wait as well: a link preview is handed a card's whole
address, so until there is a `url` no card is drawn and `page.card` has
no value. Webmentions wait too: a mention names the page it is of by its
whole address, so without `url` Statac can't tell which page that is,
and `statac refresh` doesn't ask for them.

## How to fix it

Add the site's address to `statac.yaml` when you know it. What waited is
published from then on.

## Example

`content/sitemap.md`:

```yaml
---
permalink: /sitemap.xml
layout: sitemap.xml
needs: url
---
```

With this line in `statac.yaml`, it is published:

```yaml
url: https://example.org/
```
