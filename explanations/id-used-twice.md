# Id used twice

A heading is given an id in braces, `## Title {#id}`, that something
earlier on the page already has: another heading given the same id, or a
footnote. Statac writes footnotes' ids itself: `note-1`, `note-2` and so
on for the notes, and `back-1` and the like for the markers pointing at
them. The message points at the second, and shows the first heading
where there is one.

## What you'll see {#what-youll-see}

```text
warning  Id used twice: a heading above has the id `rust`
  Give this heading an id of its own.
  More: https://statac.dev/e/id-used-twice/
```

## Why Statac warns

An id names one place on a page. A link to it lands on the first thing
that has it, so the second heading can't be linked to. Statac keeps the id
as you wrote it, since it is yours, and tells you. An id Statac makes from
a heading's words never takes one you wrote: it gets a number instead.

## How to fix it

Give the heading an id of its own, or leave the braces off and let Statac
make one from its words. Ids starting `note-` or `back-` belong to the
footnotes.

## Example

In `content/post-1.md`:

```markdown
## Getting there {#route}

## Coming back {#route}
```

Give the second heading its own id:

```markdown
## Getting there {#route}

## Coming back {#return}
```
