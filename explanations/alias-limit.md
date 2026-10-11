# Alias limit

In YAML, `&name` marks a value and `*name` repeats it somewhere else. The
aliases in one file have repeated more than Statac will read: more than 10,000
values, or more than 1 MB of text.

## What you'll see {#what-youll-see}

```text
error  Alias limit: by `*book`, aliases have repeated more than 10,000 values
  More: https://statac.dev/e/alias-limit/
```

## Why Statac stops

Aliases can repeat other aliases, so a few lines can grow into an
enormous document that uses up all of a computer's memory. Instead,
Statac stops at the alias that goes over the limit.

## How to fix it

Write the values out where you need them, or repeat fewer of them. If a file
needs this many repeats, it is usually better split into data files.

## Example

```yaml
a: &a [one, one, one, one, one, one, one, one, one, one]
b: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a]
c: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]
d: [*c, *c, *c, *c, *c, *c, *c, *c, *c, *c]
```

Each line repeats the one before it ten times, so `d` alone contains more
than 10,000 values.
