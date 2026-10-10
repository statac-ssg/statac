---
title: Post 1
date: 2026-01-01
---
<p>A post written in HTML, whose code has blank lines in it.</p>

<pre>
print "$a $b\n";

print "$b $a\n";
</pre>
<p>Code set in from the margin:</p>

<pre>
first

second
</pre>
<div>

<pre>
after a tag on its line

and a blank line
</pre></div>

<pre>
apart already

and left as it is
</pre>
<p>A script and a style:</p>

<script>
let a = 1;

let b = 2;
</script>
<style>
p { margin: 0 }

pre { margin: 0 }
</style>
<p>A text box:</p>

<textarea>
one

two
</textarea>
<!-- A comment naming <pre> is left as it is. -->
<p>A highlighted block straight after HTML:</p>

```ruby
puts 1

puts 2
```
<p>Code with no blank line in it is left where it stands:
  <textarea>one</textarea>
      <span>and the line after it stays HTML.</span>
</p>
<p>The end.</p>
