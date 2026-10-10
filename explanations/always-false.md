# Always false

A template tests whether a page has a field with `is defined` or
`is not defined`, and no page has a field by that name. A field one slip
away is one a page can have, so the name is likely misspelt, and the test
gives the same answer on every page: what it guards is never shown, or
always shown.

## What you'll see {#what-youll-see}

```text
warning  Always false: no page has `subtitel`
  Did you mean `page.subtitle`?
  More: https://statac.dev/e/always-false/
```

## Why Statac warns

A field no page has yet is often one that later pages will have, so Statac
builds the page as written. When the name is one character away from a
field pages do have, the test is more likely a slip than a plan, and
nothing on the page would show it.

## How to fix it

Write the field Statac offers. If the name is right and no page has the
field yet, add it to a page, or quieten this warning with
`quiet: [always-false]` in `statac.yaml`.

## Example

In `templates/post.html`, where posts have a `subtitle`:

```html
{% if page.subtitel is defined %}
  <p>{{ page.subtitel }}</p>
{% endif %}
```

The field is `subtitle`:

```html
{% if page.subtitle is defined %}
  <p>{{ page.subtitle }}</p>
{% endif %}
```
