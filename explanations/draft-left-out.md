# Draft left out

A page has `draft: true`, so it isn't built. The field is either written
on the page itself, or cascaded from its folder's `index.md` with
`pages.draft: true`.

## What you'll see {#what-youll-see}

```text
note  Draft left out: `content/post-1.md`
  content/post-1.md, line 3
   3 │ draft: true
     │        ^^^^
  Run with `--drafts` to build it as if published.
  More: https://statac.dev/e/draft-left-out/
```

## Why Statac says so

A draft is left out of everything: it has no page of its own, and no
folder, tag, series or feed lists it. Statac reports every page it leaves
out, so a page you marked as a draft and then forgot about never
disappears without a word. `statac check` shows each draft, and
`statac build` counts them. This can't be quietened, because it leaves a
page out.

## How to fix it

Nothing is wrong. This is what `draft: true` is for. When the page is
ready, remove the line, or write `draft: false`.

To see drafts while you write them, run `statac serve`. It includes them
and marks each one. `statac build --drafts` builds drafts as if they were
published, and says how many it included.

## Example

`content/post-1.md`:

```yaml
---
title: Post 1
draft: true
---
```

To publish it, remove the line:

```yaml
---
title: Post 1
---
```
