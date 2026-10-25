# Retired setting

`statac.yaml` has `code_style`, a setting Statac no longer reads. It
named the colours Statac gave code, in a `code.css` Statac wrote into
the built site. Statac now writes no stylesheet and colours no code
unless `code_colours: true` asks.

## What you'll see {#what-youll-see}

```text
error  Retired setting: Statac no longer reads `code_style`
  Write `code_colours: true` in its place, and copy the `ocean` theme's rules from https://statac.dev/code-colours/ into a stylesheet of the site's own.
  More: https://statac.dev/e/retired-setting/
```

## Why Statac stops

Left alone, the line would do nothing: code that was coloured would be
published without colour, and nothing would say why. So Statac stops
until the line is gone.

## How to fix it

With `code_style: none`, delete the line. Code is written as it is,
with its language in its class, such as `language-rust`, which is what
`none` asked for.

With a theme, write `code_colours: true` in its place. Statac then marks
each piece of code with a class for its kind, such as `hl-keyword`, as
before. The colours are the site's own now: copy the theme's rules from
[Code colours](https://statac.dev/code-colours/) into the site's
stylesheet. A template that asked for `code.css` with
`css("code.css")` names a file the site doesn't have, so take the call
out, or keep it and save the rules as `assets/code.css`.

## Example

`statac.yaml`:

```yaml
title: Site
code_style: ocean
```

With the line replaced:

```yaml
title: Site
code_colours: true
```
