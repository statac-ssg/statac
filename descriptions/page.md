# A page in a template

What a template reads on a page: after `page.`, or after any other name
that stands for a page, such as `post.title` or `page.next.url`. A
template reads a custom field the same way, such as `page.isbn`.

## `title`

The page's title: its `title`, or if it has none, the `# heading` its
content opens with. If the page has neither, `title` has no value, so to
show the name of any page, write `headline(page)`.

More: https://statac.dev/templates/#write

## `url`

The URL the page is published at, starting with `/`, such as
`/posts/post-1/`. If the site's `url` has a path, such as
`https://example.org/blog/`, the URL starts with that path, `/blog/`.

More: https://statac.dev/urls/#made

## `slug`

The last part of the page's URL: `post-1` for `/posts/post-1/`.

More: https://statac.dev/urls/#name

## `date`

The page's `date`, with its `year`, `month`, `day` and `weekday`. On its
own, it shows in the site's date format. If the page has no `date`, it
has no value, and neither does a list's paginated page.

More: https://statac.dev/dates-and-languages/#how-dates-look

## `updated`

The page's `updated` date. On its own, it shows in the site's date
format. If the page has no `updated`, it has no value, and neither does
a list's paginated page.

More: https://statac.dev/dates-and-languages/#how-dates-look

## `summary`

The page's `summary`, as the page writes it. If the page has none,
`summary` has no value, and `page.opening` has the words of its first
paragraph instead.

## `content`

The page's content, built into HTML. It is empty on a list's paginated
page. When another page shows it, such as a list or a feed, Statac
writes its relative URLs in full.

More: https://statac.dev/templates/#write

## `excerpt`

The start of the page's content, built into HTML: everything before its
`<!--more-->` line, or if it has none, its first paragraph. If the page
has neither, `excerpt` has no value.

More: https://statac.dev/writing/#excerpts

## `excerpt_marked`

`true` if the page's excerpt ends at a `<!--more-->` line, and `false`
if it is the first paragraph.

More: https://statac.dev/writing/#excerpts

## `opening`

The words of the page's first paragraph, as text with no markup. If the
page has no paragraph, `opening` has no value.

## `word_count`

The number of words in the page's content, not counting code blocks and
footnote markers. In Chinese and Japanese, each character
counts as a word.

## `maths`

`true` if the page's content has maths in it, so a layout can add the
maths stylesheet only where it is needed. It says nothing of the posts a
page lists, so a template listing excerpts reads each post's
`post.maths`.

More: https://statac.dev/writing/#maths

## `toc`

The page's headings, each with its `title`, `id`, `url`, `level` and the
headings under it as `headings`. Empty if the page has no headings.

## `language`

The page's language as a code, such as `en` or `fr`, for `<html lang>`.

More: https://statac.dev/dates-and-languages/#each-language-in-templates

## `direction`

Which way the page's language runs: `ltr`, left to right, or `rtl`,
right to left. For `<html dir>`.

More: https://statac.dev/dates-and-languages/#each-language-in-templates

## `translations`

The same page in the site's other languages, the main language first.
Empty if there are none.

More: https://statac.dev/dates-and-languages/#translations

## `tags`

The page's tags, each a list with its `name`, `url` and `posts`.
`"Rust" in page.tags` is `true` if the page has that tag, whatever
capitals it is written with.

More: https://statac.dev/lists/#lists-in-templates

## `series`

The series the page is in, as a list with its `name`, `url` and `posts`.
If the page isn't in a series, `series` has no value.

More: https://statac.dev/lists/#lists-in-templates

## `folder`

The folder the page is in, as a list with its `name`, `url` and
`posts`.

More: https://statac.dev/lists/#lists-in-templates

## `list`

On a list page and each of its paginated pages, the list the page is
for. On other pages, it has no value.

More: https://statac.dev/lists/#lists-in-templates

## `posts`

On a list page, the posts this page of the list shows, in the list's
order. On other pages, it has no value.

More: https://statac.dev/lists/#lists-in-templates

## `number`

On a list page, which page of the list this is, counting from 1. On
other pages, it has no value.

More: https://statac.dev/lists/#lists-in-templates

## `previous`

The page before this one in the list `neighbours` chooses. On a list
page, it is the list's page before this one. On the first page, it has
no value.

More: https://statac.dev/lists/#previous-and-next

## `next`

The page after this one in the list `neighbours` chooses. On a list
page, it is the list's next page. On the last page, it has no value.

More: https://statac.dev/lists/#previous-and-next

## `post`

On a page made for each post by `each: site.posts`, the post it was made
for.

More: https://statac.dev/lists/#a-page-for-each-post

## `key`

On a page made by `each:` from an item in `data/`, the item's name.

More: https://statac.dev/data/#pages-made-from-data

## `source`

The file Statac read the page from, starting from the site's folder:
`content/posts/post-1.md`. A page Statac makes, such as a tag's page,
has no `source`.

## `og_title`

The page's name where it is shared: its `og_title`, or if it has none,
`headline(page)`.

More: https://statac.dev/social-cards/#in-a-layout

## `card`

The social card Statac draws for the page, with its `url`, `width`,
`height` and `alt`. If the page has no card, `card` has no value.

More: https://statac.dev/social-cards/#in-a-layout

## `webmentions`

What other sites have said about the page: `comments`, the replies,
mentions and bookmarks, and `reactions`, the likes and reposts. Both are
empty if no one has said anything.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `search_box`

`page.search_box()` gives what a search box needs: `id`, the id for
its element, `first`, whether it is the page's first, and `index`, the
URL of the search index for the page's language. If a template reads
`index`, the build publishes the index.

More: https://statac.dev/search/#placing-the-box

## `image`

`page.image("photo-1.jpg")` gives an image beside the page, with
`src`, `width`, `height`, `alt` and everything else an `<img>` for it
needs. The call also takes `sizes`, `full=true` and
`decorative=true`.

More: https://statac.dev/images/#pictures-in-templates-and-snippets

## `images`

`page.images(["photo-1.jpg", "photo-2.jpg"])` gives the images named,
in that order, each with the same values as `page.image`.
`page.images()` gives every JPEG, PNG and WebP beside a page that has a
folder of its own, in file name order. It leaves out animated images and
other kinds, such as GIFs and SVGs.

More: https://statac.dev/images/#pictures-in-templates-and-snippets

## `file`

`page.file("notes-1.pdf")` gives any file beside the page, with
`url`, `name`, `size` in bytes, `text`, `code` and `language`.

More: https://statac.dev/videos/#files-videos-and-recordings

## `video`

`page.video("video-1.mp4")` gives a video beside the page, with
`src` and `type`.

More: https://statac.dev/videos/#video

## `audio`

`page.audio("recording-1.m4a")` gives a recording beside the page,
with `src` and `type`.

More: https://statac.dev/videos/#video

## `youtube`

`page.youtube(url)`, where `url` is a YouTube video's URL or its id on
its own, gives the video, with `id`, `start`, `embed` and
`thumbnail`.

More: https://statac.dev/videos/#youtube

## `vimeo`

`page.vimeo(url)`, where `url` is a Vimeo video's URL or its id on its
own, gives the video, with `id`, `start`, `embed` and
`thumbnail`.

More: https://statac.dev/videos/#youtube

## `untitled`

`true` if the page's `untitled` says it is meant to have no title.

More: https://statac.dev/lists/#pages-without-titles

## `draft`

`true` if the page is a draft. Only `statac serve` builds drafts.

More: https://statac.dev/lists/#drafts-and-posts-waiting-for-their-day

## `layout`

The `layout` the page names, or the one cascaded to it, as its front
matter writes it. If
neither names one, `layout` has no value.

More: https://statac.dev/templates/#which

## `type`

The page's `type`, as the page writes it. If the page has no `type`, it
has no value.

More: https://statac.dev/templates/#which

## `permalink`

The page's `permalink`, as the page writes it: a URL or a pattern.
`page.url` is where the page is published.

More: https://statac.dev/urls/#permalink

## `redirect_from`

The old URLs the page's `redirect_from` lists, as the page writes them.

More: https://statac.dev/urls/#redirect-from

## `redirect_to`

The URL the page's `redirect_to` names, as the page writes it.

More: https://statac.dev/urls/#sending-readers-on

## `needs`

`url` if the page's front matter has `needs: url`.

More: https://statac.dev/feeds/#sitemaps-robotstxt-and-other-files

## `translation_key`

The page's `translation_key`, as the page writes it.

More: https://statac.dev/dates-and-languages/#translations

## `neighbours`

The page's `neighbours`, as the page writes it: `site`, `folder`,
`series` or `false`. If the page has none, it has no value.

More: https://statac.dev/lists/#previous-and-next

## `each`

On a page that makes pages, its `each`, as the page writes it:
`data.people` or `site.posts`.

More: https://statac.dev/data/#pages-made-from-data

## `order`

On a list page, its `order`, as the page writes it. `page.list.order` is
the order the list runs in.

More: https://statac.dev/lists/#order

## `numbered`

On a folder's page, its `numbered`, as the page writes it.

More: https://statac.dev/lists/#numbered-folders

## `posts_per_page`

On a list page, its `posts_per_page`, as the page writes it. If the page
doesn't write one, it has no value.

More: https://statac.dev/lists/#pages-of-a-list

## `index_page`

On a list page, its `index_page`, as the page writes it.

More: https://statac.dev/lists/#a-lists-page

## `feed`

On a list page, its `feed`, as the page writes it: `true`, `false`, or
the name or URL of a feed. `page.list.feed` is the URL of the list's
feed.

More: https://statac.dev/feeds/#which-lists-have-a-feed

## `id`

The post's `id` in a feed, as the page writes it. `feed_id(page)`
returns the id every post has, whether or not it writes one.

More: https://statac.dev/feeds/#entry-ids

## `search`

`false` if the page's `search` leaves it out of the site's search.

More: https://statac.dev/search/#leave-out

## `og_image`

The page's `og_image`, as the page writes it: the social image used in
place of a card.

More: https://statac.dev/social-cards/#a-picture-of-your-own

## `mentions`

`true` if the page's `mentions` asks for its webmentions to be shown.

More: https://statac.dev/webmentions/#webmentions-in-a-layout
