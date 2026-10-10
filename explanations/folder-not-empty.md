# Folder not empty

`statac new`, or `statac import` for the new site, was given a folder
that already holds something. It names
the first few files and folders in it, hidden ones and links among them,
and how many more there are.

## What you'll see {#what-youll-see}

```text
error  Folder not empty: `site-1` holds `a.txt`
  Give `statac new` an empty folder, or the name of a new one.
  More: https://statac.dev/e/folder-not-empty/
```

## Why Statac stops

A new site is written into an empty folder, or a new one, so that
nothing already there is written over or mixed in with the site's own
files. Statac never decides for you which of the files it finds are
yours to keep.

## How to fix it

Name a folder that doesn't exist yet, and Statac makes it. Or move what
the folder holds somewhere else, and run `statac new` again.

## Example

```text
statac new notes
```

where `notes` holds `a.txt`. To make the site in a new folder instead:

```text
statac new site-1
```
