# Page in snippet

A snippet is handed a page, but it also shows a field of `page`. In a
snippet, `page` is the page being built. So on a page that lists posts,
`page.title` is the list page's title every time, not the title of the
post that the call handed to the snippet.

## What you'll see {#what-youll-see}

```text
warning  Page in snippet: `card` shows `page.title`, the page being built
  Did you mean `post.title`?
  More: https://statac.dev/e/page-in-snippet/
```

## Why Statac warns

Sometimes a snippet does mean the page being built, for example to mark
the post that is being read. Statac can't tell which you meant, so it
builds the page as written and warns you.

## How to fix it

Read the field from the name the page is handed under, as Statac
suggests.
If the page being built is what you meant, quieten this warning with
`quiet: [page-in-snippet]` in `statac.yaml`.

## Example

In `snippets/card.html`, called as `{{ card(post) }}`:

```html
<h2>{{ page.title }}</h2>
```

Show the post's own title:

```html
<h2>{{ post.title }}</h2>
```
