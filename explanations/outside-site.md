# Outside the site

Something in the site refers to a file outside the folder it is allowed
to read from: a Sass `@use`, `@forward` or `@import` that leads out of
`assets/`, a card's background or font outside `templates/og/`, or a link
in a folder such as `content/`, `assets/` or `static/` that points outside
that folder. Statac doesn't read the file.

## What you'll see {#what-youll-see}

```text
error  Outside the site: `../../shared/colours` isn't in `assets/`
  assets/site.scss, line 1
   1 │ @use "../../shared/colours";
     │       ^^^^^^^^^^^^^^^^^^^^
  Copy the file into `assets/` and load it from there.
  More: https://statac.dev/e/outside-site/
```

## Why Statac stops

Statac publishes each file only from the folder it belongs in. This means
building a site can never copy a file from anywhere else into the
published site, whoever wrote the site. Otherwise, a link in `static/` to
the site's `statac.yaml` would publish its settings, secrets included.

## How to fix it

Copy the file into the right folder instead of linking to it. Copy a
stylesheet into `assets/` and load it from there. Copy a card's background
or font into `templates/og/` and refer to it from there. Copy anything
else into `assets/`, `static/` or `content/`. A link to somewhere inside
the same folder is fine.

## Example

In `assets/site.scss`:

```scss
@use "../../shared/colours";
```

Copy `colours.scss` into `assets/` and write:

```scss
@use "colours";
```
