# The site in a template

What a template reads on the site, after `site.`. A custom setting is
read the same way, as `site.tagline`. On a site in several languages,
each is as the page's language has it.

## `title`

The site's `title`. On a site in several languages, the title for the
page's language. A site without one has none.

More: https://statac.dev/templates/#site

## `url`

The site's `url` with no `/` at the end, or the one given with `--url`
for one build. A site without one has none.

More: https://statac.dev/templates/#site

## `host`

The host in the site's `url`: `example.org` for
`https://example.org/blog/`. A site with no `url` has none.

More: https://statac.dev/templates/#site

## `language`

The page's language as a code, such as `en`. Where the site sets no
`language`, it is `en`.

More: https://statac.dev/templates/#site

## `languages`

Each of the site's languages as a code, the main one first. Looked up by
a code, as `site.languages.en`, it gives that language's home page.

More: https://statac.dev/dates-and-languages/#each-language-in-templates

## `today`

The day the site is built on: today in the site's `timezone`, or the day
given with `--today`.

More: https://statac.dev/templates/#site

## `posts`

Every post, newest first, unless the home page's `order` says otherwise.

More: https://statac.dev/templates/#site

## `pages`

Every page the site publishes, in URL order. It holds a list's paginated
pages and pages that aren't HTML, such as a `sitemap.xml` of the site's
own, but no redirects.

More: https://statac.dev/templates/#site

## `tags`

Every tag, A to Z, each a list with its `name`, `url` and `posts`.

More: https://statac.dev/templates/#site

## `series`

Every series, A to Z, each a list with its `name`, `url` and `posts`.

More: https://statac.dev/templates/#site

## `folders`

Every folder that has a page of its own, in URL order, each a list with
its `name`, `url` and `posts`.

More: https://statac.dev/templates/#site

## `feed`

The URL of the site's own feed. Empty where the site's own list has
none.

More: https://statac.dev/templates/#site

## `feeds`

The URL of each of the site's own feeds, in the order `feed:` names
them. Empty where the site's own list has none.

More: https://statac.dev/templates/#site

## `data`

The files in `data/`, by their names: `site.data.links` for
`data/links.yaml`.

More: https://statac.dev/data/#in-templates

## `webmentions`

`true` where the site has `webmentions: true`, and otherwise `false`.

More: https://statac.dev/templates/#site

## `search_without_javascript`

What a form needs to send a search elsewhere, as
`search_without_javascript` asks: `name`, `action`, `words` and
`fields`. None where the setting is unset or the site has no `url`.

More: https://statac.dev/search/#without-javascript
