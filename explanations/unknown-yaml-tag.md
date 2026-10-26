# Unknown YAML tag

A value in some YAML has a tag, written with `!`, that Statac doesn't
read. Statac reads the tags of YAML's core schema (`!!str`, `!!int`,
`!!float`, `!!bool`, `!!null`, `!!seq` and `!!map`). It also reads
`!!binary`, for bytes written in base64, and treats those values as text
if the bytes are text.

## What you'll see {#what-youll-see}

```text
error  Unknown YAML tag: `!local`
  More: https://statac.dev/e/unknown-yaml-tag/
```

## Why Statac stops

Any other tag gives the value a meaning that only the program that
defined the tag knows. If Statac read the value without that meaning, it
could get the value wrong.

## How to fix it

Remove the tag. To keep a value as text, put it in quotes or tag it
`!!str`.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
version: !local 3.10
---
```

To keep `3.10` exactly as written:

```yaml
---
title: Post 1
version: "3.10"
---
```
