# Can't remove

Statac couldn't remove a folder it made, or a file in `fetched/`.

Statac builds each site in a separate folder, and puts that folder in
place of the output folder only if the build succeeds. If it has no spare
to build into, or can't swap the two folders in one step, it makes
folders beside the output folder to do the swap in, with names ending
`.statac-new` or `.statac-old`. Those are the folders it couldn't remove.

`statac refresh` removes the file of a webmention that
`refuse_webmentions_from` refuses from `fetched/mentions/`, and the
sender's avatar from `fetched/faces/`. It never removes either one if it
is a link, and never removes anything that isn't a file.

## What you'll see {#what-youll-see}

```text
error  Can't remove `._site.statac-old`: permission denied
  More: https://statac.dev/e/cant-remove/
```

## Why Statac stops

Statac can't use a folder it can't remove for the next build either.
If this error follows a successful build, the new site is already in
place, and only the old copy beside it is left behind. If `statac refresh`
couldn't remove a refused webmention, Statac still leaves it out of the
site, and its file stays in `fetched/mentions/`.

## How to fix it

Deal with the reason the message gives, such as a program that has a
file in the folder open, then build again. Statac removes the folder on
the next build, or you can remove it yourself. For a file in `fetched/`
that is a link, replace the link with the file itself, then run
`statac refresh` again.

## Example

Close any program that has a file from the old site open, then build
again.
