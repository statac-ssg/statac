# Nested too deep

Your writing is nested deeper than Statac reads. Each quotation, each list,
each run of emphasis, strong or struck-through text, each link's words,
each footnote and each snippet call that wraps writing counts as one level
when it sits inside another. A list and its items are one level together.
The message is placed at the opening of the first level past 200, the most
Statac reads.

## What you'll see {#what-youll-see}

```text
error  Nested too deep: the writing here is nested more than 200 deep
  More: https://statac.dev/e/nested-too-deep/
```

## Why Statac stops

Reading a page means following every level of it down and back, and a
page nested thousands deep can use up the memory a computer sets aside for
that and end the program. Statac stops at the first level past the limit
instead, and leaves the page out so that nothing is published from writing
it only half read.

Raw HTML isn't counted: `<div>` inside `<div>` is as deep as you like.

## How to fix it

Write the part flat. Writing nested this deep is almost always made by a
mistake, such as a line of `>` marks that was meant to be one quotation, or
a program that wrote the Markdown. Where it is meant, split it into
separate quotations or lists.

## Example

In `content/post-1.md`, a line that starts with 201 `>` marks is a
quotation inside 200 others. Statac reads the first 200 and stops at the
201st `>`. Two marks are two levels:

```markdown
> > The words.
```
