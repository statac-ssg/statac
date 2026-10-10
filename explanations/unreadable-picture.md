# Unreadable picture

A page shows a picture whose file Statac can't read as the picture its
opening bytes say it is. The file may be cut short, as when a copy or a
download stopped halfway, or damaged, or written in a form of JPEG, PNG or
WebP that Statac doesn't open. The message names the file and says which.

## What you'll see {#what-youll-see}

```text
error  Unreadable picture: `content/photo-1.jpg` is cut short or damaged
  Copy or export the file again.
  More: https://statac.dev/e/unreadable-picture/
```

## Why Statac stops

A photograph is published as smaller copies made from it, and none can be
made from a file that doesn't open. A browser would most likely show
readers a broken picture, so the build stops rather than publish one.

`check` reads only the start of each picture, so it finds a file whose
start is wrong. A file damaged further in is found by `build`, when it
opens the photograph to make its copies.

## How to fix it

Copy the file again from where it came, or export it again from the
program that made it.

If the file is as you mean it and browsers show it, leave it as it is with
`optimise: false` in a file beside it named after it, such as
`photo-1.jpg.yaml` for `photo-1.jpg`. Statac then publishes the file
itself and makes no copies of it.

## Example

`content/photo-1.jpg` copied again whole, the page builds.
