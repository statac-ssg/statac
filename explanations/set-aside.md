# Set aside

A page sets a field that Statac doesn't use. This happens when:

- Statac fills that name itself on every page, such as `source` or
  `content`. Since 0.7.0 this includes `image` and `images`, which were
  free before, so a page that writes `image:` or `images:` has the field
  set aside.
- The page redirects with `redirect_to`, and the field, such as `tags`
  or `feed`, has nothing to act on. A page that redirects uses only its
  `title`, `date`, `permalink`, `draft`, `needs` and `redirect_from`,
  and, on a folder's list page, its `order` and `numbered`.

A card's settings in `templates/` can use `background`, `font` and
`padding`, and, for each text, `text`, `place`, `size` and `colour`
(written like `title.size`). Any other name in them is set aside.

A page's `card` is set aside if the page also names its own social
image with `og_image`, because `og_image` takes priority.

A built-in field, a setting, or a field that `statac.yaml` declares is
set aside if nothing at all is written after its colon, such as `title:`.
Statac treats it as though the line weren't there. `~` and `null` are
written on purpose, so Statac never reports them. Nor does it report an
empty custom field that isn't declared, because a template may test for
it.

## What you'll see {#what-youll-see}

```text
warning  Field set aside: Statac fills `page.source` itself
  content/post-1.md, line 3
   3 │ source: notes/post-1.txt
     │ ^^^^^^
  Give it a name of your own, such as `my_source`.
  More: https://statac.dev/e/set-aside/
```

## Why Statac warns

Statac is sure what to do here: it leaves the line alone and uses its
own value, so the build carries on. It warns you because what you wrote
doesn't reach the page. A page that redirects is in no list or
feed, and is built with no layout, so its tags, its feed and similar
fields do nothing.

## How to fix it

Give the field a name of your own if you want to use it in your
templates. A page that writes `image:` or `images:` keeps its value under
another name: rename the field, for example to `cover:`, and change the templates
that read it to `page.cover`. To choose where a page is published, write
`permalink:`. On a page that redirects, remove the field, or remove
`redirect_to` to publish the page. In the card's settings, correct the
name to one a card uses. Statac suggests the nearest one. On a page with both `card` and
`og_image`, remove one of them. If nothing is written after the colon,
give the line a value, or remove it.

## Example

In `content/post-1.md`:

```yaml
---
title: Post 1
source: notes/post-1.txt
---
```

`page.source` is the file the page was read from. Give yours a different
name:

```yaml
---
title: Post 1
my_source: notes/post-1.txt
---
```
