# Colour converted

An image has a colour profile other than sRGB, such as Display P3 or
Adobe RGB. Cameras and phones often write these profiles.
When Statac made the resized copies of the image that it publishes, it
converted their colours to sRGB.

## What you'll see {#what-youll-see}

```text
note  Colour converted: Statac converted the resized copies of `content/photo-1.jpg` to sRGB from `Display P3`
  More: https://statac.dev/e/colour-converted/
```

## Why Statac says so

The resized copies have no colour profile, and a browser shows an image
without one as sRGB. Without the conversion, the colours would look duller
or shifted. Converting them makes the copies look the same as the
original.

Some colours in the wider colour space don't exist in sRGB, so Statac
replaces each of them with the nearest sRGB colour. On most images you
can't see the difference, but on one with very strong colours you sometimes
can. Statac shows this note once for each such image that a page
shows, so the change never comes as a surprise. Statac never changes
your own file.

## How to fix it

Nothing is wrong. Once you have understood the note, you can quieten it by
name.

To publish the image's own file, with its profile, give it a sidecar
file, such as `photo-1.jpg.yaml` for `photo-1.jpg`, and write
`optimise: false` in it. Statac then makes no resized copies of the
image.

## Example

`content/photo-1.jpg.yaml`, beside `content/photo-1.jpg`:

```yaml
optimise: false
```

Statac then publishes `photo-1.jpg` itself, with its Display P3 profile,
in place of resized copies, and shows no note.
