---
title: Post 1
date: 2026-01-01
---
A post with every tag the old site ran in the writing.

{{ shown as written }}

```js
let a = 1;
```

@old_note(text='Note 1')

@line_1()

[A file](/files/file-1.pdf)

@image('photo-1.jpg', 'A photo')

A line with @image('photo-2.jpg', 'Another', 2) inside it.

@aside('tip')
Writing inside a tag of the site's own, *read as Markdown*.
@end

{% aside "note" %}Inside a line.{% endaside %}

{% if title %}{{ title | upcase }}{% endif %}

{% image page.url %}

```liquid
{% image "in-code.jpg" %}
{{ in raw }}
```

A code span: `{% image "in-span.jpg" %}` and `{{ title }}`.

Words that only look like a call, \@name(1), are escaped.
