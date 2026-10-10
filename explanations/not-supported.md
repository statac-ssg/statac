# Not supported

A template or a page uses something Statac doesn't do.

`include`, `import` and `from` are other ways of placing one template
inside another. In Statac a piece of a page is a snippet, a file in
`snippets/`, and a template places it by calling it by its file's name:
`{{ card(post) }}` places `snippets/card.html`, handed `post`. The message
offers the call to write in place of the line.

Some of Statac's own names, such as the pictures beside a page, aren't
available in this version yet. The message names the one used.

## What you'll see {#what-youll-see}

```text
error  Not supported: `include`
  Call the snippet instead: `{{ card(post) }}`.
  More: https://statac.dev/e/not-supported/
```

## Why Statac stops

One way of placing a snippet means every template reads the same way, and a
snippet always says what it was handed. A name whose work isn't there yet
would otherwise publish nothing in its place.

## How to fix it

Call the snippet in place of the `include`, `import` or `from` line. For a
name that isn't available yet, take it out of the template for now.

## Example

In `templates/folder.html`:

```html
{% for post in page.posts %}
{% include "card.html" %}
{% endfor %}
```

Call the snippet, handing it the post:

```html
{% for post in page.posts %}
{{ card(post) }}
{% endfor %}
```
