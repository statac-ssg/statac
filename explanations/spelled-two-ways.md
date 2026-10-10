# Spelled two ways

Pages write one tag, or one series, with two spellings whose addresses
come out the same, such as `Rust` on some pages and `rust` on others.

## What you'll see {#what-youll-see}

```text
warning  Tag spelled two ways: `Rust` (12 pages) and `rust` (2 pages) are one tag, shown as `Rust`
  Use one spelling on every page to choose how it's shown.
  More: https://statac.dev/e/spelled-two-ways/
```

## Why Statac warns

Both spellings would be published at one address, `/tags/rust/`, so they
can only be one tag. Statac makes them one under the spelling most pages
use, and goes on. Of two spellings used by as many pages, the one that
sorts first goes, which puts capitals first. Statac never changes the
capitals of a tag itself: the tag is shown as your pages spell it most.

It tells you because a page showing its tags shows the spelling the tag
goes by, which may not be the one written on that page.

## How to fix it

The spelling is yours to choose: use the one you want on every page. The
warning names each spelling and how many pages use it, so you can see
which pages to change.

To keep two tags apart that would come out at one address, give one of
them an address part of its own with `tags.slugs` (or `series.slugs`) in
`statac.yaml`.

## Example

`content/post-1.md` and `content/post-2.md` say:

```yaml
tags: [Rust]
```

and `content/post-3.md` says:

```yaml
tags: [rust]
```

`Rust` (2 pages) and `rust` (1 page) are one tag, shown as `Rust`. To
show it as `rust` instead, write it that way in `content/post-1.md` and
`content/post-2.md`:

```yaml
tags: [rust]
```
