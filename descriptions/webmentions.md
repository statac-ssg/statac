# Webmentions in a template

Summary: Every value a template can read on a page's webmentions, and on each comment or reaction.

The names (the variables a template can use) that a template reads on
`page.webmentions`, and on each mention in it. Every value is plain
text, never HTML.

## `comments`

The page's replies, mentions and bookmarks, oldest first. Empty if there
are none.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `reactions`

The page's likes and reposts, oldest first. Empty if there are none.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `kind`

What the mention is: `replied`, `liked`, `reposted`, `bookmarked` or
`mentioned`.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `url`

The URL of the sender's page that mentions this one.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `author`

Who sent the mention. If the mention doesn't say, `author` has no
value.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `author_url`

The URL of the sender's own page. If the mention doesn't say,
`author_url` has no value.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `published`

When the mention was published, as a date. If the mention doesn't say,
`published` has no value.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `content`

What the sender wrote, as plain text. Show it without `markdown` or
`safe`, which would put another site's HTML into the page. If the
mention has no text, `content` has no value.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `avatar`

The sender's image, with the same values as an image from `page.image`.
It has no value until `statac refresh` fetches it, or if the mention has
no image.

More: https://statac.dev/webmentions/#webmentions-in-a-layout
