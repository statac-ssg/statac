# Can't read

Statac couldn't read a file or folder of the site. The message names it
and gives the system's reason, such as no permission to read it.

## What you'll see {#what-youll-see}

```text
error  Can't read `assets/site.css`: permission denied
  More: https://statac.dev/e/cant-read/
```

## Why Statac stops

A site with a file left out isn't the site you wrote, so the build stops
rather than publish it without.

## How to fix it

Deal with the reason the message gives, such as giving yourself permission
to read the file, then build again.

## Example

Give yourself permission to read `assets/site.css`, and build again.
