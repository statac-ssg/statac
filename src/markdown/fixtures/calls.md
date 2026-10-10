---
title: Calls
---

A call mid-sentence @image('a.jpg', alt="One *two*, three") and on.

@image('b.jpg', width=800)

An *emphasised @image('c.jpg') call* and a [linked @icon(arrow) call](/x/).

A call over two lines @card(post,
  wide=true) ends here.

Shown as text: \@image('d.jpg') and `@image('e.jpg')` in code.

    @image('indented-code.jpg')

```text
@image('fenced.jpg')
```

Arguments the parser would read: @say("*not emphasis*", "[not a link](x)", "<b>not html</b>").

An email name@example.org(x) and an address https://example.org/@name(x) are not calls.

> @note('quoted')
> Inside a quotation.

- An item @image('g.jpg') in a list.

| Column | Call |
|---|---|
| One | @image('h.jpg') |

# A heading with @icon(star) in it

A maths call $x + @y()$ stays maths.

An unclosed @image('i.jpg' never closes.
