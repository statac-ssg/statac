# Option given twice

The command line gives the same option twice. The folder written after
`statac build` or `statac check` is the site's folder, as `--site` gives it,
so giving both counts as `--site` twice.

## What you'll see {#what-youll-see}

```text
error  Option given twice: `--site`
  Keep one of them.
  More: https://statac.dev/e/option-given-twice/
```

## Why Statac stops

Statac can't tell which of the two values you meant, and taking either
would quietly ignore the other.

## How to fix it

Keep the option once, with the value you want.

## Example

```text
statac build docs --site=site
```

Keep one of them:

```text
statac build docs
```
