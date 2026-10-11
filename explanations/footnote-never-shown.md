# Footnote never shown

The page has a footnote's note, `[^label]: The note.`, but no marker in
the writing points at it. A note appears at the foot of the page only if
a marker points at it, so this one doesn't appear anywhere.

## What you'll see {#what-youll-see}

```text
warning  Footnote never shown: nothing points at `[^sources]`
  Is it the note for `[^source]`?
  More: https://statac.dev/e/footnote-never-shown/
```

## Why Statac warns

Readers never see the note you wrote. Usually a marker's label is
spelled differently, or the marker was removed and the note was left
behind. Statac publishes the page without the note and tells
you. A marker can point at a note written anywhere on the page, even inside
a snippet call that wraps writing.

## How to fix it

Add a marker with the same label, `[^label]`, where the note belongs, or
remove the note. If a marker with no note has a label close to the
note's, the message asks whether the note belongs to that marker. Case
doesn't matter when labels are matched.

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
