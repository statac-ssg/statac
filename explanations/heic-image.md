# HEIC image

A page shows an image saved as HEIC or HEIF, the format many phones save
photos in. Statac published the file unchanged and made no resized
copies of it.

## What you'll see {#what-youll-see}

```text
warning  HEIC image: Statac publishes `content/photo-1.heic` unchanged, with no resized copies
  content/post-1.md, line 1
   1 │ ![A gradient from blue to orange](photo-1.heic)
     │                                   ^^^^^^^^^^^^
  Export it as a JPEG: most browsers can't show a HEIC image.
  More: https://statac.dev/e/heic-image/
```

## Why Statac warns

Statac can't read HEIC, so it can't make the resized AVIF and JPEG
copies it usually makes of an image, or write the image's width and
height into the page. Few browsers can show a HEIC image at all, so most
readers would see a broken image in its place. Statac publishes
the file anyway, because the page asks for it, and warns you.

## How to fix it

Export the image as a JPEG from the program you keep it in. Put the
JPEG beside the page's file in place of the HEIC file, and use the JPEG's
name where the image is shown. Statac then publishes it the same way as
any other image.

## Example

In `content/post-1.md`:

```markdown
![A gradient from blue to orange](photo-1.heic)
```

If you put `photo-1.jpg` beside the post and write
`![A gradient from blue to orange](photo-1.jpg)` in it instead, Statac
publishes resized copies of the image that every browser can show.
