# Not stamped

A value in a page's front matter is a stamp, such as `=today` or
`=monday 09:00`: one of Statac's words standing for a date, waiting for
the date to be written in its place.

## What you'll see {#what-youll-see}

```text
error  Not stamped: `date` is still `=monday 09:00`
  Run `statac stamp` to write the date in its place, or `statac serve`, which writes it on every save.
  More: https://statac.dev/e/not-stamped/
```

## Why Statac stops

A stamp names a different day each day it is read, and a build of the same
files is the same every time, so `statac build` and `statac check` never
choose the date. Nor can a page be published dated `=monday`. The date is
written into the file once, by `statac stamp` or by `statac serve`, and
from then on the file says the day itself.

## How to fix it

Run `statac stamp` in the site's folder. It writes the date over every
stamp in the site and says what it wrote. `statac serve` does the same as
it starts and on every save, so a site being served never holds a stamp
for long.

To publish the words themselves, write the value as a block, which is
always read as text:

```yaml
title: |-
  =today
```

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
date: =monday 09:00
---
```

After `statac stamp`, run on Tuesday 6 October 2026:

```yaml
---
title: Post 1
date: 2026-10-12 09:00
---
```
