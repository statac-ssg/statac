# Malformed call

Statac found what looks like a snippet call in your content, `@name(...)`,
but couldn't read it. The message says what it found and points at the
place to fix, such as a bracket that is never closed.

## What you'll see {#what-youll-see}

```text
error  Malformed call: `@image(` is never closed
  Close it with `)`.
  More: https://statac.dev/e/malformed-call/
```

## Why Statac stops

A call Statac can't read would appear on the page as the text you typed,
or absorb the content after it. Neither is what you meant, so Statac stops
instead of publishing either.

## How to fix it

Look at the place the caret points to. Common causes:

- A call with no closing `)` before the paragraph ends. Close it in the
  same paragraph.
- A quoted value that isn't closed on the line where it starts. End the
  value with the same quote it opened with.
- Text after a value's closing quote. Put the whole value inside the
  quotes.
- A snippet name with letters other than a to z. Name the snippet using
  only the letters a to z, digits and `_`.
- A call to a snippet that shows `body`, with no `@end` after it. A call
  like this wraps the content below it, and an `@end` on a line of its own
  marks where that content ends.
- An `@end` with no call above it for it to close. Only a call to a
  snippet that shows `body`, on a line of its own, wraps content up to an
  `@end`.
- A call to a snippet that shows `body`, in the middle of a sentence. Put
  it on a line of its own, with the content it wraps below it and `@end`
  on a line after that.
- An `@url(...)` given anything other than a single URL starting with
  `/`: no URL, two URLs, a value with a name, or a URL that doesn't start
  with a single `/`. Write the URL the page is published at, as in
  `@url(/about/)`.

To show a call as text instead, write a backslash before the `@`:
`\@image('photo-1.jpg')`.

## Example

In `content/post-1.md`:

```markdown
A photograph: @image('photo-1.jpg'
```

The call is never closed. Close it with `)`:

```markdown
A photograph: @image('photo-1.jpg')
```

If `snippets/note.html` shows `{{ body }}`, a call to it wraps the content
below it, and that content ends at an `@end` on its own line:

```markdown
@note('warning')
The content the note wraps.
@end
```
