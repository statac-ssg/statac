# Nothing made

`after_build_makes` names a file or a folder the site's `after_build`
command makes in the built site, and once the command had run there was
nothing there.

## What you'll see {#what-youll-see}

```text
warning  Nothing made: `after_build` left nothing at `search/`
  More: https://statac.dev/e/nothing-made/
```

## Why Statac warns

Statac takes every link into what `after_build_makes` names to land,
since the command makes it after the pages are built. Where the command
makes nothing there, those links lead nowhere on the published site, and
nothing else would say so.

## How to fix it

Check where the command writes. It must write into the folder Statac
hands it as `STATAC_OUT`, and `after_build_makes` names the place from
the top of that folder: `search/` for a folder, `feed.json` for a file.

Where the command no longer makes it, take the name out of
`after_build_makes`.

## Example

In `statac.yaml`:

```yaml
after_build: ./index.sh "$STATAC_OUT/find"
after_build_makes: [search/]
```

The command writes to `find/`. Write `after_build_makes: [find/]`, or
have the command write to `search/`.
