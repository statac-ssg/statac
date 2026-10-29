# Wrong argument

A snippet call hands the snippet something it can't take, or leaves out
something it reads. The message names the snippet and the value.

A snippet takes each name it reads, except `page` and `site`, which every
snippet has. A call hands each value by name, as in `card(post=post)`. If
a snippet reads only one name, a call can hand it that value without a
name, as in `card(post)`. If a snippet's first line is
`{# takes photo, alt #}`, a call can hand it those two values without
names, in that order, and anything else by name.

## What you'll see {#what-youll-see}

```text
error  Wrong argument: `card` takes no `pots`
  templates/folder.html, line 2
   2 │ {{ card(pots=post) }}
     │              ^^^^
  Did you mean `post`?
  More: https://statac.dev/e/wrong-argument/
```

## Why Statac stops

A value the snippet doesn't read would be dropped without a word. A value
it reads but wasn't given would come out as nothing.

## How to fix it

Give each value the name the snippet reads it by, and hand the snippet
every value it reads. If a name is close to one the snippet reads, the
message suggests it. To let calls give values without names, add a
`takes` line to the top of the snippet.

## Example

In `templates/folder.html`, a call to `snippets/card.html`, which reads
`post`:

```html
{{ card(pots=post) }}
```

Name the value as the snippet reads it:

```html
{{ card(post=post) }}
```
