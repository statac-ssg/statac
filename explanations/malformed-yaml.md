# Malformed YAML

Statac couldn't read some YAML: the front matter of a page, or a settings
or data file. The message says what it found and points at the place to
fix, such as a quote that is never closed or a line indented too far.

## What you'll see {#what-youll-see}

```text
error  Malformed YAML: `"` is never closed
  End the value with another `"`.
  More: https://statac.dev/e/malformed-yaml/
```

## Why Statac stops

Past the mistake, Statac can't tell which values you meant, and a guess
could publish the wrong title, date or address without anyone noticing.

## How to fix it

Look at the place the caret points to. Common causes:

- A quote or bracket opened and never closed. Close it on the same value.
- A tab used for indentation. YAML indents with spaces only.
- Text after a closing quote. Put the whole value inside the quotes.
- A backslash in double quotes that YAML doesn't know, such as `\q`. Write
  `\\` for a backslash, or use single quotes.
- A `---` line inside the YAML, which starts a second document. A file
  holds one.

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
