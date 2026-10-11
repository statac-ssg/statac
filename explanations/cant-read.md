# Can't read

Statac couldn't read a file or folder in the site. The message says which
one, and gives the reason the system reported, such as not having
permission to read it.

## What you'll see {#what-youll-see}

```text
error  Can't read `assets/site.css`: permission denied
  More: https://statac.dev/e/cant-read/
```

## Why Statac stops

A site with a file missing isn't the site you wrote, so the build stops
rather than publish it without that file.

## How to fix it

Deal with the reason the message gives, for example by giving yourself
permission to read the file. Then build again.

## Example

Give yourself permission to read `assets/site.css`, and build again.
