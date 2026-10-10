---
title: Tags shown
---
A page that shows a tag listing pages, inside `raw`, is writing and comes across:

{% for post in collections.posts %}
{{ post.url }}
{% endfor %}
