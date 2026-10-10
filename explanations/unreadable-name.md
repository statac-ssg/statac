# Unreadable name

A folder or path on the command line holds bytes that aren't UTF-8. This
covers the site's folder, the folder `statac new` or `statac import`
writes into, and the folder `--out` names.

## What you'll see {#what-youll-see}

```text
error  Unreadable name: `site-�` holds bytes that aren't UTF-8
  More: https://statac.dev/e/unreadable-name/
```

## Why Statac stops

Statac works in text. A name that isn't text could only be shown with some
of it replaced, and a path with its bytes replaced names another path, so
Statac would read or write a place you didn't name.

## How to fix it

Rename the folder so that its name is text, then name it again.

## Example

A folder whose name holds the byte `0xFF`, which is not UTF-8:

```text
statac build site-1\xff
```

After renaming the folder:

```text
statac build site-1
```
