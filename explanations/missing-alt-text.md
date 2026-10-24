# Missing alt text

A page shows an image but has no alt text for a reader who can't see it:
the `alt` of its tag is empty or missing.

## What you'll see {#what-youll-see}

```text
note  Missing alt text: `photo-1.jpg`
  Describe it in the brackets, or mark it decorative.
  More: https://statac.dev/e/missing-alt-text/
```

## Why Statac says so

A reader using a screen reader, or whose browser didn't load the image,
gets the image's alt text instead. If there is none, some hear the file's
name read out and others hear nothing at all, and they miss what the
image was there to say. That is part of the page Statac built, so
Statac tells you. It is a note, and nothing is left out of the site.

Statac reports this for:

- an image in Markdown with empty brackets, `![](photo-1.jpg)`, that no
  sidecar file describes
- an image that a template or snippet shows with `page.image`, if the
  template uses the image's `alt` and the image has none
- an `<img>` in the content with no `alt` at all

An image marked decorative is never reported.

## How to fix it

Say what the image shows in a few words, as you would to someone on the
phone.

In Markdown, write it in the brackets:
`![A gradient from blue to orange](photo-1.jpg)`.

For any image under `content/`, you can write the alt text once, in its
sidecar file: a file beside the image with the image's name and `.yaml`
on the end. Markdown with empty brackets and `page.image` both use it. In
`photo-1.jpg.yaml`:

```yaml
alt: A gradient from blue to orange
```

An image that is only decoration, such as a flourish between sections,
needs no alt text. Mark it decorative in the same file, and readers who
can't see it are told nothing about it:

```yaml
decorative: true
```

A template or snippet marks an image decorative with
`page.image("photo-1.jpg", decorative=true)`, and an `<img>` of your own
does it with `alt=""`.

Once you have understood the note, you can quieten it by name.

## Example

In `content/post-1.md`:

```markdown
![](photo-1.jpg)
```

If you write it as `![A gradient from blue to orange](photo-1.jpg)`, the
note goes away.

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [missing-alt-text]
```
