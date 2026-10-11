# Can't write

Statac couldn't write one of the built site's files. The message shows the
file and the reason the operating system gave, such as a full disk, a
folder with the same name, or no permission to write there.

`statac import` reports the same error if the folder for the new site is
inside the site being imported, or contains it, because writing one would
change the other.

## What you'll see {#what-youll-see}

```text
error  Can't write `out/index.html`: a folder already has that name
  More: https://statac.dev/e/cant-write/
```

## Why Statac stops

A site with a missing file isn't the site you wrote, so the build stops
rather than leave it half written.

## How to fix it

Fix the cause that the message gives: free some space, move the folder
that is in the way, or give yourself permission to write to the output
folder. Then build again.

## Example

Something made a folder called `index.html` inside `out/`. Remove it and
build again.
