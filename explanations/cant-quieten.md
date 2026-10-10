# Can't quieten

`quiet:` in `statac.yaml` names a problem that can't be quietened. You can
quieten a warning or a note you have understood. You can't quieten one that
stops the build, or one that leaves a page out of the published site.

## What you'll see {#what-youll-see}

```text
error  Can't quieten: `duplicate-key` stops the build
  Take it out of `quiet:`.
  More: https://statac.dev/e/cant-quieten/
```

## Why Statac stops

Quietening a problem that stops the build, or leaves a page out, would hide
a site that isn't the one you wrote. Statac always says so.

## How to fix it

Take the name out of `quiet:`, and fix the problem itself. `statac explain`
with its name says how.

## Example

In `statac.yaml`:

```yaml
quiet: [broken-link, duplicate-key]
```

A duplicate key stops the build, so it can't be quietened:

```yaml
quiet: [broken-link]
```
