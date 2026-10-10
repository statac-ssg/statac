# More written twice

A page's writing has `<!--more-->` on a line of its own more than once.
The first one ends the excerpt a list shows; the later ones end nothing.

## What you'll see {#what-youll-see}

```text
warning  More written twice: the excerpt ends at the first `<!--more-->`
  Delete the later one.
  More: https://statac.dev/e/more-written-twice/
```

## Why Statac warns

Statac is sure what it does here: the excerpt ends at the first
`<!--more-->`, as it does wherever excerpts are written this way, and the
page is published. It tells you, because a later marker may be where you
meant the excerpt to end.

## How to fix it

Delete the later `<!--more-->`. If the excerpt should end there instead,
delete the first one.

## Example

In `content/post-1.md`:

```markdown
The part a list shows.

<!--more-->

More of the post.

<!--more-->

The rest.
```

The excerpt ends at the first. Keep one:

```markdown
The part a list shows.

<!--more-->

More of the post.

The rest.
```
