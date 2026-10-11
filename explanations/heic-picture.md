# HEIC picture

A page shows a picture saved as HEIC or HEIF, the format many phones
save photographs in. Statac published the file as it is and made no copies of it.

## What you'll see {#what-youll-see}

```text
warning  HEIC picture: `content/photo-1.heic` is published as it is, with no copies made
  Export it as a JPEG: most browsers can't show a HEIC picture.
  More: https://statac.dev/e/heic-picture/
```

## Why Statac warns

Statac can't read HEIC, so it can't make the smaller AVIF and JPEG
copies it usually makes of a photograph, or write the picture's width and
height into the page. Few browsers can show a HEIC picture at all, so
most readers would see a broken picture in its place. Statac publishes
the file anyway, because the page asks for it, and warns you.

## How to fix it

Export the photograph as a JPEG from the program you keep it in. Put the
JPEG beside the page's file in place of the HEIC file, and use the JPEG's
name where the picture is shown. Statac then publishes it as it does any
other photograph.

## Example

In `content/post-1.md`:

```markdown
![A gradient from blue to orange](photo-1.heic)
```

If you put `photo-1.jpg` beside the post and write
`![A gradient from blue to orange](photo-1.jpg)` in it instead, Statac
publishes copies of the photograph that every browser can show.
