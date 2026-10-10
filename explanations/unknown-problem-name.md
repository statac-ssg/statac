# Unknown problem name

A name given to `quiet:` in `statac.yaml`, or to `statac explain`, isn't the
name of any problem Statac knows. Every problem's name is the last part of
its `More:` link.

## What you'll see {#what-youll-see}

```text
error  Unknown problem name: `brokn-link`
  Did you mean `broken-link`?
  More: https://statac.dev/e/unknown-problem-name/
```

## Why Statac stops

A misspelt name in `quiet:` would quieten nothing, and you would go on
seeing the problem you meant to quieten without knowing why. When a known
name is close, Statac offers it.

## How to fix it

Copy the name from the end of the problem's `More:` link.

## Example

In `statac.yaml`:

```yaml
quiet: [brokn-link]
```

The problem is called `broken-link`:

```yaml
quiet: [broken-link]
```
