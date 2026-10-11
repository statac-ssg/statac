# Picture with no description

A page shows a picture but has no description of it for a reader who
can't see it: the `alt` of its tag is empty or missing.

## What you'll see {#what-youll-see}

```text
note  Picture with no description: `photo-1.jpg`
  Describe it in the brackets, or mark it decorative.
  More: https://statac.dev/e/undescribed-picture/
```

## Why Statac says so

A reader using a screen reader, or whose browser didn't load the picture,
gets the picture's description instead. If there is none, some hear the
file's name read out and others hear nothing at all, and they miss what
the picture was there to say. That is part of the page Statac built, so
Statac tells you. It is a note, and nothing is left out of the site.

Statac reports this for:

- a picture in Markdown with empty brackets, `![](photo-1.jpg)`, that no
  file beside it describes
- a picture that a template or snippet shows with `page.photo`, if the
  template uses the picture's `alt` and the picture has none
- an `<img>` in the writing with no `alt` at all

A picture marked as decoration is never reported.

## How to fix it

Say what the picture shows in a few words, as you would to someone on the
phone.

In Markdown, write it in the brackets:
`![A gradient from blue to orange](photo-1.jpg)`.

For any picture under `content/`, you can write the description once, in
a file beside the picture that has the picture's name with `.yaml` on the
end. Markdown with empty brackets and `page.photo` both use it. In
`photo-1.jpg.yaml`:

```yaml
alt: A gradient from blue to orange
```

A picture that is only decoration, such as a flourish between sections,
needs no description. Say so in the same file, and readers who can't see
it are told nothing about it:

```yaml
decorative: true
```

A template or snippet marks a picture as decoration with
`page.photo("photo-1.jpg", decorative=true)`, and an `<img>` of your own
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
quiet: [undescribed-picture]
```
