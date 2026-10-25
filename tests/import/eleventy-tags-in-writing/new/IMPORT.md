This site was imported from an Eleventy site, as well as Statac can. It is a best effort: the content, its settings and its URLs came across, the old site's templates and theme did not, and what is listed below needs your eye before you publish.

It was made from `tests/import/eleventy-tags-in-writing/old` by `statac import`, which changed nothing there. It has no templates, so each page is published as its content alone until you add templates of your own: see https://statac.dev/templates/.

## What came across

- 1 post, into `content/posts/`.

The old site's home page had no content of its own, so `content/index.md` has none: write the home page there.

Eleventy's settings are a program, and Statac runs none of it: it read `.eleventy.js` as text, for the plain forms it knows, and says here what it took and what it assumed where it found none.

Eleventy's settings hold no title, URL or language for the site, so the new site's `title` is made from the old folder's name, it has no `url` yet, and it is in English, `en`: put each right in `statac.yaml`.

Eleventy reads a date written without an offset as UTC, so `timezone` is `UTC`: write the site's own, as `Europe/London`, where its dates were meant there.

The settings name no folder in a `dir` Statac can read, so Statac took Eleventy's own: `input` as `.`, `output` as `_site`, `includes` as `_includes` and `data` as `_data`.

The settings copy nothing as it is with an `addPassthroughCopy` Statac can read, so Statac copied only the files beside a page that its content names.

The settings name no `markdownTemplateEngine`, so Statac read the template tags in the Markdown files as Liquid, as Eleventy does unless told otherwise.

The settings don't set `breaks: true`, so Statac read a line end inside a paragraph as a space, as Eleventy's Markdown does unless told otherwise.

The settings don't set `linkify: true`, so Eleventy left bare URLs as words, and Statac links those starting `www.`, `http://` or `https://`, as https://statac.dev/writing/#addresses-written-bare describes.

The settings give Markdown no library of their own, so Statac took Eleventy's own, which publishes HTML in Markdown as it is, as Statac does.

The settings name no plugin for attributes in braces, so Statac took `{.class}` and the like for words, as Eleventy did.

The settings name no `excerpt_separator`, so Statac changed no excerpt marker.

Statac put a backslash before the `@` in 1 place it would have read as a snippet call, as `\@name(`, so it shows as written: see https://statac.dev/writing/#snippet-calls.

The settings written in `statac.yaml`:

```yaml
title: Old
language: en
timezone: UTC
date_format: 31 December 2025
```

and a `feed_key` made fresh for the site: see https://statac.dev/feeds/#entry-ids.

## URLs

Statac worked out where the old site published each page, read the new site, and looked for the one old URL there: it is published there.

- Every page lands where the old site published it, with nothing added.

## Left for you

### Template tags left as written

Statac's Markdown runs no template tags and these have no equal in it, so each is left as written and shows on the page as it stands: write by hand what it made, or call a snippet, as https://statac.dev/snippets/ describes.

- `posts/post-1.md`, line 29: `{% aside "note" %}`
- `posts/post-1.md`, line 29: `{% endaside %}`
- `posts/post-1.md`, line 31: `{% if title %}`
- `posts/post-1.md`, line 31: `{{ title | upcase }}`
- `posts/post-1.md`, line 31: `{% endif %}`
- `posts/post-1.md`, line 33: `{% image page.url %}`

### Template tags inside code

Eleventy ran these even inside code, and Statac shows code exactly as written, so each now shows as it stands: write by hand what it made, where that was meant.

- `posts/post-1.md`, line 36: `{% image "in-code.jpg" %}`
- `posts/post-1.md`, line 40: `{% image "in-span.jpg" %}`
- `posts/post-1.md`, line 40: `{{ title }}`

### Includes now snippet calls

Each include is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/note.html`:
  - `posts/post-1.md`, line 15: `{% include "note.html", text: "Note 1" %}`
- `snippets/line_1.html`:
  - `posts/post-1.md`, line 17: `{% render 'line-1.liquid' %}`

### Tags now snippet calls

Each of these tags of the old site's own is now a call to a snippet of its name, and the snippet holds only a comment saying it wasn't converted: write the snippet, as https://statac.dev/snippets/ describes, or take the calls out.

- `snippets/image.html`:
  - `posts/post-1.md`, line 21: `{% image "photo-1.jpg", "A photo" %}`
  - `posts/post-1.md`, line 23: `{% image "photo-2.jpg" "Another" 2 %}`
- `snippets/aside.html`:
  - `posts/post-1.md`, line 25: `{% aside "tip" %}`

### Settings Statac didn't read

Eleventy's settings file is a program, and Statac ran none of it, so it doesn't know what each of these did: read each, and write what the new site needs of it, in its templates or snippets, as https://statac.dev/templates/ describes.

- `.eleventy.js`, line 2: `addShortcode("image")`
- `.eleventy.js`, line 3: `addPairedShortcode("aside")`

## Not converted

Statac left these behind, and the new site has nothing of them:

- `_includes/`: the old site's includes and layouts; each include the content calls has a snippet of its name standing for it.

## What to do next

1. Add your own templates to `templates/`.
2. Run `statac check` in this folder, and put right what it shows.
3. Run `statac serve`, and read the site in a browser beside the old one.
4. Once nothing here is left to do, delete `IMPORT.md`.
