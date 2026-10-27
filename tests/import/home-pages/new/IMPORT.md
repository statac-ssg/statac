This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/home-pages/old` by `statac import`, which changed nothing there. It has no templates, so Statac publishes each page as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 5 posts, into `content/posts/`.

No page of the old site's came across as its home page, so `content/index.md` is empty: write the home page there.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The old site's home page showed its posts 2 to a page. `content/index.md` keeps its own list to one page with `posts_per_page: 0`, and the posts' list page shows them 2 to a page, as `posts_per_page` in `statac.yaml` says for every list: see https://statac.dev/lists/#pages-of-a-list.

The settings written in `statac.yaml`:

```yaml
title: Home pages
url: https://example.org
language: en
timezone: Europe/London
posts_per_page: 2
```

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 8 old URLs there: 8 are published or redirected.

- `content/posts/index.md` cascades `pages.permalink: /{year}/{month}/{day}/{slug}.html` to every page in `content/posts/`, which puts 5 of them at their old URLs.
- 2 pages at the top of `content/`, such as `content/page2.md`, redirect from the old paginated pages of the home page, each to the page of `content/posts/` with the same number, with `permalink` and `redirect_to`.

The old site's built output isn't in `_site/`, so no old URL was checked against the pages the old site built: build the old site and import again to check each one.

## Left for you

### Pages made by templates

Each of these pages was mostly the old site's templates at work, such as a home page or an archive listing the posts, so Statac didn't bring it across: Statac makes a list page where the site asks for one, with an `index.md` in a folder, `templates/home.html` for the home page, or `tags.pages: true`, as https://statac.dev/lists/ describes.

- `index.html`, line 4: `{% for post in paginator.posts %}`

## Not converted

Nothing: Statac read every file of the old site.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
