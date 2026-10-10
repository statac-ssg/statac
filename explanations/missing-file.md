# Missing file

A template asks for a file in `assets/` that isn't there, through `css()`,
`js()` or `asset()`, or a card's settings name a background or a font
that isn't in `templates/og/`. The message names the file it looked for.

## What you'll see {#what-youll-see}

```text
error  Missing file: there is no `assets/sit.css`
  Did you mean `site.css`?
  More: https://statac.dev/e/missing-file/
```

## Why Statac stops

Statac publishes each file in `assets/` under a new name made from its
contents, and those functions write that name into the page. With no file,
there is no name to write, and the page would link to nothing. A card
with no background or font to draw with can't be drawn.

## How to fix it

Correct the name, or add the file to `assets/`, or for a card to
`templates/og/`. When a file with a similar
name is there, Statac offers it. A file in `static/` keeps its own name, so
link to it directly instead: Statac gives the address to use.

## Example

In `templates/base.html`:

```html
{{ css("sit.css") }}
```

If the file is `assets/site.css`, write:

```html
{{ css("site.css") }}
```
