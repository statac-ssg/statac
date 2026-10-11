# Missing content folder

A field in `statac.yaml` is declared as `one of folder.` followed by a
path, but `content/` has no folder at that path. For example,
`folder.books` means `content/books/`. On a site in several languages, it
means the folder at that path inside each language's own folder.

## What you'll see {#what-youll-see}

```text
error  Missing folder: there is no `content/boks/` for `types.review.book` to name
  Did you mean `folder.books`?
  More: https://statac.dev/e/missing-content-folder/
```

## Why Statac stops

The declaration says that every value of the field refers to one of the
pages in that folder. Without the folder, there are no pages to check a
value against, and accepting every value would quietly drop the check you
asked for. If a folder in `content/` has a similar name, Statac suggests
it.

## How to fix it

Write the folder's path as it is under `content/`, with `/` between a
folder and a folder inside it. Or create the folder and its pages.

## Example

In `statac.yaml`:

```yaml
fields.review_of: one of folder.boks
```

`content/` contains:

```text
content/books/book-1.md
```

The folder is `books`, so write `folder.books`:

```yaml
fields.review_of: one of folder.books
```
