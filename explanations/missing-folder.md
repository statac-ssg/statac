# Missing folder

`statac import` was run without the folder of the site to import.

## What you'll see {#what-youll-see}

```text
error  Missing folder: `statac import` needs the folder of the site to import
  Name it after the command: `statac import <folder>`.
  More: https://statac.dev/e/missing-folder/
```

## Why Statac stops

Importing reads a site made by another tool and writes a new site beside
it, so Statac needs to know which site to read. It doesn't assume the
folder it is run in, because the new site is written beside the old one,
outside that folder.

## How to fix it

Name the old site's folder after the command. If you like, you can also
name the folder to write the new site into after that.

## Example

```text
statac import
```

To import the site in `old` into `old-statac`, beside it:

```text
statac import old
```
