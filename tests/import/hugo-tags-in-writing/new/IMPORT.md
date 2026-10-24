This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-tags-in-writing/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 4 other files, copied as they were: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each is now an Atom feed at the same URL, written by `templates/feed.xml`, as `feed: index.xml` on each list page says, and `feed_ids_before` keeps each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

`content/posts/index.md` cascades `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

Hugo made a list of each folder at the top of the content folder, and these had no `_index.md` of their own, so Statac wrote one to keep what the list was given: `content/posts/index.md` for `content/posts/`.

Hugo summed up 1 page with its opening paragraphs, as many as made its `summaryLength` of words, and Statac takes the first paragraph alone: where a summary should run further, put `<!--more-->` on a line of its own where it should end.

5 places held a tag of the site's own that showed one of the site's values and nothing else, such as `{{< by >}}`: each holds the value itself now, the site's URL as a path starting with `/`.

4 figures are now calls to the starter's `figure` snippet. `class`, `width` and `height` were taken off 1 of them, which the snippet doesn't take: each shows at the size the starter gives an image.

1 figure had a `title`, which the starter's `figure` snippet doesn't take, so it opens the caption, in bold.

The settings written in `statac.yaml`:

```yaml
title: Tags in the writing
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
feed_ids_before: 2026-10-06
author: Name
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 5 old URLs there: 5 are published or redirected.

- Every page lands where the old site published it, with nothing added.

1 image beside a page is shown as resized copies Statac makes, and the file itself isn't published, so its old URL, `/posts/post-1/Photo 3.jpg`, is no longer published. An image's own file is published where a page links to it, or where its sidecar file, such as `photo-1.jpg.yaml`, holds `optimise: false`; `optimise_images: false` in `statac.yaml` leaves every image as it is: see https://statac.dev/pictures/.

Nothing is published at the old URL of the file given another name, and a file can't redirect:

- `/posts/post-1/Photo%25203.jpg`, for `content/posts/post-1/Photo%203.jpg`

## Left for you

### Figures written as HTML

Each of these figures shows an image kept elsewhere or one the starter's `figure` snippet can't show, such as a GIF, stands inside HTML, or asks for what the snippet has no equal for, such as a link, so Statac wrote the HTML Hugo made for it.

- `content/posts/post-1/index.md`, line 12: `{{< figure src="https://example.com/photo-3.jpg" alt="Photo 3" >}}`
- `content/posts/post-1/index.md`, line 14: `{{< figure src="photo-1.jpg" link="/posts/post-2/" class="wide" width="300" >}}`
- `content/posts/post-1/index.md`, line 87: `{{< figure src="moving.gif" alt="Moving" >}}`
- `content/posts/post-1/index.md`, line 90: `{{< figure src="photo-1.jpg" alt="Photo 1" >}}`

### Tags in the content left as written

Statac's Markdown runs none of Hugo's tags, and these have no equal in it, so each is left as written and shows on the page as it stands: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `content/posts/post-1/index.md`, line 18: `{{< youtube id="w7Ft2ymGmfc" autoplay="true" >}}`
- `content/posts/post-1/index.md`, line 38: `{{< param "missing" >}}`

### Links to pages that didn't come across

Each of these names a page Statac couldn't find in the old site, so it is left as written: link to the page's URL instead.

- `content/posts/post-1/index.md`, line 24: `{{< relref "post-9" >}}`

### Options on code taken off

Statac reads the rest of a code block's first line as its caption, so options such as `{linenos=true}`, and those handed to `highlight`, were taken off each of these: the code is shown without them.

- `content/posts/post-1/index.md`, line 26: `{{< highlight go "linenos=table,hl_lines=1" >}}`
- `content/posts/post-1/index.md`, line 64: ```` ```go {linenos=true} ````

### Tags inside code

Hugo ran these even inside code, and Statac shows code exactly as written, so each now shows as it stands: write by hand what it made, where that was meant.

- `content/posts/post-1/index.md`, line 65: `{{< youtube w7Ft2ymGmfc >}}`
- `content/posts/post-1/index.md`, line 69: `{{< param "author" >}}`

### HTML Hugo left out

Hugo's settings left HTML in Markdown out of the page, and Statac publishes it as it is, so each of these now shows: read each page that has one, and take out what wasn't meant to show.

- `content/posts/post-1/index.md`, line 75: `<a href="{{< note >}}">`
- `content/posts/post-1/index.md`, line 89: `<p class="centre">`

### Tags inside a URL

Each of these tags stands inside a link's URL, where Statac runs no snippet, so it is left as written and the link leads nowhere: write the URL the tag made in its place.

- `content/posts/post-1/index.md`, line 75: `{{< both >}}`
- `content/posts/post-1/index.md`, line 75: `{{< note >}}`
- `content/posts/post-1/index.md`, line 77: `{{< line-1 >}}`

### Tags now snippet calls

Each of these tags is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/gist.html`:
  - `content/posts/post-1/index.md`, line 40: `{{< gist user 0123456789abcdef >}}`
- `snippets/instagram.html`:
  - `content/posts/post-1/index.md`, line 42: `{{< instagram BWNjjyYFxVx >}}`
- `snippets/tweet.html`:
  - `content/posts/post-1/index.md`, line 44: `{{< tweet user="user" id="877500564405444608" >}}`
- `snippets/x.html`:
  - `content/posts/post-1/index.md`, line 46: `{{< x user="user" id="877500564405444608" >}}`
- `snippets/qr.html`:
  - `content/posts/post-1/index.md`, line 48: `{{< qr text="https://example.org" />}}`
- `snippets/old_note.html`:
  - `content/posts/post-1/index.md`, line 50: `{{< note text="A note of the site's own." >}}`
- `snippets/line_1.html`:
  - `content/posts/post-1/index.md`, line 52: `{{< line-1 >}}`
- `snippets/aside.html`:
  - `content/posts/post-1/index.md`, line 54: `{{% aside %}}`
  - `content/posts/post-1/index.md`, line 58: `{{< aside />}}`
- `snippets/unknown_1.html`:
  - `content/posts/post-1/index.md`, line 60: `{{< unknown-1 "value" >}}`
- `snippets/both.html`:
  - `content/posts/post-1/index.md`, line 75: `{{< both >}}`

### Files given another name

Statac can't publish a file under the name each of these had, so each has one it can: a `%` and the two figures after it became the character they spell, as a browser reads them, and any other character a URL or a disk won't take became `-`. Links in the content that named the file reach it under its new name. Its old URL is listed under URLs, and nothing is published there now: a file can't redirect.

- `content/posts/post-1/Photo%203.jpg`: `content/posts/post-1/Photo 3.jpg`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `layouts/`: the old site's layouts, which the starter's templates take the place of.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
