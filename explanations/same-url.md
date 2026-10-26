# Same URL

Two files would be published at the same URL. Either their URLs are
exactly the same, or their names differ only in ways that most disks
ignore: capitals, or how an accented letter is stored. The message names
both files. If one of them is a paginated page of a list, such as
`/blog/page/2/`, the message names the list. If one of them is an old URL
in a page's `redirect_from`, the message calls it a redirect, and points
at that URL in `redirect_from`.

## What you'll see {#what-youll-see}

```text
error  Same URL: `assets/site.css` and `assets/site.scss` are both published at `/site.css`
  Rename one of them.
  More: https://statac.dev/e/same-url/
```

## Why Statac stops

Only one file can be published at a URL, so publishing either would
quietly lose the other. On the disks macOS and Windows use, names that
differ only in capitals refer to the same file. Names that differ only in
how an accent is stored look identical, and on macOS they are also the
same file. So the site would change depending on where it was built.

## How to fix it

Rename one of the two files, or remove the one you don't want. Files from
`assets/` and `static/` are both published at the root of the site, beside
the pages, so any of them can clash with another. Statac builds a Sass
file into a stylesheet and publishes that, so `site.scss` and `site.css`
in the same folder are published under the same name.

The site's search index is a few files in `search/`, at the root of each
language whose search box reads it: `search/index.js` and the files it
reads, or `fr/search/index.js` for a site's French pages, under the
site's path if it has one. A file of yours at one of those
URLs, such as `static/search/index.js`, clashes with it, and the
message names "the site's search index". Rename yours, or move it.

An old URL in a page's `redirect_from` publishes a redirect there, so
it clashes with anything else at that URL, and the message names it as
"a redirect in" the page. This often happens after a page is renamed: its
old URL is kept as a redirect, and a new page later given the old name
lands on it. Take the URL out of `redirect_from` if the new page should
have it, or give the new page another name.

A page's own `permalink` sets its URL whatever its file is called,
so renaming the file won't help. Change the `permalink`, which the
message points at.

A list too long for one page is paginated, with page 2 onwards beneath
it, at the URLs that `pagination.permalink` in `statac.yaml` sets. By
default, they are at `page/2/` and onwards. Give your page another URL,
or put the paginated pages elsewhere, such as
`pagination.permalink: "{list}more/{n}/"`.

## Example

If you have `static/Logo.png` and `static/logo.png`, which differ only in
capitals, keep one of them, and point every link at its name.

If page 2 of `/blog/` and `content/blog/page/2.md` are both published at
`/blog/page/2/`, move `content/blog/page/2.md`, or set `pagination.permalink`.

If `content/about.md` was renamed to `content/about-us.md`, which then
gained `redirect_from: [/about/]`, a new `content/about.md` clashes with
that redirect:

```text
error  Same URL: `content/about.md` and a redirect in `content/about-us.md` are both published at `/about/index.html`
  Take it out of `redirect_from` in `content/about-us.md`.
  More: https://statac.dev/e/same-url/
```
