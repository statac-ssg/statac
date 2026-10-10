# Alias limit

In YAML, `&name` marks a value and `*name` repeats it somewhere else. The
aliases in one file have repeated more than Statac reads: more than 10,000
values, or more than 1 MB of text.

## What you'll see {#what-youll-see}

```text
error  Alias limit: by `*book`, aliases have repeated more than 10,000 values
  More: https://statac.dev/e/alias-limit/
```

## Why Statac stops

Aliases can repeat aliases, so a few lines can grow into an enormous
document that takes all the memory a computer has. Statac stops at the
alias that passes the limit instead.

## How to fix it

Write the values out where you need them, or repeat fewer of them. A file
that needs this many repeats is usually better split into data files.

## Example

```yaml
a: &a [one, one, one, one, one, one, one, one, one, one]
b: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a]
c: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b, *b]
d: [*c, *c, *c, *c, *c, *c, *c, *c, *c, *c]
```

Each line repeats the one before ten times, so `d` alone holds more than
10,000 values.
