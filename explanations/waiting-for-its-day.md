# Waiting for its day

A post is dated after the day of the build, so it isn't built yet.

## What you'll see {#what-youll-see}

```text
note  Waiting for its day: `content/post-2.md` is dated `2026-10-05`
  Run with `--drafts` to build it as if published.
  More: https://statac.dev/e/waiting-for-its-day/
```

## Why Statac says so

A post waits for its day: until then it has no page of its own, and no
folder, tag, series or feed lists it. The first build on or after its date
publishes it. The day is judged on the site's own clock, its `timezone`,
so a post dated tomorrow there waits even where it is already tomorrow
somewhere else.

Statac names every post it leaves out, so a post dated in the future by
mistake never goes missing without a word. `statac check` shows each one,
and `statac build` counts them. This can't be quietened, because it leaves
a page out.

## How to fix it

Nothing is wrong if the post is meant to wait. If the date is a mistake,
correct it.

To see a waiting post while you write it, run `statac serve`, which shows
it and says which day it waits for. `statac build --drafts` builds it as
if published, and says how many it included. `--today=2026-10-05` builds
the site as it will be on that day.

## Example

Built on 4 October 2026, `content/post-2.md`:

```yaml
---
title: Post 2
date: 2026-10-05
---
```

waits until 5 October. To publish it now, date it today:

```yaml
---
title: Post 2
date: 2026-10-04
---
```
