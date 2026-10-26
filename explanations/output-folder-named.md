# Output folder named

The site's `after_build` hook contains the name of the folder the
built site is published in. That folder is `_site` unless `--out` names
another. While the command runs, the folder still has the site built
last time in it.

## What you'll see {#what-youll-see}

```text
warning  Output folder named: `_site` holds the site built last time while `after_build` runs
  Write it as `./index.sh "$STATAC_OUT"`.
  More: https://statac.dev/e/output-folder-named/
```

## Why Statac warns

Statac builds into a separate folder, and puts it in place of the
published site only after the whole build, including `after_build`, has
succeeded. So a command that names `_site` works on the previous build. It
makes an index of the old pages, or compresses old files or sends them
to the server, and the new site replaces anything it writes there a
moment later. On a first build, the folder doesn't exist at all.

The command's output may show no sign of this, so Statac warns you.

## How to fix it

In the command, replace the output folder's name with
`STATAC_OUT`, the folder Statac hands the command. In `sh` you write it as
`"$STATAC_OUT"`, and in `cmd` as `"%STATAC_OUT%"`. The message shows the
line rewritten this way.

If the command really is meant to use the site built last time,
`quiet: [output-folder-named]` stops the warning from being shown.

## Example

In `statac.yaml`:

```yaml
after_build: ./index.sh _site
```

While the command runs, `_site` still holds the site built last time.
To act on the new build in `$STATAC_OUT`, write:

```yaml
after_build: ./index.sh "$STATAC_OUT"
```
