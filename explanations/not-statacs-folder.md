# Not Statac's folder

The folder the built site should go in already holds files Statac didn't
put there, or a file or link stands where the folder should be.

## What you'll see {#what-youll-see}

```text
error  Not Statac's folder: `docs` already holds 2 files and 1 folder
  Statac replaces only folders it built, so build into a new folder or empty this one first.
  More: https://statac.dev/e/not-statacs-folder/
```

## Why Statac stops

Each build replaces the whole output folder, so nothing from an earlier
build lingers. Statac leaves a `.statac-built` file in every folder it
builds, and replaces only a folder that has one, or an empty one. Anything
else could be your own files, sent there by a mistyped folder name.

## How to fix it

Build into a new or empty folder. If the folder really is an old built
site, from before Statac marked it, empty it yourself and build again.

## Example

Choose another output folder, or move what is in `docs` somewhere else.
