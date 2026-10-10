# Unexpected argument

The command line has a word the command has no use for. `statac build` and
`statac check` take one word, the site's folder, and `statac explain` takes
one name.

## What you'll see {#what-youll-see}

```text
error  Unexpected argument: `docs`
  Did you mean `--site=docs`?
  More: https://statac.dev/e/unexpected-argument/
```

## Why Statac stops

Statac can't tell what the word was meant to do, and leaving it out could
build a different site from the one you meant.

## How to fix it

Remove the word, or give it as an option.

## Example

```text
statac build docs public
```

To build the site in `docs` into `public`:

```text
statac build docs --out=public
```
