# No time zone

The site's settings don't set a `timezone`, and some of its dates are
written without an offset, such as `2026-10-04` or `2026-10-04 09:30`.
Statac reads a date without an offset in the site's time zone, so with
no zone set, it reads it as UTC.

## What you'll see {#what-youll-see}

```text
warning  No time zone: dates without an offset are read as UTC
  Add a line such as `timezone: Europe/London` to `statac.yaml`, naming the zone your dates are written in.
  More: https://statac.dev/e/no-time-zone/
```

## Why Statac warns

Statac knows what to do, so the build goes on. It warns you because the
times in the site's feeds, and the order of posts written on the same
day, depend on the time zone, and you may not be writing in UTC. Statac
never takes the time zone from your computer, so the site builds the
same wherever it is built.

## How to fix it

Add a `timezone` line to `statac.yaml`, giving the zone you write in by
the name of a place, such as `Europe/London` or `America/Chicago`. If your dates are
meant as UTC, write `timezone: UTC`.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
date: 2026-10-04 09:30
---
```

In `statac.yaml`, set the zone the date was written in:

```yaml
timezone: Europe/London
```
