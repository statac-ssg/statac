# Control character

Your content contains a control character. These are invisible
characters that were once used to tell a printer or a terminal what to
do, such as a vertical tab, U+000B. They usually come from text pasted
from a word processor, which uses one to mark a line break inside a
paragraph, or from text saved in an old encoding. Statac leaves the
character out of the page and tells you where it was.

The message shows the character in a form you can see: `␋` for a
vertical tab, `␀` to `␟` for the others in that range, `␡` for U+007F,
and its code, such as `<U+0092>`, for one between U+0080 and U+009F.

## What you'll see {#what-youll-see}

```text
warning  Control character: U+000B is left out of the page
  Delete it, or type the character you meant.
  More: https://statac.dev/e/control-character/
```

## Why Statac warns

A reader sees nothing where a control character is, and HTML treats one
as an error in the page, so leaving it out makes no difference a reader
can see. Statac leaves it out of code too, because a reader copying the code
would copy a character they can't see. Statac tells you because the
character may stand in for one you meant to type.

Tabs and line endings are the only control characters Statac keeps in
your content.

## How to fix it

Delete the character. If it stood for something, type that instead:

- A vertical tab from a word processor marks a line break. To break the
  line there, end it with a backslash, `\`. Or start a new paragraph.
- A character between U+0080 and U+009F usually comes from text saved in
  an old encoding, where U+0092 stood for `’` and U+009C for `œ`. Type the
  character you meant.

A character written as an HTML reference, such as `&#11;`, counts as HTML
you wrote on purpose, so Statac leaves it in.

## Example

In `content/post-1.md`, an apostrophe saved in an old encoding is shown in
the message as `Qu<U+0092>un`, and Statac publishes `Quun`. Type the
apostrophe again:

```markdown
Qu’un
```
