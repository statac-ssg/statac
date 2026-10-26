# Front matter fields

Each field of a page's front matter that Statac reads. Any other field
is a custom field, which templates read as `page.<name>`.

## `title`

The page's title. Without it, the page's opening `# heading` is its
title.

More: https://statac.dev/front-matter/#page-fields

## `untitled`

`true` if the page is meant to have no title, such as a short note.
Statac then names it by its date.

More: https://statac.dev/front-matter/#page-fields

## `summary`

A line about the page.

More: https://statac.dev/front-matter/#page-fields

## `date`

The page's date, year first: `2026-10-04`, or `2026-10-04 09:30` with a
time. If you write `=today`, `statac serve` or `statac stamp` replaces
it with the day's date.

More: https://statac.dev/front-matter/#page-fields

## `updated`

When the page last changed, written the same way as `date`.

More: https://statac.dev/front-matter/#page-fields

## `draft`

`true` leaves the page out of the build. `statac serve` still shows it,
marked as a draft.

More: https://statac.dev/front-matter/#page-fields

## `layout`

The template the page is built with, by its name in `templates/`. On a
page with `each:`, the template for the pages it makes.

More: https://statac.dev/front-matter/#page-fields

## `type`

What kind of page this is, in a word of your own. If `templates/` has a
template of that name and the page sets no `layout`, Statac builds the
page with that template.

More: https://statac.dev/front-matter/#page-fields

## `permalink`

The page's URL, or a pattern such as `/{year}/{slug}/`.

More: https://statac.dev/front-matter/#address-fields

## `redirect_from`

Old URLs of the page. Each one redirects to it.

More: https://statac.dev/front-matter/#address-fields

## `redirect_to`

The URL the page redirects to. Statac publishes nothing else for the
page.

More: https://statac.dev/front-matter/#address-fields

## `needs`

`needs: url` leaves the page out until the site has a `url`.

More: https://statac.dev/front-matter/#address-fields

## `translation_key`

Links the page to its translations if their names differ. Pages with the
same key are translations of each other.

More: https://statac.dev/front-matter/#address-fields

## `tags`

The page's tags, as a list in brackets: `[Rust, Go]`.

More: https://statac.dev/front-matter/#list-fields

## `series`

The one series the page is part of.

More: https://statac.dev/front-matter/#list-fields

## `neighbours`

Which list `page.previous` and `page.next` move through: `site`,
`folder`, `series`, or `false` for none.

More: https://statac.dev/front-matter/#list-fields

## `posts`

A list the page makes of its own, chosen from all the posts, such as
`site.posts | tagged("Rust")`.

More: https://statac.dev/front-matter/#list-page-fields

## `each`

Makes a page for each item of a table in `data/`, as in `each:
data.people`, or for each post, as in `each: site.posts`.

More: https://statac.dev/front-matter/#list-page-fields

## `order`

The list's order: `newest first`, `oldest first`, or the names of its
pages in the order you want.

More: https://statac.dev/front-matter/#list-page-fields

## `numbered`

In a folder's `index.md`, `false` says that the numbers at the start of
its pages' names are part of the names.

More: https://statac.dev/front-matter/#list-page-fields

## `posts_per_page`

How many posts each page of the list shows. `0` shows them all on one
page.

More: https://statac.dev/front-matter/#list-page-fields

## `index_page`

`false` stops Statac publishing the list's page and its paginated pages.
Templates can still use the list.

More: https://statac.dev/front-matter/#list-page-fields

## `feed`

`true` gives the list a feed, published beside the list's page as
`feed.xml`. A file name or a URL publishes the feed there instead. A
list of them gives the list a feed at each, and templates link to the
first. A list has no feed unless a `feed:` line asks for one.

More: https://statac.dev/front-matter/#list-page-fields

## `id`

What identifies a post in a feed, even if the post moves.

More: https://statac.dev/front-matter/#sharing-fields

## `search`

`false` leaves the page out of the site's search.

More: https://statac.dev/front-matter/#sharing-fields

## `card`

Which of the site's social cards Statac draws for the page, if the site
has cards turned on.

More: https://statac.dev/front-matter/#sharing-fields

## `og_image`

The page's social image, in place of a card.

More: https://statac.dev/front-matter/#sharing-fields

## `og_title`

The page's title on a card and in a link preview.

More: https://statac.dev/front-matter/#sharing-fields

## `mentions`

`true` if the page shows its webmentions. A template reads it as
`page.mentions`.

More: https://statac.dev/front-matter/#sharing-fields
