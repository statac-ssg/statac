# Search in a template

What a template reads on the search box `page.search_box()` gives, and
on the form `site.search_without_javascript` gives.

## `id`

The id for the search box's element: `statac-search` for the page's
first box, and `statac-search-2` and on for any after it.

More: https://statac.dev/search/#placing-the-box

## `first`

`true` for the page's first search box, the one to carry what every box
on the page shares.

More: https://statac.dev/search/#placing-the-box

## `index`

The URL of the search index for the page's language. Reading it has the
build publish the index.

More: https://statac.dev/search/#placing-the-box

## `name`

The name of the search engine the form sends the search to, as
`DuckDuckGo`.

More: https://statac.dev/search/#without-javascript

## `action`

The URL the form sends the search to, for the `action` of a `<form>`.

More: https://statac.dev/search/#without-javascript

## `words`

The name the reader's words are sent under, for the `name` of the
form's text `<input>`.

More: https://statac.dev/search/#without-javascript

## `fields`

The hidden fields the form sends with the reader's words to keep the
search to the site, each with its `name` and `value`.

More: https://statac.dev/search/#without-javascript
