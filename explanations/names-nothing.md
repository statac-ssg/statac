# Names nothing

A field that `statac.yaml` declares as `one of data.` and a table, or
`one of folder.` and a folder, holds a value that is none of the names
there. A table's names are the names its items are written under. A
folder's names are its pages' names as they stand in their addresses:
`book-1` for `book-1.md`, and `day-one` for `03-day-one.md` in a numbered
folder.

## What you'll see {#what-youll-see}

```text
error  Names nothing: `data/people.yaml` has no `persn-1`
  Did you mean `person-1`?
  More: https://statac.dev/e/names-nothing/
```

## Why Statac stops

The rule is yours: you said the field names one of these. A name that
matches none is most often a slip, and Statac can't tell which record or
page you meant. When a name is close, Statac offers it.

## How to fix it

Write one of the names the table or the folder has, or add the record or
the page the value names. A draft, and a post waiting for its day, can be
named.

## Example

In `statac.yaml`:

```yaml
fields.author: one of data.people
```

In `data/people.yaml`:

```yaml
person-1:
  name: Person 1
person-2:
  name: Person 2
```

In `content/post-1.md`:

```yaml
---
title: Post 1
author: persn-1
---
```

The table's name is `person-1`:

```yaml
---
title: Post 1
author: person-1
---
```
