# A loop in a template

Summary: Every value a template can read inside a for loop, such as the item's number and whether it is the last.

The names (variables) a template reads on `loop` inside a `for` loop.

## `index`

The number of the current item, counting from 1.

## `index0`

The number of the current item, counting from 0.

## `revindex`

How many items are left, this one included, counting down to 1.

## `revindex0`

How many items are left after this one, counting down to 0.

## `first`

`true` on the loop's first item.

## `last`

`true` on the loop's last item.

## `length`

How many items the loop goes through.

## `depth`

In a loop written with `recursive`, how deep this level is, counting
from 1.

## `depth0`

In a loop written with `recursive`, how deep this level is, counting
from 0.

## `previtem`

The item before this one. On the first item, it has no value.

## `nextitem`

The item after this one. On the last item, it has no value.

## `changed`

`loop.changed(post.date.year)` gives `true` if the value it is given
differs from the one it was given on the item before. It is always
`true` on the first item.

## `cycle`

`loop.cycle("odd", "even")` gives the values it is given in turn, the
next one on each item: `odd` on the first item, `even` on the
second, and `odd` again on the third.
