# Colon in call

An argument in a snippet call was named with a colon, as in
`caption: 'Post 1'`. A call names its arguments with `=`, as in
`caption='Post 1'`.

## What you'll see {#what-youll-see}

```text
warning  Colon in call: Statac read `caption:` as `caption=`
  Write `caption=` instead.
  More: https://statac.dev/e/colon-in-call/
```

## Why Statac warns

The meaning is clear, so Statac uses the value as though you had written
`=`, and warns you. A call is written the same way in your content as in a
template, and templates name arguments with `=`.

## How to fix it

Write `=` in place of the colon.

## Example

In `content/post-1.md`:

```markdown
@image('photo-1.jpg', caption: 'Post 1')
```

Write it as:

```markdown
@image('photo-1.jpg', caption='Post 1')
```
