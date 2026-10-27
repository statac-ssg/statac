# Filters

Each filter a template can write after `|`, such as
`{{ page.date | moment }}`: Statac's own, and the template engine's.

## `absolute`

Turns a URL starting with `/` into a full URL, by putting the start of
the site's `url` in front of it. If it is given HTML, it does this to
each URL in an `href`, `src`, `srcset` or `poster`. Text joined to HTML
with `~` is text, so it changes no URL in it. It needs the site's
`url`.

More: https://statac.dev/feeds/#the-template

## `date`

Writes a date the way an example date shows it, the same way
`date_format` does: `date("31 December 2025")`. It also takes one of the
language's looks: `long`, `short`, `day` or `stamp`. With nothing in the
brackets, it writes the date in the site's date format.

More: https://statac.dev/dates/#how-dates-look

## `moment`

Writes a date in the form machines read, such as
`2026-10-04T15:04:00+01:00`, for `<time datetime>`. `moment("rss")`
writes it in the form RSS uses.

More: https://statac.dev/feeds/#the-template

## `markdown`

Builds text into HTML as Markdown, such as a `summary` with emphasis in
it.

More: https://statac.dev/writing/#markdown

## `highlight`

Marks text as code, ready to go inside a `<code>` element, with a class
on each piece for what kind of piece it is. It takes the language the
way a code block names it: `highlight("rust")`. If `code_colours` isn't
`true`, and in a feed, it adds no classes.

More: https://statac.dev/code-colours/

## `plain`

Takes the markup out of some HTML, leaving its words on one line.

## `truncate`

Cuts text, or the words of some HTML, at a word so that it fits in the
number of characters given, and adds the ending given if it cuts
anything: `truncate(160, "…")`.

## `digits`

Writes a number, or the numbers in a text, in the digits of the page's
language: `۱۲` for 12 on a Persian page. Only `0` to `9` change. If it
is given HTML, it returns only the words, as `plain` does.

More: https://statac.dev/languages/#language-files

## `json`

Writes a value as JSON. It is safe inside a `<script>`, and inside an
attribute in single quotes, but not in double quotes, since JSON writes
text between double quotes.

## `urlencode`

Makes text safe to use as part of a URL.

## `tagged`

The pages of a list that have the tag given: `site.posts |
tagged("Rust")`.

More: https://statac.dev/lists/#lists-of-your-own

## `in`

The pages of a list that are in the folder given: `site.posts |
in("blog")`.

More: https://statac.dev/lists/#lists-of-your-own

## `where`

The pages of a list whose field has the value given: `site.posts |
where("type", "book")`.

More: https://statac.dev/lists/#lists-of-your-own

## `newest_first`

A list of pages in date order, the newest first.

More: https://statac.dev/lists/#lists-of-your-own

## `newest_date`

The newest date among a list of pages. If no page in the list has a
date, it has no value.

More: https://statac.dev/feeds/#the-template

## `sort`

Sorts a list, smallest first, or by a field: `sort(attribute="title")`.
`reverse=true` sorts it the other way. A page that doesn't have the
field goes after the rest.

More: https://statac.dev/templates/#no-value

## `abs`

A number without its sign: `-3` becomes `3`.

## `attr`

The field of a value that the text given names: `post | attr("title")`
is `post.title`.

## `batch`

Splits a list into lists of the number given. The last one is shorter,
unless a second value is given to fill it up.

## `bool`

A value as `true` or `false`, the way an `{% if %}` would treat it.

## `capitalize`

Text with its first letter a capital and the rest small letters.

## `chain`

Two or more lists one after another, as one list: `site.posts |
chain(other)`.

## `count`

How many items a list has, or how many characters a text has. The same
as `length`.

## `d`

A shorter name for `default`.

## `default`

The value, or if it has no value, the one given: `page.summary |
default("")`. With `true` after it, it also replaces an empty value.

More: https://statac.dev/templates/#no-value

## `dictsort`

The names and values of a table, as pairs sorted by name, or with
`by="value"`, by value.

## `e`

A shorter name for `escape`.

## `escape`

Escapes `<`, `>`, `&` and quotes in text, so a browser shows them
instead of reading them as HTML.

## `first`

The first item of a list, or the first character of a text. If the list
is empty, it has no value.

## `float`

A value as a number that may have a fraction.

## `format`

Puts values into text in place of each `%s`: `"%s of %s" | format(1,
3)`.

## `groupby`

Groups a list by the field named. Each group has `grouper`, the field's
value, and `list`, the items with that value. A page that doesn't have
the field stops the build.

More: https://statac.dev/templates/#no-value

## `indent`

Indents every line of text but the first by the number of spaces given.
`first=true` indents the first too.

## `int`

A value as a whole number.

## `items`

The names and values of a table, as pairs.

## `join`

The items of a list as one text, with the text given between each:
`page.tags | join(", ")`.

## `last`

The last item of a list, or the last character of a text. If the list
is empty, it has no value.

## `length`

How many items a list has, or how many characters a text has.

## `lines`

Text as a list of its lines.

## `list`

A value as a list: a text becomes a list of its characters, and a table
a list of its names.

## `lower`

Text in small letters.

## `map`

The field of each item, such as `map(attribute="title")`, or each item
passed through the filter named, such as `map("upper")`. A page that
doesn't have the field stops the build.

More: https://statac.dev/templates/#no-value

## `max`

The largest item of a list, or with `attribute="rank"`, the item whose
field is largest.

## `min`

The smallest item of a list, or with `attribute="rank"`, the item whose
field is smallest.

## `pprint`

Writes out a value and everything in it, to help find a fault in a
template.

## `reject`

The items of a list that don't pass the test named: `reject("none")`.

## `rejectattr`

The items of a list whose field doesn't pass the test named, such as
`rejectattr("draft")`. A page that doesn't have the field is kept.

More: https://statac.dev/templates/#no-value

## `replace`

Replaces each occurrence of the first text given with the second.

## `reverse`

A list or a text the other way round.

## `round`

A number rounded to the number of places given, or to a whole number.

## `safe`

Marks text as HTML, so that the template writes it into the page without
escaping it.

## `select`

The items of a list that pass the test named: `select("odd")`. With no
test, those that are true.

## `selectattr`

The items of a list whose field passes the test named, such as
`selectattr("group", "eq", "a")`. A page that doesn't have the field is
left out.

More: https://statac.dev/templates/#no-value

## `slice`

Splits a list into the number of lists given, such as for columns. The
last one is shorter, unless a second value is given to fill it up.

## `split`

Splits text into a list at each occurrence of the text given, or at
spaces if none is given.

## `string`

A value as text.

## `sum`

The numbers of a list added up.

## `title`

Text with each word starting with a capital.

## `trim`

Text without the spaces at its start and end, or without the characters
given.

## `unique`

A list with each item only once, in the order the items first appear.
With `attribute="rank"`, it keeps one item for each value of the field.
A page that doesn't have the field stops the build.

More: https://statac.dev/templates/#no-value

## `upper`

Text in capitals.

## `zip`

Takes two or more lists item by item, and makes a list of the items that
stand at the same place in each. It is as long as the shortest list.
