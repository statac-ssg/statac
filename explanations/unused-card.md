# Unused card

The site has the settings of a sharing card that no page is ever shared
with. Either no layout reads `page.card`, so no page asks for any card,
or the card is one of several, its settings in
`templates/og.<name>.yaml`, and no page picks it with `card: <name>`.

## What you'll see {#what-youll-see}

```text
warning  Unused card: no layout reads `page.card`
  Write the card's `<meta>` lines in a layout, or remove the card's settings and `templates/og/`.
  More: https://statac.dev/e/unused-card/
```

## Why Statac warns

A card is drawn only for a page whose layout reads `page.card`, since
that is where its address is written for a link preview to find. The
card's settings do nothing where nothing reads them, and the likeliest
reason is a step not yet taken, so Statac tells you.

## How to fix it

Where no layout reads `page.card`, write the lines that name the card in
the `<head>` of a layout:

```html
{% if page.card is defined %}
<meta property="og:image" content="{{ page.card.url | absolute }}">
{% if page.card.width %}
<meta property="og:image:width" content="{{ page.card.width }}">
<meta property="og:image:height" content="{{ page.card.height }}">
{% endif %}
<meta property="og:image:alt" content="{{ page.card.alt }}">
{% endif %}
```

The first test leaves the lines out of a page with no card. The test
`{% if page.card.width %}` leaves out the width and height where the
page is shared with a picture of its own whose size Statac can't tell.

Where no page picks a card, write `card: <name>` in the front matter of
the pages it is for, or hand it down to a folder's pages with
`pages.card: <name>`.

If you don't want the card, remove the card's settings. Once the site
has no card's settings at all, the files in `templates/og/` do nothing,
and can be removed too.

## Example

In `templates/og.wide.yaml`, a card no page picks:

```yaml
background: wide.png
```

With `card: wide` in a page's front matter, that page is shared with it.
