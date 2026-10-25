# No home page

The site has pages, but nothing is published at its top, the URL a
reader reaches by its address alone. There is no `content/index.md`, and
no `templates/home.html` to build a page listing the posts.

## What you'll see {#what-youll-see}

```text
note  No home page: nothing is published at `/`
  Add `content/index.md`, or `templates/home.html` to list the posts there.
  More: https://statac.dev/e/no-home-page/
```

## Why Statac says so

Statac publishes only the pages you ask for. The home page is
`content/index.md`, or on a site in several languages, the `index.md` at
the top of each language's folder. Where there is none, Statac builds a
page listing the site's posts there only if the site has
`templates/home.html`. Without either, a reader who visits the site's
address finds nothing, and nothing else would tell you.

`statac check` says so once, naming every language's top with nothing
published. `statac build` and `statac serve` don't show this note or
count it.

## How to fix it

Write `content/index.md`, with whatever the home page should say.

Or add `templates/home.html`, which is handed the site's posts as
`page.posts`, to publish a page listing them.

If the site needs no home page, because something else is published at
its top, nothing is wrong. Quieten the note by name and it is no longer
shown.

## Example

A site whose posts are at the top of `content/`:

```
content/
  post-1.md
  post-2.md
templates/
  page.html
```

With `templates/home.html` added, a page listing the posts is published
at `/`:

```
{% for post in page.posts %}
<a href="{{ post.url }}">{{ post.title }}</a>
{% endfor %}
```
