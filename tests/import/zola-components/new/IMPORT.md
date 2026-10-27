This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-components/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

The settings written in `statac.yaml`:

```yaml
title: Components in the writing
url: https://example.org
language: en
timezone: UTC
typography: false
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 2 old URLs there: 2 are published or redirected.

- Every page lands at the URL Statac worked out for it, with nothing added.

The old site's built output isn't in `public/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Shortcodes left as written

Statac's Markdown runs none of Zola's shortcodes, and these have no equal in it, so each appears on the page as written: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `content/posts/post-1.md`, line 15: `{{ <ui.button label="Label 2" /> }}`
- `content/posts/post-1.md`, line 17: `{{ <figure_1 {...values} /> }}`
- `content/posts/post-1.md`, line 19: `{{ <figure_1 n={page.title} /> }}`
- `content/posts/post-1.md`, line 21: `{% <aside title> %}`
- `content/posts/post-1.md`, line 23: `{% </aside> %}`

### Shortcodes inside code

Zola ran these even inside code, and Statac shows code exactly as written, so each now shows as written: write by hand what it made, where that was meant.

- `content/posts/post-1.md`, line 25: `{{ <youtube id="abcdefghijk" /> }}`

### Shortcodes now snippet calls

Each of these shortcodes is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/youtube.html`:
  - `content/posts/post-1.md`, line 5: `{{ <youtube id="abcdefghijk" /> }}`
- `snippets/vimeo.html`:
  - `content/posts/post-1.md`, line 7: `{{<vimeo id="123456789"/>}}`
- `snippets/quote.html`:
  - `content/posts/post-1.md`, line 9: `{% <quote author="Name 1"> %}`
- `snippets/figure_1.html`:
  - `content/posts/post-1.md`, line 13: `{{ <figure_1 src='photo-1.jpg' n={3} wide={true} label={"Label 1"} /> }}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `templates/`: the old site's templates.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
