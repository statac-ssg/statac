# A list in a template

Summary: Every value a template can read on a folder, a tag, a series or another list, such as its posts and feed.

What a template reads on a list: a folder, a tag, a series, or a list a
page makes with `posts:` or `each:`. A template reaches lists through
`page.list`, `page.folder`, `page.tags`, `site.tags` and the like, and
through `list` in a feed's template.

## `name`

The list's name: a folder's own name, a tag's or a series' name as the
pages spell it, or, for a list a page makes, that page's title.

More: https://statac.dev/lists/#lists-in-templates

## `title`

The title of the list page. If there is no list page, or it has no
title, it is the site's title for the site's own list, and the list's
name for any other.

More: https://statac.dev/lists/#lists-in-templates

## `url`

The URL of the list page. If the list has no list page, `url` has no
value.

More: https://statac.dev/lists/#lists-in-templates

## `page`

The list page. If the list has no list page, `page` has no value.

More: https://statac.dev/lists/#lists-in-templates

## `posts`

The list's posts, in the list's order.

More: https://statac.dev/lists/#lists-in-templates

## `pages`

The list's posts, then its pages that have no date.

More: https://statac.dev/lists/#lists-in-templates

## `pager`

The list page and each of its paginated pages, first to last, each
with its `number` and `url`. Empty if the list page isn't published.

More: https://statac.dev/lists/#lists-in-templates

## `feed`

The URL of the list's feed, or of its first feed if it has several.
Empty if it has none.

More: https://statac.dev/lists/#lists-in-templates

## `feeds`

The URL of each of the list's feeds, in the order `feed:` names them.
Empty if it has none.

More: https://statac.dev/lists/#lists-in-templates

## `word`

Which kind of list it is: `site`, `folder`, `tag`, `series`, or `chosen`
for a list a page makes with `posts:` or `each:`.

More: https://statac.dev/lists/#lists-in-templates

## `order`

`newest first` or `oldest first`. Empty if the numbers its file names
start with, or an `order:` list of names, put its posts in order.

More: https://statac.dev/lists/#lists-in-templates
