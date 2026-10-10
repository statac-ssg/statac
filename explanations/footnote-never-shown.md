# Footnote never shown

The page has a footnote's note, `[^label]: The note.`, and no marker in the
writing points at it. Notes are shown at the foot of the page only for the
markers that point at them, so this one is on no page.

## What you'll see {#what-youll-see}

```text
warning  Footnote never shown: nothing points at `[^sources]`
  Is it the note for `[^source]`?
  More: https://statac.dev/e/footnote-never-shown/
```

## Why Statac warns

The note is writing you wrote that readers never see. It usually means a
marker's label is spelled another way, or the marker was taken out and the
note left behind. Statac publishes the page without the note and tells
you. A marker can point at a note written anywhere on the page, even inside
a snippet call that wraps writing.

## How to fix it

Point at the note with a marker of the same label, `[^label]`, where it
belongs, or take the note out. When a marker with no note has a label close
to the note's, Statac asks whether the note is that marker's. Labels match
whatever their case.

## Example

In `content/post-1.md`:

```markdown
The figures are from 2026.[^source]

[^sources]: The yearly report.
```

Give the marker and its note the same label:

```markdown
The figures are from 2026.[^sources]

[^sources]: The yearly report.
```
