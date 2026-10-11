# Hook stopped

One of the site's own commands, `before_build` or `after_build` in
`statac.yaml`, failed. The message says which command it was and how it
ended. It also shows what the command printed (the last 40 lines, if it
printed more).

A command succeeds if it ends with exit code 0. The build stops if the
command ends with any other code, if something else ends the command
before it finishes, or if the system's shell doesn't start at all.

## What you'll see {#what-youll-see}

```text
error  Hook stopped: `after_build` ended with exit code 3
  More: https://statac.dev/e/hook-stopped/
```

## Why Statac stops

A `before_build` command makes something the site is built from, such
as a stylesheet. An `after_build` command finishes off the built site,
for example by making its search index. Statac can't tell how far a
failed command got, so it publishes nothing, and the site from the last
build stays as it was.

## How to fix it

Read what the command printed. That is where it says what went wrong.
If you run the same line yourself in the site's folder, you see the same
output.

Exit code 127 (9009 on Windows) means the shell couldn't find a command.
Check that the command is installed and on the `PATH` Statac was started
with. The command runs in the site's folder, so a path such as
`./tools/make.sh` is read from there.

A command is handed five values:

- `STATAC_SITE`: the site's folder.
- `STATAC_OUT`: the folder to work on.
- `STATAC_DRAFTS`: `yes` or `no`.
- `STATAC_SERVE`: `yes` if `statac serve` is running the command, and `no` otherwise.
- `STATAC_HOOK`: the setting the command is written in.

An `after_build` command runs before the built site is moved into place,
so it must work on `STATAC_OUT`, not on `_site`. At that point `_site`
still holds the site from the last build.

## Example

In `statac.yaml`:

```yaml
after_build: gzip -k "$STATAC_OUT/missing.html"
```

```text
error  Hook stopped: `after_build` ended with exit code 1
  statac.yaml, line 5
  What it printed:
    gzip: can't stat: missing.html: No such file or directory
```

Change the command to name a file that the built site has, then build
again.
