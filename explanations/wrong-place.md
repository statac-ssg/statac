# Wrong place

A setting is written where it can't take effect. A `pages.` line in a
folder's `index.md` hands a field down to the pages inside, so it can't
hand down what each page has of its own, such as `title` or `date`, and it
can't be written on a page with nothing inside it. `pages.index_page`
can't be handed down from `statac.yaml`, where it would reach every list
of the site, the home page included. Site settings, such as
`tags.permalink`, and `site.` values belong in `statac.yaml`. An item of a
table in `data/` can't make pages of its own with `each:`.

The same goes for `<!--more-->` in the writing. It ends a page's excerpt
only on a line of its own between two paragraphs, or as the last line of a
block of HTML: not inside a list, a quotation, a footnote, a call or a
block of HTML, and not on a line with other writing.

A setting that describes one of the site's own commands needs the
command: `after_build_makes` says what `after_build` makes, and
`before_build_watches` says which files `before_build` is run again for.
Either one alone, on a site that doesn't set its command, describes
nothing.

## What you'll see {#what-youll-see}

```text
error  Wrong place: `pages.title` can't be handed down
  Write `title:` on each page instead.
  More: https://statac.dev/e/wrong-place/
```

## Why Statac stops

The line would do nothing where it is, and Statac can't tell what you
wanted it to do instead.

## How to fix it

Move the line where the message says. A `pages.` line for the whole site
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

Each page writes its own `title`. Hand down what the pages share:

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

The marker is inside the quotation. Move it out, below:

```markdown
> The part a list shows.

<!--more-->
```
