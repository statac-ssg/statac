# Tests

Summary: Every test a template can use, and what each one checks.

Each test a template can write after `is`, such as
`{% if page.title is defined %}`, or name in `select` and `reject`. They
are the template engine's. `is not` reverses a test.

## `defined`

Whether the value is set, such as a field some pages don't have. Testing
it never stops the build, but reading a value that isn't set does.

More: https://statac.dev/templates/#no-value

## `undefined`

Whether the value isn't set: the opposite of `defined`.

More: https://statac.dev/templates/#no-value

## `none`

Whether the value is `none`.

## `true`

Whether the value is `true` itself, not only a value an `if` treats as
true, such as text with something in it.

## `false`

Whether the value is `false` itself, not only a value an `if` treats as
false, such as empty text or `0`.

## `boolean`

Whether the value is `true` or `false`.

## `number`

Whether the value is a number, whole or not: `"42"` is text, not a
number.

## `integer`

Whether the value is a whole number: `42`, not `42.0`.

## `int`

Another name for `integer`.

## `float`

Whether the value is a number with a decimal point: `42.0`, not `42`.

## `string`

Whether the value is text.

## `sequence`

Whether the value is a list.

## `mapping`

Whether the value is a table of names and values, such as
`{"name": "value"}`.

## `iterable`

Whether a `for` loop can go through the value.

## `odd`

Whether the number is odd: `loop.index is odd`.

## `even`

Whether the number is even.

## `divisibleby`

Whether the number divides by the one given with nothing left over:
`loop.index is divisibleby(3)`.

## `eq`

Whether the value equals the one given, as `==` does:
`select("eq", 2)`.

## `equalto`

Another name for `eq`.

## `ne`

Whether the value differs from the one given, as `!=` does.

## `lt`

Whether the value is less than the one given, as `<` does.

## `lessthan`

Another name for `lt`.

## `le`

Whether the value is less than or equal to the one given, as `<=` does.

## `gt`

Whether the value is greater than the one given, as `>` does.

## `greaterthan`

Another name for `gt`.

## `ge`

Whether the value is greater than or equal to the one given, as `>=`
does.

## `in`

Whether the value is one of the items of the list given, or part of the
text given: `page.language is in ["en", "fr"]`.

## `startingwith`

Whether the text starts with the text given: `page.url is
startingwith("/notes/")`.

## `endingwith`

Whether the text ends with the text given.

## `lower`

Whether the text has letters only in lower case.

## `upper`

Whether the text has letters only in upper case.

## `safe`

Whether the text is marked as HTML to write into the page unchanged.
`safe` and `markdown` mark text this way.

## `escaped`

Another name for `safe`.

## `sameas`

Whether the value is the very same value as the one given, not merely
equal to it: two lists written alike are equal but not the same.

## `filter`

Whether there is a filter with the name given: `"markdown" is filter`.

## `test`

Whether there is a test with the name given.
