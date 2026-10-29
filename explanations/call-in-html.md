# Call in HTML

A page contains HTML written by hand, and inside that HTML is `@`
followed by the name of one of the site's snippets and an opening
bracket, such as `@book(`. Statac built it as a call to that snippet.

## What you'll see {#what-youll-see}

```text
note  Call in HTML: Statac builds `@book` as a snippet
  content/post-1.md, line 1
   1 │ <div class="aside">@book('Post 1')</div>
     │                    ^^^^^^^^^^^^^^^
  To show it as text, write `\@book`.
  More: https://statac.dev/e/call-in-html/
```

## Why Statac says so

Calls work inside a page's HTML the same way they do in its Markdown, so
you can place a snippet inside a `<div>` or a `<figure>` of your own. In
HTML, only the names of the site's snippets count as calls. Inside
`<style>`, `<script>`, `<pre>`, `<code>` and comments, nothing does.

You usually mean text in HTML to appear unchanged, so an email address
or a handle that happens to match a snippet's name would otherwise be
replaced without any warning. Statac tells you once for the whole site,
at the first call it builds inside HTML. Once you have understood it,
you can quieten it.

## How to fix it

If you meant the call, nothing is wrong.

If you want the text to appear unchanged, put a backslash before the
`@`. Statac then shows the call as text, without the backslash.

## Example

With `snippets/book.html` in the site, a page contains:

```html
<div class="aside">@book('Post 1')</div>
```

Statac builds the call as the snippet. To show it as written instead:

```html
<div class="aside">\@book('Post 1')</div>
```
