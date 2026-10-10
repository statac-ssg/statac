# Card not drawn

A page's sharing card couldn't be drawn from what the page says, so the
page has no card: `page.card` has no value on it, and a layout testing
for one writes no picture for it. The message says why:

- a text in the middle of the card doesn't fit the room it has, even at
  the smallest size Statac tries;
- the texts at the top and the bottom leave no room between them;
- the page's texts hold a character none of the card's fonts has.

Pages left without a card for the same reason are told together, with
how many there are and the first of them. Pages needing some of the same
characters are told together too, with the characters the most of them
need named first. Pages with nothing missing in common, such as one
needing an arrow and one needing letters of another script, are told
apart.

## What you'll see {#what-youll-see}

```text
warning  Card not drawn: the `title` of `content/post-1.md` doesn't fit, even at 45 pixels
  Give the page fewer words there, or the middle of the card more room.
  More: https://statac.dev/e/card-not-drawn/
```

## Why Statac warns

Statac never cuts a text short, lets one run off the card, or draws a
box where a character should be. A card it can't draw whole it doesn't
draw at all. The page is still built and published, so the build goes
on, and Statac tells you which pages are shared without a picture.

## How to fix it

For a text that doesn't fit, give those pages fewer words there: a
shorter `og_title` shortens a title on the card and nowhere else. Or give
the middle of the card more room, with a smaller `padding` or smaller
texts at the top and bottom.

For a character no font has, add a font that has it to the card's `font`.
Fonts are tried in the order written. `statac new --cards` writes two, Noto
Sans for text and Noto Emoji for emoji. Where one of them has the character
but isn't in `templates/og/`, the message says so:
`statac new --update --missing` writes it again, and then you name it in
the card's `font`.

To share a page with a picture of your own instead, name it with
`og_image`.

## Example

In `templates/og.yaml`:

```yaml
title.size: up to 75
```

In `content/post-1.md`, a title for the card alone:

```yaml
og_title: A shorter title
```
