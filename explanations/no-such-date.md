# No such date

A date is written in the right form, but the calendar has no such day: 30
February, month 13, a time such as `25:70`, or an offset more than 14 hours
from UTC.

## What you'll see {#what-youll-see}

```text
error  No such date: there is no 30 February in 2026
  More: https://statac.dev/e/no-such-date/
```

## Why Statac stops

Statac can't tell which day you meant, and choosing the nearest real one
could put a post on the wrong day.

## How to fix it

Correct the date. The message names the part that doesn't exist.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
date: 2026-02-30
---
```

February 2026 has 28 days:

```yaml
---
title: Post 1
date: 2026-02-28
---
```
