# Describes no picture

A file under `content/` is named as the description of a picture, such as
`photo-1.jpg.yaml`, and no picture of that name is beside it.

## What you'll see {#what-youll-see}

```text
warning  Describes no picture: there is no `content/photo-1.jpg`
  Did you mean `photo-1.jpeg`?
  More: https://statac.dev/e/describes-no-picture/
```

## Why Statac warns

A file named after a picture, with `.yaml` after the picture's whole name,
holds what Statac should know about that picture: its description, its
caption, whether to leave it as it is. It is never published. With no
picture beside it there is nothing for it to describe, so Statac goes on
without it and says so, in case the picture was renamed or moved and its
description left behind.

## How to fix it

Name the file after the picture it describes, the picture's ending
included, and keep the two in the same folder. If the picture is gone,
delete the file.

## Example

`content/photo-1.jpg.yaml` beside `content/photo-1.jpeg`:

```yaml
alt: A gradient from blue to orange
```

Renamed `content/photo-1.jpeg.yaml`, it describes the picture.
