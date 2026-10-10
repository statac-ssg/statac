# Sass stopped

A Sass file in `assets/` couldn't be compiled to CSS. The message gives
Sass's reason, and points at the file and line where it stopped, which may
be a file the stylesheet uses.

## What you'll see {#what-youll-see}

```text
error  Sass stopped: undefined variable
  More: https://statac.dev/e/sass-stopped/
```

## Why Statac stops

Without the compiled stylesheet, the pages using it would be published
without their styles.

## How to fix it

Correct the line the message points at, then build again. A Sass file
whose name starts with `_` is only part of others: it is never published
on its own, only as part of the stylesheets that use it.

## Example

In `assets/_parts.scss`:

```scss
a {
  color: $link;
}
```

If no `$link` is defined, define it before it is used:

```scss
$link: #0645ad;

a {
  color: $link;
}
```
