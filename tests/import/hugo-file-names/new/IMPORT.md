This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-file-names/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/. Its pages and feeds need `templates/index.xml`.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.
- 4 other files, copied as they were: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/index.xml` once you add it, and `feed_ids_before` and `feed_ids_from: hugo` keep each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. Copy the docs' RSS template as `templates/index.xml`: see https://statac.dev/feeds/#the-template. The feed Hugo made for each tag has no equal.

Hugo summed up 1 page with its opening paragraphs, as many as made its `summaryLength` of words, and Statac takes the first paragraph alone: where a summary should run further, put `<!--more-->` on a line of its own where it should end.

The settings written in `statac.yaml`:

```yaml
title: File names
url: https://example.org
language: en
timezone: UTC
feed_ids_before: 2026-10-06
feed_ids_from: hugo
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 4 old URLs there: 4 are published or redirected.

- Every page lands at the URL Statac worked out for it, with nothing added.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

Nothing is published at the old URLs of the files given another name, and a file can't redirect:

- `/posts/post-1/File%25201.txt`, for `content/posts/post-1/File%201.txt`
- `/posts/post-1/File%25202.txt`, for `content/posts/post-1/File%202.txt`
- `/posts/post-1/a%23b.txt`, for `content/posts/post-1/a#b.txt`
- `/Plan%25201.pdf`, for `static/Plan%201.pdf`

## Left for you

### Files given another name

Statac can't publish a file under the name each of these had, so each has one it can: a `%` and the two figures after it became the character they spell, as a browser reads them, and any other character a URL or a disk won't take became `-`. Links in the content that named the file reach it under its new name. Its old URL is listed under URLs, and nothing is published there now: a file can't redirect.

- `content/posts/post-1/File%201.txt`: `content/posts/post-1/File 1.txt`
- `content/posts/post-1/File%202.txt`: `content/posts/post-1/File 2.txt`
- `content/posts/post-1/a#b.txt`: `content/posts/post-1/a-b.txt`
- `static/Plan%201.pdf`: `static/Plan 1.pdf`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
