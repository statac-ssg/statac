# Sharing image may not show

A page uses `og_image` to choose the picture it is shared with, and the
file is of a kind that some link previews don't show, such as a WebP or an
SVG. Statac still uses the picture. It is published unchanged, and
`page.card` has its address, the same as for any other picture.

If several pages use the same picture, Statac reports it once, at the
first page that uses it, and says how many pages there are.

## What you'll see {#what-youll-see}

```text
warning  Sharing image may not show: `content/cover.webp` is a WebP
  For a picture more link previews show, save it as a PNG.
  More: https://statac.dev/e/image-may-not-show/
```

## Why Statac warns

A link preview is made by the site or program that the link is pasted
into, and each one has its own code for reading the picture. Some of them
can read a given kind of picture and others can't, and which ones can
changes from year to year. PNG is the kind most widely shown. If a
preview can't read the picture, the page is shared with no picture at
all, and nothing on your own site shows that this happened.

Statac checks what the file contains, not its name, so a WebP saved as
`cover.png` is reported as a WebP.

The picture is your choice, so Statac uses the one you name. It never
sets it aside because of its kind of file, and never draws a card in its
place.

## How to fix it

To have the picture shown in more previews, save it as a PNG and set
`og_image` to that file.

If the previews that matter to you show the picture as it is, nothing
needs changing. Once you have understood the warning, quieten it by name.

## Example

In `content/post-1/index.md`:

```yaml
og_image: cover.webp
```

With `cover.png` beside the post and `og_image: cover.png` in the post,
the warning goes away.

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [image-may-not-show]
```
