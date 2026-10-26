# Functions

Each function a template can call by its name, such as
`{{ css("site.css") }}`: Statac's own, and the template engine's. A
template calls a snippet the same way, by its file's name.

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

Turns a URL written from the top of the site into one that works
wherever the site is published. On a site at
`https://example.org/blog/`, `url("/about/")` returns `/blog/about/`.

More: https://statac.dev/templates/#functions

## `headline`

A page's name, as text with no markup. It is the page's title if it has
one. Otherwise, Statac uses the best name it can find, down to the
page's URL, so every page has a headline.

More: https://statac.dev/templates/#headline

## `related`

Called as `related(page)`, the posts that share a tag with the page,
those sharing the most tags first.

More: https://statac.dev/lists/#related-posts

## `feed_id`

A page's id in a feed: its own `id` if it has one, or else an id Statac
makes from the site's `feed_key` and the page's URL. For a list page, it
is the id of the list's feed. It needs the site's `feed_key`.

More: https://statac.dev/feeds/#entry-ids

## `webmention`

The line for the `<head>` that tells other sites where to send the
site's webmentions. Without `webmentions: true`, it writes nothing.

More: https://statac.dev/webmentions/#where-other-sites-send-them

## `range`

A list of whole numbers: `range(3)` is 0, 1 and 2, and `range(1, 4)` is
1, 2 and 3.

## `dict`

A table of the names and values it is given: `dict(title="Post 1",
rank=1)`.

## `namespace`

A table whose fields a `{% set %}` inside a loop can change, so a value
can be kept from one item to the next:
`{% set seen = namespace(count=0) %}`.

## `debug`

Writes out everything the template has been handed, to help find a
fault in it.
