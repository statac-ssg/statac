# Already a site

The folder given to `statac new`, or the folder `statac import` was to
write the new site into, already contains a site. It has a `statac.yaml`,
with the site's settings, or a `content/` folder, with the site's
content.

## What you'll see {#what-youll-see}

```text
error  Already a site: `site-1` has a `statac.yaml`
  To bring its starter files up to date, run `statac new --update`.
  More: https://statac.dev/e/already-a-site/
```

## Why Statac stops

Writing a new site's files over an existing site's files would change
how its pages look and what they say. `statac new` only ever writes into
an empty folder, or a new one.

## How to fix it

To build the site that is already there, run `statac build` or
`statac serve` in its folder. To bring its starter files (the templates,
snippets and stylesheet that `statac new` wrote) up to date, run
`statac new --update` in that folder. To start another site, give
`statac new` the name of a new folder.

## Example

```text
statac new site-1
```

Here `site-1` already has a `statac.yaml`. To start another site beside
it:

```text
statac new site-2
```
