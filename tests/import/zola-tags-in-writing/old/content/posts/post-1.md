+++
title = "Post 1"
date = 2026-01-01
+++
A post with every tag Zola runs in the writing.

<!-- more -->

{{ youtube(id="abcdefghijk") }}

{{ vimeo(id="123456789") }}

{{ youtube(id="abcdefghijk", autoplay=true) }}

{{ gist(url="https://gist.github.com/user/0123456789abcdef", file="a.rs") }}

{{ streamable(id="92ok4") }}

{{ note(text="A note of the site's own.") }}

A line with {{ line_1(n=3, on=false) }} inside it.

{% aside() %}
Writing inside a tag of the site's own, *read as Markdown*.
{% end %}

{% aside(kind='tip') %}Inside a line.{% end %}

{{ unknown_1(values=[1, 2]) }}

{{/* youtube(id="shown") */}}

A link to [post 2](@/posts/post-2.md), to [its heading](@/posts/post-2.md#a-heading), to [post 3](@/posts/post-3/index.md), to [the list](@/posts/_index.md), to [the draft](@/posts/post-4.md) and to [no page](@/posts/post-9.md).

[ref]: @/posts/post-2.md

A summary marker inside a line <!-- more --> is taken out.

```rust,linenos,hl_lines=1
{{ youtube(id="in-code") }}
```

A code span: `{{ youtube(id="in-span") }}` and `@/posts/post-2.md`.

Words that only look like a call, @name(1), are escaped, and the calls above and their `@end` are not.

Code whose lines are numbered or marked:

```rust,linenos,hl_lines=2 4-5,linenostart=1
println!("one");
println!("two");
println!("three");
println!("four");
println!("five");
```

```rust,hl_lines=3-1 9
println!("one");
println!("two");
println!("three");
```

```rust, linenos, linenostart=10, hide_lines=1, name=main.rs
println!("one");
println!("two");
```
