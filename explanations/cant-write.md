# Can't write

Statac couldn't write one of the built site's files. The message shows the
file and the reason the operating system gave, such as a full disk or no
permission to write there. Statac also reports it for a file in
`content/` whose name can't be part of a URL, such as one with `#` or `%`
in it.

`statac import` reports the same error if the folder for the new site is
inside the site being imported, or contains it, because writing one would
change the other.

## What you'll see {#what-youll-see}

```text
error  Can't write `_site`: permission denied
  More: https://statac.dev/e/cant-write/
```

## Why Statac stops

A site with a missing file isn't the site you wrote, so the build stops
rather than leave it half written.

## How to fix it

Fix the cause that the message gives: free some space, give yourself
permission to write to the output folder, or rename the file. Then build
again.

## Example

The folder the site is built into, `_site`, is read-only. Make it
writable, then build again.
