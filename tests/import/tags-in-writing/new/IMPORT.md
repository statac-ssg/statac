This site was imported from a Jekyll site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/tags-in-writing/old` by `statac import`, which changed nothing there. The templates, snippets and stylesheet are Statac's starter, as `statac new` writes it: see https://statac.dev/new/.

## What came across

- 2 posts, into `content/posts/`.
- 1 page, into `content/` where it stood in the old site.

The old site's home page had no content of its own, so the starter's home page stays, listing the posts: `content/index.md`.

The settings name no language, so the site is in English, `en`: change `language` in `statac.yaml` if it isn't.

The settings name no time zone, so none is written, and Statac reads a date written without an offset as UTC, and says so: add `timezone` to `statac.yaml`, as `Europe/London`.

The settings written in `statac.yaml`:

```yaml
title: Tags in the writing
url: https://example.org
language: en
date_format: 31 December 2025
date_short: 31 Dec 2025
date_day: 31 Dec
date_stamp: 31 Dec 2025, 15:04
tags.pages: true
series.pages: true
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for each of the 3 old URLs there: 3 are published or redirected.

- `content/posts/index.md` cascades `pages.permalink: /{year}/{month}/{day}/{slug}.html` to every page in `content/posts/`, which puts 2 of them at their old URLs.
- 1 page carries a `permalink` of its own, where no pattern fits.

## Left for you

### Links to pages that didn't come across

Each of these names a page Statac couldn't find in the old site, or one Jekyll didn't publish, so it is left as written: link to the page's URL instead.

- `_posts/2026-01-01-post-1.md`, line 18: `{% post_url 2026-01-09-post-9 %}`

### Template tags left as written

Statac's Markdown runs no template tags and these have no equal in it, so each is left as written and shows on the page as it stands: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `_posts/2026-01-01-post-1.md`, line 30: `{% if page.title %}`
- `_posts/2026-01-01-post-1.md`, line 30: `{% endif %}`
- `_posts/2026-01-01-post-1.md`, line 53: `{{ "left" | upcase }}`
- `_posts/2026-01-01-post-1.md`, line 55: `{{ page.authors }}`
- `_posts/2026-01-01-post-1.md`, line 55: `{{ page.date }}`
- `_posts/2026-01-01-post-1.md`, line 55: `{{ page.url }}`

### Template tags inside code

Jekyll filled these in even inside code, and Statac shows code exactly as written, so each now shows as it stands: write by hand what it made, where that was meant.

- `_posts/2026-01-01-post-1.md`, line 34: `{% highlight ruby %}`
- `_posts/2026-01-01-post-1.md`, line 35: `{% post_url 2026-01-02-post-2 %}`
- `_posts/2026-01-01-post-1.md`, line 35: `{% link page-1.md %}`
- `_posts/2026-01-01-post-1.md`, line 36: `{{ site.baseurl }}`
- `_posts/2026-01-01-post-1.md`, line 36: `{{ site.url }}`
- `_posts/2026-01-01-post-1.md`, line 36: `{{ "/a" | relative_url }}`
- `_posts/2026-01-01-post-1.md`, line 36: `{{ "/a" | absolute_url }}`
- `_posts/2026-01-01-post-1.md`, line 37: `{% include note.html %}`
- `_posts/2026-01-01-post-1.md`, line 41: `{{ site.url }}`
- `_posts/2026-01-01-post-1.md`, line 41: `{% include note.html %}`
- `_posts/2026-01-01-post-1.md`, line 41: `{% post_url 2026-01-02-post-2 %}`

### Includes now snippet calls

Each include is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/old_note.html`:
  - `_posts/2026-01-01-post-1.md`, line 24: `{% include note.html text="one" %}`
  - `_posts/2026-01-01-post-1.md`, line 26: `{% include note.html kind="two" %}`
- `snippets/line_1.html`:
  - `_posts/2026-01-01-post-1.md`, line 28: `{% include line-1.html %}`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_includes/`: the old site's includes; each one the content calls has a snippet of its name standing for it.

## What to do next

1. Run `statac check` in this folder, and put right what it shows.
2. Run `statac serve`, and read the site in a browser beside the old one.
3. Once nothing here is left to do, delete `IMPORT.md`.
