# Unknown YAML tag

A value in some YAML carries a tag, written with `!`, that Statac doesn't
read. Statac reads the tags of YAML's core schema, `!!str`, `!!int`,
`!!float`, `!!bool`, `!!null`, `!!seq` and `!!map`, and `!!binary`, bytes
written in base64, which are text where the bytes are text.

## What you'll see {#what-youll-see}

```text
error  Unknown YAML tag: `!local`
  More: https://statac.dev/e/unknown-yaml-tag/
```

## Why Statac stops

Any other tag asks for a meaning only the program that defined it knows.
Reading the value without it could give the wrong one.

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
