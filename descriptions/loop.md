# A loop in a template

What a template reads on `loop` inside a `for` loop.

## `index`

How far the loop has got, counting from 1.

## `index0`

How far the loop has got, counting from 0.

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

The item before this one. The first item has none.

## `nextitem`

The item after this one. The last item has none.

## `changed`

Called as `loop.changed(post.date.year)`, `true` where what it is given
differs from what it was given on the item before, and on the first
item.

## `cycle`

Called as `loop.cycle("odd", "even")`, each of what it is given in turn,
one for each item.
