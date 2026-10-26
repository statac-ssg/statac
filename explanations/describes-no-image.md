# Describes no image

A file under `content/` is named like an image's sidecar file, such as
`photo-1.jpg.yaml`, but there is no image of that name beside it.

## What you'll see {#what-youll-see}

```text
warning  Describes no image: there is no `content/photo-1.jpg`
  Did you mean `photo-1.jpeg`?
  More: https://statac.dev/e/describes-no-image/
```

## Why Statac warns

A sidecar file, named after an image with `.yaml` added after the
image's full name, tells Statac about that image: its alt text, its
caption, and whether to publish it unchanged. Statac never publishes
the sidecar file.
If there is no image beside it, there is nothing for it to describe.
Statac carries on without the file and warns you, in case the image was
renamed or moved and its sidecar file left behind.

## How to fix it

Name the file after the image it describes, including the image's file
extension, and keep the two in the same folder. If the image is gone,
delete the file.

## Example

`content/photo-1.jpg.yaml`, beside `content/photo-1.jpeg`:

```yaml
alt: A gradient from blue to orange
```

Once the file is renamed to `content/photo-1.jpeg.yaml`, it describes
the image.
