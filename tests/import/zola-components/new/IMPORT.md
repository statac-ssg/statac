This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-components/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page, unless its list page gives `paginate_by`, so `posts_per_page: 0` in `statac.yaml` does the same.

`content/posts/index.md` cascades `pages.layout: post`, so the pages beneath it, every one dated, are built with the starter's template for posts.

The settings written in `statac.yaml`:

```yaml
title: Components in the writing
url: https://example.org
language: en
timezone: UTC
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
typography: false
posts_per_page: 0
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 2 old URLs there: 2 are published or redirected.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Tags in the content left as written

Statac's Markdown runs none of Zola's tags, and these have no equal in it, so each is left as written and shows on the page as it stands: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `content/posts/post-1.md`, line 15: `{{ <ui.button label="Label 2" /> }}`
- `content/posts/post-1.md`, line 17: `{{ <figure_1 {...values} /> }}`
- `content/posts/post-1.md`, line 19: `{{ <figure_1 n={page.title} /> }}`
- `content/posts/post-1.md`, line 21: `{% <aside title> %}`
- `content/posts/post-1.md`, line 23: `{% </aside> %}`

### Tags inside code

Zola ran these even inside code, and Statac shows code exactly as written, so each now shows as it stands: write by hand what it made, where that was meant.

- `content/posts/post-1.md`, line 25: `{{ <youtube id="abcdefghijk" /> }}`

### Tags now snippet calls

Each of these tags is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/old_youtube.html`:
  - `content/posts/post-1.md`, line 5: `{{ <youtube id="abcdefghijk" /> }}`
- `snippets/old_quote.html`:
  - `content/posts/post-1.md`, line 9: `{% <quote author="Name 1"> %}`
- `snippets/figure_1.html`:
  - `content/posts/post-1.md`, line 13: `{{ <figure_1 src='photo-1.jpg' n={3} wide={true} label={"Label 1"} /> }}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `templates/`: the old site's templates, which the starter's templates take the place of.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
