# Option given twice

The same option is given twice on the command line.

## What you'll see {#what-youll-see}

```text
error  Option given twice: `--out`
  Keep one of them.
  More: https://statac.dev/e/option-given-twice/
```

## Why Statac stops

Statac can't tell which of the two values you meant, and using either one
would silently ignore the other.

## How to fix it

Keep the option once, with the value you want.

## Example

```text
statac build --out=public --out=dist
```

Keep one of them:

```text
statac build --out=public
```
