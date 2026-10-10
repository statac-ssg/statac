# Text never shown

A template that extends another fills a block the other doesn't have, so
what the block holds appears on no page.

A template that starts `{% extends "base" %}` is built as `base`, with each
of its blocks put in place of the block of the same name in `base`. A block
`base` doesn't have, nor any template `base` extends, has nowhere to go.

## What you'll see {#what-youll-see}

```text
error  Text never shown: `templates/base.html` has no block called `content`
  Did you mean `main`?
  More: https://statac.dev/e/text-never-shown/
```

## Why Statac stops

The block's text would be left out of every page without a word, and Statac
can't tell whether the block's name is a slip or the other template is
missing a block.

## How to fix it

Name the block as the template it extends does. When a block with a similar
name is there, Statac offers it. If the other template should show it, add
the block there.

## Example

In `templates/post.html`:

```html
{% extends "base" %}
{% block content %}{{ page.content }}{% endblock %}
```

If `templates/base.html` has `{% block main %}{% endblock %}`, write:

```html
{% extends "base" %}
{% block main %}{{ page.content }}{% endblock %}
```
