# Unreadable image

A page or its social card shows an image that Statac can't use: it can't
read the file, or can't make the resized copies it publishes. The
message names the file and gives the reason, which is one of these:

- The file is cut short (for example, if a copy or a download stopped
  halfway), or damaged.
- The file is saved in a form Statac doesn't read, such as a variant of
  JPEG, PNG or WebP that Statac can't open. Statac goes by the format
  the file's first bytes say it is, not by its name.
- The image is too large to open: its pixels would take more than 1 GB
  of memory. The message gives its size in pixels.
- Statac opened the image, but can't make a resized copy of it. The
  message ends with the reason, such as `too large for JPEG`.
- A file in `fetched/`, such as a video's thumbnail, isn't a JPEG, a PNG
  or a still WebP, the kinds Statac makes resized copies of.

## What you'll see {#what-youll-see}

```text
error  Unreadable image: `content/photo-1.jpg` is cut short or damaged
  content/post-1.md, line 1
   1 │ ![A gradient from blue to orange](photo-1.jpg)
     │                                   ^^^^^^^^^^^
  Copy or export the file again.
  More: https://statac.dev/e/unreadable-image/
```

## Why Statac stops

Statac publishes a JPEG, PNG or WebP image as resized copies made from
it, not as the file itself. If it can't open the file, or can't make a
copy, a browser would most likely show readers a broken image, so the
build stops rather than publish one.

`check` reads only the start of each image, so it finds only files whose
start is wrong, and files in `fetched/` of another kind. `build` opens
each image to make its resized copies, so it also finds a file damaged
further in, an image too large to open, and a copy it can't make.

## How to fix it

**Cut short or damaged.** Copy the file again from where it came from,
or export it again from the program that made it.

**Saved in a form Statac doesn't read.** Export the image again as an
ordinary JPEG, PNG or WebP. Or, if browsers can show the file, publish
it unchanged, as below.

**Too large to open.** Export a smaller version of the image, with fewer
pixels. Or publish it unchanged, as below.

**Can't make a resized copy.** If the reason says the image is too
large, export a smaller version of it: no side of a JPEG copy can be
more than 65,535 pixels. Or publish it unchanged, as below.

**Not a JPEG, a PNG or a still WebP, in `fetched/`.** Delete the file,
then run `statac refresh`, which fetches it again.

To publish an image unchanged, put `optimise: false` in its sidecar
file, `photo-1.jpg.yaml` for `photo-1.jpg`. Statac then publishes the
original file and makes no resized copies of it. Only an image in
`content/` has a sidecar file. For an image in `static/`,
`optimise_images: false` in `statac.yaml` publishes every image on the
site unchanged. Statac never publishes a social card's background, or a
file in `fetched/`, unchanged.

## Example

Once `content/photo-1.jpg` has been copied again in full, the page
builds.
