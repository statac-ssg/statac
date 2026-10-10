# Missing data

A field is declared in `statac.yaml` as `one of data.` and a name, and
nothing in `data/` has that name. `data.people` is `data/people.yaml`, and
`data.people.staff` is `data/people/staff.yaml`.

## What you'll see {#what-youll-see}

```text
error  Missing data: there is no `data.poeple` for `fields.author` to name
  Did you mean `data.people`?
  More: https://statac.dev/e/missing-data/
```

## Why Statac stops

The declaration says every value of the field is one of the table's names.
With no table there are no names to hold a value to, and passing every
value would drop the check you asked for without saying so. When a name in
`data/` is close, Statac offers it.

## How to fix it

Write the table's name as its file is named, or add the file to `data/`.

## Example

In `statac.yaml`:

```yaml
fields.author: one of data.poeple
```

`data/` holds:

```text
data/people.yaml
```

The table is `data.people`:

```yaml
fields.author: one of data.people
```
