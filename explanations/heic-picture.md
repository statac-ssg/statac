# HEIC picture

A page shows a picture saved as HEIC or HEIF, as many phones save their
photographs. Statac published the file as it is and made no copies of it.

## What you'll see {#what-youll-see}

```text
warning  HEIC picture: `content/photo-1.heic` is published as it is, with no copies made
  Export it as a JPEG: most browsers can't show a HEIC picture.
  More: https://statac.dev/e/heic-picture/
```

## Why Statac warns

Statac doesn't read HEIC, so it can't make the smaller AVIF and JPEG
copies it makes of a photograph, nor write the picture's width and height
into the page. Few browsers show a HEIC picture at all: most readers
would see a broken picture where it should be. The file is published all
the same, since that is what the page asks for, and Statac says so.

## How to fix it

Export the photograph as a JPEG from the program that holds it, keep the
JPEG beside the writing in place of the HEIC file, and name that where
the picture is shown. Statac then publishes it as it does any photograph.

## Example

In `content/post-1.md`:

```markdown
![A gradient from blue to orange](photo-1.heic)
```

With `photo-1.jpg` beside the post and
`![A gradient from blue to orange](photo-1.jpg)` in it, the photograph is published as copies every browser shows.
