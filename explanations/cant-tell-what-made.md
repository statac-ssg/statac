# Can't tell what made the site

`statac import` reads a site another tool made, and tells which tool by
the site's files: a Jekyll site has `_config.yml`, `_config.yaml` or
`_config.toml` at its top, and a Hugo site `hugo.toml`, `hugo.yaml` or
`hugo.json`, a `config.toml` or the like that sets `baseURL`, or a
`config/_default/` folder, a Zola site `zola.toml`, or a `config.toml`
that sets `base_url`, and an Eleventy site `.eleventy.js` or
`eleventy.config.js`, `.mjs` or `.cjs`, or a `package.json` that names
`@11ty/eleventy`. The folder it was given has the files of no tool Statac
has a reader for, or those of more than one.

## What you'll see {#what-youll-see}

```text
error  Can't tell what made the site: `old` has no settings file Statac knows
  Say what made it with `--from=jekyll`.
  More: https://statac.dev/e/cant-tell-what-made/
```

## Why Statac stops

Each tool keeps its writing, its settings and its addresses its own way,
and reading a site by the wrong tool's rules would bring it across wrong
without a word. Statac doesn't guess which tool you used.

## How to fix it

Say which tool made the site with `--from`, such as `--from=jekyll`,
`--from=hugo`, `--from=zola` or `--from=eleventy`. Or check that the folder is the top of the old site, where
its settings file is, and not a folder inside it or above it.

## Example

```text
statac import old
```

where `old` holds `index.md` and `posts/`, but no settings file. Where
Jekyll made it:

```text
statac import old --from=jekyll
```
