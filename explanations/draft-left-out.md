# Draft left out

A page says `draft: true`, written on the page or handed down from its
folder's `index.md` as `pages.draft: true`, so it isn't built.

## What you'll see {#what-youll-see}

```text
note  Draft left out: `content/post-1.md`
  Run with `--drafts` to build it as if published.
  More: https://statac.dev/e/draft-left-out/
```

## Why Statac says so

A draft is left out of everything: it has no page of its own, and no
folder, tag, series or feed lists it. Statac names every page it leaves
out, so a page marked as a draft and forgotten never goes missing without
a word. `statac check` shows each draft, and `statac build` counts them.
This can't be quietened, because it leaves a page out.

## How to fix it

Nothing is wrong: this is what `draft: true` asks for. When the page is
ready, take the line out, or write `draft: false`.

To see drafts while you write them, run `statac serve`, which shows them
and marks each one. `statac build --drafts` builds them as if published,
and says how many it included.

## Example

`content/post-1.md`:

```yaml
---
title: Post 1
draft: true
---
```

Published, with the line taken out:

```yaml
---
title: Post 1
---
```
