---
title: Post 1
date: 2026-01-01
edition: 7
authors:
  - a
  - b
---
A post with every tag in the writing.

<!--more-->

{{ kept as written }}

```ruby numbered=true
puts 1
```

- [Post 2](/2026/01/02/post-2.html)
- [Page 1](/page-1.html)
- [Post 9]({% post_url 2026-01-09-post-9 %})
- [Base](/page-1.html)
- [Site](/page-1.html)
- [Relative](/page-1.html)
- [Absolute](/page-1.html)

@note(text='one')

@note(kind='two')

@line_1()

A tag with no equal: Post 1, and {% if page.title %}another{% endif %}.

```
{{ code }}
{% highlight ruby %}
{% post_url 2026-01-02-post-2 %} {% link page-1.md %}
{{ site.baseurl }} {{ site.url }} {{ "/a" | relative_url }} {{ "/a" | absolute_url }}
{% include note.html %}
<!--cut-->
```

In code spans: `x`, `{{ site.url }}`, `{% include note.html %}`, `{% post_url 2026-01-02-post-2 %}`.

~~~
{{ kept in code, the tags round it on lines of their own }}
~~~

{{ kept, the tags round it on lines of their own }}

Text in quotes: written as it is, {{ in braces}}, and through a filter: {{ "left" | upcase }}.

The page's own: Post 1, 7, and what it hasn't as plain text: {{ page.authors }}, {{ page.date }}, {{ page.url }}.

    {{ in code}} and Post 1

Code whose lines are numbered or marked:

```ruby numbered=true mark=1,3
puts 1
puts 2
puts 3
```

```ruby mark=2
puts 1
puts 2
```
