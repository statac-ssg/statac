# Not supported

A template or a page uses something Statac doesn't support.

`include`, `import` and `from` are other ways to place one template inside
another. In Statac, a piece of a page is a snippet, which is a file in
`snippets/`. A template places a snippet by calling it by its file name:
`{{ card(post) }}` places `snippets/card.html` and hands it `post`. The
message suggests the call to write instead of the line.

Some of Statac's own names, such as the one for the pictures beside a
page, aren't available in this version yet. The message says which one
you used.

## What you'll see {#what-youll-see}

```text
error  Not supported: `include`
  Call the snippet instead: `{{ card(post) }}`.
  More: https://statac.dev/e/not-supported/
```

## Why Statac stops

With only one way to place a snippet, every template reads the same way,
and a snippet call always shows what the snippet was handed. A name that
isn't available yet would otherwise publish nothing in its place.

## How to fix it

Replace the `include`, `import` or `from` line with a call to the snippet.
If you used a name that isn't available yet, remove it from the template
for now.

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
