# No names to check

A field is declared in `statac.yaml` as `one of data.` and a table, and
the table is written as a list. The items of a list have no names, so
there is nothing for a value of the field to be one of.

## What you'll see {#what-youll-see}

```text
error  No names to check: `data/people.yaml` is a list, so `fields.author` can't name one of its items
  Give each item a name with its fields beneath, or declare the field as `text`.
  More: https://statac.dev/e/no-names-to-check/
```

## Why Statac stops

The declaration says every value of the field is one of the table's names.
Statac won't choose a field of the items to stand for their names, and
passing every value would drop the check you asked for without saying so.

## How to fix it

Write the table with each item under a name of its own, or declare the
field as `text` if its values need no checking.

## Example

In `statac.yaml`:

```yaml
fields.author: one of data.people
```

In `data/people.yaml`:

```yaml
- name: Person 1
- name: Person 2
```

Written with a name for each item:

```yaml
person-1:
  name: Person 1
person-2:
  name: Person 2
```
