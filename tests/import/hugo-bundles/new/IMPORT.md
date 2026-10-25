This site was imported from a Hugo site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/hugo-bundles/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/. Its pages and feeds need `templates/feed.xml`.

## What came across

- 2 posts, into `content/posts/`.
- 2 pages, into `content/` where they stood in the old site.
- 2 other files, copied as they were: files published as they are into `static/` at the same path, so their URLs stay, and data files into `data/`.

The old site's home page is `content/index.md`.

Hugo's feeds were RSS, at `index.xml` beside the home page and each list page. Each stays at the same URL, as `feed: index.xml` on each list page says, built with `templates/feed.xml` once you add it, and `feed_ids_before` keeps each post's id the URL Hugo gave it, so feed readers see nothing new: see https://statac.dev/feeds/#entry-ids. The feed Hugo made for each tag has no equal.

Hugo summed up 1 page with its opening paragraphs, as many as made its `summaryLength` of words, and Statac takes the first paragraph alone: where a summary should run further, put `<!--more-->` on a line of its own where it should end.

The settings written in `statac.yaml`:

```yaml
title: Bundles
url: https://example.org
language: en
timezone: UTC
feed_ids_before: 2026-10-06
posts_per_page: 10
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 6 old URLs there: 6 are published or redirected.

- 1 page carries a `permalink` of its own, where no pattern fits.

1 image beside a page is shown as resized copies Statac makes, and the file itself isn't published, so its old URL, `/posts/post-1/photo-1.jpg`, is no longer published. An image's own file is published where a page links to it, or where its sidecar file, such as `photo-1.jpg.yaml`, holds `optimise: false`; `optimise_images: false` in `statac.yaml` leaves every image as it is: see https://statac.dev/pictures/.

## Left for you

### Tags now snippet calls

Each of these tags is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/figure.html`:
  - `content/posts/post-1/index.md`, line 7: `{{< figure src="photo-1.jpg" alt="Photo 1" caption="The picture beside the post." >}}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `content/posts/post-2/notes.md`: content kept among a page's files, which Hugo doesn't publish as a page.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
