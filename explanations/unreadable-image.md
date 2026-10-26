# Unreadable image

A page shows an image, but Statac can't read the file as the format its
first bytes say it is. The file may be cut short (for example,
if a copy or a download stopped halfway), or damaged, or saved in a
variant of JPEG, PNG or WebP that Statac can't open. The message names the
file and says which of these it is.

## What you'll see {#what-youll-see}

```text
error  Unreadable image: `content/photo-1.jpg` is cut short or damaged
  Copy or export the file again.
  More: https://statac.dev/e/unreadable-image/
```

## Why Statac stops

Statac publishes a JPEG, PNG or WebP image as resized copies made from
it, and it can't make resized copies from a file that doesn't open. A
browser would most likely show readers a broken image, so the build stops
rather than publish one.

`check` reads only the start of each image, so it finds only files whose
start is wrong. `build` finds a file that is damaged further in, when it
opens the image to make the resized copies.

## How to fix it

Copy the file again from where it came from, or export it again from the
program that made it.

If the file is the way you want it and browsers can show it, you can
publish it unchanged. Put `optimise: false` in its sidecar file, `photo-1.jpg.yaml` for
`photo-1.jpg`. Statac then publishes the original file and makes no
resized copies of it.

## Example

Once `content/photo-1.jpg` has been copied again in full, the page
builds.
