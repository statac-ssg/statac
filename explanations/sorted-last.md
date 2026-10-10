# Sorted last

A template sorts pages by a field with `sort(attribute=...)`, and some of
the pages it sorts don't have that field. Statac puts those pages after the
rest, in the order they came. The message says how many there were and
names the first.

## What you'll see {#what-youll-see}

```text
warning  Sorted last: `content/post-2.md` has no `rank`
  Write `site.pages | selectattr("rank", "defined") | sort(attribute="rank")` to leave out the items without it.
  More: https://statac.dev/e/sorted-last/
```

## Why Statac warns

Pages without the field have no place in the order, and the end is the
place that keeps every page and the order of the rest as written. Statac
tells you, because a page left out of the order may be a field you meant
to set.

## How to fix it

Where every page should have the field, add it to the pages the message
names. Where pages without it don't belong in the list, leave them out
before sorting:

```html
{% for post in site.pages | selectattr("rank", "defined") | sort(attribute="rank") %}
```

Where they belong at the end, say so in the template, which gives the same
order without the warning:

```html
{% set ranked = site.pages | selectattr("rank", "defined") | sort(attribute="rank") %}
{% for post in ranked + (site.pages | rejectattr("rank", "defined")) %}
```

## Example

In `templates/page.html`:

```html
{% for post in site.pages | sort(attribute="rank") %}
<p>{{ post.title }}</p>
{% endfor %}
```

With `content/post-2.md` given `rank: 3`, every page has its place.
