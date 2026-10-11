# Missing definition

Your writing refers to a footnote, `[^label]`, or to a link by its label,
`[text][label]` or `[label][]`, but no line on the page defines that
label. Statac publishes what you wrote as plain text, brackets and all.

## What you'll see {#what-youll-see}

```text
warning  Missing definition: there is no `[^2]:` line
  Did you mean `[^1]`?
  More: https://statac.dev/e/missing-definition/
```

## Why Statac warns

Markdown shows a reference with no definition as the text you typed. The
page is still readable, but the note or link you meant is missing. A
footnote's note and a link's address can be written anywhere on the page,
even inside a snippet call that wraps writing. So the usual cause is a
label spelled differently in the two places. Statac ignores brackets that
aren't a reference, such as `[draft]` on its own or `x[i][j]`.

## How to fix it

Add the line the reference needs, or change the label to one the page
defines. If a defined label is close to the one you wrote, Statac suggests
it. Labels match regardless of case. To show the brackets as text, put a
backslash before the first one: `\[^label]`.

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
