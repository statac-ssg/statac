# Footnote never shown

The page has a footnote definition, `[^label]: The note.`, but no
footnote reference in the content points at it. A definition appears at
the foot of the page only if a reference points at it, so this one
doesn't appear anywhere.

## What you'll see {#what-youll-see}

```text
warning  Footnote never shown: nothing points at `[^sources]`
  content/post-1.md, line 3
   3 │ [^sources]: The yearly report.
     │ ^^^^^^^^^^
  Is it the definition for `[^source]`?
  More: https://statac.dev/e/footnote-never-shown/
```

## Why Statac warns

Readers never see the footnote you wrote. Usually a reference's label is
spelled differently, or the reference was removed and the definition was
left behind. Statac publishes the page without the footnote and tells
you. A reference can point at a definition written anywhere on the page,
even inside a snippet call that wraps content.

## How to fix it

Add a reference with the same label, `[^label]`, where the footnote
belongs, or remove the definition. If a reference with no definition has
a label close to the definition's, the message asks whether the
definition belongs to that reference. Case
doesn't matter when labels are matched.

## Example

In `content/post-1.md`:

```markdown
The figures are from 2026.[^source]

[^sources]: The yearly report.
```

Give the reference and its definition the same label:

```markdown
The figures are from 2026.[^sources]

[^sources]: The yearly report.
```
