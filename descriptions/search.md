# Search in a template

What a template reads on the search box that `page.search_box()`
returns, and on the search form `site.search_without_javascript`
describes.

## `id`

The id for the search box's element: `statac-search` for the page's
first box, and `statac-search-2` and on for any after it.

More: https://statac.dev/search/#placing-the-box

## `first`

`true` for the page's first search box, so a layout can write what
every box on the page shares only once.

More: https://statac.dev/search/#placing-the-box

## `index`

The URL of the search index for the page's language. If a template
reads it, the build publishes the index.

More: https://statac.dev/search/#placing-the-box

## `name`

The name of the search engine the form sends the search to, such as
`DuckDuckGo`.

More: https://statac.dev/search/#without-javascript

## `action`

The URL the form sends the search to, for the `action` of a `<form>`.

More: https://statac.dev/search/#without-javascript

## `words`

The name the form sends the reader's words under, for the `name` of its
text `<input>`.

More: https://statac.dev/search/#without-javascript

## `fields`

The hidden fields the form sends with the reader's words, so that the
search engine searches only this site. Each has a `name` and a
`value`.

More: https://statac.dev/search/#without-javascript
