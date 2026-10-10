# Unknown option

The command line gives an option the command doesn't take. Either Statac
has no option by that name, or it belongs to another command.

## What you'll see {#what-youll-see}

```text
error  Unknown option: `--sight=docs`
  Did you mean `--site=docs`?
  More: https://statac.dev/e/unknown-option/
```

## Why Statac stops

Leaving the option out would do something other than what you asked. When
an option is close to what you wrote, Statac offers it.

## How to fix it

Use an option the command takes, written with two hyphens and its value
after an `=`. `--help` after the command, as `statac check --help`, lists
the options that command takes; `statac --help` lists every option.

## Example

```text
statac check --out=public
```

`--out` says where `statac build` writes the site, and `check` writes
nothing. To write the site to `public`:

```text
statac build --out=public
```

`statac serve` shows drafts and posts waiting for their day without being
asked, so it has no `--drafts`; `statac serve --no-drafts` leaves them
out.
