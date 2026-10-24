# Nothing made

`after_build_outputs` lists a file or folder that the site's `after_build`
hook makes in the built site, but after the hook had run, nothing was
there.

## What you'll see {#what-youll-see}

```text
warning  Nothing made: `after_build` left nothing at `search/`
  More: https://statac.dev/e/nothing-made/
```

## Why Statac warns

Statac treats every link into what `after_build_outputs` lists as working,
because the command makes those files after the pages are built. If the
command makes nothing there, those links lead nowhere on the published
site, and nothing else would tell you.

## How to fix it

Check where the command writes. It must write into the folder Statac
passes to it as `STATAC_OUT`, and `after_build_outputs` gives each path from
the top of that folder: `search/` for a folder, `feed.json` for a file.

If the command no longer makes it, remove the name from
`after_build_outputs`.

## Example

In `statac.yaml`:

```yaml
after_build: ./index.sh "$STATAC_OUT/find"
after_build_outputs: [search/]
```

The command writes to `find/`. Either write `after_build_outputs: [find/]`,
or make the command write to `search/`.
