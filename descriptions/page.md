# A page in a template

What a template reads on a page: after `page.`, or after any other page
it holds, as `post.title` or `page.next.url`. A custom field is read the
same way, as `page.isbn`.

## `title`

The page's title: its `title`, or else the `# heading` its content opens
with. A page with neither has none, so to show the names of pages write
`headline(page)`.

More: https://statac.dev/templates/#what-a-template-sees

## `url`

Where the page is published, from the top of the host and with the
site's path: `/posts/post-1/`.

More: https://statac.dev/templates/#what-a-template-sees

## `slug`

The last part of the page's URL: `post-1` for `/posts/post-1/`.

More: https://statac.dev/templates/#what-a-template-sees

## `date`

The page's `date`, shown as the site writes dates, with its `year`,
`month`, `day` and `weekday`. A page without one has none, and nor does
a list's paginated page.

More: https://statac.dev/templates/#what-a-template-sees

## `updated`

The page's `updated` date, shown as the site writes dates. A page
without one has none, and nor does a list's paginated page.

More: https://statac.dev/templates/#what-a-template-sees

## `summary`

The page's `summary`, as written. A page without one has none, and
`page.opening` holds the words of its first paragraph instead.

More: https://statac.dev/templates/#what-a-template-sees

## `content`

The page's content, built into HTML, and empty on a list's paginated
page. Shown on another page, as in a list or a feed, its relative URLs
are written in full.

More: https://statac.dev/templates/#what-a-template-sees

## `excerpt`

The start of the page's content, built into HTML: everything before its
`<!--more-->` line, or else its first paragraph. A page with neither has
none.

More: https://statac.dev/templates/#what-a-template-sees

## `excerpt_marked`

`true` where the page's excerpt ends at a `<!--more-->` line, and
`false` where it is the first paragraph.

More: https://statac.dev/templates/#what-a-template-sees

## `opening`

The words of the page's first paragraph, as text with no markup. A page
with no paragraph has none.

More: https://statac.dev/templates/#what-a-template-sees

## `word_count`

How many words a reader reads in the page's content, not counting code
blocks and footnote markers. In Chinese and Japanese, each character
counts as a word.

More: https://statac.dev/templates/#what-a-template-sees

## `maths`

`true` where the page's content holds maths, so a layout can add the
maths stylesheet where it is needed.

More: https://statac.dev/templates/#what-a-template-sees

## `toc`

The page's headings, each with its `title`, `id`, `url`, `level` and the
headings beneath it as `headings`. Empty where it has none.

More: https://statac.dev/templates/#what-a-template-sees

## `language`

The page's language as a code, such as `en` or `fr`, for `<html lang>`.

More: https://statac.dev/dates-and-languages/#each-language-in-templates

## `direction`

Which way the page's language runs: `ltr`, left to right, or `rtl`,
right to left. For `<html dir>`.

More: https://statac.dev/dates-and-languages/#each-language-in-templates

## `translations`

The same page in the site's other languages, the main language first.
Empty where there are none.

More: https://statac.dev/dates-and-languages/#translations

## `tags`

The page's tags, each a list with its `name`, `url` and `posts`. `"Rust"
in page.tags` asks whether the page has a tag, however its capitals are
written.

More: https://statac.dev/templates/#what-a-template-sees

## `series`

The series the page is in, as a list with its `name`, `url` and `posts`.
A page in no series has none.

More: https://statac.dev/templates/#what-a-template-sees

## `folder`

The folder the page is filed in, as a list with its `name`, `url` and
`posts`.

More: https://statac.dev/templates/#what-a-template-sees

## `list`

On a list page, and on each of its paginated pages, the list it is the
page of. Other pages have none.

More: https://statac.dev/lists/#lists-in-templates

## `posts`

On a list page, the posts this page of the list shows, in the list's
order. Other pages have none.

More: https://statac.dev/lists/#lists-in-templates

## `number`

On a list page, which page of the list this is, counting from 1. Other
pages have none.

More: https://statac.dev/lists/#lists-in-templates

## `previous`

The page before this one in its list, as `neighbours` chooses, or on a
list page, the list's page before this one. The first has none.

More: https://statac.dev/lists/#previous-and-next

## `next`

The page after this one in its list, as `neighbours` chooses, or on a
list page, the list's next page. The last has none.

More: https://statac.dev/lists/#previous-and-next

## `post`

On a page made for each post by `each: site.posts`, the post it was made
for.

More: https://statac.dev/lists/#a-page-for-each-post

## `key`

On a page made by `each:` from an item in `data/`, the item's name.

More: https://statac.dev/data/#pages-made-from-data

## `source`

The file the page was read from, from the top of the site:
`content/posts/post-1.md`. A page Statac makes, such as a tag's, has
none.

More: https://statac.dev/templates/#what-a-template-sees

## `og_title`

What the page is called where it is shared: its `og_title`, or else what
`headline(page)` gives.

More: https://statac.dev/sharing-cards/#in-a-layout

## `card`

The social card drawn for the page, with its `url`, `width`, `height`
and `alt`. A page with no card has none.

More: https://statac.dev/sharing-cards/#in-a-layout

## `webmentions`

What other sites have said about the page: `comments`, the replies,
mentions and bookmarks, and `reactions`, the likes and reposts. Both are
empty where nothing has been said.

More: https://statac.dev/webmentions/#webmentions-in-a-layout

## `search_box`

Called as `page.search_box()`, what a search box needs: `id`, the id for
its element, `first`, whether it is the page's first, and `index`, the
URL of the search index for the page's language. Reading `index` has the
build publish the index.

More: https://statac.dev/search/#placing-the-box

## `image`

Called as `page.image("photo-1.jpg")`, an image beside the page, with
`src`, `width`, `height`, `alt` and the rest a tag showing it needs. It
also takes `sizes`, `full=true` and `decorative=true`.

More: https://statac.dev/templates/#files

## `images`

Called as `page.images(["photo-1.jpg", "photo-2.jpg"])`, the images
named, in that order, each as `page.image` gives it. Called as
`page.images()`, every image beside a page that has a folder of its own,
in file name order.

More: https://statac.dev/templates/#files

## `file`

Called as `page.file("notes-1.pdf")`, any file beside the page, with
`url`, `name`, `size` in bytes, `text`, `code` and `language`.

More: https://statac.dev/templates/#files

## `video`

Called as `page.video("video-1.mp4")`, a video beside the page, with
`src` and `type`.

More: https://statac.dev/templates/#files

## `audio`

Called as `page.audio("recording-1.m4a")`, a recording beside the page,
with `src` and `type`.

More: https://statac.dev/templates/#files

## `youtube`

Called as `page.youtube(url)` with a YouTube video's URL or its id
alone, the video, with `id`, `start`, `embed` and `thumbnail`.

More: https://statac.dev/templates/#files

## `vimeo`

Called as `page.vimeo(url)` with a Vimeo video's URL or its id alone,
the video, with `id`, `start`, `embed` and `thumbnail`.

More: https://statac.dev/templates/#files

## `untitled`

`true` where the page's `untitled` says it is meant to have no title.

More: https://statac.dev/front-matter/#page-fields

## `draft`

`true` where the page is a draft. Only `statac serve` builds drafts.

More: https://statac.dev/front-matter/#page-fields

## `layout`

The `layout` the page names, or is cascaded, as written. A page that
names none has none.

More: https://statac.dev/front-matter/#page-fields

## `type`

The page's `type`, as written. A page without one has none.

More: https://statac.dev/front-matter/#page-fields

## `permalink`

The page's `permalink`, as written: a URL or a pattern. `page.url` is
where the page is published.

More: https://statac.dev/front-matter/#address-fields

## `redirect_from`

The old URLs the page's `redirect_from` lists, as written.

More: https://statac.dev/front-matter/#address-fields

## `redirect_to`

The URL the page's `redirect_to` names, as written.

More: https://statac.dev/front-matter/#address-fields

## `needs`

`url` where the page's front matter has `needs: url`.

More: https://statac.dev/front-matter/#address-fields

## `translation_key`

The page's `translation_key`, as written.

More: https://statac.dev/front-matter/#address-fields

## `neighbours`

The page's `neighbours`, as written: `site`, `folder`, `series` or
`false`. A page without one has none.

More: https://statac.dev/front-matter/#list-fields

## `each`

On a page that makes pages, its `each`, as written: `data.people` or
`site.posts`.

More: https://statac.dev/front-matter/#list-page-fields

## `order`

On a list page, its `order`, as written. `page.list.order` is the order
the list runs in.

More: https://statac.dev/front-matter/#list-page-fields

## `numbered`

On a folder's page, its `numbered`, as written.

More: https://statac.dev/front-matter/#list-page-fields

## `posts_per_page`

On a list page, its `posts_per_page`, as written. Where the page doesn't
write one, it has none.

More: https://statac.dev/front-matter/#list-page-fields

## `index_page`

On a list page, its `index_page`, as written.

More: https://statac.dev/front-matter/#list-page-fields

## `feed`

On a list page, its `feed`, as written: `true`, `false`, or the name or
URL of a feed. `page.list.feed` is the URL of the list's feed.

More: https://statac.dev/front-matter/#list-page-fields

## `id`

The post's `id` in a feed, as written. `feed_id(page)` gives the id
every post has.

More: https://statac.dev/front-matter/#sharing-fields

## `search`

`false` where the page's `search` leaves it out of the site's search.

More: https://statac.dev/front-matter/#sharing-fields

## `og_image`

The page's `og_image`, as written: the social image used in place of a
card.

More: https://statac.dev/front-matter/#sharing-fields

## `mentions`

`true` where the page's `mentions` asks for its webmentions to be shown.

More: https://statac.dev/front-matter/#sharing-fields
