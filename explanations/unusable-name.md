# Unusable name

A folder or path on the command line contains a control character, such
as the escape character that starts a terminal's colour codes. This
applies to the site's folder, the folder that `statac new` or
`statac import` writes into, and the folder given to `--out`.

## What you'll see {#what-youll-see}

```text
error  Unusable name: `site-1␛` holds a character that can't be shown
  Name the folder without it.
  More: https://statac.dev/e/unusable-name/
```

## Why Statac stops

A control character can't be shown as it is, because a terminal acts on it
instead of displaying it. So Statac couldn't show you the folder's name in
its messages. Statac stops before it creates a folder with such a name.

## How to fix it

Name the folder without the character. If an existing folder has it in its
name, rename the folder.

## Example

A folder name with an escape character in it, shown here as `<ESC>`:

```text
statac new site-1<ESC>
```

Without it:

```text
statac new site-1
```
