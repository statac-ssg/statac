# More written twice

A page's writing has `<!--more-->` on a line of its own more than once.
The first one marks the end of the excerpt that a list shows. The later
ones have no effect.

## What you'll see {#what-youll-see}

```text
warning  More written twice: the excerpt ends at the first `<!--more-->`
  Delete the later one.
  More: https://statac.dev/e/more-written-twice/
```

## Why Statac warns

There is no doubt about what happens here. The excerpt ends at the first
`<!--more-->`, which is the usual rule wherever excerpts are marked this
way, and the page is published. Statac warns you because you may have
meant the excerpt to end at a later marker.

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

The excerpt ends at the first marker. Keep only one:

```markdown
The part a list shows.

<!--more-->

More of the post.

The rest.
```
