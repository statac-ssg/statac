# Definition written twice

A page has two footnotes' notes with one label, such as two lines opening
`[^1]:`. Labels match whatever their case, so `[^note]:` and `[^Note]:`
are one label too. Every marker with that label points at the later note,
so the first is shown nowhere.

## What you'll see {#what-youll-see}

```text
warning  Definition written twice: the first `[^n]:` line isn't shown
  Give one of them another name, or take one out.
  More: https://statac.dev/e/definition-written-twice/
```

## Why Statac warns

Statac shows the later note and publishes the page. It
tells you, because the first note is writing you wrote that readers never
see, and it is often the one you meant: a note copied from another page,
or a label given to two notes by mistake.

## How to fix it

Give one of the notes another label, and point a marker at it where it
belongs, or take one of them out. The message points at the first note,
and at the later one that is shown instead.

## Example

In `content/post-1.md`:

```markdown
The figures are from 2026.[^1] The photograph is from 2025.[^1]

[^1]: The yearly report.

[^1]: Photograph by the author.
```

Give each note its own label:

```markdown
The figures are from 2026.[^1] The photograph is from 2025.[^2]

[^1]: The yearly report.

[^2]: Photograph by the author.
```
