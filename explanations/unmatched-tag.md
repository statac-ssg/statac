# Unmatched tag

HTML in a page's Markdown, or in a value a template shows through the
`markdown` filter, opens an element and never closes it, or closes an
element it never opened. The message points at the tag. It is one of
these:

- A start tag, such as `<div>`, with no end tag after it.
- A comment, `<!--`, with no `-->` after it.
- An end tag, such as `</div>`, with nothing of that name open before it.

## What you'll see {#what-youll-see}

```text
warning  Unmatched tag: `<div>` is never closed
  Close it with `</div>` where it should end.
  More: https://statac.dev/e/unmatched-tag/
```

## Why Statac warns

Statac publishes what you wrote as written. A browser reads your HTML as
part of the whole page, not on its own, so a tag that doesn't match
reaches past your content. An element left open takes in what the layout
puts after the content: on a list of posts, the posts after this one, the
links to more pages and the footer. A comment left open hides all of it.
A closing tag with nothing to close is ignored where the layout has
nothing of that name open round the content, and closes part of the
layout where it has, so what follows falls out of place. Which it does
turns on the layout, which can change, so every one is warned of.

Statac doesn't add or remove a tag for you, because it can't know where
you meant the element to end.

HTML that browsers close without harm isn't warned about:

- Elements whose closing tag may be left out, such as `<p>`, `<li>`,
  `<tr>` and `<td>`, and elements that hold nothing, such as `<br>` and
  `<img>`.
- An element left open inside one that is closed, such as a `<div>`
  inside a `<figure>` that `</figure>` closes, or inside a paragraph, a
  list item, a quotation, a heading or a table's cell, which ends where
  Markdown ends it. A browser closes both. The exceptions are `<a>`, `<b>`,
  `<i>`, `<em>`, `<strong>`, `<code>` and the like, which a browser opens
  again round what follows, until their own closing tag.
- HTML in a code span or a code block, which is shown as text.

Each piece of Markdown is matched on its own: a page's content, each
footnote definition, the content each snippet call wraps, and each value
shown through `markdown`. HTML that a snippet writes isn't counted.

## How to fix it

Write the closing tag where the element should end, or remove the tag you
don't need. A tag matches its closing tag whatever its letters' case, so
`</table>` closes `<TABLE>`. Close a comment with `-->`.

If a tag is left open on purpose, for a template to close, quieten the
warning with `quiet: [unmatched-tag]` in `statac.yaml`.

## Example

In `content/post-1.md`, the box is never closed, so it takes in the rest
of the page:

```markdown
<div class="box">

A note in a box.
```

Close it where the box ends:

```markdown
<div class="box">

A note in a box.

</div>
```

In `content/post-2.md`, the `</div>` closes nothing the post opened:

```markdown
The first paragraph.

</div>

The second paragraph.
```

Remove it, or open the element above it.
