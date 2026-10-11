# Describes no picture

A file under `content/` is named like a picture's description, such as
`photo-1.jpg.yaml`, but there is no picture of that name beside it.

## What you'll see {#what-youll-see}

```text
warning  Describes no picture: there is no `content/photo-1.jpg`
  Did you mean `photo-1.jpeg`?
  More: https://statac.dev/e/describes-no-picture/
```

## Why Statac warns

A file named after a picture, with `.yaml` added after the picture's
full name, tells Statac about that picture: its description, its
caption, and whether to leave it as it is. The file is never published.
If there is no picture beside it, there is nothing for it to describe.
Statac carries on without the file and warns you, in case the picture was
renamed or moved and its description left behind.

## How to fix it

Name the file after the picture it describes, including the picture's
file extension, and keep the two in the same folder. If the picture is gone,
delete the file.

## Example

`content/photo-1.jpg.yaml`, beside `content/photo-1.jpeg`:

```yaml
alt: A gradient from blue to orange
```

Once the file is renamed to `content/photo-1.jpeg.yaml`, it describes
the picture.
