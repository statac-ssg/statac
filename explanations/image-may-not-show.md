# Sharing image may not show

A page names the picture it is shared with, with `og_image`, and the file
is of a kind that some link previews don't show: a WebP or an SVG, say.
Statac uses the picture all the same. It is published as it is, and
`page.card` answers with its address, exactly as for any other picture.

A picture several pages name is told once, with how many pages there are,
where the first of them names it.

## What you'll see {#what-youll-see}

```text
warning  Sharing image may not show: `content/cover.webp` is a WebP
  For a picture more link previews show, save it as a PNG.
  More: https://statac.dev/e/image-may-not-show/
```

## Why Statac warns

A link preview is made by the site or the program a link is pasted into,
each with a reader of its own for the picture. Some kinds of picture are
read by some of them and not by others, and which those are changes from
year to year. A PNG is the kind most widely shown. Where a preview
can't read the picture, the page is shared with no picture at all, and
nothing on your own site shows that it happened.

Statac goes by what the file holds, not by its name, so a WebP saved as
`cover.png` is told as a WebP.

The picture is yours to choose, so Statac uses the one you name, never
sets it aside for the kind of file it is, and never draws a card in its
place.

## How to fix it

For a picture more previews show, save it as a PNG and name that with
`og_image`.

Where the previews that matter to you show the picture as it is, nothing
needs changing. Once you have understood it, quieten the warning by
name.

## Example

In `content/post-1/index.md`:

```yaml
og_image: cover.webp
```

With `cover.png` beside the post and `og_image: cover.png` in it, the
warning goes.

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [image-may-not-show]
```
