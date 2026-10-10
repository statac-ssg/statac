# Hook stopped

One of the site's own commands, `before_build` or `after_build` in
`statac.yaml`, didn't end well. The message says which, how it ended, and
shows what the command printed, the last 40 lines where it printed more.

A command ends well when it ends with exit code 0. Any other code, the
command being ended by something else before it finished, or the system's
shell not starting at all, stops the build.

## What you'll see {#what-youll-see}

```text
error  Hook stopped: `after_build` ended with exit code 3
  More: https://statac.dev/e/hook-stopped/
```

## Why Statac stops

A `before_build` command makes something the site is built from, such as
a stylesheet, and an `after_build` command finishes the built site, such
as by making its search index. Statac can't tell how far a command that
ended badly got, so it publishes nothing: the site built last time stays
as it was.

## How to fix it

Read what the command printed, which is where it says what went wrong.
Running the same line yourself, in the site's folder, shows the same.

An exit code of 127 is how the shell says it couldn't find a command (on
Windows, 9009). Check the command is installed and that the shell can
find it from where Statac was run.

A command is handed five values: `STATAC_SITE`, the site's folder;
`STATAC_OUT`, the folder to act on; `STATAC_DRAFTS`, `yes` or `no`;
`STATAC_SERVE`, `yes` where `statac serve` runs it and `no` otherwise;
and `STATAC_HOOK`, the setting it is written in. An `after_build` command
runs before the built site is put in its place, so it must act on
`STATAC_OUT`, not on `_site`, which still holds the site built last time.

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

Name a file the built site has, and build again.
