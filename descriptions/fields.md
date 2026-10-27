# Front matter fields

Each field of a page's front matter that Statac reads. Any other field
is a custom field, which templates read as `page.<name>`.

## `title`

The page's title. Without it, the page's opening `# heading` is its
title.

More: https://statac.dev/writing/#page

## `untitled`

`true` if the page is meant to have no title, such as a short note.
Statac then names it by its date. A folder of such pages usually
cascades it with `pages.untitled: true`.

More: https://statac.dev/lists/#pages-without-titles

## `summary`

A line about the page.

More: https://statac.dev/writing/#page

## `date`

The page's date, year first: `2026-10-04`, or `2026-10-04 09:30` with a
time. The time can have seconds and a `T` in place of the space, and an
offset such as `Z`, `+01:00`, `+0100` or `+01`, with or without a space
before it. A file named `2026-10-04-post-1.md` takes its date from its
name. If you write a stamp, `statac serve` or `statac stamp` replaces it
with the date: `=today`, `=now`, `=tomorrow`, `=yesterday`, or `=monday`
to `=sunday`, the next such day, so on a Monday `=monday` is a week away.
A time can follow, as in `=monday 09:00`. A stamp is a stamp only as the
whole of a front matter value, and plain text anywhere else.

More: https://statac.dev/dates-and-languages/#writing-a-date

## `updated`

When the page last changed, written the same way as `date`.

More: https://statac.dev/dates-and-languages/#writing-a-date

## `draft`

`true` leaves the page out of the build. `statac serve` still shows it,
marked as a draft.

More: https://statac.dev/lists/#drafts-and-posts-waiting-for-their-day

## `layout`

The template the page is built with, by its name in `templates/`. On a
page with `each:`, the template for the pages it makes.

More: https://statac.dev/templates/#which

## `type`

What kind of page this is, in a word you choose. If `templates/` has a
template of that name and the page sets no `layout`, Statac builds the
page with that template.

More: https://statac.dev/templates/#which

## `permalink`

The page's URL, or a pattern such as `/{year}/{slug}/`.

More: https://statac.dev/urls/#permalink

## `redirect_from`

Old URLs of the page. Each one redirects to it.

More: https://statac.dev/urls/#redirect-from

## `redirect_to`

The URL the page redirects to. Statac publishes nothing else for the
page.

More: https://statac.dev/urls/#sending-readers-on

## `needs`

`needs: url` leaves the page out until the site has a `url`. A
`sitemap.xml` needs it, since its URLs must be full.

More: https://statac.dev/feeds/#sitemaps-robotstxt-and-other-files

## `translation_key`

Links the page to its translations if their names differ. Pages with the
same key are translations of each other.

More: https://statac.dev/dates-and-languages/#translations

## `tags`

The page's tags, as a list in brackets: `[Rust, Go]`.

More: https://statac.dev/lists/#tags

## `series`

The one series the page is part of.

More: https://statac.dev/lists/#series

## `neighbours`

Which list `page.previous` and `page.next` move through: `site`, the
site's own list; `series`, the page's series; `folder`, the outermost
folder holding the page below the top of `content/`; or `false` for
none. Without it, a page moves through its series, or else that folder.
It acts only on pages that aren't list pages: a list page's
`page.previous` and `page.next` are always its list's own pages.

More: https://statac.dev/lists/#previous-and-next

## `posts`

A list the page makes of its own, chosen from all the posts with
`tagged`, `in` or `where`, such as `site.posts | tagged("Rust")` or
`site.posts | in("blog")`.

More: https://statac.dev/lists/#lists-of-your-own

## `each`

Makes a page for each item of a table in `data/`, such as
`each: data.people`, or for each post, such as `each: site.posts`. Posts
can be chosen with `tagged`, `in` and `where`, as for `posts:`.

More: https://statac.dev/data/#pages-made-from-data

## `order`

The list's order: `newest first`, `oldest first`, or the names of its
pages in the order you want. Cascaded with `pages.order`, `newest first`
or `oldest first` reaches the lists of the folders inside, but not the
folder's own list. A list of names works only on the list's own page, and
`pages.order` can't cascade one.

More: https://statac.dev/lists/#order

## `numbered`

In a folder's `index.md`, `false` says that the numbers at the start of
its pages' names are part of the names, and `true` that they order the
folder, as meant, so Statac doesn't note it.

More: https://statac.dev/lists/#numbered-folders

## `posts_per_page`

How many posts each page of the list shows. `0` shows them all on one
page.

More: https://statac.dev/lists/#pages-of-a-list

## `index_page`

`false` stops Statac publishing the list's page and its paginated pages.
Templates can still use the list. Cascaded with `pages.index_page`, it
reaches the list pages of the folders inside, but not the folder's
own.

More: https://statac.dev/lists/#a-lists-page

## `feed`

`true` gives the list a feed, published beside the list's page as
`feed.xml`. A file name or a URL publishes the feed there instead. A
list of them gives the list a feed at each, and templates link to the
first. A list has no feed unless a `feed:` line asks for one. Cascaded
with `pages.feed`, it reaches the lists of the folders inside, but not
the folder's own list.

More: https://statac.dev/feeds/#which-lists-have-a-feed

## `id`

What identifies a post in a feed, even if the post moves.

More: https://statac.dev/feeds/#entry-ids

## `search`

`false` leaves the page out of the site's search.

More: https://statac.dev/search/#leave-out

## `card`

Which of the site's social cards Statac draws for the page, if the site
has cards turned on.

More: https://statac.dev/social-cards/#other-cards

## `og_image`

The page's social image, in place of a card.

More: https://statac.dev/social-cards/#a-picture-of-your-own

## `og_title`

The page's title on a card and in a link preview.

More: https://statac.dev/social-cards/#what-a-preview-says

## `mentions`

`true` if the page shows its webmentions. A template reads it as
`page.mentions`.

More: https://statac.dev/webmentions/#webmentions-in-a-layout
