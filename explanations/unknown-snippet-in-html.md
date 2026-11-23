# Unknown snippet in HTML

A page contains HTML written by hand, and inside that HTML is `@`
followed by a name and an opening bracket, such as `@bok(`. The site has
no snippet of that name, so Statac published the text unchanged.

## What you'll see {#what-youll-see}

```text
warning  Unknown snippet in HTML: `@bok` is published as text
  content/post-1.md, line 1
   1 │ <div class="aside">@bok('Post 1')</div>
     │                    ^^^^^^^^^^^^^^
  Did you mean `@book`? To show it as text, write `\@bok`.
  More: https://statac.dev/e/unknown-snippet-in-html/
```

## Why Statac warns

In HTML, only the names of the site's snippets count as calls, so text
such as `@someone(work)` stays as you wrote it and the page still builds.
The same text could be a call with a mistyped name, which readers would
then see on the page, so Statac says where it is.

Statac says nothing of an email address, a handle with no bracket
straight after it, a call with a backslash before its `@`, or anything
inside `<style>`, `<script>`, `<pre>`, `<code>` or a comment. In Markdown,
outside HTML, a call to a name the site has no snippet for stops the
build with [`unknown-word`](https://statac.dev/e/unknown-word/).

## How to fix it

If you meant a snippet, correct the name, or add the snippet's file to
`snippets/`.

If you meant the text, there are two ways to keep it:

- Put a backslash before the `@`. Statac publishes the text without the
  backslash and stops warning about that one place.
- Add `quiet: [unknown-snippet-in-html]` to `statac.yaml`. Statac stops
  showing this warning anywhere on the site and still counts it, so a
  mistyped call in HTML would no longer be shown to you.

## Example

With `snippets/book.html` in the site, a page contains:

```html
<div class="aside">@bok('Post 1')</div>
<p>Ask @someone(work).</p>
```

Correct the first name, and put a backslash before the second `@` to
keep it as text:

```html
<div class="aside">@book('Post 1')</div>
<p>Ask \@someone(work).</p>
```
