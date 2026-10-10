Setext heading
==============

Entities &amp; &copy; &#169; and escapes \* \_ \[ and a tab	here.

Multibyte first: é ü 中文 @image('after-multibyte.jpg') then *émphasis*.

| A | B |
|---|---|
| one \| two \| three | ``code | in`` @image('cell.jpg') |

> Lazy continuation
of a quotation, with @image('lazy.jpg').

1. First
   - nested *one*
   - nested __two__

     with a second paragraph and @image('nested.jpg').
2. Second[^n]

Hard break with backslash\
and with spaces  
and on.

A [reference][r] link, a [collapsed][] one, a <https://example.org/a> autolink,
a www.example.org bare one, and a [titled](/t "Title") link.

Inline <span
class="two-lines">HTML over two lines</span> and @image('after-html.jpg').

``` rust
let a = "@image('fenced-with-space.jpg')";
```

~~~
@image('tilde-fence.jpg')
~~~

* * *

[^n]: A note with @image('in-note.jpg').

[r]: /r
[collapsed]: /c
