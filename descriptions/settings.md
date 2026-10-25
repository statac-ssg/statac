# Settings

Each line of `statac.yaml` that Statac reads. Any other line is a custom
setting, which templates read as `site.<name>`.

## `title`

The site's name, which templates show as `site.title`. Feeds need it.

More: https://statac.dev/settings/#site

## `url`

The site's full URL, starting with `https://` or `http://`. Feeds,
social cards, webmentions and pages with `needs: url` are left out until
it is set.

More: https://statac.dev/settings/#site

## `language`

The site's language as a code, such as `en`, `en-GB` or `fr`. A site in
several lists them all, the main one first: `[ar, en]`. Unset, it is
`en`.

More: https://statac.dev/settings/#site

## `timezone`

The time zone your clock is in, such as `Europe/London`. Dates with no
offset are read in it, and it decides which day is today. Unset, it is
UTC.

More: https://statac.dev/settings/#site

## `date_format`

How dates look, written as an example date: `31 December 2025`. Unset,
dates look as the site's language usually writes them.

More: https://statac.dev/settings/#dates

## `date_zeros`

Where the month is written in figures, `false` drops the leading zero
from the day and the month: `4/3/2026`, not `04/03/2026`. Unset, it is
`true`.

More: https://statac.dev/settings/#dates

## `typography`

`false` leaves quotes, `--`, `---` and `...` in the content exactly as
typed. Unset, it is `true`.

More: https://statac.dev/settings/#writing-and-code

## `code_colours`

`true` marks each piece of code in the content with a class for its
kind, such as `hl-keyword`, for the site's own stylesheet to colour.
Unset, it is `false`, and code is written as it is.

More: https://statac.dev/settings/#writing-and-code

## `posts_per_page`

How many posts each page of a list shows, the rest going on to paginated
pages. A list page can set its own number. Unset, or `0`, a list shows
all its posts on one page.

More: https://statac.dev/settings/#lists

## `pagination.permalink`

Where a list's paginated pages go: a pattern that starts with `{list}`,
holds `{n}` once and ends with `/`. Unset, it is `{list}page/{n}/`.

More: https://statac.dev/settings/#lists

## `tags.permalink`

Where each tag's list page is published: a pattern ending with
`{slug}/`. Unset, it is `/tags/{slug}/`.

More: https://statac.dev/settings/#lists

## `series.permalink`

Where each series' list page is published: a pattern ending with
`{slug}/`. Unset, it is `/series/{slug}/`.

More: https://statac.dev/settings/#lists

## `tags.pages`

`true` makes a list page for each tag where no page of yours is
published at its URL. The tags work in templates either way. Unset, it
is `false`.

More: https://statac.dev/settings/#lists

## `series.pages`

`true` makes a list page for each series where no page of yours is
published at its URL. The series work in templates either way. Unset, it
is `false`.

More: https://statac.dev/settings/#lists

## `tags.slugs`

The slug for a tag: each tag on its own line beneath the setting, as in
`C++: cpp`.

More: https://statac.dev/settings/#lists

## `series.slugs`

The slug for a series: each series on its own line beneath the setting,
as in `C++: cpp`.

More: https://statac.dev/settings/#lists

## `max_width`

The widest resized copy Statac makes of an image, in pixels: a whole
number, 1 or more. Unset, it is `1600`.

More: https://statac.dev/settings/#pictures

## `image_sizes`

How wide images are shown on the page, written as the `sizes` of an
`<img>`. Unset, it is `(max-width: 46rem) 100vw, 46rem`.

More: https://statac.dev/settings/#pictures

## `optimise_images`

`false` stops Statac making resized copies of images: each is published
as it is. Unset, it is `true`.

More: https://statac.dev/settings/#pictures

## `feed`

`true` gives the site's own list a feed, beside its page as `feed.xml`,
and a name or a URL puts it there. A list of them gives it a feed at
each, templates linking the first. On a site in several languages, each
language's own list has one. A `feed:` line on the list page wins over
it.

More: https://statac.dev/settings/#feeds

## `feed_key`

What each post's id in a feed is made from. `statac new` writes a new
one. Once a feed is published, never change it.

More: https://statac.dev/settings/#feeds

## `feed_ids_before`

A date, for a site moving from another tool. Each post dated before it
keeps its full URL as its id in a feed.

More: https://statac.dev/settings/#feeds

## `feed_ids_from`

`jekyll`. With `feed_ids_before`, the posts dated before it keep the ids
that tool's feed gave them.

More: https://statac.dev/settings/#feeds

## `write_feed_ids`

`true` makes `statac serve` and `statac stamp` write an `id:` line into
each post of a feed that `statac build` hasn't yet published from this
computer. Unset, it is `false`.

More: https://statac.dev/settings/#feeds

## `webmentions`

`true` means the site receives webmentions. They need `url`. Unset, it
is `false`.

More: https://statac.dev/settings/#webmentions

## `refuse_webmentions_from`

Full URLs whose webmentions are left out: one sent from a page at or
under a URL listed, or by a sender whose own URL is. `statac refresh`
deletes their files for good.

More: https://statac.dev/settings/#webmentions

## `search_without_javascript`

`duckduckgo` gives a reader whose browser runs no JavaScript a box on
the search page that sends the search to DuckDuckGo, limited to the
site's host. Their searches go to another company.

More: https://statac.dev/settings/#search

## `quiet`

The kinds of warning and note that `statac check` no longer shows, such
as `[broken-link]`. `build` and `check` still count them. Errors can't
be quietened.

More: https://statac.dev/settings/#quiet

## `before_build`

A hook run before the site is read, such as a command that makes a
stylesheet.

More: https://statac.dev/settings/#hooks

## `before_build_watches`

The files and folders the `before_build` command reads, so `statac
serve` runs it again when they change. Folders end with `/`:
`[styles/]`.

More: https://statac.dev/settings/#hooks

## `after_build`

A hook run on the built files after the site builds with no errors,
before they are published.

More: https://statac.dev/settings/#hooks

## `after_build_outputs`

The files and folders the `after_build` command makes, from the top of
the built site: `[indexer/]`. Links into them count as working links.

More: https://statac.dev/settings/#hooks

## `other_hosts`

Other hosts the site is published at, such as `[www.example.org]`. A
link to one of them is checked as a link into the site.

More: https://statac.dev/settings/#links

## `served_elsewhere`

Paths on the site's host that something else publishes, such as
`[/video/]`. Links into them aren't checked.

More: https://statac.dev/settings/#links

## `exclude`

Files and folders in `content/` that are never published: a name, such as
`drafts`, wherever it is, or a path from `content/`, such as
`blog/one.md`. Setting it replaces the whole list. Unset, it is
`.DS_Store`, `Thumbs.db` and `desktop.ini`.

More: https://statac.dev/settings/#links

## `fields.<name>`

The kind of value a custom field holds, with `required` before it where
every page must have it: `fields.isbn: required text`.

More: https://statac.dev/settings/#fields

## `types.<type>.<name>`

The kind of value a custom field holds on the pages of one `type`, with
`required` before it where each of them must have it: `types.book.isbn:
required text`.

More: https://statac.dev/settings/#fields
