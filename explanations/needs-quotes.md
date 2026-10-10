# Needs quotes

A field that holds text is given a value YAML reads as something else, and
reading it that way would change what you wrote. `3.10` is the number 3.1
to YAML, `0x10` is 16, and `1e3` is 1000. A colour such as `#F2E9D3`
written bare is a comment to YAML, which leaves the line with no value.

## What you'll see {#what-youll-see}

```text
error  Needs quotes: YAML reads `3.10` as 3.1
  Write it as `"3.10"` to keep it as written.
  More: https://statac.dev/e/needs-quotes/
```

## Why Statac stops

Statac takes a bare value as typed where YAML's reading keeps every
character, so `title: 1984` is the text `1984`. Where the reading would
change the characters, keeping them would mean reading YAML differently
from every other tool, and taking YAML's reading would change your words.
Quotes make it text in every reader.

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
