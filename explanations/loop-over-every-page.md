# Loop over every page

A template loops over a list of all the site's pages, such as
`site.pages` or `site.posts`, and so many pages use it that the loops go
through millions of pages between them. The message says which list,
and how many pages ran the loop.

## What you'll see {#what-youll-see}

```text
note  Loop over every page: 2,301 pages each go through `site.pages`
  templates/base.html, line 1
   1 │ {% for entry in site.pages if entry.menu %}
     │                 ^^^^^^^^^^
  Write `site.pages | selectattr("menu")` to choose the pages once for the site.
  More: https://statac.dev/e/loop-over-every-page/
```

## Why Statac says so

Statac builds nothing wrongly. A loop over every page, run on every
page, does work that grows with the square of the site: twice the pages
take four times as long. On a small site that is too little to notice,
and Statac says nothing. It says so once the pages running one loop go
through five million pages between them in one build, which a site of
about 2,000 pages reaches, because from there the loop is most of the
time a build takes.

A filter is different. Statac works out `site.pages | selectattr("menu")`
once for the whole site and gives every page the answer, however many
pages ask.

## How to fix it

Choose the pages with a filter in place of an `if`, so the loop runs only
over the pages it shows:

```html
{% for entry in site.pages | selectattr("menu") %}
```

Statac works out each of `selectattr`, `rejectattr`, `tagged`, `in`,
`where`, `sort` and `newest_first` once for the site, and the same goes
for several of them written one after another. To show a few pages, take them from the list, as
`site.posts[:5]` does; Statac then reads only those.

Some loops have to look at every page, such as one that finds a page's
place among all the posts. If the loop is on one page, such as a page
that lists everything, nothing needs to change. If it is on every page
and you would rather keep it, quieten the note in `statac.yaml`:

```yaml
quiet: [loop-over-every-page]
```

## Example

In `templates/base.html`, a menu of the pages that ask to be in it:

```html
{% for entry in site.pages if entry.menu %}
<li><a href="{{ entry.url }}">{{ entry.title }}</a></li>
{% endfor %}
```

Every page of the site is built with this layout, and each one looks at
every page to find the few for the menu. With a filter, Statac chooses
the pages once:

```html
{% for entry in site.pages | selectattr("menu") %}
<li><a href="{{ entry.url }}">{{ entry.title }}</a></li>
{% endfor %}
```
