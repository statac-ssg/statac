# Unusable date format

`date_format` sets how dates look on the site's pages, by example. You
write one moment, Wednesday 31 December 2025 at 15:04, the way you want
every date written, and Statac writes each date the same way. A template
can do the same for a single date, as in
`{{ page.date | date("December 2025") }}`, or name one of the looks a
date takes in the page's language: `long`, `short`, `day` or `stamp`, as
in `{{ page.date | date("short") }}`.

Each piece of that moment is a different number or word, so Statac can
tell which piece you wrote where: 31 is the day, 12 is the month, 2025 is
the year, 15 or 3 is the hour, 04 is the minute, and Wednesday is the
weekday. An example of any other moment, such as `1 January 2020`, can't
be read. Neither can an example with the same piece written twice.

Months and weekdays in words are written in the site's language, using
its file in `languages/` or Statac's own translations for that language. The
site-wide `date_format` must have the day, the month and the year, with
the year in full, because every date on the site is written with it.

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

Write the moment Statac expects: 31 December 2025, with 15:04 or 3.04pm
if you want the time. Write the month in the site's language, or as a
number. In a template, you can name a look instead.

## Example

In `statac.yaml`:

```yaml
date_format: 1 January 2020
```

The same style, written as 31 December 2025:

```yaml
date_format: 31 December 2025
```

A date on 7 March 2026 is then shown as `7 March 2026`.
