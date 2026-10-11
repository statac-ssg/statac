# Missing value

An option that needs a value was given without one. Statac's options take
their value after an `=`, with no space: `--site=docs`.

## What you'll see {#what-youll-see}

```text
error  Missing value: `--site` needs a folder
  Write it as `--site=docs`.
  More: https://statac.dev/e/missing-value/
```

## Why Statac stops

Without a value, the option means nothing, and Statac doesn't guess one.
If you wrote the value separately from the option, the message suggests
the two joined together.

## How to fix it

Write the value straight after the option and an `=`.

## Example

```text
statac build --site docs
```

Join the option and its value:

```text
statac build --site=docs
```
