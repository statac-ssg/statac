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

Statac publishes the HTML you wrote unchanged. A browser reads your HTML
as part of the whole page, not on its own, so a tag that doesn't match
reaches past your content. An element left open takes in what the layout
puts after the content: on a list of posts, the posts after this one,
the pagination links and the footer. A comment left open hides all of
it. If the layout has nothing of that name open round the content, a
browser ignores a closing tag with nothing to close. If the layout has,
the tag closes part of the layout, and what follows falls out of place.
Which of these happens depends on the layout, which can change, so
Statac warns of every one.

Statac doesn't add or remove a tag for you, because it can't know where
you meant the element to end.

Statac doesn't warn about HTML that browsers close without harm:

- Elements whose closing tag may be left out, such as `<p>`, `<li>`,
  `<tr>` and `<td>`, and elements that hold nothing, such as `<br>` and
  `<img>`.
- An element left open inside one that is closed, such as a `<div>`
  inside a `<figure>` that `</figure>` closes, or inside a paragraph, a
  list item, a quotation, a heading or a table's cell, which ends where
  Markdown ends it. A browser closes both. The exceptions are `<a>`, `<b>`,
  `<i>`, `<em>`, `<strong>`, `<code>` and the like, which a browser opens
  again round what follows, until their own closing tag.
- HTML in a code span or a code block, which Statac shows as text.

Statac matches the tags in each piece of Markdown on its own: a page's
content, each footnote definition, the content each snippet call wraps,
and each value shown through `markdown`. It doesn't count HTML that a
snippet writes.

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
