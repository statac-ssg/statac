# Wrong place

A setting is written somewhere it can't take effect. A `pages.` line in a
folder's `index.md` cascades a field to the pages in that folder. So it
can't cascade a field that each page has of its own, such as `title` or
`date`, and it can't be written on a page that has no pages inside it.
`pages.order` can cascade `newest first` or `oldest first`, but not a
list of names, because a list of names orders the pages of one folder.
`pages.index_page` can't cascade from `statac.yaml`, because there
it would apply to every list on the site, including the home page. Site
settings, such as `tags.permalink`, and `site.` values belong in
`statac.yaml`. An item in a table in `data/` can't make pages of its own
with `each:`.

The same applies to `<!--more-->` in a page's content. It ends the excerpt
only when it is on a line of its own between two paragraphs, or is the
last line of a block of HTML. It doesn't work inside a list, a quotation,
a footnote, a call, a block of HTML or an element your HTML opens before
it and closes after it, or on a line with other text.

A setting that describes one of the site's hooks needs that hook to be
set. `after_build_outputs` says what `after_build` makes, and
`before_build_watches` lists the files that make `before_build` run again.
On a site that doesn't set the hook, either setting on its own has
nothing to describe.

A file in `languages/` can't have a `dates:` line. Statac takes how each
language writes its dates from the Unicode CLDR, and a site's file can't
change that. To choose how the site writes dates, write `date_format` in
`statac.yaml`.

## What you'll see {#what-youll-see}

```text
error  Wrong place: `pages.title` can't cascade
  content/notes/index.md, line 3
   3 │ pages.title: Note
     │ ^^^^^^^^^^^
  Write `title:` on each page instead.
  More: https://statac.dev/e/wrong-place/
```

## Why Statac stops

The line would do nothing where it is, and Statac can't tell what you
wanted it to do instead.

## How to fix it

Move the line to where the message says. A `pages.` line for the whole site
goes in `statac.yaml`, not in the `index.md` at the top of `content/`.
Put `<!--more-->` on a line of its own where the excerpt should end.

## Example

In `content/notes/index.md`:

```yaml
---
title: Notes
pages.title: Note
---
```

Each page has its own `title`. Cascade something the pages share
instead:

```yaml
---
title: Notes
pages.layout: note
---
```

In `content/post-1.md`:

```markdown
> The part a list shows.
>
> <!--more-->
```

The marker is inside the quotation. Move it out, below the quotation:

```markdown
> The part a list shows.

<!--more-->
```
