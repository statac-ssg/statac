# Call in HTML

A page holds HTML written by hand, and inside it is `@` followed by the
name of one of the site's snippets and a bracket, such as `@book(`. Statac
built it as a call to that snippet.

## What you'll see {#what-youll-see}

```text
note  Call in HTML: `@book` is built as a snippet
  To show it as text, write `\@book`.
  More: https://statac.dev/e/call-in-html/
```

## Why Statac says so

Calls work inside a page's HTML as they do in its writing, so a snippet
can be placed inside a `<div>` or a `<figure>` of your own. Only the
names of the site's snippets are calls there; inside `<style>`,
`<script>`, `<pre>`, `<code>` and comments, nothing is.

Text in HTML is usually meant as written, so an address or a handle that
happens to share a snippet's name would otherwise be replaced without a
word. Statac says so once for the whole site, at the first call it builds
inside HTML, and once you have understood it, it can be quietened.

## How to fix it

If the call is meant, nothing is wrong.

If the text is meant to be shown as written, put a backslash before the
`@`, and the call is shown as text without the backslash.

## Example

With `snippets/book.html` in the site, a page holds:

```html
<div class="aside">@book('Post 1')</div>
```

The call is built as the snippet. To show it as written instead:

```html
<div class="aside">\@book('Post 1')</div>
```
