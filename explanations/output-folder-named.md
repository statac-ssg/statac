# Output folder named

The site's `after_build` command names the folder the built site is
published in, `_site` unless the build was asked for another. While the
command runs, that folder still holds the site built last time.

## What you'll see {#what-youll-see}

```text
warning  Output folder named: `_site` holds the site built last time while `after_build` runs
  Write it as `./index.sh "$STATAC_OUT"`.
  More: https://statac.dev/e/output-folder-named/
```

## Why Statac warns

Statac builds into a folder of its own and puts it in the published
site's place only once the whole build, `after_build` included, has ended
well. A command that names `_site` therefore acts on the last build: an
index is made of the old pages, old files are compressed or sent to the
server, and whatever the command writes there is replaced a moment later
by the new site. On a first build there is no such folder at all.

Nothing in what the command prints need show it, so Statac says so.

## How to fix it

Name the folder Statac hands the command, `STATAC_OUT`, where the line
names the output folder. `sh` writes it `"$STATAC_OUT"` and `cmd`
writes it `"%STATAC_OUT%"`. The message shows the line written that way.

Where the command really does mean the site built last time,
`quiet: [output-folder-named]` stops the warning being shown.

## Example

In `statac.yaml`:

```yaml
after_build: ./index.sh _site
```

While the command runs, `_site` still holds the site built last time.
Written to act on the new build, in `$STATAC_OUT`:

```yaml
after_build: ./index.sh "$STATAC_OUT"
```
