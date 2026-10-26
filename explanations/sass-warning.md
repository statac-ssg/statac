# Sass warning

Sass printed a message while compiling a stylesheet in `assets/`. The
message comes from an `@warn` or `@debug` in your Sass, or is a warning
from Sass itself. Statac quotes it exactly as Sass printed it, and
points at the line that produced it.

## What you'll see {#what-youll-see}

```text
warning  Sass warning: `$columns is deprecated`
  More: https://statac.dev/e/sass-warning/
```

## Why Statac warns

The stylesheet compiled, and Statac publishes it. Statac passes on the
message
so it isn't lost, because an `@warn` is usually there to point out
something you want to know about.

## How to fix it

Do what the message asks, or remove the `@warn` or `@debug` once it has
done its job.

## Example

In `assets/_grid.scss`:

```scss
@warn "$columns is deprecated";
```

Stop using `$columns`, then remove the `@warn`.
