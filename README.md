<h1 align="center">
  <img src=".github/readme/statac-light.svg#gh-light-mode-only" alt="Statac" width="300">
  <img src=".github/readme/statac-dark.svg#gh-dark-mode-only" alt="Statac" width="300">
</h1>

<p align="center">A friendlier static site generator.</p>

<p align="center">
  <a href="https://git.statac.dev/statac/statac/releases/latest"><img alt="The latest release" src="https://img.shields.io/gitea/v/release/statac/statac?gitea_url=https%3A%2F%2Fgit.statac.dev&label=release&color=CC4E00"></a>
  <a href="https://statac.dev/install/"><img alt="Runs on macOS, Linux and Windows" src="https://img.shields.io/badge/runs%20on-macOS%20%C2%B7%20Linux%20%C2%B7%20Windows-191919"></a>
  <a href="#licence"><img alt="Licence: MIT or Apache-2.0" src="https://img.shields.io/badge/licence-MIT%20or%20Apache--2.0-191919"></a>
</p>

<p align="center"><b><a href="https://statac.dev/getting-started/">Get started</a> · <a href="https://statac.dev/install/">Install</a> · <a href="https://statac.dev">Documentation</a> · <a href="https://statac.dev/e/">Problems explained</a> · <a href="CHANGELOG.md">Changes</a></b></p>

Statac makes a website from a folder of Markdown. You write pages in
Markdown, choose how they look with templates, and run `statac build`. It
writes a site of plain files, ready to put on any web host. Statac adds
no JavaScript to a page unless you ask it to.

It is one program in one file, with no plugins and nothing else to
install beside it.

## It explains itself

<p align="center">
  <img src=".github/readme/debug-panel-light.png#gh-light-mode-only" alt="Statac's debug panel open at its Names tab over a page being served. A list shows each name a template can write, such as {{ page.title }}, {{ page.date }} and {{ page.language }}, with what it holds on this page. Beside it, {{ page.title }} is described, with its value on this page, Ben Ledi, and the file and line it was written at, content/walks/2026-09-13-ben-ledi.md, line 2." width="860">
  <img src=".github/readme/debug-panel-dark.png#gh-dark-mode-only" alt="Statac's debug panel open at its Names tab over a page being served. A list shows each name a template can write, such as {{ page.title }}, {{ page.date }} and {{ page.language }}, with what it holds on this page. Beside it, {{ page.title }} is described, with its value on this page, Ben Ledi, and the file and line it was written at, content/walks/2026-09-13-ben-ledi.md, line 2." width="860">
</p>

When something in the site is wrong, Statac shows you the file, the line
and the exact text, says what it did or why it stopped, and links to a
page about that kind of problem. `statac explain broken-link` shows the
same explanation in the terminal.

While `statac serve` runs, the
[debug panel](https://statac.dev/debug-panel/) on every page shows the
problems, each name a template can write, what it holds on that page and
where its value came from.

## What you get

- **Broken links caught.** Statac checks every
  [link within your site](https://statac.dev/problems/#links) before
  anything is published, including the heading it points to.
- **Images handled.** Write an ordinary Markdown
  [image](https://statac.dev/images/). Readers get it in several sizes,
  as AVIF with a JPEG fallback.
- **Search built in.** Once you add a
  [search box](https://statac.dev/search/) to your site, Statac writes a
  search index at each build, and the box finds pages as the reader
  types, with nothing outside your site needed. `statac search` gives
  the same results in the terminal.
- **Social cards.** Statac draws a
  [social card](https://statac.dev/social-cards/) for each page, from a
  layout you set, in fonts and over a background of your own.
- **Bring your site with you.** `statac import` brings a Hugo, Jekyll,
  Zola or Eleventy site across, and lists anything it couldn't bring.
  See [Arriving from another tool](https://statac.dev/import/).
- **One program.** Sass, code colours, feeds and a sitemap need no
  plugin: Statac does each once you ask. The stylesheet that
  [colours code](https://statac.dev/code-colours/) and the templates for
  [feeds and a sitemap](https://statac.dev/feeds/) are your own, and the
  docs have them ready to copy.

And the rest: tags and series, several languages, webmentions, a preview
that reloads as you save, and nothing published unless the whole build
succeeds.

## Installing

On a Mac, with [Homebrew](https://brew.sh):

```sh
brew tap statac/statac https://git.statac.dev/statac/homebrew-statac
brew install statac
```

On Linux:

```sh
curl -fsSL https://get.statac.dev | sh
```

On Windows, with [Scoop](https://scoop.sh):

```powershell
scoop bucket add statac https://git.statac.dev/statac/scoop-statac
scoop install statac
```

For another way to install, each release has a package for each kind
of machine. [Installing](https://statac.dev/install/) says which to
take and how to check it.

## A first site

```sh
statac new site-1
cd site-1
statac serve
```

`statac new` asks five questions and writes the site's settings; the
pages, templates and stylesheet are yours to write, as
[Templates](https://statac.dev/templates/) describes. `statac serve`
builds the site, shows it at `http://localhost:3000/` and builds it again
on every save.

## Documentation

Everything else is at [statac.dev](https://statac.dev).

Report a problem in [Statac's issues](https://git.statac.dev/statac/statac/issues)
on git.statac.dev, Statac's home. The copy on GitHub is
read-only, with its issues turned off.

## Licence

Statac is offered under the MIT licence or the Apache License 2.0, at your
choice. `THIRD-PARTY.md` lists the work of others that it carries, each
with its licence.
