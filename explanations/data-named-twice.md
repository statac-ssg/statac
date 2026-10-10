# Data named twice

Two things in `data/` would reach templates by one name: two files with the
same name and different endings, such as `people.yaml` and `people.json`,
or a file and a folder, such as `people.yaml` and `people/`.

## What you'll see {#what-youll-see}

```text
error  Data named twice: `data/people.json` and `data/people.yaml` are both `site.data.people`
  Rename one of them.
  More: https://statac.dev/e/data-named-twice/
```

## Why Statac stops

Templates reach `data/people.yaml` as `site.data.people`, and a folder
`data/people/` by the same name. With two at one name, Statac can't tell
which you meant, and taking either would quietly hide the other.

## How to fix it

Rename one of them, or remove the one you no longer use.

## Example

`data/` holds:

```text
data/people.yaml
data/people.json
```

Keep one, or give the other a name of its own:

```text
data/people.yaml
data/people-old.json
```
