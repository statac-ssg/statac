# Malformed YAML

Statac couldn't read some YAML: the front matter of a page, or a settings
or data file. The message says what it found and points to what needs
fixing, such as a quote that is never closed or a line indented too far.

## What you'll see {#what-youll-see}

```text
error  Malformed YAML: `"` is never closed
  End the value with another `"`.
  More: https://statac.dev/e/malformed-yaml/
```

## Why Statac stops

After the mistake, Statac can't tell which values you meant, and a guess
could publish the wrong title, date or URL without anyone noticing.

## How to fix it

Look at the place the caret points to. Common causes:

- A quote or bracket that is opened and never closed. Close it within the
  same value.
- A text in quotes that runs onto another line, where the later line
  starts no deeper than the field the text belongs to. Start each later
  line with at least the number of spaces the message gives, or put the
  whole text on one line.
- A tab used for indentation. YAML indents with spaces only.
- Text after a closing quote. Put the whole value inside the quotes.
- A backslash escape in double quotes that YAML doesn't recognise, such
  as `\q`. Write `\\` for a backslash, or use single quotes.
- A `---` line inside the YAML, which starts a second document. A file
  can contain only one.

## Example

In `content/post-1.md`:

```yaml
---
title: "Post 1
---
```

The quote is never closed. End the value with another `"`:

```yaml
---
title: "Post 1"
---
```
