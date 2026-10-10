# Location published

A JPEG, PNG or WebP beside your writing holds where it was taken, as a
phone or a camera with GPS writes into its photographs. Statac publishes
the file exactly as it is, so the position is published with it.

## What you'll see {#what-youll-see}

```text
note  Location published: `content/photo-1.jpg` holds where it was taken in its metadata
  Publish a copy without it, or add `location-published` to `quiet:` in `statac.yaml`.
  More: https://statac.dev/e/location-published/
```

## Why Statac says so

A file you put in `content/` is published byte for byte, whatever it
holds: Statac never changes it. Anyone who has the picture can read the
latitude and longitude in it. That is often what you mean, and often not,
and Statac can't tell which. So `statac check` says so, once for each such
file, and the choice stays with you.

The file itself is all that holds the position. The resized copies Statac
makes of a photograph a page shows hold the picture and nothing else. A
photograph a page shows and nothing links is published as its copies
alone, and nothing is said of it. One a page links, one no page shows and
one left alone by `optimise: false` are published as the files they are.

Statac looks for the position a camera writes, a latitude and a
longitude, in a JPEG, PNG or WebP. It looks at no other kind of file, and
for nothing else a file may say of a place. A file in `static/` is never
looked at, and nothing is said of it.

`statac build` and `statac serve` say nothing of this and don't count it.

## How to fix it

Nothing is wrong where you mean the position to be published. Quieten the
note by name and it is no longer shown.

To publish the picture without it, make a copy without the location and
put it in the file's place. Most programs that export photographs can
leave the location out, and so can a phone when it shares one. The copy
is yours to make: Statac publishes the file you give it.

## Example

`content/photo-1.jpg` stands beside a post that links it, and holds a
position. `statac check` says so. Once a copy exported without the
location stands in its place, nothing is said.

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [location-published]
```
