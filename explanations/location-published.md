# Location published

A JPEG, PNG or WebP image in `content/` records where it was taken. A
phone or a camera with GPS writes this into its photos.
Statac publishes the file unchanged, so the location is published with
it.

## What you'll see {#what-youll-see}

```text
note  Location published: `content/photo-1.jpg` holds where it was taken in its metadata
  Publish a copy without it, or add `location-published` to `quiet:` in `statac.yaml`.
  More: https://statac.dev/e/location-published/
```

## Why Statac says so

Statac publishes a file you put in `content/` byte for byte, whatever it
contains, and never changes it. Anyone who has the image can read the
latitude and longitude in it. Sometimes you mean that to happen and
sometimes you don't, and Statac can't tell which. So `statac check` tells
you, once for each such file, and leaves the choice to you.

Only the file itself contains the location. The resized copies Statac
makes of an image that a page shows contain the image and nothing else.
An image that a page shows but nothing links to is published only as its
resized copies, and no note is shown for it. An image that a page links
to, one that no page shows, and one marked `optimise: false` are all
published as the original files.

Statac looks for the location a camera writes, a latitude and a
longitude, in JPEG, PNG and WebP files. It doesn't look in any other kind
of file, or for any other information a file may hold about a place. A
Statac never checks a file in `static/`, and shows no note for it.

`statac build` and `statac serve` don't show this note or count it.

## How to fix it

If you mean the location to be published, nothing is wrong. Quieten the
note by name and it is no longer shown.

To publish the image without the location, make a copy without it and put
the copy in place of the file. Most programs that export photos can leave
the location out, and so can a phone when it shares a photo. You have to
make the copy yourself, because Statac publishes the file you give it.

## Example

`content/photo-1.jpg` is next to a post that links to it, and contains a
location. `statac check` shows the note. Once you replace the file with a
copy exported without the location, the note is no longer shown.

To stop Statac showing the note, add this to `statac.yaml`:

```yaml
quiet: [location-published]
```
