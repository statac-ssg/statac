# Can't write

Statac couldn't write a file of the built site. The message names the file
and gives the system's reason, such as a full disk, a folder with the same
name, or no permission to write there.

`statac import` says the same of a new site's folder inside the site being
imported, or holding it, since writing one would change the other.

## What you'll see {#what-youll-see}

```text
error  Can't write `out/index.html`: a folder already has that name
  More: https://statac.dev/e/cant-write/
```

## Why Statac stops

A site with a file missing isn't the site you wrote, so the build stops
rather than leave it half written.

## How to fix it

Deal with the reason the message gives: free some space, move the folder
that is in the way, or give yourself permission to write to the output
folder. Then build again.

## Example

Something made a folder called `index.html` inside `out/`. Remove it and
build again.
