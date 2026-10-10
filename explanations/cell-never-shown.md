# Cell never shown

A row of a table has more cells than the table has columns. A table has
as many columns as its first two lines have: the line of headings and the
line of dashes under it. A cell past the last column is dropped, so the
row is shown without it.

## What you'll see {#what-youll-see}

```text
warning  Cell never shown: `Extra` is past the table's 2 columns
  Add a column to the table's first two lines, or take the cell out.
  More: https://statac.dev/e/cell-never-shown/
```

## Why Statac warns

Statac shows each row with the table's columns, as Markdown does, and
publishes the page. It tells you, because the dropped cell is writing you
wrote that readers never see. It is often a column added to some rows and
not to the first two lines, or a `|` meant as part of a cell's words.

## How to fix it

Add a column to the table's first two lines, a heading and its dashes, or
take the cell out. To write `|` inside a cell, write it `\|`, inside a
code span too. A row with empty cells past the last column loses nothing,
and Statac says nothing of it.

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
