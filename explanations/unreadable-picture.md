# Unreadable picture

A page shows a picture, but Statac can't read the file as the kind of
picture its first bytes say it is. The file may be cut short (for example,
if a copy or a download stopped halfway), or damaged, or saved in a
variant of JPEG, PNG or WebP that Statac can't open. The message names the
file and says which of these it is.

## What you'll see {#what-youll-see}

```text
error  Unreadable picture: `content/photo-1.jpg` is cut short or damaged
  Copy or export the file again.
  More: https://statac.dev/e/unreadable-picture/
```

## Why Statac stops

Statac publishes a photograph as smaller copies made from it, and it
can't make copies from a file that doesn't open. A browser would most likely show
readers a broken picture, so the build stops rather than publish one.

`check` reads only the start of each picture, so it finds only files
whose start is wrong. `build` finds a file that is damaged further in,
when it opens the photograph to make the copies.

## How to fix it

Copy the file again from where it came from, or export it again from the
program that made it.

If the file is as you intend and browsers can show it, you can publish it
unchanged. Put `optimise: false` in a file beside it, named after it, such
as `photo-1.jpg.yaml` for `photo-1.jpg`. Statac then publishes the
original file and makes no copies of it.

## Example

Once `content/photo-1.jpg` has been copied again in full, the page
builds.
