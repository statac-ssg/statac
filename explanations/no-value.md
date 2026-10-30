# No value

A template shows or uses a value that doesn't exist. It may be a field
the page doesn't set, a setting that isn't in `statac.yaml`, a file or
value that isn't in `data/`, or a value that was written empty. The
message says which, and names the page or file.

## What you'll see {#what-youll-see}

```text
error  No value: `page.summary` isn't set on `content/post-1.md`
  templates/post.html, line 1
   1 │ <p>{{ page.summary }}</p>
     │       ^^^^^^^^^^^^
  While building: content/post-1.md
  Write `page.summary | default("")` to show nothing if it isn't set.
  More: https://statac.dev/e/no-value/
```

## Why Statac stops

On a published page, a missing value comes out as nothing, or as a word
such as `none`, and the page looks finished when it isn't. Statac stops
instead, so that you decide what a page without the value shows.

The same applies to a value that may be missing inside a list or map that
the template writes itself, such as `[page.summary, page.title]`. Nothing
would show that the value was missing, so Statac asks for a default as
soon as it reads the template.

## How to fix it

If every page should have the value, add it to the page the message
names. If some pages don't have it, say what they show instead:

- `{{ page.summary | default("") }}` shows nothing.
- `{% if page.summary %}...{% endif %}` leaves out the text around it as well.
- `{{ page.description | default(page.summary) | default("") }}` shows the
  description, or else the summary, or else nothing. If you leave out the
  last `default`, the message names `page.summary`, the last value in the
  chain, which was missing.

**A filter that groups or collects pages by a field.** `groupby("rank")`,
`unique(attribute="rank")` and `map(attribute="rank")` need the field on
every page they are given, and the message names the page that doesn't
have it. Leave out the pages without it first, as in
`site.pages | selectattr("rank", "defined") | groupby("rank")`, or give
`map` a `default=`. [When a page has no such
value](https://statac.dev/templates/#no-value) says what each filter
does with a page that doesn't have the field.

**A tag's or a series' `url`.** A tag or a series has a `url` and a
`page` only if it has a list page. If `tag.url` has no value because
`statac.yaml` doesn't ask for tag pages, add `tags.pages: true` to
publish a page for each tag, or `series.pages: true` for each series.
For a site made before 0.7.0, see [Upgrading to
0.7.0](https://statac.dev/upgrading/#other-stops).

**A page Statac makes.** A list page that Statac makes itself, such as a
tag's, has no `title`. For such a page, `{{ headline(page) }}` shows the
list's name. For any other page, it shows the page's `title` if it has
one.

**`page` in a snippet.** In a snippet, `page` is the page being built,
not a page that the call passed to the snippet. If a call passes a post
to a snippet as `post`, the snippet reads the post's fields as
`post.summary` and so on.

## Example

In `templates/post.html`:

```html
<p>{{ page.summary }}</p>
```

On pages without a `summary:` line, leave the paragraph out:

```html
{% if page.summary %}
<p>{{ page.summary }}</p>
{% endif %}
```
