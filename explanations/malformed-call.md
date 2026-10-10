# Malformed call

Statac found what looks like a snippet call in your writing, `@name(...)`,
but couldn't read it. The message says what it found and points at the
place to fix, such as a bracket that is never closed.

## What you'll see {#what-youll-see}

```text
error  Malformed call: `@image(` is never closed
  Close it with `)`.
  More: https://statac.dev/e/malformed-call/
```

## Why Statac stops

A call it can't read would reach the page as the text you typed, or swallow
the writing after it. Neither is what you meant, so Statac stops rather than
publish either.

## How to fix it

Look at the place the caret points to. Common causes:

- A call with no closing `)` before the paragraph ends. Close it on the
  same paragraph.
- A quoted value whose quote doesn't close on its own line. End the value
  with the same quote it opened with.
- Text after a value's closing quote. Put the whole value inside the
  quotes.
- A snippet's name in letters other than a to z. Name the snippet with the
  letters a to z, digits and `_`.
- A call to a snippet that shows `body`, with no `@end` after it. Such a
  call wraps the writing below it, and `@end` on a line of its own says
  where that writing ends.
- An `@end` with no call above it to close. Only a call to a snippet that
  shows `body`, on a line of its own, wraps writing up to an `@end`.
- A call to a snippet that shows `body`, in the middle of a sentence. Put
  it on a line of its own, with the writing it wraps below it and `@end`
  on a line after that.
- An `@url(...)` handed anything but one address from the top of the
  site: none, two, a value with a name, or an address that doesn't start
  with one `/`. Write the address the page is published at, as
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

And where `snippets/note.html` shows `{{ body }}`, the writing it wraps
ends at an `@end` of its own:

```markdown
@note('warning')
The writing the note wraps.
@end
```
