# Names nothing

A field that `statac.yaml` declares as `one of data.` and a table, or
`one of folder.` and a folder, has a value that matches none of the names
there. A table's names are the names its items are written under. A
folder's names are the names of its pages as they appear in their
addresses: `book-1` for `book-1.md`, and `day-one` for `03-day-one.md` in
a numbered folder.

## What you'll see {#what-youll-see}

```text
error  Names nothing: `data/people.yaml` has no `persn-1`
  Did you mean `person-1`?
  More: https://statac.dev/e/names-nothing/
```

## Why Statac stops

You made the rule: you said the field refers to one of these. A value
that matches none of them is most often a typing mistake, and Statac
can't tell which record or page you meant. If a name is close, Statac
suggests it.

## How to fix it

Write one of the names that the table or the folder has, or add the
record or page that the value refers to. The value can refer to a draft,
or to a post waiting for its day.

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

The name in the table is `person-1`:

```yaml
---
title: Post 1
author: person-1
---
```
