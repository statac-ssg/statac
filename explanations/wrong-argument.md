# Wrong argument

A snippet call hands the snippet something it can't take, or leaves out
something it reads. The message names the snippet and the value.

A snippet takes the names it reads, other than `page` and `site`, which
every snippet has. A call hands each value by name, `card(post=post)`. A
snippet that reads one name may be handed it without one, `card(post)`. A
snippet whose first line is `{# takes photo, alt #}` may be handed those two
without names, in that order, and anything else by name.

## What you'll see {#what-youll-see}

```text
error  Wrong argument: `card` takes no `pots`
  Did you mean `post`?
  More: https://statac.dev/e/wrong-argument/
```

## Why Statac stops

A value the snippet doesn't read would be dropped without a word, and a
value it reads and wasn't given would come out as nothing.

## How to fix it

Name each value as the snippet reads it, and give it every value it reads.
When a name is close to one the snippet reads, Statac offers it. To have
values given without names, add a `takes` line to the top of the snippet.

## Example

In `templates/folder.html`, calling `snippets/card.html`, which reads
`post`:

```html
{{ card(pots=post) }}
```

Name the value as the snippet reads it:

```html
{{ card(post=post) }}
```
