# Card not drawn

Statac couldn't draw a page's social card from what the page says, so
the page has no card. `page.card` has no value on that page, and a layout
that tests for one writes no image for it. The message gives one of
these reasons:

- a text in the middle of the card doesn't fit the space it has, even at
  the smallest size Statac tries;
- the texts at the top and the bottom leave no space between them;
- a text contains a character that none of its fonts has.

Statac reports pages left without a card for the same reason together,
with the number of pages and the first of them. It also reports pages
that need some of the same characters together, and lists first the
characters the most pages need. It reports pages whose missing
characters have nothing in common separately, such as one that needs an
arrow and one that needs letters from another script.

## What you'll see {#what-youll-see}

```text
warning  Card not drawn: the `title` of `content/post-1.md` doesn't fit, even at 45 pixels
  templates/og.yaml, line 6
   6 │ title.size: up to 75
     │             ^^^^^^^^
  Give the page fewer words there, or the middle of the card more room.
  More: https://statac.dev/e/card-not-drawn/
```

## Why Statac warns

Statac never cuts a text short, lets a text run off the card, or draws
a box in place of a character. If it can't draw the whole card, it
doesn't draw the card at all. The page is still built and published, so
the build carries on, and Statac tells you which pages will be shared
without an image.

## How to fix it

**A text in the middle that doesn't fit.** Give those pages fewer words
there. A shorter `og_title` shortens the title on the card and nowhere
else. Or give the middle of the card more space, with a smaller
`padding` or smaller texts at the top and bottom.

**No space between the top and bottom texts.** Make the texts at the top
and the bottom smaller or shorter, or give the card a smaller `padding`.

**A character no font has.** Add a font that has it to the fonts the
message names: the card's `font`, or a text's own, such as `title.font`.
Statac tries the fonts in the order they are written. If a font in
`templates/og/` that the list doesn't name has every character missing,
the message names it. Otherwise, put the file of a font that has them in
`templates/og/`, such as an emoji font for emoji, and add it to the list.

To share a page with an image of your own instead, name that image in
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
