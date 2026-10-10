# Unwritable name

A file in `assets/` or `static/` has a name some common disk can't hold.
Windows keeps names such as `CON`, `NUL`, `COM1` and `LPT1` for devices,
with or without an extension; refuses the characters `< > : " | ? * \`;
and drops a dot or a space at the end of a name.

## What you'll see {#what-youll-see}

```text
error  Unwritable name: `static/con.txt` uses `con`, a device name on Windows
  More: https://statac.dev/e/unwritable-name/
```

## Why Statac stops

Statac holds every site to the rules of the strictest disk, so a site that
builds on one computer builds on all of them, and a copy of the built site
can be put anywhere.

## How to fix it

Rename the file or folder the message names, and update anything that
links to it.

## Example

Rename `static/con.txt` to something else, such as `static/contact.txt`.
