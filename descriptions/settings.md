# Settings

Each line of `statac.yaml` that Statac reads. Any other line is a custom
setting, which templates read as `site.<name>`.

## `title`

The site's name, which templates show as `site.title`. Feeds need it.

More: https://statac.dev/settings/#site

## `url`

The site's full URL, starting with `https://` or `http://`. Until it is
set, Statac leaves out feeds, social cards, webmentions and pages with
`needs: url`.

More: https://statac.dev/settings/#site

## `language`

The site's language as a code, such as `en`, `en-GB` or `fr`. A site in
several languages lists them all, the main one first: `[ar, en]`. The
default is `en`.

More: https://statac.dev/settings/#site

## `timezone`

The time zone your clock is in, such as `Europe/London`. Statac reads a
date with no offset in this time zone, and uses it to decide which day
is today. The default is UTC.

More: https://statac.dev/settings/#site

## `date_format`

How the site writes dates, given as an example date:
`31 December 2025`. Without it, the site writes dates the way its
language usually does.

More: https://statac.dev/settings/#dates

## `date_zeros`

When a date's month is written in figures, `false` drops the leading
zero from the day and the month: `4/3/2026`, not `04/03/2026`. The
default is `true`.

More: https://statac.dev/settings/#dates

## `typography`

`false` leaves quotes, `--`, `---` and `...` in the content exactly as
typed. The default is `true`.

More: https://statac.dev/settings/#writing-and-code

## `code_colours`

`true` marks each piece of code in the content with a class for what
kind of piece it is, such as `hl-keyword`, for the site's own stylesheet
to colour. The default is `false`, and Statac writes code exactly as
typed.

More: https://statac.dev/settings/#writing-and-code

## `posts_per_page`

How many posts each page of a list shows. The rest go on paginated
pages. A list page can set its own number. If it is unset or `0`, a list
shows all its posts on one page.

More: https://statac.dev/settings/#lists

## `pagination.permalink`

Where Statac publishes a list's paginated pages: a pattern that starts
with `{list}`, has `{n}` in it once, and ends with `/`. The default is
`{list}page/{n}/`.

More: https://statac.dev/settings/#lists

## `tags.permalink`

Where Statac publishes each tag's list page: a pattern ending with
`{slug}/`. The default is `/tags/{slug}/`.

More: https://statac.dev/settings/#lists

## `series.permalink`

Where Statac publishes each series' list page: a pattern ending with
`{slug}/`. The default is `/series/{slug}/`.

More: https://statac.dev/settings/#lists

## `tags.pages`

`true` makes a list page for each tag, unless a page of yours is
published at the tag's URL. Templates can use the tags either way. The
default is `false`.

More: https://statac.dev/settings/#lists

## `series.pages`

`true` makes a list page for each series, unless a page of yours is
published at the series' URL. Templates can use the series either way.
The default is `false`.

More: https://statac.dev/settings/#lists

## `tags.slugs`

The slug for each tag you name, one tag per line under the setting, such
as `C++: cpp`.

More: https://statac.dev/settings/#lists

## `series.slugs`

The slug for each series you name, one series per line under the
setting, such as `C++: cpp`.

More: https://statac.dev/settings/#lists

## `max_width`

The widest resized copy Statac makes of an image, in pixels: a whole
number, 1 or more. The default is `1600`.

More: https://statac.dev/settings/#pictures

## `image_sizes`

How wide the browser shows images on the page, written as the `sizes` of
an `<img>`. The default is `(max-width: 46rem) 100vw, 46rem`.

More: https://statac.dev/settings/#pictures

## `optimise_images`

`false` stops Statac making resized copies of images, so it publishes
each image unchanged. The default is `true`.

More: https://statac.dev/settings/#pictures

## `feed`

`true` gives the site's own list a feed, published beside its page as
`feed.xml`. A file name or a URL publishes the feed there instead. A
list of them gives the list a feed at each, and templates link to the
first. On a site in several languages, each language's own list gets a
feed. A `feed:` line on the list page takes the place of this
setting.

More: https://statac.dev/settings/#feeds

## `feed_key`

What each post's id in a feed is made from. If a list asks for a feed
and there is no key, the build stops and offers a newly made one.
`statac import` writes one for a site whose feeds come across with it.
Once a feed is published, never change it.

More: https://statac.dev/settings/#feeds

## `feed_ids_before`

A date, for a site moving from another tool. Each post dated before it
keeps its full URL as its id in a feed.

More: https://statac.dev/settings/#feeds

## `feed_ids_from`

`jekyll`, `hugo`, `zola` or `eleventy`. With `feed_ids_before`, the
posts dated before that date keep the ids that tool's feed gave them.
For `jekyll`, Statac makes each id from the post's folder and file name,
as Jekyll did. For `hugo`, `zola` and `eleventy`, each id is the post's
full URL spelt the way that tool spelt it, since each tool escapes a
space, `é` or `[` in its own way. Without it, Statac spells the full URL
its own way.

More: https://statac.dev/settings/#feeds

## `write_feed_ids`

`true` makes `statac serve` and `statac stamp` write an `id:` line into
each post of a feed that `statac build` hasn't yet published from this
computer. The default is `false`.

More: https://statac.dev/settings/#feeds

## `webmentions`

`true` means the site receives webmentions. They need `url`. The
default is `false`.

More: https://statac.dev/settings/#webmentions

## `refuse_webmentions_from`

Full URLs whose webmentions Statac leaves out: a webmention sent from a
page at or under a URL in the list, or by a sender whose own URL is in
the list. `statac refresh` deletes their files for good.

More: https://statac.dev/settings/#webmentions

## `search_without_javascript`

`duckduckgo` gives a reader whose browser runs no JavaScript a box on
the search page that sends the search to DuckDuckGo, which searches only
the site's host. The reader's searches then go to another company.

More: https://statac.dev/settings/#search

## `quiet`

The kinds of warning and note that `statac check` no longer shows, such
as `[broken-link]`. `build` and `check` still count them. Errors can't
be quietened.

More: https://statac.dev/settings/#quiet

## `before_build`

A hook Statac runs before it reads the site, such as a command that
makes a stylesheet.

More: https://statac.dev/settings/#hooks

## `before_build_watches`

The files and folders the `before_build` command reads, so `statac
serve` runs it again when they change. Write a folder with `/` at the
end: `[styles/]`.

More: https://statac.dev/settings/#hooks

## `after_build`

A hook Statac runs on the built files once the site builds with no
errors, before it publishes them.

More: https://statac.dev/settings/#hooks

## `after_build_outputs`

The files and folders the `after_build` command makes, from the top of
the built site: `[indexer/]`. Links into them count as working links.

More: https://statac.dev/settings/#hooks

## `other_hosts`

Other hosts the site is published at, such as `[www.example.org]`.
Statac checks a link to one of them as a link into the site.

More: https://statac.dev/settings/#links

## `served_elsewhere`

Paths on the site's host that something else publishes, such as
`[/video/]`. Statac doesn't check links into them.

More: https://statac.dev/settings/#links

## `exclude`

Files and folders in `content/` that Statac never publishes: a name, such
as `drafts`, wherever it is, or a path from `content/`, such as
`blog/one.md`. Setting it replaces the whole default list, which is
`.DS_Store`, `Thumbs.db` and `desktop.ini`.

More: https://statac.dev/settings/#links

## `fields.<name>`

The kind of value a custom field takes, with `required` before it if
every page must have it: `fields.isbn: required text`.

More: https://statac.dev/settings/#fields

## `types.<type>.<name>`

The kind of value a custom field takes on the pages of one `type`, with
`required` before it if each of them must have it:
`types.book.isbn: required text`.

More: https://statac.dev/settings/#fields
