# Unknown command

The first word after `statac` isn't one of its commands. Statac's commands
are `build`, `check`, `serve` and `explain`, and `statac --help` lists them.

## What you'll see {#what-youll-see}

```text
error  Unknown command: `biuld`
  Did you mean `statac build`?
  More: https://statac.dev/e/unknown-command/
```

## Why Statac stops

Statac can't tell what you meant to do, and doing the wrong thing could
write files you didn't ask for. If a command is close to what you wrote,
Statac suggests it.

## How to fix it

Use one of Statac's commands. Statac accepts only one spelling for each of
its words, so `statac help` must be written as `statac --help`.

## Example

```text
statac biuld
```

The command is `build`:

```text
statac build
```
