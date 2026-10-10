# Colon in call

An argument in a snippet call was named with a colon, `caption: 'Post 1'`,
where a call names its arguments with `=`, `caption='Post 1'`.

## What you'll see {#what-youll-see}

```text
warning  Colon in call: `caption:` was read as `caption=`
  Write `caption=` instead.
  More: https://statac.dev/e/colon-in-call/
```

## Why Statac warns

The meaning is clear, so Statac uses the value as though you had written
`=` and tells you. A call is written the same way in your writing and in a
template, and a template names arguments with `=`.

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
