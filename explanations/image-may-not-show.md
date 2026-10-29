# Social image may not show

A page uses `og_image` to choose the social image it is shared with, and
the file is in a format that some link previews don't show, such as WebP
or SVG. Statac still uses the image. It is published unchanged, and
`page.card` has its URL, the same as for any other image.

If several pages use the same image, Statac reports it once, at the
first page that uses it, and says how many pages there are.

## What you'll see {#what-youll-see}

```text
warning  Social image may not show: `content/post-1/cover.webp` is a WebP
  content/post-1/index.md, line 2
   2 │ og_image: cover.webp
     │           ^^^^^^^^^^
  For an image more link previews show, save it as a PNG.
  More: https://statac.dev/e/image-may-not-show/
```

## Why Statac warns

A link preview is made by the site or program that the link is pasted
into, and each one has its own code for reading the image. Some of them
can read a given format and others can't, and which ones can changes from
year to year. PNG is the format most widely shown. If a preview can't read
the image, the page is shared with no image at all, and nothing on your
own site shows that this happened.

Statac checks what the file contains, not its name, so a WebP saved as
`cover.png` is reported as a WebP.

The image is your choice, so Statac uses the one you name. It never
sets it aside because of its format, and never draws a card in its
place.

## How to fix it

To have the image shown in more previews, save it as a PNG and set
`og_image` to that file.

If the previews that matter to you already show the image, nothing
needs changing. Once you have understood the warning, quieten it by name.

## Example

In `content/post-1/index.md`:

```yaml
og_image: cover.webp
```

With `cover.png` beside the post and `og_image: cover.png` in the post,
the warning goes away.

To stop Statac showing the warning, add this to `statac.yaml`:

```yaml
quiet: [image-may-not-show]
```
