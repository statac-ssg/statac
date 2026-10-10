# Address taken

A draft, or a post dated after the build's day, would be published at an
address where something else already is. The message names what stands
there now.

## What you'll see {#what-youll-see}

```text
warning  Address taken: this draft would be published at `/post-1/`, as `content/post-2.md` is
  Give one of them another `permalink` before it is published.
  More: https://statac.dev/e/address-taken/
```

## Why Statac warns

A page left out of the build publishes nothing, so nothing stands in its
way today. On the day it is published, two things would be published at
one address, and the build would stop. Statac says so now, while there
is time to choose, and builds the site as before. With `--drafts`, and
in `statac serve`, the page is built, and the two are told as
`same-address`.

## How to fix it

Give one of the two another address with `permalink`, or move or rename
its file. If the page left out is meant to replace the other, take the
other out on the day it is published.

Once you have understood it, the warning can be quietened by name.

## Example

`content/post-1.md`, published at `/post-1/`, and a draft in
`content/post-2.md`:

```yaml
---
title: Post 2
draft: true
permalink: /post-1/
---
```

Give the draft an address of its own:

```yaml
---
title: Post 2
draft: true
permalink: /post-2/
---
```
