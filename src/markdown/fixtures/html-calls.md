<div class="box">
  <p>@image('in-a-div.jpg')</p>
  <img src="@asset('in-an-attribute.png')">
  <!-- @image('in-a-comment.jpg') -->
  <code>@image('in-code.jpg')</code>
  <span>\@image('escaped-in-html.jpg')</span>
</div>

<style>
.x::after { content: "@image('in-style.jpg')"; }
</style>

<script>
const s = "@image('in-script.jpg')";
</script>

<pre>
@image('in-pre.jpg')

@image('in-pre-after-a-blank-line.jpg')
</pre>

Inline: <code>@image('inline-code.jpg')</code> then @image('after-code.jpg') and <!-- @image('inline-comment.jpg') --> then <span>@image('in-a-span.jpg')</span>.

<code>"quotes" -- left</code> but "these" -- not.
