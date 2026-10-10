# Page in snippet

A snippet is handed a page, and shows a field of `page` too. In a snippet,
`page` is the page being built, so on a page listing posts, `page.title`
is the list's title every time, not the title of the post the call handed
it.

## What you'll see {#what-youll-see}

```text
warning  Page in snippet: `card` shows `page.title`, the page being built
  Did you mean `post.title`?
  More: https://statac.dev/e/page-in-snippet/
```

## Why Statac warns

Sometimes a snippet means the page being built, such as marking the post
that is being read. Statac can't tell which you meant, so it builds the page
as written and says so.

## How to fix it

Read the field from the name the call hands the page by, as Statac offers.
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
