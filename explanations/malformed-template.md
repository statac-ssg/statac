# Malformed template

Statac couldn't read a template or a snippet. The message says what it
found, in the template language's own terms, and points to the place to
fix. Typical causes are a tag that is never closed, an `{% endif %}` with
no `{% if %}`, or a `takes` line that Statac can't follow.

## What you'll see {#what-youll-see}

```text
error  Malformed template: unexpected end of input, expected end of block
  More: https://statac.dev/e/malformed-template/
```

## Why Statac stops

Statac can't build any page that uses a template it can't read, and
guessing what the template meant would publish something you didn't
write.

## How to fix it

Look at the place the caret points to. Common causes:

- A `{{` or `{%` with no `}}` or `%}` after it.
- A block opened and never closed, such as `{% if %}` with no
  `{% endif %}`, or closed twice.
- A snippet's first line, `{# takes photo, alt #}`, listing something the
  snippet never uses, or listing more than two names. This line lists, in
  order, the values a call may pass without a name. A call passes any
  others by name.

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
