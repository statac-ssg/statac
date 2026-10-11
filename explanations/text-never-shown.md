# Text never shown

A template that extends another template fills a block that the other
one doesn't have, so the block's contents don't appear on any page.

A template that starts `{% extends "base" %}` is built as `base`, with each
of its blocks replacing the block of the same name in `base`. If neither
`base` nor any template `base` extends has a block of that name, the block
has nowhere to go.

## What you'll see {#what-youll-see}

```text
error  Text never shown: `templates/base.html` has no block called `content`
  Did you mean `main`?
  More: https://statac.dev/e/text-never-shown/
```

## Why Statac stops

The block's text would be left out of every page without a word. Statac
can't tell whether the block's name is a mistake or the other template is
missing a block.

## How to fix it

Give the block the name it has in the template it extends. If that
template has a block with a similar name, the message suggests it. If the
other template should show the block, add the block to that template.

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
