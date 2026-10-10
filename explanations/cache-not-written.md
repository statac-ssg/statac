# Cache not written

Statac couldn't keep something in `.statac/`, the folder in the site's
folder where it keeps what saves the next build work: the copies of
photographs it made, the sharing cards it drew, the spare copy of the
built site, or where each post in a feed was published. The message
names which, and gives the system's reason, such as no permission to
write there.

## What you'll see {#what-youll-see}

```text
warning  Cache not written: can't write `.statac/pictures`: permission denied
  More: https://statac.dev/e/cache-not-written/
```

## Why Statac warns

Making a photograph's copies is the slowest part of a build, so Statac
keeps each copy and makes it once. Where it can't keep them the site is
still built whole and published, with every copy in it; the next build
makes them all again, and takes as long as this one did. Where it can't
keep the spare, `.statac/spare`, each build writes every file of the site
again and removes the old ones, where it would have written only what
changed. Where it can't keep `.statac/feeds`, a post that moves before
the next build is not offered the `id:` that keeps it the same post in
feed readers. Statac says so once, however much it couldn't keep.

## How to fix it

Deal with the reason the message gives, such as giving yourself permission
to write in the site's folder, or freeing room on the disk.

Where the reason is that something is a link, `.statac`, the folder
named in it, or a file there is a link to somewhere else. Statac never
reads, writes or removes anything through a link there, since it may lead
out of the site. Remove the link, and Statac makes a folder of its own.

Where the reason is that something else has its name, `.statac/spare` is
a folder Statac didn't build, or a folder stands where a file of
Statac's goes. Statac removes nothing it didn't make, so move the folder
away or delete it.

`.statac/` holds nothing that can't be made again, so deleting it is
always safe.

## Example

Give yourself permission to write in `.statac/`, and build again.
