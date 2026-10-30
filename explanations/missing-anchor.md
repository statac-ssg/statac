# Missing anchor

A link points at a place on a page (the part after `#`) that isn't
there. The page exists, but nothing on it has that id. A reader who
follows the link lands at the top of the page instead of the part you
meant.

## What you'll see {#what-youll-see}

```text
warning  Missing anchor: nothing on `/about/` has the id `getting-here`
  content/post-1.md, line 1
   1 │ See [how to get there](/about/#getting-here).
     │                        ^^^^^^^^^^^^^^^^^^^^
  Did you mean `#getting-there`?
  More: https://statac.dev/e/missing-anchor/
```

## Why Statac warns

Once every page is built, Statac knows each id on each page: every
heading's, every footnote's, and every `id` in HTML you or a template
wrote. It publishes the page with the link as written and tells you,
because the page is still worth reading. A heading's id is made from its
words. If you change the words, the id changes, and links to the heading
from other pages no longer find it.

## How to fix it

Point the link at an id that the page has. If one is close, the message
suggests it. Two ids that look the same can still differ in how their
letters were typed, for example when accents or marks were typed in a
different order. In that case, Statac suggests the id that is on the
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
