# Same address

Two files would be published at the same address. Either their addresses
are exactly the same, or their names differ only in ways that most disks
ignore: capitals, or how an accented letter is stored. The message names
both files. If one of them is a later page of a list, such as
`/blog/page/2/`, the message names the list.

## What you'll see {#what-youll-see}

```text
error  Same address: `assets/site.css` and `assets/site.scss` are both published at `/site.css`
  Rename one of them.
  More: https://statac.dev/e/same-address/
```

## Why Statac stops

Only one file can be published at an address, so publishing either would
quietly lose the other. On the disks macOS and Windows use, names that
differ only in capitals refer to the same file. Names that differ only in
how an accent is stored look identical, and on macOS they are also the
same file. So the site would change depending on where it was built.

## How to fix it

Rename one of the two files, or remove the one you don't want. Files from
`assets/` and `static/` are both published at the top of the site, beside
the pages, so any of them can clash with another. A Sass file is published as the
stylesheet it becomes, so `site.scss` and `site.css` in the same folder
are published under the same name.

A list too long for one page continues on later pages beneath it, at the
addresses that `later_pages` in `statac.yaml` sets. By default, they are
at `page/2/` and onwards.
Give your page another address, or put the later pages elsewhere, such as
`later_pages: "{list}more/{n}/"`.

## Example

If you have `static/Logo.png` and `static/logo.png`, which differ only in
capitals, keep one of them, and point every link at its name.

If page 2 of `/blog/` and `content/blog/page/2.md` are both published at
`/blog/page/2/`, move `content/blog/page/2.md`, or set `later_pages`.
