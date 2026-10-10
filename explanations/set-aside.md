# Set aside

A page writes a field Statac reads nothing from. Either Statac fills that
name on every page itself, such as `source` or `content`, or the field is
a `permalink` on a folder's own `index.md`, whose page always stays at the
folder's address, or the page sends readers on with `redirect_to` and the
field, such as `tags` or `feed`, has nothing to act on. A page sending
readers on reads its `title`, `date`, `permalink`, `draft`, `needs` and
`redirect_from`, and on a folder's own page its `order` and `numbered`.

A card's settings in `templates/` read `background`, `font` and
`padding`, and for each text `text`, `place`, `size` and `colour`, as
`title.size`. Any other name in them is set aside.

A page's `card` is set aside where the page names a picture of its own
with `og_image`, which comes first.

One of Statac's words, one of its settings, or a field `statac.yaml`
declares is set aside where nothing at all is written after its colon, as
`title:` is. Statac reads it as though the line weren't there. `~` and
`null` are written on purpose and never told, and nor is an empty field
of your own that isn't declared, which a template may test for.

## What you'll see {#what-youll-see}

```text
warning  Field set aside: Statac fills `page.source` itself
  Give it a name of your own, such as `my_source`.
  More: https://statac.dev/e/set-aside/
```

## Why Statac warns

Statac is sure what it does here: it leaves the line alone and uses its
own value, so the build goes on. It tells you, because what you wrote
doesn't reach the page. A page sending readers on is in no list or feed
and is built with no layout, so its tags, its feed and the like do
nothing.

## How to fix it

Give the field a name of your own if you want it in your templates. To
choose where a page is published, write `permalink:`. To move a folder's
page, rename the folder. On a page sending readers on, take the field out,
or take out `redirect_to` to publish the page. In the card's settings,
correct the name to one a card reads; Statac offers the nearest. On a
page with both `card` and `og_image`, take out one.
Where nothing is written after the colon, give the line a value, or take
it out.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
source: notes/post-1.txt
---
```

`page.source` is the file the page was read from. Name yours another way:

```yaml
---
title: Post 1
my_source: notes/post-1.txt
---
```
