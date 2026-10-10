# Malformed template

Statac couldn't read a template or a snippet. The message says what it
found, in the template's own words, and points at the place to fix: a tag
that is never closed, an `{% endif %}` with no `{% if %}`, or a `takes`
line it can't follow.

## What you'll see {#what-youll-see}

```text
error  Malformed template: unexpected end of input, expected end of block
  More: https://statac.dev/e/malformed-template/
```

## Why Statac stops

A template it can't read can't build any page that uses it, and guessing
at what it meant would publish something you didn't write.

## How to fix it

Look at the place the caret points to. Common causes:

- A `{{` or `{%` with no `}}` or `%}` after it.
- A block opened and never closed, such as `{% if %}` with no
  `{% endif %}`, or closed twice.
- A snippet's first line, `{# takes photo, alt #}`, naming something the
  snippet never reads, or more than two names. The line says which values
  a call may give without a name, in order; any others are given by name.

## Example

In `templates/post.html`:

```html
{% if page.date %}
<p>{{ page.date }}</p>
```

The `if` is never closed. Close it:

```html
{% if page.date %}
<p>{{ page.date }}</p>
{% endif %}
```
