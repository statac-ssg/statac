# Names a template is handed (variables)

Summary: The names a template or snippet can use on their own, and which templates are handed each one.

The names (variables) that a template, a snippet or a file in
`snippets/markdown/` reads on their own, before any dot. Statac hands
every template `page` and `site`, and each of the others only where its
entry says.

## `page`

The page being built. In a snippet too, it is the page being built, not
a page the call hands the snippet. In a feed's template, it is the list
page the feed links to, and in `redirect.html`, the page whose
`redirect_to` or
`redirect_from` makes the redirect.

More: https://statac.dev/templates/#handed

## `site`

The site, the same on every page. On a site in several languages, its
values are the ones for the page's language.

More: https://statac.dev/templates/#handed

## `loop`

Inside a `for` loop, how far the loop has got: `loop.index`,
`loop.first`, `loop.last` and the rest. In a loop written with
`recursive`, `loop(...)` goes a level deeper.

## `self`

The blocks of the template being built, by name: `self.title()` writes
the `title` block again, elsewhere in the template.

More: https://statac.dev/templates/#extends

## `super`

Inside a block of a template that `extends` another, `super()` writes
the block from the template this one extends.

More: https://statac.dev/templates/#extends

## `caller`

Inside a `macro` called with a `call` block, `caller()` writes the
contents of the `call` block.

## `varargs`

Inside a `macro`, the values a call hands it beyond those the macro
names, as a list.

## `kwargs`

Inside a `macro`, the values a call hands it by names the macro doesn't
have, as a table of names and values.

## `list`

In a feed's template, the list the feed is published for.

More: https://statac.dev/feeds/#the-template

## `posts`

In a feed's template, the list's dated posts, newest first, whatever
order the list itself has.

More: https://statac.dev/feeds/#the-template

## `feed`

In a feed's template, the URL the feed is published at.

More: https://statac.dev/feeds/#the-template

## `to`

In `redirect.html`, the URL the page redirects to, written in full if
the site has a `url`.

More: https://statac.dev/urls/#redirect-page

## `title`

In `redirect.html`, the redirecting page's `title`, or if it has none,
the name of the page it redirects to. In `link.html` and `image.html`,
the title written after the URL. In `callout.html`, the callout's title.
In these three, it is empty if no title is written.

More: https://statac.dev/urls/#redirect-page

## `standalone`

In a snippet called from the content, `true` if the call is on a line of
its own, and `false` if it is inside a sentence.

More: https://statac.dev/snippets/#handed

## `body`

In a snippet, the content a call wraps, built into HTML. In a file in
`snippets/markdown/`, what is inside the heading, link, code block or
callout, built into HTML: a heading's or a link's text, a code block's
code, or a callout's content. It is HTML, so the template writes it into the page unescaped.

More: https://statac.dev/snippets/#handed

## `lines`

In a snippet, the lines of the content a call wraps. For now, only a
call in a template can hand a snippet `lines`.

More: https://statac.dev/snippets/#handed

## `level`

In `heading.html`, the heading's level, from 1 to 6.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `id`

In `heading.html`, the heading's id, and in `table.html`, the table's
id. In `footnote.html`, the id of the place in the content that refers
to the note.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `classes`

In `heading.html` and `table.html`, the classes written in braces at the
end of the heading or the table, separated by spaces. Empty if none are
written.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `url`

In `link.html` and `image.html`, the URL as written in the content.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `external`

In `link.html`, `true` if the link leads to another site, which means its
host is neither the one in the site's `url` nor one in `other_hosts`.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `alt`

In `image.html`, the alt text written in the brackets, or else the one
in the image's own `.yaml` file.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `width`

In `image.html`, the image's width in pixels. Empty if Statac doesn't
know the image's size.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `height`

In `image.html`, the image's height in pixels. Empty if Statac doesn't
know the image's size.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `image`

In `image.html`, if the URL names a file in the site, that image, with
the same values as an image from `page.image`. Empty if it doesn't.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `code`

In `code.html` and `inline_code.html`, the code as written.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `language`

In `code.html`, the first word after the opening fence, or if a file's
name is there, the language its name stands for. Empty if the fence
names neither, or if the line after the fence starts with a quote.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `caption`

In `code.html`, the rest of the opening fence's line after the language,
or the whole line if it starts with a file's name or a quote. A caption
in quotes, such as `rust "The example"`, is handed without them. Empty if
there is nothing more.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `number`

In `footnote.html`, the note's number, in the digits `0` to `9`.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `shown`

In `footnote.html`, the note's number in the digits of the page's
language, such as `۱` on a Persian page.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `note`

In `footnote.html`, the id of the note the marker refers to.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `notes`

In `footnotes.html`, every note, each with `number`, `shown`, `id`,
`back`, the id of the first place that refers to it, and `body`.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `label`

In `footnotes.html`, what a screen reader says for the link back to the
text: `back to the text` in English, or else the language's
`back_to_the_text` translation. Empty in another language whose file has no
such line.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `kind`

In `callout.html`, the kind of callout: `note`, `tip`, `important`,
`warning` or `caution`.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `head`

In `table.html`, the cells of the heading row, each with its `body` and
its column's `align`.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `rows`

In `table.html`, every row after the heading row, each a list of cells
with their `body` and their column's `align`.

More: https://statac.dev/snippets/#pieces-of-the-writing
