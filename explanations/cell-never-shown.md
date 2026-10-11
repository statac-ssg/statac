# Cell never shown

A row in a table has more cells than the table has columns. The table's
first two lines set the number of columns: the line of headings and the
line of dashes under it. Any cell past the last column is dropped, and the
row is shown without it.

## What you'll see {#what-youll-see}

```text
warning  Cell never shown: `Extra` is past the table's 2 columns
  Add a column to the table's first two lines, or take the cell out.
  More: https://statac.dev/e/cell-never-shown/
```

## Why Statac warns

Statac shows each row with only the table's columns, as Markdown does, and
publishes the page. It warns you because readers never see the dropped
cell, even though you wrote it. The cause is often a column added to some
rows but not to the first two lines, or a `|` meant as part of a cell's
text.

## How to fix it

Add a column to the table's first two lines (a heading and its dashes), or
remove the cell. To put a `|` inside a cell, write `\|`. This applies
inside a code span too. If the extra cells in a row are empty, nothing is
lost, and Statac doesn't warn about it.

## Example

In `content/post-1.md`:

```markdown
| Name   | Count |
|--------|-------|
| Post 1 | 1     | Extra |
```

Add the column:

```markdown
| Name   | Count | Note  |
|--------|-------|-------|
| Post 1 | 1     | Extra |
```
