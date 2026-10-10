# Statac

Statac makes a website from a folder of writing. You write pages in
Markdown, choose how they look with templates, and run `statac build`. It
writes a site of plain files, ready to put on any web host, with nothing
for a reader's browser to run unless you put it there.

Statac explains itself. When something in the site is wrong, it shows you
the file, the line and the exact text, says what it did or why it
stopped, and links to a page about that kind of problem.

It is one program in one file, with no plugins and nothing else to
install beside it.

## Installing

On a Mac, with [Homebrew](https://brew.sh):

```sh
brew install statac-ssg/statac/statac
```

On Linux:

```sh
curl -fsSL https://get.statac.dev | sh
```

For Windows, or another way to install, each
[release](https://github.com/statac-ssg/statac/releases) has a package
for each kind of machine. [Installing](https://statac.dev/install/)
says which to take and how to check it.

## A first site

```sh
statac new site-1
cd site-1
statac serve
```

`statac new` asks four questions and writes a site you can change;
`statac serve` builds it, shows it at `http://localhost:3000/` and builds
it again on every save.

## Documentation

Everything else is at [statac.dev](https://statac.dev).

## Licence

Statac is offered under the MIT licence or the Apache License 2.0, at your
choice. `THIRD-PARTY.md` lists the work of others that it carries, each
with its licence.
