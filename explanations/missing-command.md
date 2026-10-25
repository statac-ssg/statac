# Missing command

The command line has options, but no command for them to apply to.

## What you'll see {#what-youll-see}

```text
error  Missing command: no `build`, `check`, `serve`, `stamp`, `refresh`, `new`, `import` or `explain`
  Run `statac --help` to see the commands.
  More: https://statac.dev/e/missing-command/
```

## Why Statac stops

Options say how to do something, not what to do. Statac doesn't guess
which command you meant.

## How to fix it

Add `build`, `check`, `serve`, `stamp`, `refresh`, `new`, `import` or
`explain` after `statac`. `statac --help` lists the commands
and what each does, and `statac check --help` shows only the options `check`
takes.

## Example

```text
statac --out=public
```

To build the site into `public`:

```text
statac build --out=public
```
