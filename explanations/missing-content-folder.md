# Missing content folder

A field is declared in `statac.yaml` as `one of folder.` and a path, and
`content/` has no folder at that path. `folder.books` is `content/books/`.
On a site in several languages it is the folder at that path in each
language's own folder.

## What you'll see {#what-youll-see}

```text
error  Missing folder: there is no `content/boks/` for `types.review.book` to name
  Did you mean `folder.books`?
  More: https://statac.dev/e/missing-content-folder/
```

## Why Statac stops

The declaration says every value of the field names one of the folder's
pages. With no folder there are no pages to hold a value to, and passing
every value would drop the check you asked for without saying so. When a
folder in `content/` is close, Statac offers it.

## How to fix it

Write the folder's path as it is under `content/`, with `/` between a
folder and one inside it, or add the folder and its pages.

## Example

In `statac.yaml`:

```yaml
fields.review_of: one of folder.boks
```

`content/` holds:

```text
content/books/book-1.md
```

The folder is `folder.books`:

```yaml
fields.review_of: one of folder.books
```
