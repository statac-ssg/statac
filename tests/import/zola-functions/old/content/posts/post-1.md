+++
title = "Post 1"
date = 2026-01-01
+++
See [post 2]({{ get_url(path="@/posts/post-2.md") }}), its [heading]({{ get_url(path="@/posts/post-2.md#heading-1") }}) and [a file]({{ get_url(path="files/a.txt") }}).

Built {{ now() }}.

Not here: {{ get_url(path="@/posts/post-3.md") }}.
