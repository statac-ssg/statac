# Unusable card file

A sharing card's settings name a background or a font that Statac can't
draw a card with. The message names the file and says what is wrong
with it:

- a background that isn't 1,200 pixels wide and 630 tall;
- a background that isn't a JPEG, a PNG or a still WebP;
- a font file that doesn't read as a font.

## What you'll see {#what-youll-see}

```text
error  Unusable card file: `templates/og/background.png` is 1,600 by 900 pixels, not 1,200 by 630
  Save the background 1,200 pixels wide and 630 tall.
  More: https://statac.dev/e/unusable-card-file/
```

## Why Statac stops

Every card those settings describe is drawn over that background, in
those fonts. Statac never stretches or crops a background to fit, because
you decide where the text sits on it. It also never draws in a font you
didn't name. If no card can be drawn, every page that reads `page.card`
would be shared with no picture, so the build stops.

## How to fix it

Save the background at 1200 by 630 pixels, as a PNG or a JPEG. Name
TrueType or OpenType font files, ending `.ttf`, `.otf` or `.ttc`. A web
font ending `.woff2` is a different kind of file, even if you rename it.

## Example

Once `templates/og/background.png` is saved again at 1200 by 630, the
cards are drawn.
