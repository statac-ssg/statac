# Missing data

A field is declared in `statac.yaml` as `one of data.` followed by a
name, but nothing in `data/` has that name. `data.people` means
`data/people.yaml`, and `data.people.staff` means
`data/people/staff.yaml`.

## What you'll see {#what-youll-see}

```text
error  Missing data: there is no `data.poeple` for `fields.author` to name
  statac.yaml, line 1
   1 │ fields.author: one of data.poeple
     │                ^^^^^^^^^^^^^^^^^^
  Did you mean `data.people`?
  More: https://statac.dev/e/missing-data/
```

## Why Statac stops

The declaration says every value of the field is one of the table's names.
Without the table there are no names to check a value against, and
letting every value pass would quietly drop the check you asked for. If
a name in `data/` is close, Statac suggests it.

## How to fix it

Write the table's name to match its file's name, or add the file to
`data/`.

## Example

In `statac.yaml`:

```yaml
fields.author: one of data.poeple
```

`data/` contains:

```text
data/people.yaml
```

The table is called `data.people`:

```yaml
fields.author: one of data.people
```
