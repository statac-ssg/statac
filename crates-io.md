<h1 align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://git.statac.dev/statac/statac/raw/branch/main/.github/readme/statac-dark.svg">
    <img src="https://git.statac.dev/statac/statac/raw/branch/main/.github/readme/statac-light.svg" alt="Statac" width="300">
  </picture>
</h1>

<p align="center">A friendlier static site generator.</p>

Statac makes a website from a folder of Markdown. You write pages in
Markdown, choose how they look with templates, and run `statac build`. It
writes a site of plain files, ready to put on any web host. Statac adds
no JavaScript to a page unless you ask it to.

It is one program in one file, with no plugins and nothing else to
install beside it.

<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://git.statac.dev/statac/statac/raw/branch/main/.github/readme/debug-panel-dark.png">
    <img src="https://git.statac.dev/statac/statac/raw/branch/main/.github/readme/debug-panel-light.png" alt="Statac's debug panel open at its Names tab over a page being served. A list shows each name a template can write, such as {{ page.title }}, {{ page.date }} and {{ page.language }}, with what it holds on this page. Beside it, {{ page.title }} is described, with its value on this page, Ben Ledi, and the file and line it was written at, content/walks/2026-09-13-ben-ledi.md, line 2." width="860">
  </picture>
</p>

While `statac serve` runs, the
[debug panel](https://statac.dev/debug-panel/) on every page shows the
problems, each name a template can write, what it holds on that page and
where its value came from.

## Installing

```sh
cargo install --locked statac
```

Cargo builds Statac from its source, which needs Rust 1.99 or later, a
C and C++ compiler, CMake and Perl, and NASM on a machine with an Intel
or AMD processor.

[Installing](https://statac.dev/install/) has the other ways: a script on
a Mac or Linux, Homebrew on a Mac, Scoop on Windows, and a package for
each kind of machine.

## Documentation

Everything else is at [statac.dev](https://statac.dev).

Report a problem in [Statac's issues](https://git.statac.dev/statac/statac/issues)
on git.statac.dev, Statac's home.

## Licence

Statac is offered under the
[MIT licence](https://git.statac.dev/statac/statac/src/branch/main/LICENSE-MIT)
or the
[Apache License 2.0](https://git.statac.dev/statac/statac/src/branch/main/LICENSE-APACHE),
at your choice.
[`THIRD-PARTY.md`](https://git.statac.dev/statac/statac/src/branch/main/THIRD-PARTY.md)
lists the work of others that it carries, each with its licence.
