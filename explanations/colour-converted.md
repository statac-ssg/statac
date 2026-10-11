# Colour converted

A photograph has a colour profile other than sRGB, such as Display P3 or
Adobe RGB. Cameras and phones often write these profiles. When Statac made
the copies of the photograph that it publishes, it converted their colours
to sRGB.

## What you'll see {#what-youll-see}

```text
note  Colour converted: the copies of `content/photo-1.jpg` are in sRGB, converted from `Display P3`
  More: https://statac.dev/e/colour-converted/
```

## Why Statac says so

The copies have no colour profile, and a browser shows a picture without
one as sRGB. Without the conversion, the colours would look duller or
shifted. Converting them makes the copies look the same as the photograph.

Some colours in the wider colour space don't exist in sRGB, so Statac
replaces each of them with the nearest sRGB colour. On most photographs you
can't see the difference, but on one with very strong colours you sometimes
can. Statac shows this note once for each such photograph that a page
shows, so the change never comes as a surprise. Your own file is never
changed.

## How to fix it

Nothing is wrong. Once you have understood the note, you can quieten it by
name.

To publish the photograph's own file, with its profile, put a file next to
it that is named after it, such as `photo-1.jpg.yaml` for `photo-1.jpg`, and
write `optimise: false` in that file. Statac then makes no copies of the
photograph.

## Example

To stop showing the note, add this to `statac.yaml`:

```yaml
quiet: [colour-converted]
```
