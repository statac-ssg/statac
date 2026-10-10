# Hook not run

The site sets a command of its own, `before_build` or `after_build` in
`statac.yaml`, and `statac check` doesn't run it. `statac build` does.
`statac serve` runs them too: `before_build` as it starts and whenever a
file `before_build_watches` names changes, and `after_build` once, after
its first build with no errors.

## What you'll see {#what-youll-see}

```text
note  Hook not run: `check` doesn't run `before_build` or `after_build`
  More: https://statac.dev/e/hook-not-run/
```

## Why Statac says so

`check` reads the site and writes nothing, so that it is always safe to
run. A command may write files, send the site somewhere or take a long
time, and an `after_build` command has no built site to act on.

What `check` reports is therefore the site as it stands. Where a
`before_build` command makes files the site is built from, `check` reads
the ones made the last time the command ran, and reports any that are
missing.

## How to fix it

Nothing needs fixing. Run `statac build` or `statac serve` to run the
commands. Once you
have understood the note, `quiet: [hook-not-run]` stops it being shown.

## Example

In `statac.yaml`:

```yaml
after_build: gzip -k -r "$STATAC_OUT"
```

`statac check` doesn't run the command, and says so. `statac build` runs it.
