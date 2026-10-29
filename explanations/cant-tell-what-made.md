# Can't tell what made the site

`statac import` reads a site made by another tool, and works out which
tool from the site's files:

- A Jekyll site has `_config.yml` or `_config.yaml` at its top. Statac
  doesn't read Jekyll's settings from `_config.toml`: write them as YAML
  in `_config.yml`.
- A Hugo site has `hugo.toml`, `hugo.yaml` or `hugo.json`, a
  `config.toml` or similar file that sets `baseURL`, or a
  `config/_default/` folder.
- A Zola site has `zola.toml`, or a `config.toml` that sets `base_url`.
- An Eleventy site has `.eleventy.js`, `eleventy.config.js` (or `.mjs`
  or `.cjs`), or a `package.json` that lists `@11ty/eleventy`.

The folder you gave it has no files from any tool Statac can import from,
or has files from more than one.

## What you'll see {#what-youll-see}

```text
error  Can't tell what made the site: `old` has no settings file Statac knows
  Say what made it with `--from=jekyll`, `--from=hugo`, `--from=zola` or `--from=eleventy`.
  More: https://statac.dev/e/cant-tell-what-made/
```

## Why Statac stops

Each tool stores its content, settings and URLs in its own way.
Reading a site by the wrong tool's rules would import it wrongly, with no
warning. So Statac doesn't guess which tool you used.

## How to fix it

Use `--from` to say which tool made the site, such as `--from=jekyll`,
`--from=hugo`, `--from=zola` or `--from=eleventy`. Or check that the
folder you gave is the top of the old site, where its settings file is,
and not a folder inside it or above it.

## Example

```text
statac import old
```

Here `old` contains `index.md` and `posts/`, but no settings file. If
Jekyll made it:

```text
statac import old --from=jekyll
```
