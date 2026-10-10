# Outside the site

Something in the site reaches for a file outside where it may: a Sass
`@use`, `@forward` or `@import` that leads out of `assets/`, a card's
background or font named outside `templates/og/`, or a link in a folder
such as `content/`, `assets/` or `static/` whose target is outside that
folder. Statac doesn't read the file.

## What you'll see {#what-youll-see}

```text
error  Outside the site: `../../outside` isn't in `assets/`
  Copy the file into `assets/` and load it from there.
  More: https://statac.dev/e/outside-site/
```

## Why Statac stops

Everything Statac publishes comes from the folder it is published from, so
building a site can never copy a file from elsewhere into the published
site, whoever wrote the site. A link in `static/` to the site's
`statac.yaml` would publish its settings, secrets included.

## How to fix it

Copy the file into the folder in place of the link: a stylesheet into
`assets/`, loaded from there; a card's background or font into
`templates/og/`, named from there; anything else into `assets/`,
`static/` or `content/`. A link to somewhere inside the same folder is
fine.

## Example

In `assets/site.scss`:

```scss
@use "../../shared/colours";
```

Copy `colours.scss` into `assets/` and write:

```scss
@use "colours";
```
