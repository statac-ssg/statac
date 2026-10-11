# Needs quotes

A text field has a value that YAML reads as something else, and reading it
that way would change what you wrote. To YAML, `3.10` is the number 3.1,
`0x10` is 16, and `1e3` is 1000. A colour such as `#F2E9D3` written
without quotes is a comment to YAML, so the line is left with no value.

## What you'll see {#what-youll-see}

```text
error  Needs quotes: YAML reads `3.10` as 3.1
  Write it as `"3.10"` to keep it as written.
  More: https://statac.dev/e/needs-quotes/
```

## Why Statac stops

If YAML's reading of a value without quotes keeps every character, Statac
uses the value as you typed it, so `title: 1984` is the text `1984`. If
YAML's reading would change the characters, Statac can't keep them without
reading YAML differently from every other tool, and can't use YAML's
reading without changing your words. With quotes, every YAML reader treats
the value as text.

## How to fix it

Put the value in quotes, as the message shows.

## Example

In `content/post-1.md`:

```yaml
---
title: Version 3.10
version: 3.10
---
```

If `version` is declared as text, quote it:

```yaml
---
title: Version 3.10
version: "3.10"
---
```
