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

Statac reads a data file exactly as it is written, so templates get what
you see in the file. The merge key isn't part of the current version of
YAML, and programs that read YAML disagree about it. Some copy the fields
in, and others read `<<` as an ordinary field. Statac doesn't guess which
you meant.

## How to fix it

Write the fields out in full in each item that needs them. If you want a
field called `<<`, put its name in quotes: `"<<":`.

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
