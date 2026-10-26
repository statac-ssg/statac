# Id used twice

A heading or a table has an id in braces, such as `## Title {#id}` or a
line of `{#id}` under a table's last row, but something earlier on the
page already has that id: another heading or table with the same id. The
message points at the second use of the id, and also shows the first.

## What you'll see {#what-youll-see}

```text
warning  Id used twice: a heading above has the id `rust`
  Give this heading an id of its own.
  More: https://statac.dev/e/id-used-twice/
```

## Why Statac warns

An id marks one place on a page. A link to it goes to the first thing
with that id, so nothing can link to the second. Statac keeps the id as
you wrote it, because you chose it, and warns you. An id that Statac
makes, for a heading from its words or for a footnote, never reuses one
you wrote. Statac adds a number to its own id instead: a footnote's is
`note-1` unless you gave something that id, and then `note-1-1`.

## How to fix it

Give the heading or table its own id, or remove the braces: Statac then
makes a heading's id from its words, and leaves a table without one.

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
