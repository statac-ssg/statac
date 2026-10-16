# Cache not written

Statac couldn't save something in `.statac/`. This folder, inside the
site's folder, is where Statac keeps work that saves time on the next
build: the copies of photographs it made, the sharing cards it drew, the
spare copy of the built site, or a record of where each post in a feed was
published. It also keeps there the lock, `.statac/lock`, that lets only
one build of the site run at a time. The message says which of these it
couldn't save, and gives the system's reason, such as no permission to
write there.

## What you'll see {#what-youll-see}

```text
warning  Cache not written: can't write `.statac/pictures`: permission denied
  More: https://statac.dev/e/cache-not-written/
```

## Why Statac warns

Making copies of a photograph is the slowest part of a build, so Statac
keeps each copy and makes it only once. If it can't keep them, the site
is still built in full and published, with every copy in it, but the
next build makes them all again and takes as long as this one did. If it
can't keep the spare, `.statac/spare`, each build rewrites every file of
the site and removes the old ones, instead of writing only the files that
changed. If it can't keep `.statac/feeds` and a post moves before the next build,
Statac can't offer that post the `id:` line that keeps it the same post in
feed readers. If it can't keep the lock, a build doesn't wait for
another build of the same site, such as `statac serve` in another
window, and two that run at once may get in each other's way. Statac
warns only once, however much it couldn't keep.

## How to fix it

Fix the cause the message gives. For example, give yourself permission
to write in the site's folder, or free some space on the disk. If the
disk can't lock a file, as some network disks can't, keep the site on a
disk of the computer's own, or run one build of it at a time.

If the reason is that something is a link, then `.statac`, the folder
named in the message, or a file in it is a link to somewhere else. Statac
never reads, writes or removes anything through a link there, because the
link may lead outside the site. Remove the link, and Statac creates a
folder of its own.

If the reason is that something else has its name, then either
`.statac/spare` is a folder Statac didn't build, or a folder is in the
place where one of Statac's files goes. Statac removes nothing it didn't
make, so move the folder away or delete it.

Everything in `.statac/` can be made again except two things:
`.statac/feeds`, the record of where each feed post was published, and
`.statac/token`, if you keep your webmention token there. Deleting
anything else is safe.

## Example

Give yourself permission to write in `.statac/`, then build again.
