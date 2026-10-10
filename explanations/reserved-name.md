# Reserved name

One of your own values takes a name Statac already gives a meaning to. In
`statac.yaml`, names such as `posts`, `tags` and `today` are Statac's on
`site`. In a field declaration, names such as `title` and `date` are
Statac's on `page`. In `snippets/`, `url.html` takes the name of
`@url(...)`, which is Statac's own in the writing.

## What you'll see {#what-youll-see}

```text
error  Reserved name: `site.posts` is Statac's own
  Give it a name of your own, such as `my_posts`.
  More: https://statac.dev/e/reserved-name/
```

## Why Statac stops

Templates read Statac's value and yours under the same name, so one of the
two would be lost. Statac can't know which you wanted.

## How to fix it

Give your value a name of your own. Statac offers one.

## Example

In `statac.yaml`:

```yaml
title: Site 1
posts: 12
```

`site.posts` is Statac's list of posts. Name yours another way:

```yaml
title: Site 1
my_posts: 12
```
