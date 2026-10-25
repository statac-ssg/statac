# Names a template is handed

The names a template, a snippet or a file in `snippets/markdown/` reads
at its top, before any dot. Each template is handed `page` and `site`;
the rest are handed only where each says.

## `page`

The page being built. In a snippet too, it is the page being built, not
a page the call hands the snippet.

More: https://statac.dev/templates/#handed

## `site`

The site, the same on every page. On a site in several languages, as the
page's language has it.

More: https://statac.dev/templates/#handed

## `loop`

Inside a `for` loop, where the loop has got to: `loop.index`,
`loop.first`, `loop.last` and the rest. In a loop written with
`recursive`, `loop(...)` goes a level deeper.

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

In `redirect.html`, the URL the page redirects to, in full where the
site has a `url`.

More: https://statac.dev/addresses/#redirect-page

## `title`

In `redirect.html`, the redirecting page's `title`, or else the name of
the page it redirects to. In `link.html` and `image.html`, the title
written after the URL, and in `callout.html`, the callout's title: empty
where none is written.

More: https://statac.dev/addresses/#redirect-page

## `standalone`

In a snippet called from the content, `true` where the call is on a line
of its own, and `false` where it is inside a sentence.

More: https://statac.dev/snippets/#handed

## `body`

In a snippet, the content a call wraps, built into HTML. In a file in
`snippets/markdown/`, what the piece holds, built into HTML: a heading's
or a link's words, a code block's code, or a callout's content.

More: https://statac.dev/snippets/#handed

## `lines`

In a snippet, the lines of the content a call wraps. Only a call in a
template can hand it yet.

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
end of the heading or the table, separated by spaces. Empty where none
are written.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `url`

In `link.html` and `image.html`, the URL as written in the content.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `external`

In `link.html`, `true` where the link leads to another site: its host is
neither the one in the site's `url` nor one in `other_hosts`.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `alt`

In `image.html`, the alt text written in the brackets, or else the one
in the image's own `.yaml` file.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `width`

In `image.html`, the image's width in pixels, where its size is known.
Empty where it isn't.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `height`

In `image.html`, the image's height in pixels, where its size is known.
Empty where it isn't.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `image`

In `image.html`, the image as `page.image` gives it, where the URL names
a file in the site. Empty where it doesn't.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `code`

In `code.html` and `inline_code.html`, the code as written.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `language`

In `code.html`, the first word after the opening fence, or the language
a file's name there stands for. Empty where the fence names none.

More: https://statac.dev/snippets/#pieces-of-the-writing

## `caption`

In `code.html`, the rest of the opening fence's line after the language,
or the whole line where it opens with a file's name. Empty where there
is nothing more.

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
`back_to_the_text` line.

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

In `table.html`, every other row, each a list of cells with their `body`
and their column's `align`.

More: https://statac.dev/snippets/#pieces-of-the-writing
