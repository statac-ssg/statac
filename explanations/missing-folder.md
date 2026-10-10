# Missing folder

`statac import` was run without the folder of the site to import.

## What you'll see {#what-youll-see}

```text
error  Missing folder: `statac import` needs the folder of the site to import
  Name it after the command: `statac import <folder>`.
  More: https://statac.dev/e/missing-folder/
```

## Why Statac stops

Importing reads a site another tool made and writes a new one beside it,
so it needs to know which site. Statac doesn't take the folder it is run
in, since the new site is written beside the old, outside that folder.

## How to fix it

Name the old site's folder after the command, and, if you like, the
folder to write the new site into after that.

## Example

```text
statac import
```

To import the site in `old` into `old-statac`, beside it:

```text
statac import old
```
