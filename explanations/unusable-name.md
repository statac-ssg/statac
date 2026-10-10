# Unusable name

A folder or path on the command line holds a control character, such as
the escape that starts a terminal's colour codes. This covers the site's
folder, the folder `statac new` or `statac import` writes into, and the
folder `--out` names.

## What you'll see {#what-youll-see}

```text
error  Unusable name: `site-1␛` holds a character that can't be shown
  Name the folder without it.
  More: https://statac.dev/e/unusable-name/
```

## Why Statac stops

A control character can't be shown as it is: a terminal acts on it rather
than drawing it, so the folder's name could not be told back to you. Statac
stops before it makes a folder with such a name.

## How to fix it

Name the folder without the character. If an existing folder has it in its
name, rename the folder.

## Example

A folder named with an escape character, written here as `<ESC>`:

```text
statac new site-1<ESC>
```

Without it:

```text
statac new site-1
```
