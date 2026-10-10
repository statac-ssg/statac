# No time zone

The site's settings name no `timezone`, and some of its dates are written
without an offset, such as `2026-10-04` or `2026-10-04 09:30`. A date
without an offset is read in the site's time zone, so with none named it
is read as UTC.

## What you'll see {#what-youll-see}

```text
warning  No time zone: dates without an offset are read as UTC
  Add a line such as `timezone: Europe/London` to `statac.yaml`, naming the zone your dates are written in.
  More: https://statac.dev/e/no-time-zone/
```

## Why Statac warns

Statac is sure what it does, so the build goes on. It tells you, because
the moments in the site's feeds, and the order of posts written on one
day, follow the zone, and UTC may not be where you write. Statac never
takes the zone from your computer, so the site builds the same wherever
it is built.

## How to fix it

Add a `timezone` line to `statac.yaml`, naming the zone you write in by
a place, such as `Europe/London` or `America/Chicago`. If your dates are
meant as UTC, write `timezone: UTC`.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
date: 2026-10-04 09:30
---
```

In `statac.yaml`, the zone the date was written in:

```yaml
timezone: Europe/London
```
