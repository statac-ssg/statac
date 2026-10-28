# Functions

Summary: Every function a template can call, and what each one does.

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

Turns a URL written from the top of the site, starting with a single
`/`, into one that works wherever the site is published, keeping
anything after `?` or `#`. On a site at `https://example.org/blog/`,
`url("/about/")` returns `/blog/about/`, and on a site at the top of its
domain, `/about/`. Statac leaves a link written without it unchanged. In
the content, `@url(/about/)` does the same.

More: https://statac.dev/urls/#a-site-in-a-folder-of-its-domain

## `headline`

A page's name, as text with no markup, so every page has one. It is the
first there is of: the page's `title`; for the home page, the site's
`title`; the `# heading` its content opens with, without its Markdown;
a list page's list name; its `summary` or first paragraph, cut at a
word to 70 characters with `…`; its date; its URL. A list's paginated
pages share its first page's name, and on an `untitled` page the date
comes before the summary.

## `related`

`related(page)` gives the posts that share a tag with the page,
those sharing the most tags first.

More: https://statac.dev/lists/#related-posts

## `feed_id`

A page's id in a feed, written as `urn:uuid:` and a UUID: the page's
own `id` where that is a UUID, or else one Statac makes from the site's
`feed_key` and the page's `id` or URL. For a list page, it is the id of
the list's feed. It needs the site's `feed_key`. On a site with
`feed_ids_before`, a list page and each post dated before it keep the
ids the old tool gave them instead.

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
