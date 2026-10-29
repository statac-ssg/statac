# Missing file

A template uses `css()`, `js()` or `asset()` to ask for a file in
`assets/` that isn't there, or a card's settings refer to a background or
font that isn't in `templates/og/`. The message gives the name of the
file Statac looked for.

## What you'll see {#what-youll-see}

```text
error  Missing file: there is no `assets/sit.css`
  templates/base.html, line 1
   1 │ {{ css("sit.css") }}
     │    ^^^^^^^^^^^^^^
  While building: content/post-1.md
  Did you mean `site.css`?
  More: https://statac.dev/e/missing-file/
```

## Why Statac stops

Statac publishes each file in `assets/` under a new name made from its
contents, and those functions write that name into the page. With no file,
there is no name to write, and the page would link to nothing. A card
can't be drawn without its background or font.

## How to fix it

Correct the name, or add the file to `assets/` (or, for a card, to
`templates/og/`). If there is a file with a similar name, Statac
suggests it. A file in `static/` keeps its own name, so link to it
directly instead. Statac tells you the URL to use.

Statac before 0.7.0 wrote a `code.css` of its own, holding the colours
of coloured code, and a template asked for it with `css("code.css")`.
Statac writes no stylesheet now, so a site asking for `code.css` needs
one of its own. Copy a theme's rules from [Code
colours](https://statac.dev/code-colours/) into the site's stylesheet
and take the call out, or save them as `assets/code.css`. Either way,
`code_colours: true` in `statac.yaml` asks Statac to mark the code with
the classes the rules colour.

## Example

In `templates/base.html`:

```html
{{ css("sit.css") }}
```

If the file is `assets/site.css`, write:

```html
{{ css("site.css") }}
```
