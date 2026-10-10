# Missing definition

The writing points at a footnote, `[^label]`, or names a link by a label,
`[text][label]` or `[label][]`, and no line on the page defines that label.
Statac publishes what you wrote as words, brackets and all.

## What you'll see {#what-youll-see}

```text
warning  Missing definition: there is no `[^2]:` line
  Did you mean `[^1]`?
  More: https://statac.dev/e/missing-definition/
```

## Why Statac warns

Markdown shows a reference with no definition as the words you typed, so
the page is still readable, but the note or the link you meant isn't on it.
A footnote's note and a link's address can be written anywhere on the
page, even inside a snippet call that wraps writing, so a label spelled
two ways is the usual cause. Brackets that aren't a reference, such as
`[draft]` alone or `x[i][j]`, are left alone.

## How to fix it

Add the line the reference needs, or correct the label to one the page
defines. When a defined label is close to the one you wrote, Statac offers
it. Labels match whatever their case. To show the brackets as words, write
a backslash before the first: `\[^label]`.

## Example

In `content/post-1.md`:

```markdown
The figures are from 2026.[^source]

[^sources]: The yearly report.
```

Write the marker and its note with the same label:

```markdown
The figures are from 2026.[^sources]

[^sources]: The yearly report.
```
