---
title: Post 1
date: 2026-01-01
---
Bare: /blog/about/

[About](/blog/about/)

[Single quotes](/blog/about/#part-1)

<a href="/blog/about/">In HTML</a>

[Not from the top](about/)

[A whole address](https://example.org/about/)

[A whole address in single quotes](https://example.com/b/)

[A name, not quoted]({{ page.url | url }})

[Two filters]({{ "/about/" | url | upcase }})
