# Picture with no description

A page shows a picture and says nothing of it for a reader who can't see
it: the `alt` of its tag is empty or missing.

## What you'll see {#what-youll-see}

```text
note  Picture with no description: `photo-1.jpg`
  Describe it in the brackets, or mark it decorative.
  More: https://statac.dev/e/undescribed-picture/
```

## Why Statac says so

A reader using a screen reader, or whose picture didn't arrive, is given a
picture's description in its place. With none, some hear the file's name
read out and others nothing at all, and what the picture was there to say
is lost to them. That is part of the page Statac built, so it says so. It
is a note, and nothing is left out of the site.

Statac says so for:

- a picture in Markdown with empty brackets, `![](photo-1.jpg)`, that no
  file beside it describes
- a picture a template or snippet shows with `page.photo`, where the
  template reads the picture's `alt` and it has none
- an `<img>` in the writing with no `alt` at all

A picture marked as decoration is never mentioned.

## How to fix it

Say what the picture shows, in a few words, as you would to someone on the
phone.

In Markdown, write it in the brackets:
`![A gradient from blue to orange](photo-1.jpg)`.

For any picture under `content/`, you can write it once in a file beside
the picture, named after it with `.yaml` on the end. Markdown with empty
brackets and `page.photo` both use it. In `photo-1.jpg.yaml`:

```yaml
alt: A gradient from blue to orange
```

A picture that is only decoration, such as a flourish between sections,
needs no description. Say so in the same file, and readers who can't see
it are told nothing of it:

```yaml
decorative: true
```

A template or snippet says the same with
`page.photo("photo-1.jpg", decorative=true)`, and an `<img>` of your own
with `alt=""`.

Once you have understood it, the note can be quietened by name.

## Example

In `content/post-1.md`:

```markdown
![](photo-1.jpg)
```

Written `![A gradient from blue to orange](photo-1.jpg)`, the note goes.

In `statac.yaml`, to stop it being shown:

```yaml
quiet: [undescribed-picture]
```
