# Control character

Your writing holds a control character: one of the invisible characters
that once told a printer or a terminal what to do, such as a vertical tab,
U+000B. They usually arrive with text pasted from a word processor, which
marks a line break inside a paragraph with one, or with text saved in an
old encoding. Statac leaves the character out of the page and tells you
where it was.

The message draws the character where you can see it: `␋` for a vertical
tab, `␀` to `␟` for the others in that range, `␡` for U+007F, and
`<U+0092>` for one between U+0080 and U+009F.

## What you'll see {#what-youll-see}

```text
warning  Control character: U+000B is left out of the page
  Delete it, or type the character you meant.
  More: https://statac.dev/e/control-character/
```

## Why Statac warns

A reader sees nothing where a control character stands, and HTML reads one
as a mistake in the page, so leaving it out changes nothing a reader can
see. It is left out in code too: a reader copying the code would copy a
character they can't see. Statac tells you because the character may stand
for one you meant to type.

The tab and the line endings are the only control characters your writing
keeps.

## How to fix it

Delete the character. Where it stood for something, type that instead:

- A vertical tab from a word processor marks a line break. End the line
  with a backslash, `\`, to break it there, or start a new paragraph.
- A character between U+0080 and U+009F is usually one saved in an old
  encoding: U+0092 stood for `’` and U+009C for `œ`. Type the character
  you meant.

A character written as an HTML reference, such as `&#11;`, is HTML you
wrote on purpose, and Statac leaves it in.

## Example

In `content/post-1.md`, an apostrophe saved in an old encoding is drawn in
the message as `Qu<U+0092>un`, and Statac publishes `Quun`. Type the
apostrophe again:

```markdown
Qu’un
```
