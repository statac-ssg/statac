# Unused card

The site has settings for a social card, but no page is ever shared
with it. Either no layout reads `page.card`, so no page asks for any
card, or the card is one of several, with its settings in
`templates/og.<name>.yaml`, and no page picks it with `card: <name>`.

## What you'll see {#what-youll-see}

```text
warning  Unused card: no page picks `wide`
  templates/og.wide.yaml
  Write `card: wide` on the pages it is for, or remove the card's settings.
  More: https://statac.dev/e/unused-card/
```

## Why Statac warns

Statac draws a card only for a page whose layout reads `page.card`,
because that is where the layout writes the card's URL for link
previews to find. If nothing reads the card's settings, they do nothing. The
likeliest reason is a step you haven't taken yet, so Statac tells you.

## How to fix it

**No layout reads `page.card`.** Add the lines that point to the card to
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

The first test leaves these lines out of any page with no card. The
test `{% if page.card.width %}` leaves out the width and height if the
page is shared with its own social image, whose size Statac can't tell.

**No page picks the card.** Write `card: <name>` in the front matter of
the pages it is for, or cascade it to a folder's pages with
`pages.card: <name>`.

If you don't want the card, remove its settings. Once the site has no
card settings at all, the files in `templates/og/` do nothing, and you
can remove them too.

## Example

The site's layout reads `page.card`, and `templates/og.wide.yaml` holds
a card that no page picks:

```yaml
background: wide.png
font: text.ttf

title.text: "{{ page.og_title }}"
title.place: centre
title.size: up to 75
title.colour: "#F2E9D3"
```

Once a page has `card: wide` in its front matter, that page is shared
with this card.
