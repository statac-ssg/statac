# Can't remove

Statac couldn't remove a folder it made, or a file in `fetched/`. It
builds each site in a folder of its own and puts that in the output's
place only when the build succeeds. Where it has no spare to build into,
or can't change the two over in one step, it makes folders beside the
output for the purpose, named with `.statac-new` or `.statac-old` at the
end; those are the folders it couldn't remove. `statac refuse` removes a
webmention's file from `fetched/mentions/`, and its sender's face from
`fetched/faces/`, but never where it is a link, or anything but a file.

## What you'll see {#what-youll-see}

```text
error  Can't remove `._site.statac-old`: permission denied
  More: https://statac.dev/e/cant-remove/
```

## Why Statac stops

A folder Statac can't remove is one it can't use for the next build
either. When this comes after a successful build, the new site is already
in place; only the old copy beside it is left. A webmention `statac
refuse` couldn't remove is still shown.

## How to fix it

Deal with the reason the message gives, such as a program holding a file
in the folder open, then build again. Statac removes the folder itself
next time, or you can remove it. For a file in `fetched/`, put the file
itself in place of a link, then run `statac refuse` again.

## Example

Close whatever has a file from the old site open, and build again.
