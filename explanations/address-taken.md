# Address taken

A draft, or a post dated after the day of the build, would be published
at an address that something else already uses. The message says what is
published there now.

## What you'll see {#what-youll-see}

```text
warning  Address taken: this draft would be published at `/post-1/`, as `content/post-2.md` is
  Give one of them another `permalink` before it is published.
  More: https://statac.dev/e/address-taken/
```

## Why Statac warns

A page left out of the build isn't published, so for now it doesn't
clash with anything. On the day it is published, two things would be
published at the same address, and the build would stop. Statac warns
you now, while there is still time to choose, and builds the site as
usual. With `--drafts`, and in `statac serve`, the page is built, so the
clash is reported as `same-address` instead.

## How to fix it

Give one of the two a different address with `permalink`, or move or
rename its file. If the page that is left out is meant to replace the
other, remove the other on the day it is published.

Once you have understood the warning, you can quieten it by name.

## Example

`content/post-1.md` is published at `/post-1/`, and this draft in
`content/post-2.md` asks for the same address:

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
