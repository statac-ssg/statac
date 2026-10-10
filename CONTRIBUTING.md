# Contributing to Statac

## Reporting a problem

Open an [issue](https://github.com/statac-ssg/statac/issues). Say what
you ran, what Statac printed and what you expected, with the version
`statac --version` gives. The smallest site that shows the problem helps
most: often one page and one template.

A message that is unclear, blames the wrong file or line, or offers a
suggestion that doesn't work is a problem worth reporting, as much as a
wrongly built page.

## Changing the code

Open an issue before a pull request, so the change can be agreed before
you write it. Statac is developed on another forge and this repository
is a copy of it, so an accepted pull request is carried across by hand
and appears here as a commit with you as its author.

A change passes three commands:

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Every sentence Statac prints lives in `src/wording.rs`, and each kind of
problem has an explanation under `explanations/`. A change to what Statac
says changes both.

## What Statac won't take

Statac has no plugins and no code in its configuration, and neither will
be added. Hooks and snippets are how a site does what Statac doesn't.

## Licence

Unless you say otherwise, what you contribute is offered under the MIT
licence or the Apache License 2.0, at the user's choice, as Statac is.
