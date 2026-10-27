This site was imported from a Zola site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/zola-heading-ids/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 2 pages, into `content/` where they stood in the old site.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

Zola has no setting for a time zone, and reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

Zola's settings left straight quotes, `--` and `...` as typed, so `typography: false` keeps them so: take the line out to have Statac set them as print does.

Zola shows all of a list's pages on one page unless its list page gives `paginate_by`, and Statac does the same unless it gives `posts_per_page`.

Statac wrote the id Zola gave 7 headings after each in braces, as `## C++ & C# {#c-c}` in `content/page-1.md`, where Statac's own id would differ, so a link to one by its old id still lands.

Zola gave each footnote its label as its id, as `#1` for `[^1]`, or `#fn-1` under `bottom_footnotes`, and Statac numbers them, as `#note-1`, so a link from elsewhere to a footnote in 1 page, `content/page-2.md`, lands at the top of the page.

The settings written in `statac.yaml`:

```yaml
title: Heading ids
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

### Shortcodes now snippet calls

Each of these shortcodes is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/youtube.html`:
  - `content/page-1.md`, line 8: `{{ <youtube id="abcdefghijk" /> }}`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
