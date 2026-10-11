# Card not drawn

Statac couldn't draw a page's sharing card from what the page says, so
the page has no card. `page.card` has no value on that page, and a layout
that tests for one writes no picture for it. The message gives one of
these reasons:

- a text in the middle of the card doesn't fit the space it has, even at
  the smallest size Statac tries;
- the texts at the top and the bottom leave no space between them;
- the page's texts contain a character that none of the card's fonts has.

Pages left without a card for the same reason are reported together,
with the number of pages and the first of them. Pages that need some of
the same characters are reported together too, and the characters the
most pages need are listed first. Pages whose missing
characters have nothing in common, such as one that needs an arrow and
one that needs letters from another script, are reported separately.

## What you'll see {#what-youll-see}

```text
warning  Card not drawn: the `title` of `content/post-1.md` doesn't fit, even at 45 pixels
  Give the page fewer words there, or the middle of the card more room.
  More: https://statac.dev/e/card-not-drawn/
```

## Why Statac warns

Statac never cuts a text short, lets a text run off the card, or draws
a box in place of a character. If it can't draw the whole card, it
doesn't draw the card at all. The page is still built and published, so
the build carries on, and Statac tells you which pages will be shared
without a picture.

## How to fix it

If a text doesn't fit, give those pages fewer words there. A shorter
`og_title` shortens the title on the card and nowhere else. Or give the
middle of the card more space, with a smaller `padding` or smaller texts
at the top and bottom.

If no font has a character, add a font that has it to the card's `font`.
Fonts are tried in the order they are written. `statac new --cards`
writes two: Noto Sans for text and Noto Emoji for emoji. If one of these
has the character but isn't in `templates/og/`, the message says so. Run
`statac new --update --missing` to write the font again, then add it to
the card's `font`.

To share a page with a picture of your own instead, give that picture in
the page's `og_image`.

## Example

In `templates/og.yaml`:

```yaml
title.size: up to 75
```

In `content/post-1.md`, a title used only on the card:

```yaml
og_title: A shorter title
```
