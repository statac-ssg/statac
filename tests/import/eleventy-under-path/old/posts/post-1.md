---
title: Post 1
date: 2026-01-01
---
Bare: {{ "/about/" | url }}

[About]({{ "/about/" | url }})

[Single quotes]({{ '/about/#part-1' | url }})

<a href="{{ '/about/' | url }}">In HTML</a>

[Not from the top]({{ "about/" | url }})

[A whole address]({{ "https://example.org/about/" | url }})

[A whole address in single quotes]({{ 'https://example.com/b/' | url }})

[A name, not quoted]({{ page.url | url }})

[Two filters]({{ "/about/" | url | upcase }})
