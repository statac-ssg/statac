# Id used twice

A heading has an id in braces, such as `## Title {#id}`, but something
earlier on the page already has that id. It may be another heading with
the same id, or a footnote. Statac gives footnotes their ids itself:
`note-1`, `note-2` and so on for the notes, and `back-1` and so on for the
markers that link to them. The message points at the second use of the
id, and also shows the first if it is a heading.

## What you'll see {#what-youll-see}

```text
warning  Id used twice: a heading above has the id `rust`
  Give this heading an id of its own.
  More: https://statac.dev/e/id-used-twice/
```

## Why Statac warns

An id marks one place on a page. A link to it goes to the first thing
with that id, so nothing can link to the second heading. Statac keeps the
id as you wrote it, because you chose it, and warns you. An id that Statac
makes from a heading's words never reuses one you wrote. Statac adds a
number to its own id instead.

## How to fix it

Give the heading its own id, or remove the braces and let Statac make one
from the heading's words. Ids starting with `note-` or `back-` are used by
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
