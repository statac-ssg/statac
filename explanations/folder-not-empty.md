# Folder not empty

`statac new`, or `statac import` when it makes the new site, was given a
folder that already has something in it. The message lists the first few
files and folders in it, including hidden ones and links, and says how
many more there are.

## What you'll see {#what-youll-see}

```text
error  Folder not empty: `notes` holds `a.txt`
  Give `statac new` an empty folder, or the name of a new one.
  More: https://statac.dev/e/folder-not-empty/
```

## Why Statac stops

Statac writes a new site only into an empty folder or a new one, so that
nothing already there is overwritten or mixed in with the site's own
files. Statac never decides for you which of the files it finds you want
to keep.

## How to fix it

Give the name of a folder that doesn't exist yet, and Statac creates it.
Or move the folder's contents somewhere else, and run `statac new` again.

## Example

```text
statac new notes
```

This fails because `notes` already contains `a.txt`. To make the site in
a new folder instead:

```text
statac new site-1
```
