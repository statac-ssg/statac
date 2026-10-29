# Sorted last

A template sorts pages by a field with `sort(attribute=...)`, and some of
the pages it sorts don't have that field. Statac puts those pages after the
rest, in the order they came. The message says how many there were and
names the first.

## What you'll see {#what-youll-see}

```text
warning  Sorted last: `content/post-2.md` has no `rank`
  templates/page.html, line 1
   1 │ {% for post in site.pages | sort(attribute="rank") %}
     │                                            ^^^^^^
  Write `site.pages | selectattr("rank", "defined") | sort(attribute="rank")` to leave out the items without it.
  More: https://statac.dev/e/sorted-last/
```

## Why Statac warns

Pages without the field have no place in the order. Putting them at the
end keeps every page in the list, and keeps the rest sorted as the
template asks. Statac warns you, because a page missing from the order may be
missing a field you meant to set.

## How to fix it

If every page should have the field, add it to the pages the message
names. If pages without it don't belong in the list, leave them out
before sorting:

```html
{% for post in site.pages | selectattr("rank", "defined") | sort(attribute="rank") %}
```

If they belong at the end, say so in the template. This gives the same
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

Once `content/post-2.md` has `rank: 3`, every page has a place in the
order.
