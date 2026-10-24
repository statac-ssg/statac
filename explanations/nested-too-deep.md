# Nested too deep

Your Markdown is nested deeper than Statac will read. Each of these
counts as one level when it is inside another: a quotation, a list, a
run of emphasis, strong or struck-through text, a link's text, a
footnote, and a snippet call that wraps content. A list and its items
count as one level together. The message points to the start of the
first level past 200, which is the most Statac reads.

## What you'll see {#what-youll-see}

```text
error  Nested too deep: the Markdown here is nested more than 200 deep
  More: https://statac.dev/e/nested-too-deep/
```

## Why Statac stops

To read a page, Statac follows every level of it down and back up
again. A page nested thousands of levels deep can use up the memory the
computer sets aside for this, and crash the program. Statac stops at the
first level past the limit instead, and leaves the page out, so that
nothing is published from Markdown it has only half read.

Raw HTML isn't counted: you can nest `<div>` inside `<div>` as deep as you
like.

## How to fix it

Flatten that part of the Markdown. Markdown nested this deep is almost
always caused by a mistake, such as a line of `>` marks that was meant to
be one quotation, or by a program that wrote the Markdown. If the nesting
is intended, split it into separate quotations or lists.

## Example

In `content/post-1.md`, a line that starts with 201 `>` marks is a
quotation inside 200 others. Statac reads the first 200 and stops at the
201st `>`. Each mark is one level, so this line has two:

```markdown
> > The words.
```
