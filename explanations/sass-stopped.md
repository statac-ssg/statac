# Sass stopped

A Sass file in `assets/` couldn't be compiled to CSS. The message says
why, and points at the file and line where Sass stopped. This may be
another file that the stylesheet uses. If Sass gives no place, such as
for a value CSS has no way to write, the message names only the
stylesheet. If an `@error` in your Sass stopped it, the message quotes
what the `@error` says.

## What you'll see {#what-youll-see}

```text
error  Sass stopped: no variable called `$link`
  More: https://statac.dev/e/sass-stopped/
```

## Why Statac stops

Without the compiled stylesheet, the pages that use it would be
published without their styles.

## How to fix it

Correct the line the message points at, then build again. A Sass file
whose name starts with `_` is only ever part of other stylesheets. It is
never published on its own, only as part of the stylesheets that use it.

## Example

In `assets/_parts.scss`:

```scss
a {
  color: $link;
}
```

If `$link` isn't defined anywhere, define it before it is used:

```scss
$link: #0645ad;

a {
  color: $link;
}
```
