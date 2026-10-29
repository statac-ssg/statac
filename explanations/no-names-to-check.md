# No names to check

A field is declared in `statac.yaml` as `one of data.` followed by a
table, but the table is written as a list. Items in a list have no names,
so there are no names for the field's values to be one of.

## What you'll see {#what-youll-see}

```text
error  No names to check: `data/people.yaml` is a list, so `fields.author` can't name one of its items
  statac.yaml, line 1
   1 │ fields.author: one of data.people
     │                ^^^^^^^^^^^^^^^^^^
  Give each item a name with its fields beneath, or declare the field as `text`.
  More: https://statac.dev/e/no-names-to-check/
```

## Why Statac stops

The declaration says that every value of the field is one of the
table's names. Statac won't pick one of the items' fields to use as their
names. Accepting every value would drop the check you asked for without
telling you.

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

Rewritten with a name for each item:

```yaml
person-1:
  name: Person 1
person-2:
  name: Person 2
```
