# Reserved name

One of your own values uses a name that already has a meaning in Statac.
In `statac.yaml`, names such as `posts`, `tags` and `today` are reserved,
because Statac uses them on `site`. In a field declaration, names such as
`title` and `date` are reserved because Statac uses them on `page`. In
`snippets/`, a file named `url.html` would have the same name as
`@url(...)`, which Statac provides for use in a page's writing.

## What you'll see {#what-youll-see}

```text
error  Reserved name: `site.posts` is Statac's own
  Give it a name of your own, such as `my_posts`.
  More: https://statac.dev/e/reserved-name/
```

## Why Statac stops

Templates would find Statac's value and yours under the same name, so one
of them would be lost. Statac can't know which one you wanted.

## How to fix it

Give your value a different name. The message suggests one.

## Example

In `statac.yaml`:

```yaml
title: Site 1
posts: 12
```

`site.posts` is Statac's list of posts. Give yours a different name:

```yaml
title: Site 1
my_posts: 12
```
