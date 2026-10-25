# Functions

Each function a template can call by its name, as `{{ css("site.css")
}}`: Statac's own, and those of the template engine. A snippet is called
the same way, by its file's name.

## `css`

A `<link>` to a stylesheet in `assets/`, by its file's name:
`css("site.css")`.

More: https://statac.dev/assets/

## `js`

A `<script>` for a script in `assets/`, by its file's name:
`js("app.js")`.

More: https://statac.dev/assets/#scripts

## `asset`

The URL a file in `assets/` is published at, for an element written by
hand: `asset("fonts/font-1.woff2")`.

More: https://statac.dev/assets/

## `url`

A URL in the site, from its top, wherever the site is published:
`url("/about/")` is `/blog/about/` on a site at
`https://example.org/blog/`.

More: https://statac.dev/templates/#functions

## `headline`

A page's name, as text with no markup: its title, or else whatever names
it best, down to its URL. Every page has one.

More: https://statac.dev/templates/#headline

## `related`

The posts that share a tag with the page given, those sharing most
first: `related(page)`.

More: https://statac.dev/lists/#related-posts

## `feed_id`

A page's id in a feed: its own `id`, or else one made from the site's
`feed_key` and the page's URL. For a list page, the id of its feed. It
needs the site's `feed_key`.

More: https://statac.dev/feeds/#entry-ids

## `webmention`

The line for the `<head>` that tells other sites where to send the
site's webmentions. Without `webmentions: true`, it writes nothing.

More: https://statac.dev/webmentions/#where-other-sites-send-them

## `range`

A list of whole numbers: `range(3)` is 0, 1 and 2, and `range(1, 4)` is
1, 2 and 3.

## `dict`

A table made of the names and values given: `dict(title="Post 1",
rank=1)`.

## `namespace`

A table whose fields a `{% set %}` inside a loop can change, for a value
kept from one item to the next: `{% set seen = namespace(count=0) %}`.

## `debug`

What the template has been handed, written out, for finding a fault in a
template.
