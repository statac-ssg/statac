# Scheduled post

A post is dated after the day of the build, so Statac doesn't publish it
yet: it is a scheduled post.

## What you'll see {#what-youll-see}

```text
note  Scheduled post: `content/post-2.md` is dated `2026-10-05`
  Run with `--drafts` to build it as if published.
  More: https://statac.dev/e/scheduled-post/
```

## Why Statac says so

A scheduled post is published on the day it is dated. Until then it has
no page of its own, and no folder, tag, series or feed lists it. The first
build on or after its date publishes it. Statac decides the day by the
site's own clock, its `timezone`, so a post dated tomorrow in that time
zone stays scheduled, even if it is already tomorrow somewhere else.

Statac reports every post it leaves out, so a post dated in the future by
mistake never goes missing without a word. `statac check` shows each one,
and `statac build` counts them. This note can't be quietened, because it
is about a page being left out.

## How to fix it

Nothing is wrong if the post is meant to be scheduled. If the date is a
mistake, correct it.

To see a scheduled post while you write it, run `statac serve`, which
shows the post and says which day it is scheduled for. `statac build --drafts`
builds it as if it were published, and says how many such posts it
included. `--today=2026-10-05` builds the site as it will be on that day.

## Example

If the site is built on 4 October 2026, this post in `content/post-2.md`:

```yaml
---
title: Post 2
date: 2026-10-05
---
```

is scheduled for 5 October. To publish it now, give it today's date:

```yaml
---
title: Post 2
date: 2026-10-04
---
```
