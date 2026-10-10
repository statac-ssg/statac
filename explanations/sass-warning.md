# Sass warning

Sass said something while compiling a stylesheet in `assets/`: a message
from an `@warn` or `@debug` in your Sass, or a warning of Sass's own. The
message is quoted as Sass gave it, at the line that said it.

## What you'll see {#what-youll-see}

```text
warning  Sass warning: `$columns is deprecated`
  More: https://statac.dev/e/sass-warning/
```

## Why Statac warns

The stylesheet compiled and is published. Statac passes on what Sass said
so it isn't lost, because an `@warn` is usually there to catch something
you want to know about.

## How to fix it

Do what the message asks, or remove the `@warn` or `@debug` once it has
done its job.

## Example

In `assets/_grid.scss`:

```scss
@warn "$columns is deprecated";
```

Stop using `$columns`, then remove the `@warn`.
