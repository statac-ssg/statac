---
title: Post 1
date: 2026-01-01
---
A post with every tag the old site ran in the writing.

{% raw %}
{{ shown as written }}
{% endraw %}

{% highlight "js" %}
let a = 1;
{% endhighlight %}

{% include "note.html", text: "Note 1" %}

{% render 'line-1.liquid' %}

[A file]({{ "/files/file-1.pdf" | url }})

{% image "photo-1.jpg", "A photo" %}

A line with {% image "photo-2.jpg" "Another" 2 %} inside it.

{% aside "tip" %}
Writing inside a tag of the site's own, *read as Markdown*.
{% endaside %}

{% aside "note" %}Inside a line.{% endaside %}

{% if title %}{{ title | upcase }}{% endif %}

{% image page.url %}

```liquid
{% image "in-code.jpg" %}
{% raw %}{{ in raw }}{% endraw %}
```

A code span: `{% image "in-span.jpg" %}` and `{{ title }}`.

Words that only look like a call, @name(1), are escaped.
