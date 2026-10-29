# Unknown option

The command line includes an option that the command doesn't take. Either Statac
has no option by that name, or it belongs to another command.

## What you'll see {#what-youll-see}

```text
error  Unknown option: `statac check` has no `--out`
  `--out` is for `statac build`.
  More: https://statac.dev/e/unknown-option/
```

## Why Statac stops

Ignoring the option would do something other than what you asked. If an
option is close to what you wrote, the message suggests it.

## How to fix it

Use an option the command takes, written with two hyphens and its value
after an `=`. To list the options a command takes, put
`--help` after it, as in `statac check --help`. `statac --help` lists
every option.

Statac before 0.7.0 had `--cards`, `--update`, `--missing` and
`--no-starter`, for writing its own templates, snippets and card files
into a site and keeping them up to date. Statac no longer writes them, so
it refuses these options with a message saying so, and the note that
said a newer version of those files was available, `update-available`,
went with them. Leave the options out: a
site's templates are its own, written as
[Templates](https://statac.dev/templates/) describes.

Statac before 0.7.0 also had `--site`, for naming the site's folder. The
folder written after the command names it now, so Statac refuses
`--site` with a message saying how to write it: `statac build docs`, not
`statac build --site=docs`.

## Example

```text
statac check --out=public
```

`--out` sets where `statac build` writes the site, but `check` writes
nothing. To write the site to `public`:

```text
statac build --out=public
```

`statac serve` shows drafts and scheduled posts by default,
so it has no `--drafts` option. To leave them out, use
`statac serve --no-drafts`.
