# Spelled two ways

Your pages spell one tag, or one series, in two ways that give the same
URL, such as `Rust` on some pages and `rust` on others.

## What you'll see {#what-youll-see}

```text
warning  Tag spelled two ways: `Rust` (2 pages) and `rust` (1 page) are one tag, which Statac shows as `Rust`, the commonest
  content/post-3.md, line 2
   2 │ tags: [rust]
     │        ^^^^
  Use one spelling on every page to choose how it's shown.
  More: https://statac.dev/e/spelled-two-ways/
```

## Why Statac warns

Both spellings would be published at the same URL, `/tags/rust/`, so
they can only be one tag. Statac merges them into one tag under the
spelling most pages use, and carries on. If two spellings are used by the
same number of pages, Statac uses the one that sorts first, which puts
capitals first. Statac never changes the capitals of a tag on its own: it
shows the tag the way most of your pages spell it.

Statac warns you because a page that shows its tags shows this one
spelling, which may not be the one written on that page.

## How to fix it

You choose the spelling. Use the one you want on every page. The warning
lists each spelling and how many pages use it, so you can see which pages
to change.

If you want two separate tags that would otherwise get the same URL, use
`tags.slugs` (or `series.slugs`) in `statac.yaml` to give one of them its
own slug.

## Example

`content/post-1.md` and `content/post-2.md` contain:

```yaml
tags: [Rust]
```

and `content/post-3.md` contains:

```yaml
tags: [rust]
```

`Rust` (2 pages) and `rust` (1 page) are one tag, which Statac shows as
`Rust`, the commonest. To show it as `rust` instead, write it that way in
`content/post-1.md` and `content/post-2.md`:

```yaml
tags: [rust]
```
