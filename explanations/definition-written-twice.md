# Definition written twice

A page has two footnote notes with the same label, such as two lines that
start with `[^1]:`. Labels match whatever their case, so `[^note]:` and
`[^Note]:` also count as the same label. Every marker with that label
points to the later note, so the first note is never shown.

## What you'll see {#what-youll-see}

```text
warning  Definition written twice: the first `[^n]:` line isn't shown
  Give one of them another name, or take one out.
  More: https://statac.dev/e/definition-written-twice/
```

## Why Statac warns

Statac shows the later note and publishes the page. It warns you because
readers never see the first note, which you wrote, and it is often the
note you meant. This happens when a note is copied from another page, or
when two notes are given the same label by mistake.

## How to fix it

Give one of the notes another label, and change the marker where that
note belongs to use the new label. Or delete one of the notes. The message
points to the first note, and to the later one that is shown instead.

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
