---
title: Post 1
date: 2026-01-01
---
Code whose lines the highlighting plugin marked, in Markdown.

```js/1,3
let a = 1;
let b = 2;
let c = 3;
let d = 4;
```

```js/0,2-3 example.js
let a = 1;
let b = 2;
let c = 3;
let d = 4;
```

```js/1/2
let a = 1;
let b = 2;
let c = 3;
```

```js/9
let a = 1;
```

A language holding a slash that isn't the plugin's:

```text/html
<p>Text</p>
```

And in Liquid:

{% highlight js 1,3 %}
let a = 1;
let b = 2;
let c = 3;
let d = 4;
{% endhighlight %}

{% highlight js linenos %}
let a = 1;
{% endhighlight %}

{% highlight js 0 2 %}
let a = 1;
let b = 2;
let c = 3;
{% endhighlight %}
