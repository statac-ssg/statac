# No value

A template shows or uses a value that isn't there: a field the page
doesn't set, a setting `statac.yaml` doesn't have, a file or a value
`data/` doesn't have, or a value written empty. The message says which,
and on which page or in which file.

## What you'll see {#what-youll-see}

```text
error  No value: `page.summary` isn't set on `content/post-1.md`
  Write `page.summary | default("")` to show nothing where it isn't set.
  More: https://statac.dev/e/no-value/
```

## Why Statac stops

Published, a missing value comes out as nothing, or as a word such as
`none`, and the page looks finished when it isn't. Statac stops instead, so
you decide what a page without the value shows.

The same goes for a value that may be missing inside a list or map the
template writes itself, such as `[page.summary, page.title]`: nothing would
say so when it is, so Statac asks for a default when it reads the template.

## How to fix it

Where every page should have the value, add it to the page the message
names. Where some pages don't, say what they show instead:

- `{{ page.summary | default("") }}` shows nothing.
- `{% if page.summary %}...{% endif %}` leaves out what's around it too.
- `{{ page.description | default(page.summary) | default("") }}` shows the
  description, else the summary, else nothing. Where the last `default`
  is left out, the message names `page.summary`, the value that was
  missing at the end.

A filter that groups or gathers pages by a field, such as
`groupby("rank")`, `unique(attribute="rank")` or `map(attribute="rank")`,
needs it on every page it is handed: where to put a page without it would
be a guess. The message names the page that lacks it. Leave out the pages
without it first, as in
`site.pages | selectattr("rank", "defined") | groupby("rank")`, or give
`map` a `default=`. A filter that only tests a field, such as
`selectattr("rank", "eq", 1)`, passes over a page without it, as `if` does.
`sort(attribute="rank")` puts pages without it last and warns.

A page Statac makes for a list, such as a tag's page, has no `title`.
`{{ headline(page) }}` names it by its list's name, and any other page by
its `title` where it has one.

In a snippet, `page` is the page being built, not a page the call handed
it. A snippet handed a post as `post` reads its fields as `post.summary`.

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
