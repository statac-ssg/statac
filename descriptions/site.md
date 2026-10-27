# The site in a template

What a template reads on the site, after `site.`. A template reads a
custom setting the same way, such as `site.tagline`. On a site in
several languages, each value is the one for the page's language.

## `title`

The site's `title`. On a site in several languages, it is the title for
the page's language. If the site has no `title`, it has no value.

## `url`

The site's `url`, without the `/` at the end. A build run with `--url`
uses that URL instead. If the site has no `url`, it has no value.

## `host`

The host in the site's `url`: `example.org` for
`https://example.org/blog/`. If the site has no `url`, it has no
value.

## `language`

The page's language as a code, such as `en`. If the site sets no
`language`, it is `en`.

More: https://statac.dev/languages/#each-language-in-templates

## `languages`

Each of the site's languages as a code, the main one first. A site that
sets no `language` has only `en`. Followed by a code, as in
`site.languages.en`, it is that language's home page.

More: https://statac.dev/languages/#each-language-in-templates

## `today`

The day the site is built on: today in the site's `timezone`, or the day
given with `--today`.

More: https://statac.dev/build/#options

## `posts`

Every post, newest first, unless the home page's `order` says otherwise.

More: https://statac.dev/lists/#lists-in-templates

## `pages`

Every page the site publishes, in URL order. It includes a list's
paginated pages, the 404 page, and pages of the site's own that aren't
HTML, such as a `sitemap.xml` or a `robots.txt`, but not redirects or
feeds.

More: https://statac.dev/feeds/#sitemap

## `tags`

Every tag, A to Z, each a list with its `name`, `url` and `posts`.

More: https://statac.dev/lists/#lists-in-templates

## `series`

Every series, A to Z, each a list with its `name`, `url` and `posts`.

More: https://statac.dev/lists/#lists-in-templates

## `folders`

Every folder that has a page of its own, in URL order, each a list with
its `name`, `url` and `posts`.

More: https://statac.dev/lists/#lists-in-templates

## `feed`

The URL of the site's own feed. Empty if the site's own list has no
feed.

More: https://statac.dev/feeds/#the-template

## `feeds`

The URL of each of the site's own feeds, in the order `feed:` names
them. Empty if the site's own list has no feeds.

More: https://statac.dev/feeds/#the-template

## `data`

The files in `data/`, by their names: `site.data.links` for
`data/links.yaml`.

More: https://statac.dev/data/#in-templates

## `webmentions`

`true` if the site has `webmentions: true`, and otherwise `false`.

## `search_without_javascript`

What a form needs to send a search to the search engine the
`search_without_javascript` setting names: `name`, `action`, `words`
and `fields`. It has no value if the setting is unset or the site has no
`url`.

More: https://statac.dev/search/#without-javascript
