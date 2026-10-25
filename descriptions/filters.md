# Filters

Each filter a template can write after `|`, as `{{ page.date | moment
}}`: Statac's own, and those of the template engine.

## `absolute`

A URL starting with `/` made into a full URL, with the start of the
site's `url` put in front. Given HTML, it does this to each URL in an
`href`, `src`, `srcset` or `poster`. It needs the site's `url`.

More: https://statac.dev/templates/#filters

## `date`

A date written as an example date, as `date_format` is, or in one of the
language's looks: `long`, `short`, `day` or `stamp`. Given nothing, the
date as the site writes dates.

More: https://statac.dev/dates-and-languages/#how-dates-look

## `moment`

A date in the form machines read, `2026-10-04T15:04:00+01:00`, for
`<time datetime>`. `moment("rss")` gives the form RSS uses.

More: https://statac.dev/templates/#filters

## `markdown`

Text built as Markdown into HTML, such as a `summary` with emphasis in
it.

More: https://statac.dev/templates/#markdown

## `highlight`

Text as code to go inside a `<code>` element, each piece marked with a
class for its kind, given the language as a code block names it:
`highlight("rust")`. Where `code_colours` isn't `true`, and in a feed,
the text as written.

More: https://statac.dev/templates/#filters

## `plain`

The words of some HTML on one line, with the markup taken out.

More: https://statac.dev/templates/#filters

## `truncate`

Text, or the words of some HTML, cut at a word to fit the number of
characters given, with the ending given where it is cut: `truncate(160,
"…")`.

More: https://statac.dev/templates/#filters

## `digits`

A number, or text holding numbers, written in the digits of the page's
language: `۱۲` for 12 on a Persian page. Only `0` to `9` change.

More: https://statac.dev/templates/#filters

## `json`

A value written as JSON. It is safe inside a `<script>`, and inside an
attribute in single quotes.

More: https://statac.dev/templates/#filters

## `urlencode`

Text made safe to use as part of a URL.

More: https://statac.dev/templates/#filters

## `tagged`

The pages of a list that have the tag given: `site.posts |
tagged("Rust")`.

More: https://statac.dev/lists/#lists-of-your-own

## `in`

The pages of a list filed in the folder given: `site.posts |
in("blog")`.

More: https://statac.dev/lists/#lists-of-your-own

## `where`

The pages of a list whose field has the value given: `site.posts |
where("type", "book")`.

More: https://statac.dev/lists/#lists-of-your-own

## `newest_first`

A list of pages in date order, the newest first.

More: https://statac.dev/templates/#filters

## `newest_date`

The newest date among a list of pages. A list with no dated page gives
none.

More: https://statac.dev/templates/#filters

## `sort`

A list in order, smallest first, or by a field:
`sort(attribute="title")`, with `reverse=true` for the other way. A page
that lacks the field goes after the rest.

More: https://statac.dev/templates/#no-value

## `abs`

A number without its sign: `-3` gives `3`.

## `attr`

The field of a value named by the text given: `post | attr("title")` is
`post.title`.

## `batch`

A list in lists of the number given, the last one shorter, or filled up
with the second value given.

## `bool`

A value as `true` or `false`, as an `{% if %}` would take it.

## `capitalize`

Text with its first letter a capital and the rest small letters.

## `chain`

Two or more lists one after another, as one list: `site.posts |
chain(other)`.

## `count`

How many items a list holds, or how many characters a text holds. The
same as `length`.

## `d`

A shorter name for `default`.

## `default`

The value, or where it has no value, the one given: `page.summary |
default("")`. With `true` after it, an empty value is replaced too.

More: https://statac.dev/templates/#no-value

## `dictsort`

The names and values of a table, as pairs in order of their names, or
with `by="value"`, of their values.

## `e`

A shorter name for `escape`.

## `escape`

Text with `<`, `>`, `&` and quotes written so a browser shows them
rather than reads them as HTML.

## `first`

The first item of a list, or the first character of a text. An empty
list gives none.

## `float`

A value as a number that may have a fraction.

## `format`

Text with values put into it where it has `%s`: `"%s of %s" | format(1,
3)`.

## `groupby`

A list in groups, by the field named: each group has `grouper`, the
field's value, and `list`, the items with it. A page that lacks the
field stops the build.

More: https://statac.dev/templates/#no-value

## `indent`

Text with every line but the first indented by the number of spaces
given. `first=true` indents the first too.

## `int`

A value as a whole number.

## `items`

The names and values of a table, as pairs.

## `join`

The items of a list as one text, with the text given between each:
`page.tags | join(", ")`.

## `last`

The last item of a list, or the last character of a text. An empty list
gives none.

## `length`

How many items a list holds, or how many characters a text holds.

## `lines`

Text as a list of its lines.

## `list`

A value as a list: a text gives its characters, and a table its names.

## `lower`

Text in small letters.

## `map`

Each item's field, as `map(attribute="title")`, or each item through the
filter named, as `map("upper")`. A page that lacks the field stops the
build.

More: https://statac.dev/templates/#no-value

## `max`

The largest item of a list, or with `attribute="rank"`, the item whose
field is largest.

## `min`

The smallest item of a list, or with `attribute="rank"`, the item whose
field is smallest.

## `pprint`

A value written out to show what it holds, for finding a fault in a
template.

## `reject`

The items of a list that don't pass the test named: `reject("none")`.

## `rejectattr`

The items of a list whose field doesn't pass the test named, as
`rejectattr("draft")`. A page that lacks the field is kept.

More: https://statac.dev/templates/#no-value

## `replace`

Text with each of the first text given replaced by the second.

## `reverse`

A list or a text the other way round.

## `round`

A number rounded to the number of places given, or to a whole number.

## `safe`

Text marked as HTML to write as it is, without escaping it.

## `select`

The items of a list that pass the test named: `select("odd")`. With no
test, those that are true.

## `selectattr`

The items of a list whose field passes the test named, as
`selectattr("group", "eq", "a")`. A page that lacks the field is left
out.

More: https://statac.dev/templates/#no-value

## `slice`

A list in the number of lists given, as columns, the last shorter, or
filled up with the second value given.

## `split`

Text as a list of the pieces between each of the text given, or between
spaces.

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

A list with each item once, in the order they first come, or with
`attribute="rank"`, one for each value of the field. A page that lacks
the field stops the build.

More: https://statac.dev/templates/#no-value

## `upper`

Text in capitals.

## `zip`

Two or more lists taken together, item by item, as a list of pairs, as
long as the shortest.
