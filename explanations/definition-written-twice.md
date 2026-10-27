# Definition written twice

A page has two footnote definitions with the same label, such as two
lines that start with `[^1]:`. Statac matches labels whatever their case,
so `[^note]:` and `[^Note]:` also count as the same label. Every footnote
reference with that label points to the later definition, so the first
definition is never shown.

## What you'll see {#what-youll-see}

```text
warning  Definition written twice: Statac doesn't show the first `[^n]:` line
  Give one of them another label, or take one out.
  More: https://statac.dev/e/definition-written-twice/
```

## Why Statac warns

Statac shows the later definition and publishes the page. It warns you
because readers never see the first definition, which you wrote, and it
is often the one you meant. This happens when a footnote is copied from
another page, or when two footnotes are given the same label by mistake.

## How to fix it

Give one of the definitions another label, and change the reference
where that footnote belongs to use the new label. Or delete one of the
definitions. The message points to the first definition, and to the
later one that is shown instead.

## Example

In `content/post-1.md`:

```markdown
The figures are from 2026.[^1] The photograph is from 2025.[^1]

[^1]: The yearly report.

[^1]: Photograph by the author.
```

Give each footnote its own label:

```markdown
The figures are from 2026.[^1] The photograph is from 2025.[^2]

[^1]: The yearly report.

[^2]: Photograph by the author.
```
