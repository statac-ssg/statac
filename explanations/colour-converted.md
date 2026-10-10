# Colour converted

A photograph carries a colour profile that isn't sRGB, such as Display P3
or Adobe RGB, as cameras and phones often write. Statac converted its
colours to sRGB when it made the copies it publishes.

## What you'll see {#what-youll-see}

```text
note  Colour converted: the copies of `content/photo-1.jpg` are in sRGB, converted from `Display P3`
  More: https://statac.dev/e/colour-converted/
```

## Why Statac says so

The copies carry no colour profile, and a browser shows a picture without
one as sRGB. Left unconverted, the photograph's colours would come out
duller or shifted, so Statac converts them and the copies look as the
photograph does.

Colours the wider space holds and sRGB doesn't are brought to the nearest
sRGB has. On most photographs the difference can't be seen; on one with
very strong colours it sometimes can. Statac says so, once for each such
photograph a page shows, so the change is never a surprise. Your own file
is never altered.

## How to fix it

Nothing is wrong. Once you have understood it, quieten it by name.

To publish a photograph as the file it is, its profile kept, write
`optimise: false` in a file beside it named after it, such as
`photo-1.jpg.yaml` for `photo-1.jpg`. Statac then makes no copies of it.

## Example

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [colour-converted]
```
