# Unreadable name

A folder or path on the command line contains bytes that aren't valid
UTF-8. This applies to the site's folder, the folder that `statac new`
or `statac import` writes into, and the folder given to `--out`.

## What you'll see {#what-youll-see}

```text
error  Unreadable name: `site-�` holds bytes that aren't UTF-8
  More: https://statac.dev/e/unreadable-name/
```

## Why Statac stops

Statac works with text. A name that isn't text could only be shown with
some of its bytes replaced, and a path with its bytes replaced is a
different path, so Statac would read or write somewhere you didn't ask
for.

## How to fix it

Rename the folder so that its name is valid text, then run the command
again with the new name.

## Example

A folder whose name contains the byte `0xFF`, which is not UTF-8:

```text
statac build site-1\xff
```

After renaming the folder:

```text
statac build site-1
```
