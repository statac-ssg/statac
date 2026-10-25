# A list in a template

What a template reads on a list: a folder, a tag, a series, or a list a
page makes with `posts:` or `each:`. Lists are reached as `page.list`,
`page.folder`, `page.tags`, `site.tags` and the like, and as `list` in a
feed's template.

## `name`

The list's name: a folder's own name, a tag or a series as it is spelt,
or for a list a page makes, that page's title.

More: https://statac.dev/lists/#lists-in-templates

## `title`

The title of the list's page, or for the site's own list the site's
title, or else the list's name.

More: https://statac.dev/lists/#lists-in-templates

## `url`

The URL of the list's page. A list with no page has none.

More: https://statac.dev/lists/#lists-in-templates

## `page`

The list's page. A list with no page has none.

More: https://statac.dev/lists/#lists-in-templates

## `posts`

The list's posts, in the list's order.

More: https://statac.dev/lists/#lists-in-templates

## `pages`

The list's posts, then the pages it holds that have no date.

More: https://statac.dev/lists/#lists-in-templates

## `pager`

The list's page and each of its paginated pages, first to last, each
with its `number` and `url`. Empty where the list's page isn't
published.

More: https://statac.dev/lists/#lists-in-templates

## `feed`

The URL of the list's feed, or of the first where it has several. Empty
where it has none.

More: https://statac.dev/lists/#lists-in-templates

## `feeds`

The URL of each of the list's feeds, in the order `feed:` names them.
Empty where it has none.

More: https://statac.dev/lists/#lists-in-templates

## `word`

Which kind of list it is: `site`, `folder`, `tag`, `series`, or `chosen`
for a list a page makes with `posts:` or `each:`.

More: https://statac.dev/lists/#lists-in-templates

## `order`

`newest first` or `oldest first`. Empty where numbers or an `order:`
list of names put its posts in order.

More: https://statac.dev/lists/#lists-in-templates
