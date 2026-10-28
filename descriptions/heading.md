# A heading in a template

Summary: Every value a template can read on a heading in a page's table of contents.

What a template reads on each heading in `page.toc`, and in each
heading's `headings`.

## `title`

The heading's words, as plain text.

## `id`

The id of the heading's element.

## `url`

A link to the heading on its page: `#` followed by its id.

## `level`

The heading's level, from 1 for a `#` heading to 6.

## `headings`

The headings under this one, each with the same values as a heading in
`page.toc`. Empty if there are none.
