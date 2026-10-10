# Unusable date format

`date_format` says how dates look on the site's pages, by example: you
write one moment, Wednesday 31 December 2025 at 15:04, the way you want
every date written, and Statac writes each date the same way. A template
can do the same for one date, as `{{ page.date | date("December 2025") }}`.

Every piece of that moment is a different number or word, so Statac can
tell which piece you wrote where: 31 is the day, 12 the month, 2025 the
year, 15 or 3 the hour, 04 the minute, and Wednesday the weekday.
Something else in the example, such as `1 January 2020`, can't be read,
and neither can a piece written twice.

Months and weekdays in words are written in the site's language, from
its file in `languages/` or Statac's own words for it. `date_format` for
the whole site must have the day, the month and the year, the year in
full, because every date on the site is written with it.

## What you'll see {#what-youll-see}

```text
error  Unusable date format: `31 December` has no year
  Add the year, which every date on the site is written with.
  More: https://statac.dev/e/unusable-date-format/
```

## Why Statac stops

A format Statac can't read would put the wrong date, or the same words,
on every page.

## How to fix it

Write the moment Statac reads: 31 December 2025, with 15:04 or 3.04pm if
you want the time. Write the month in the site's language, or in
figures.

## Example

In `statac.yaml`:

```yaml
date_format: 1 January 2020
```

The same style, written as 31 December 2025:

```yaml
date_format: 31 December 2025
```

A date on 7 March 2026 is then written `7 March 2026`.
