# Missing anchor

A link points at a place on a page, the part after `#`, and the page is
there but nothing on it has that id. A reader who follows the link lands
at the top of the page instead of the part you meant.

## What you'll see {#what-youll-see}

```text
warning  Missing anchor: nothing on `/posts/post-1/` has the id `rust`
  Did you mean `#rust-1`?
  More: https://statac.dev/e/missing-anchor/
```

## Why Statac warns

Once every page is built, Statac knows each id on each page: every
heading's, every footnote's, and every `id` in HTML you or a template
wrote. It publishes the page with the link as written and tells you,
because the page is still worth reading. A heading's id comes from its
words, so changing the words changes the id, and links to it from other
pages then miss.

## How to fix it

Point the link at an id the page has. When one is close, Statac offers it.
Two ids that look the same can differ in how their letters were typed, as
with accents or marks typed in another order; Statac offers the one on the
page. To keep an id when you change a heading, give the heading its id in
braces: `## New words {#old-words}`.

## Example

In `content/post-1.md`:

```markdown
See [how to get there](/about/#getting-here).
```

If the heading on the page is `## Getting there`, its id is
`getting-there`:

```markdown
See [how to get there](/about/#getting-there).
```
