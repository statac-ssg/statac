---
title: Archive
---
{% for post in collections.all %}
- {{ post.data.title }}
{% endfor %}
