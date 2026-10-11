# Published as text

A page's writing contains something that Statac treats as ordinary words,
though you may have meant something else, so the page shows it exactly as
written. It is one of these:

- A call in doubled braces with an angle bracket or a percent sign, or a
  tag in a brace and a percent sign.
- A line containing only a brace, a colon and whatever follows up to the
  closing brace. Another tool reads this as classes for the writing above
  it, or as a table of contents.
- Front matter with no `---` lines: the file starts with a line that
  begins with one of Statac's own words, such as `title:`, and every line
  up to the first blank one is a field or continues one.

```markdown
{{< figure src="photo-1.jpg" >}}
{{% note %}}
{% include note.html %}
{: .box}
{:toc}
```

Writing is never read as a template, so Statac treats none of these calls
and marks as anything but words. Front matter is read only between two
`---` lines at the very top of the file.

## What you'll see {#what-youll-see}

```text
warning  Published as text: `{{< figure src="photo-1.jpg" >}}` is another tool's call
  Call a snippet in its place, written `@name(...)`, or take it out.
  More: https://statac.dev/e/published-as-text/
```

## Why Statac warns

Statac publishes the page with what you wrote unchanged, as it does with
any words, and warns you, because a reader would see it where you meant
something else to appear. Writing kept as code (in a code span, a code
block, or HTML's own `<pre>` or comment) is left alone, and so is a brace
written after a backslash.

## How to fix it

Statac's way to place a snippet in the writing is `@name(...)`, with a
snippet of that name in `snippets/`. Call one instead of the other tool's
call, or remove the call. For a mark, remove it, or write the HTML it
stood for. To show the braces on purpose, put them in a code span. For
front matter, put a `---` line above the first field and another below the
last.

## Example

In `content/post-1.md`:

```markdown
{{< figure src="photo-1.jpg" >}}

A note in a box.
{: .box}
```

With a snippet named `figure` in `snippets/`, call it, and write the box
as HTML:

```markdown
@figure(src="photo-1.jpg")

<div class="box">A note in a box.</div>
```

In `content/post-2.md`, the fields are published as the first paragraph:

```markdown
title: Post 2
date: 2026-10-04

The first paragraph.
```

Put them between `---` lines:

```markdown
---
title: Post 2
date: 2026-10-04
---

The first paragraph.
```
