---
title: Post 1
date: 2026-01-01
subtitle: A subtitle of its own
---
A post with every tag in the writing.

@figure('photo-1.jpg', alt='Photo 1', caption='Beside the post.')

@figure('/images/photo-2.jpg', caption='**Photo 2** Kept in static.')

<figure><img src="https://example.com/photo-3.jpg" alt="Photo 3" /></figure>

<figure class="wide"><a href="/posts/post-2/"><img src="photo-1.jpg" width="300" /></a></figure>

@youtube('w7Ft2ymGmfc')

{{< youtube id="w7Ft2ymGmfc" autoplay="true" >}}

@vimeo('146022717')

- [Post 2](https://example.org/posts/post-2/)
- [Post 2, relatively](/posts/post-2/)
- [Post 9]({{< relref "post-9" >}})

```go
fmt.Println("one")
```

```python
print("two")
```

<details open>
<summary>More</summary>

Writing *inside* the details.
</details>

By A subtitle of its own, and by Name, and by {{< param "missing" >}}.

@gist(user='user', id='0123456789abcdef')

@instagram(id='BWNjjyYFxVx')

@tweet(user='user', id='877500564405444608')

@x(user='user', id='877500564405444608')

@qr(text='https://example.org')

@old_note(text='A note of the site\'s own.')

@line_1()

@aside()
Writing the site's own tag wraps.
@end

@aside()
@end

A tag no one knows: @unknown_1(value_1='value').

To show a tag as text: {{< figure src="photo-1.jpg" >}}.

```go
{{< youtube w7Ft2ymGmfc >}}
{{< vimeo 146022717 >}}
```

In a code span: `{{< param "author" >}}`.

Tags of the site's own that show one of its values: [Post 2](/posts/post-2/), [the top](/), Tags in the writing, written by Name, and a link defined below, [Post 2 again][two].

[two]: /posts/post-2/

A tag that shows more than one value stays a call: @both(). Inside an address no tag becomes a call: [a link]({{< both >}}/x/), <a href="{{< note >}}">another</a>, and [a third][three].

[three]: {{< line-1 >}}/y/

Figures the starter's snippet shows, each with something taken off or spelled another way:

@figure('photo-1.jpg', alt='Photo 1')

@figure('Photo%203.jpg', alt='Photo 3')

And those it doesn't: a picture that moves, and a figure inside HTML.

<figure><img src="moving.gif" alt="Moving" /></figure>

<p class="centre">
<figure><img src="photo-1.jpg" alt="Photo 1" /></figure>
</p>
