# External link moved

A link points at a page on another site, and when `statac refresh`
last asked for it, the site answered that the page has moved for good to
another URL, which works.

## What you'll see {#what-youll-see}

```text
note  External link moved: `http://example.org/post-2/` redirects permanently to `https://example.org/posts/post-2/`
  content/post-1.md, line 1
   1 │ Read [Post 2](http://example.org/post-2/) next.
     │               ^^^^^^^^^^^^^^^^^^^^^^^^^^
  Link to `https://example.org/posts/post-2/` instead.
  More: https://statac.dev/e/external-link-moved/
```

## Why Statac says so

If `statac.yaml` has `check_external_links: true`, each `statac refresh`
asks the sites your pages link to about the links that are due, and
records each answer in `fetched/links`. A permanent redirect, 301 or
308, means the site has moved the page and expects links to follow. The
link still works today, but only while the old site keeps sending readers
on, and a reader who follows a link to `http://` may be sent over a
connection anyone on the way can read.

Statac says so only when the new URL changes something that matters:
`http://` becoming `https://`, another host, or another path. A redirect
that only adds or removes `www.` or a final slash isn't worth changing,
and Statac says nothing of it. A temporary redirect, 302, 303 or 307,
is the site's own business, such as a short link, and Statac says
nothing of it either.

## How to fix it

Change the link to the URL in the note.

Once you have understood the note, you can quieten it by name.

## Example

In `content/post-1.md`:

```markdown
Read [Post 2](http://example.org/post-2/) next.
```

`fetched/links`, as `statac refresh` wrote it:

```text
statac links 1
http://example.org/post-2/	moved https://example.org/posts/post-2/	2026-10-06
```

With the link changed:

```markdown
Read [Post 2](https://example.org/posts/post-2/) next.
```

To stop Statac showing the note, add this to `statac.yaml`:

```yaml
quiet: [external-link-moved]
```
