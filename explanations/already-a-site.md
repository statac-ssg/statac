# Already a site

`statac new`, or `statac import` for the new site, was given a folder
that is a site already: it has a
`statac.yaml`, the site's settings, or a `content/` folder, the site's
writing.

## What you'll see {#what-youll-see}

```text
error  Already a site: `site-1` has a `statac.yaml`
  To bring its starter files up to date, run `statac new --update`.
  More: https://statac.dev/e/already-a-site/
```

## Why Statac stops

Writing a new site's files over a site's own would change how its pages
look and what it says. `statac new` only ever writes into an empty
folder, or a new one.

## How to fix it

To build the site that is there, run `statac build` or `statac serve` in
its folder. To bring its starter files up to date, the templates,
snippets and stylesheet `statac new` wrote, run `statac new --update`
there. To start another site, give `statac new` the name of a new
folder.

## Example

```text
statac new site-1
```

where `site-1` has a `statac.yaml`. To start another site beside it:

```text
statac new site-2
```
