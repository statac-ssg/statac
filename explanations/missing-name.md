# Missing name

`statac explain` was given no name. It explains one kind of problem at a
time, by its name.

## What you'll see {#what-youll-see}

```text
error  Missing name: `statac explain` needs the name of a problem
  The name is the last part of a problem's `More:` link.
  More: https://statac.dev/e/missing-name/
```

## Why Statac stops

Without a name, there is nothing to explain.

## How to fix it

Give the problem's name after `statac explain`. Every problem's name is the
last part of its `More:` link.

## Example

```text
statac explain
```

For a problem whose link ends `/e/broken-link`:

```text
statac explain broken-link
```
