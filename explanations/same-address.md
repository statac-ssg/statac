# Same address

Two files would be published at one address. Either they land at exactly
the same place, or their names differ only in ways most disks ignore:
capitals, or how an accented letter is stored. The message names both
files. Where one of them is a later page of a list, such as `/blog/page/2/`,
it names the list.

## What you'll see {#what-youll-see}

```text
error  Same address: `assets/site.css` and `assets/site.scss` are both published at `/site.css`
  Rename one of them.
  More: https://statac.dev/e/same-address/
```

## Why Statac stops

Only one file can stand at an address. Publishing either would quietly
lose the other. Names that differ only in capitals are one file on the
disks macOS and Windows use, and names that differ only in how an accent
is stored look identical and are one file on macOS, so the site would
change depending on where it was built.

## How to fix it

Rename one of the two files, or remove the one you don't want. Files from
`assets/` and `static/` are both published at the top of the site, beside
the pages, so any of them can land on another. A Sass file is published as the stylesheet it becomes, so `site.scss` and
`site.css` in one folder claim the same name.

A list too long for one page goes on onto later pages beneath it, where
`later_pages` in `statac.yaml` puts them: `page/2/` and on unless you say.
Give your page another address, or put the later pages elsewhere, such as
`later_pages: "{list}more/{n}/"`.

## Example

Where `static/Logo.png` and `static/logo.png` differ only in capitals, keep
one of them, and point every link at its name.

Where page 2 of `/blog/` and `content/blog/page/2.md` are both published at
`/blog/page/2/`, move `content/blog/page/2.md`, or set `later_pages`.
