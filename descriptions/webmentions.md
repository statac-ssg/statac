# Webmentions in a template

What a template reads on `page.webmentions`, and on each mention in it.
Every value is plain text, never HTML.

## `comments`

The page's replies, mentions and bookmarks, oldest first. Empty where
there are none.

More: https://statac.dev/templates/#mention

## `reactions`

The page's likes and reposts, oldest first. Empty where there are none.

More: https://statac.dev/templates/#mention

## `kind`

What the mention is: `replied`, `liked`, `reposted`, `bookmarked` or
`mentioned`.

More: https://statac.dev/templates/#mention

## `url`

The URL of the sender's page that mentions this one.

More: https://statac.dev/templates/#mention

## `author`

Who sent the mention. A mention that doesn't say has none.

More: https://statac.dev/templates/#mention

## `author_url`

The URL of the sender's own page. A mention that doesn't say has none.

More: https://statac.dev/templates/#mention

## `published`

When the mention was published, as a date. A mention that doesn't say
has none.

More: https://statac.dev/templates/#mention

## `content`

What the sender wrote, as plain text. Show it as it is: `markdown` or
`safe` would put another site's HTML into the page. A mention that says
nothing has none.

More: https://statac.dev/templates/#mention

## `avatar`

The sender's image, as `page.image` gives one, once `statac refresh` has
fetched it. A mention without one has none.

More: https://statac.dev/templates/#mention
