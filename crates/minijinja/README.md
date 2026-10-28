# statac-minijinja

MiniJinja 3.0.1, the template engine by Armin Ronacher, changed for
[Statac](https://statac.dev), a static site generator. Statac's changes
add hooks and tweaks only Statac needs; each file a change touches says
so at its top.

It is published so that Statac can be installed from crates.io, and is
kept for Statac's use alone. It follows Statac's needs, not MiniJinja's
releases, and makes no promise to anyone else: its interface may change
in any release, whatever its version says.

For a template engine to use in your own project, use MiniJinja itself:
[crates.io/crates/minijinja](https://crates.io/crates/minijinja), with
its source at
[github.com/mitsuhiko/minijinja](https://github.com/mitsuhiko/minijinja).

## Licence

Apache-2.0, as MiniJinja is, its text in the `LICENSE` file beside this
one.
