# Always false

A template uses `is defined` or `is not defined` to test whether a page
has a field, but no page has a field with that name. A page can have a
field whose name is one slip away (a character added, left out or changed,
or two neighbours swapped), so the name is probably misspelled. The test
then has the same result on every page, so whatever it wraps is either
never shown or always shown.

## What you'll see {#what-youll-see}

```text
warning  Always false: no page has `subtitel`
  Did you mean `page.subtitle`?
  More: https://statac.dev/e/always-false/
```

## Why Statac warns

A field that no page has yet is often one that pages written later will
have, so Statac builds the page with the test you wrote. But if the name
is one slip away from a field that pages do have, the test is more
likely a mistake than a plan, and nothing on the built page would show
the mistake.

## How to fix it

Use the field Statac suggests. If the name is right and no page has the
field yet, add it to a page, or quieten this warning with
`quiet: [always-false]` in `statac.yaml`.

## Example

In `templates/post.html`, on a site whose posts have a `subtitle`:

```html
{% if page.subtitel is defined %}
  <p>{{ page.subtitel }}</p>
{% endif %}
```

The field is called `subtitle`:

```html
{% if page.subtitle is defined %}
  <p>{{ page.subtitle }}</p>
{% endif %}
```
