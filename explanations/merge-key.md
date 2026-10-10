# Merge key

A data file uses `<<`, YAML's merge key, to copy fields into an item from
elsewhere in the file.

## What you'll see {#what-youll-see}

```text
error  Merge key: Statac doesn't copy fields in with `<<`
  Write the fields out in full.
  More: https://statac.dev/e/merge-key/
```

## Why Statac stops

Statac reads a data file exactly as it stands, so what you see in the file
is what templates get. The merge key isn't part of YAML's current version,
and readers disagree about it: some copy the fields in, others read `<<` as
a field of its own. Statac won't guess which you meant.

## How to fix it

Write the fields out in full in each item that has them. To keep a field
called `<<`, put it in quotes: `"<<":`.

## Example

`data/people.yaml`:

```yaml
- &first
  name: Person 1
  role: Role 1
- <<: *first
  name: Person 2
```

Written out in full:

```yaml
- name: Person 1
  role: Role 1
- name: Person 2
  role: Role 1
```
