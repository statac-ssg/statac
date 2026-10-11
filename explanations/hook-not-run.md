# Hook not run

The site sets one of its own commands, `before_build` or `after_build`, in
`statac.yaml`, and `statac check` doesn't run it. `statac build` does run
it. `statac serve` runs these commands too: `before_build` when it starts
and whenever a file listed in `before_build_watches` changes, and
`after_build` once, after its first build with no errors.

## What you'll see {#what-youll-see}

```text
note  Hook not run: `check` doesn't run `before_build` or `after_build`
  More: https://statac.dev/e/hook-not-run/
```

## Why Statac says so

`check` reads the site and writes nothing, so it is always safe to run.
A command may write files, send the site somewhere or take a long time,
and an `after_build` command has no built site to work on.

So `check` reports on the site as it is now. If a `before_build` command
makes files that the site is built from, `check` reads the files left by
the last time the command ran, and reports any that are missing.

## How to fix it

Nothing needs fixing. To run the commands, run `statac build` or
`statac serve`. Once you have understood the note, add
`quiet: [hook-not-run]` to `statac.yaml` to stop it being shown.

## Example

In `statac.yaml`:

```yaml
after_build: gzip -k -r "$STATAC_OUT"
```

`statac check` doesn't run the command, and says so. `statac build` runs it.
