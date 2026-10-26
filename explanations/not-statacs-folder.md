# Not Statac's folder

The folder the built site should go in already contains files that
Statac didn't put there, or there is a file or link where the folder
should be.

## What you'll see {#what-youll-see}

```text
error  Not Statac's folder: `docs` already holds 2 files and 1 folder
  Statac replaces only folders it built, so build into a new folder or empty this one first.
  More: https://statac.dev/e/not-statacs-folder/
```

## Why Statac stops

Each build replaces the whole output folder, so nothing from an earlier
build is left behind. Statac leaves a `.statac-built` file in every
folder it builds, and only replaces a folder that has this file, or an
empty folder. Any other folder could have your own files in it, and a
mistyped folder name could point the build at it.

## How to fix it

Build into a new or empty folder. If the folder really is an old built
site from before Statac marked its folders, empty it yourself and build
again.

## Example

Choose another output folder, or move what is in `docs` somewhere else.
