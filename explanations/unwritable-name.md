# Unwritable name

A file in `assets/` or `static/` has a name that some common disks can't
store. Windows reserves names such as `CON`, `NUL`, `COM1` and `LPT1` for
devices, with or without an extension. It also refuses the characters
`< > : " | ? * \`, and drops a dot or a space at the end of a name.

## What you'll see {#what-youll-see}

```text
error  Unwritable name: `static/con.txt` uses `con`, a device name on Windows
  More: https://statac.dev/e/unwritable-name/
```

## Why Statac stops

Statac applies the rules of the strictest disk to every site. That way, a
site that builds on one computer builds on all of them, and you can copy
the built site anywhere.

## How to fix it

Rename the file or folder the message names, and update anything that
links to it.

## Example

Rename `static/con.txt` to something else, such as `static/contact.txt`.
