# Can't quieten

`quiet:` in `statac.yaml` lists a problem that can't be quietened. You can
quieten a warning or a note once you have understood it. You can't quieten a
problem that stops the build, or one that leaves a page out of the published
site.

## What you'll see {#what-youll-see}

```text
error  Can't quieten: `duplicate-key` stops the build
  Take it out of `quiet:`.
  More: https://statac.dev/e/cant-quieten/
```

## Why Statac stops

If you could quieten a problem that stops the build or leaves a page out,
the published site would no longer be the one you wrote, and nothing would
tell you. Statac always reports these problems.

## How to fix it

Take the name out of `quiet:`, and fix the problem itself. Run
`statac explain` with the problem's name to see how.

## Example

In `statac.yaml`:

```yaml
quiet: [broken-link, duplicate-key]
```

A duplicate key stops the build, so it can't be quietened. Take it out of
the list:

```yaml
quiet: [broken-link]
```
