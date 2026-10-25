---
title: Post 1
date: 2026-01-01
---
@youtube(id='abcdefghijk')

@vimeo(id='123456789')

@quote(author='Name 1')
Quoted 1.
@end

@figure_1(src='photo-1.jpg', n=3, wide=true, label='Label 1')

{{ <ui.button label="Label 2" /> }}

{{ <figure_1 {...values} /> }}

{{ <figure_1 n={page.title} /> }}

{% <aside title> %}
Aside 1.
{% </aside> %}

A call in code: `{{ <youtube id="abcdefghijk" /> }}`.
