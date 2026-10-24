# Unknown option

The command line includes an option that the command doesn't take. Either Statac
has no option by that name, or it belongs to another command.

## What you'll see {#what-youll-see}

```text
error  Unknown option: `--sight=docs`
  Did you mean `--site=docs`?
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
